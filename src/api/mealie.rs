//! Recettes, livres, catégories et favoris : lecture et écriture dans Mealie,
//! mises en forme pour les écrans.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::LazyLock;

use chrono::{Datelike, Local};
use futures::lock::Mutex;
use futures::{StreamExt, TryStreamExt, stream};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::error::ApiError;
use crate::api::http::{self, send, send_json, send_with, send_with_json};
use crate::api::models::{
    Cookbook, DraftIngredient, Ingredient, RecipeCategory, RecipeDetail, RecipeDraft,
    RecipeSummary, SeasonalRecipe,
};
use crate::api::{seasons, storage};
use crate::py;

/// Mealie n'a pas de notion de couleur par recette : ces teintes reprennent
/// la palette de la maquette pour les cartes sans photo, choisies par recette
/// via un hash stable (pas de random : la couleur ne doit pas changer à
/// chaque rechargement).
const PLACEHOLDER_COLORS: [&str; 8] = [
    "#E3B9A4", "#E7C7A6", "#C9D1B8", "#D9C9A0", "#B9C9B0", "#E4CDBF", "#DEC2A4", "#C7CDB0",
];

pub async fn search_recipes(query: &str, cookbook: &str) -> Result<Vec<Value>, ApiError> {
    let mut params = vec![("perPage", "9999")];
    if !query.is_empty() {
        params.push(("search", query));
    }
    if !cookbook.is_empty() {
        params.push(("cookbook", cookbook));
    }
    let data = send_json(http::get("/api/recipes").query(params)).await?;
    Ok(py::list_or_empty(&data, "items")?.to_vec())
}

pub async fn list_cookbooks() -> Result<Vec<Cookbook>, ApiError> {
    let data = send_json(http::get("/api/households/cookbooks")).await?;
    py::list(py::at(&data, "items")?)?
        .iter()
        .map(|item| {
            Ok(Cookbook {
                slug: py::string(py::at(item, "slug")?)?,
                name: py::string(py::at(item, "name")?)?,
                manual: parse_id_list_filter(py::get(item, "queryFilterString")?)?.is_some(),
            })
        })
        .collect()
}

pub async fn list_recipe_categories() -> Result<Vec<RecipeCategory>, ApiError> {
    let data = send_json(http::get("/api/organizers/categories")).await?;
    py::list(py::at(&data, "items")?)?
        .iter()
        .map(|item| {
            Ok(RecipeCategory {
                id: py::string(py::at(item, "id")?)?,
                name: py::string(py::at(item, "name")?)?,
            })
        })
        .collect()
}

/// Deux types de livre : manuel (l'utilisateur choisit les recettes une à
/// une, stocké comme un filtre `id IN [...]`) ou automatique par catégorie
/// (filtre `recipe_category.id IN [...]`, se met à jour tout seul quand la
/// catégorie d'une recette change). Un livre manuel démarre toujours avec un
/// filtre explicite plutôt que vide — voir `build_id_list_filter`.
pub async fn create_cookbook(name: &str, category_id: Option<&str>) -> Result<Cookbook, ApiError> {
    let query_filter = match category_id {
        Some(id) if !id.is_empty() => format!(r#"recipe_category.id IN ["{id}"]"#),
        _ => build_id_list_filter(&[]),
    };

    let data = send_with_json(
        http::post("/api/households/cookbooks"),
        &json!({"name": name, "queryFilterString": query_filter}),
    )
    .await?;
    Ok(Cookbook {
        slug: py::string(py::at(&data, "slug")?)?,
        name: py::string(py::at(&data, "name")?)?,
        manual: category_id.is_none(),
    })
}

static ID_LIST_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^id IN \[(.*)\]$").expect("regex valide"));
static QUOTED_ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""([0-9a-fA-F-]{36})""#).expect("regex valide"));

/// Mealie traite un `queryFilterString` vide comme "pas de filtre" = TOUTES les
/// recettes (bug constaté : un livre manuel fraîchement créé sans recette
/// affichait les 50 recettes de la base). Et `id IN []` (tableau vide) est
/// rejeté par Mealie avec un 422. Un livre manuel vide utilise donc un id
/// sentinelle qui ne correspond à aucune vraie recette.
const EMPTY_COOKBOOK_SENTINEL_ID: &str = "00000000-0000-0000-0000-000000000000";

/// Un livre géré par l'app utilise toujours un filtre `id IN [...]`
/// (liste explicite de recettes). Si le filtre est vide, la liste est vide.
/// Si c'est autre chose (catégorie, tag... comme "Répertoire des sauces",
/// déjà présent côté foyer, ou un livre créé par catégorie depuis l'app), on
/// ne sait pas le modifier sans risquer de le casser : on retourne `None` pour
/// signaler "non gérable manuellement depuis l'app".
fn parse_id_list_filter(query_filter: &Value) -> Result<Option<Vec<String>>, ApiError> {
    let text = if py::truthy(query_filter) {
        py::as_str(query_filter)?
    } else {
        ""
    };
    let stripped = py::strip(text);
    if stripped.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let Some(captures) = ID_LIST_PATTERN.captures(stripped) else {
        return Ok(None);
    };
    Ok(Some(
        QUOTED_ID_PATTERN
            .captures_iter(&captures[1])
            .map(|id| id[1].to_owned())
            .filter(|id| id != EMPTY_COOKBOOK_SENTINEL_ID)
            .collect(),
    ))
}

fn build_id_list_filter(ids: &[String]) -> String {
    if ids.is_empty() {
        return format!(r#"id IN ["{EMPTY_COOKBOOK_SENTINEL_ID}"]"#);
    }
    let quoted: Vec<String> = ids.iter().map(|id| format!(r#""{id}""#)).collect();
    format!("id IN [{}]", quoted.join(", "))
}

async fn update_cookbook_recipe_ids(
    cookbook_slug: &str,
    recipe_slug: &str,
    add: bool,
) -> Result<(), ApiError> {
    let mut cookbook = send_json(http::get(&format!(
        "/api/households/cookbooks/{cookbook_slug}"
    )))
    .await?;

    let Some(mut ids) = parse_id_list_filter(py::at(&cookbook, "queryFilterString")?)? else {
        return Err(ApiError::Invalid(
            "Ce livre utilise un filtre personnalisé (catégorie, tag…) non modifiable depuis l'application."
                .to_owned(),
        ));
    };

    let recipe = send_json(http::get(&format!("/api/recipes/{recipe_slug}"))).await?;
    let recipe_id = py::string(py::at(&recipe, "id")?)?;

    if add {
        if !ids.contains(&recipe_id) {
            ids.push(recipe_id);
        }
    } else {
        ids.retain(|id| *id != recipe_id);
    }

    let cookbook_id = py::fstr(py::at(&cookbook, "id")?);
    if let Value::Object(map) = &mut cookbook {
        map.insert(
            "queryFilterString".to_owned(),
            Value::String(build_id_list_filter(&ids)),
        );
    }
    // PUT (et DELETE) exigent l'id UUID réel du cookbook dans l'URL, pas
    // son slug — contrairement à GET, qui accepte les deux. Utiliser le
    // slug ici fait planter Mealie avec un 500 (vérifié empiriquement :
    // erreur Postgres "invalid input syntax for type uuid").
    send_with(
        http::put(&format!("/api/households/cookbooks/{cookbook_id}")),
        &cookbook,
    )
    .await?;
    Ok(())
}

pub async fn add_recipe_to_cookbook(
    cookbook_slug: &str,
    recipe_slug: &str,
) -> Result<(), ApiError> {
    update_cookbook_recipe_ids(cookbook_slug, recipe_slug, true).await
}

pub async fn remove_recipe_from_cookbook(
    cookbook_slug: &str,
    recipe_slug: &str,
) -> Result<(), ApiError> {
    update_cookbook_recipe_ids(cookbook_slug, recipe_slug, false).await
}

fn placeholder_color(slug: &str) -> &'static str {
    PLACEHOLDER_COLORS[crc32fast::hash(slug.as_bytes()) as usize % PLACEHOLDER_COLORS.len()]
}

/// Les images sont servies par Mealie derrière le même proxy que l'application.
fn media_url(recipe_id: &Value, variant: &str) -> String {
    format!(
        "/api/media/recipes/{}/images/{variant}",
        py::fstr(recipe_id)
    )
}

fn to_summary(item: &Value, favorite_ids: &HashSet<String>) -> Result<RecipeSummary, ApiError> {
    let recipe_category = py::at(item, "recipeCategory")?;
    let category = if py::truthy(recipe_category) {
        py::at(py::first(recipe_category)?, "name")?
    } else {
        &Value::Null
    };
    let tag = if py::truthy(category) {
        category
    } else {
        let tags = py::at(item, "tags")?;
        if py::truthy(tags) {
            py::at(py::first(tags)?, "name")?
        } else {
            &Value::Null
        }
    };

    let id = py::at(item, "id")?;
    let image_url = py::truthy(py::at(item, "image")?).then(|| media_url(id, "min-original.webp"));

    let slug = py::string(py::at(item, "slug")?)?;
    let total_time = py::at(item, "totalTime")?;
    let time = if py::truthy(total_time) {
        total_time
    } else {
        py::at(item, "prepTime")?
    };
    Ok(RecipeSummary {
        name: py::string(py::at(item, "name")?)?,
        time: py::opt_string(time)?,
        tag: py::opt_string(tag)?,
        image_url,
        color: placeholder_color(&slug).to_owned(),
        is_favorite: id.as_str().is_some_and(|id| favorite_ids.contains(id)),
        slug,
    })
}

pub async fn list_recipe_summaries(
    query: &str,
    cookbook: &str,
) -> Result<Vec<RecipeSummary>, ApiError> {
    let items = search_recipes(query, cookbook).await?;
    let favorite_ids = list_favorite_recipe_ids().await?;
    items
        .iter()
        .map(|item| to_summary(item, &favorite_ids))
        .collect()
}

thread_local! {
    static SELF_USER_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// L'API de favoris de Mealie exige l'UUID réel de l'utilisateur pour
/// POST/DELETE (contrairement à GET, qui accepte l'alias "self") — l'id est
/// stable pour ce token, donc mis en cache après le premier appel.
async fn get_self_user_id() -> Result<String, ApiError> {
    if let Some(id) = SELF_USER_ID.with(|id| id.borrow().clone()) {
        return Ok(id);
    }
    let data = send_json(http::get("/api/users/self")).await?;
    let id = py::fstr(py::at(&data, "id")?);
    SELF_USER_ID.with(|cached| *cached.borrow_mut() = Some(id.clone()));
    Ok(id)
}

pub async fn list_favorite_recipe_ids() -> Result<HashSet<String>, ApiError> {
    let user_id = get_self_user_id().await?;
    let data = send_json(http::get(&format!("/api/users/{user_id}/favorites"))).await?;
    let mut ids = HashSet::new();
    for rating in py::list(py::at(&data, "ratings")?)? {
        if py::truthy(py::get(rating, "isFavorite")?) {
            ids.insert(py::fstr(py::at(rating, "recipeId")?));
        }
    }
    Ok(ids)
}

pub async fn set_favorite(recipe_slug: &str, favorite: bool) -> Result<(), ApiError> {
    let user_id = get_self_user_id().await?;
    let path = format!("/api/users/{user_id}/favorites/{recipe_slug}");
    send(if favorite {
        http::post(&path)
    } else {
        http::delete(&path)
    })
    .await?;
    Ok(())
}

pub async fn fetch_recipe(slug: &str) -> Result<Value, ApiError> {
    send_json(http::get(&format!("/api/recipes/{slug}"))).await
}

fn to_ingredient(ing: &Value) -> Result<Ingredient, ApiError> {
    let food = py::get(ing, "food")?;
    let unit = py::get(ing, "unit")?;
    if py::truthy(food) {
        let unit_name = if py::truthy(unit) {
            py::get(unit, "name")?
        } else {
            &Value::Null
        };
        return Ok(Ingredient {
            quantity: py::opt_float(py::or(py::get(ing, "quantity")?, &Value::Null))?,
            unit: if py::truthy(unit_name) {
                Some(py::string(unit_name)?)
            } else {
                None
            },
            food: py::string(py::at(food, "name")?)?,
        });
    }
    // Ingrédient non structuré (ex: recette importée par URL sans parsing fiable) :
    // pas de food/unit à afficher séparément, on retombe sur le texte brut.
    let note = py::get(ing, "note")?;
    let text = if py::truthy(note) {
        note
    } else {
        py::get(ing, "display")?
    };
    Ok(Ingredient {
        quantity: None,
        unit: None,
        food: py::string_or_empty(text)?,
    })
}

/// Textes des étapes (`recipeInstructions`) qui ne sont pas vides une fois `strip()`.
fn non_blank_step_texts(item: &Value) -> Result<Vec<String>, ApiError> {
    let mut steps = Vec::new();
    for step in py::list(py::at(item, "recipeInstructions")?)? {
        let text = py::at(step, "text")?;
        if !py::strip(py::as_str(text)?).is_empty() {
            steps.push(py::string(text)?);
        }
    }
    Ok(steps)
}

fn to_detail(item: &Value) -> Result<RecipeDetail, ApiError> {
    let image_url = if py::truthy(py::at(item, "image")?) {
        Some(media_url(py::at(item, "id")?, "original.webp"))
    } else {
        None
    };

    // Dédoublonne en gardant l'ordre (catégorie et tag peuvent se recouper).
    let mut tags: Vec<String> = Vec::new();
    for organizer in py::list(py::at(item, "recipeCategory")?)?
        .iter()
        .chain(py::list(py::at(item, "tags")?)?)
    {
        let name = py::string(py::at(organizer, "name")?)?;
        if !tags.contains(&name) {
            tags.push(name);
        }
    }

    let ingredients = py::list(py::at(item, "recipeIngredient")?)?
        .iter()
        .map(to_ingredient)
        .collect::<Result<Vec<_>, _>>()?;
    let produce = seasons::current_month_produce(Local::now().date_naive());
    let in_season = ingredients
        .iter()
        .any(|ing| seasons::is_seasonal(&ing.food, produce));

    let total_time = py::at(item, "totalTime")?;
    let time = if py::truthy(total_time) {
        total_time
    } else {
        py::at(item, "prepTime")?
    };
    Ok(RecipeDetail {
        slug: py::string(py::at(item, "slug")?)?,
        name: py::string(py::at(item, "name")?)?,
        description: py::string_or_empty(py::at(item, "description")?)?,
        image_url,
        time: py::opt_string(time)?,
        servings: py::opt_float(py::or(py::at(item, "recipeServings")?, &Value::Null))?,
        tags,
        ingredients,
        steps: non_blank_step_texts(item)?,
        in_season,
    })
}

pub async fn get_recipe_detail(slug: &str) -> Result<RecipeDetail, ApiError> {
    to_detail(&fetch_recipe(slug).await?)
}

const SEASONAL_CACHE_KEY: &str = "ma-cuisine.recettes-de-saison";
const SEASONAL_CACHE_TTL_MS: f64 = 3600.0 * 1000.0;
const SEASONAL_CONCURRENCY: usize = 6;

/// Entrée du cache, gardée en mémoire et dans le `localStorage`.
#[derive(Serialize, Deserialize)]
struct SeasonalCacheEntry {
    month: u32,
    cached_at_ms: f64,
    recipes: Vec<SeasonalRecipe>,
}

/// Cache en mémoire : mois, date du calcul (ms) et recettes.
type SeasonalMemory = Option<(u32, f64, Rc<Vec<SeasonalRecipe>>)>;

thread_local! {
    static SEASONAL_CACHE: RefCell<SeasonalMemory> = const { RefCell::new(None) };
    static SEASONAL_LOCK: Rc<Mutex<()>> = Rc::new(Mutex::new(()));
}

fn fresh_seasonal_cache(month: u32) -> Option<Rc<Vec<SeasonalRecipe>>> {
    let is_fresh = |cached_month: u32, cached_at_ms: f64| {
        cached_month == month && storage::now_ms() - cached_at_ms < SEASONAL_CACHE_TTL_MS
    };
    if let Some(recipes) = SEASONAL_CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .filter(|(cached_month, cached_at_ms, _)| is_fresh(*cached_month, *cached_at_ms))
            .map(|(_, _, recipes)| Rc::clone(recipes))
    }) {
        return Some(recipes);
    }
    let entry: SeasonalCacheEntry =
        serde_json::from_str(&storage::get(SEASONAL_CACHE_KEY)?).ok()?;
    if !is_fresh(entry.month, entry.cached_at_ms) {
        return None;
    }
    let recipes = Rc::new(entry.recipes);
    SEASONAL_CACHE.with(|cache| {
        *cache.borrow_mut() = Some((entry.month, entry.cached_at_ms, Rc::clone(&recipes)))
    });
    Some(recipes)
}

fn store_seasonal_cache(month: u32, recipes: &Rc<Vec<SeasonalRecipe>>) {
    let cached_at_ms = storage::now_ms();
    SEASONAL_CACHE
        .with(|cache| *cache.borrow_mut() = Some((month, cached_at_ms, Rc::clone(recipes))));
    let entry = SeasonalCacheEntry {
        month,
        cached_at_ms,
        recipes: recipes.as_ref().clone(),
    };
    // Le cache mémoire suffit si le stockage du navigateur est indisponible.
    if let Ok(text) = serde_json::to_string(&entry) {
        let _ = storage::set(SEASONAL_CACHE_KEY, &text);
    }
}

/// Un appel détail par recette (la liste Mealie ne renvoie pas les
/// ingrédients). Vérifié empiriquement : tirer les appels tous en même temps
/// (asyncio.gather sans limite) fait planter Mealie sur ~36 recettes — une
/// requête finit par expirer et fait échouer tout le lot. D'où la borne
/// `SEASONAL_CONCURRENCY` sur le nombre de requêtes envoyées en parallèle.
async fn compute_recipes_with_season() -> Result<Vec<SeasonalRecipe>, ApiError> {
    let summaries = list_recipe_summaries("", "").await?;
    stream::iter(summaries)
        .map(|summary| async move {
            let detail = get_recipe_detail(&summary.slug).await?;
            Ok(SeasonalRecipe {
                slug: summary.slug,
                name: summary.name,
                in_season: detail.in_season,
                category: summary.tag,
            })
        })
        .buffered(SEASONAL_CONCURRENCY)
        .try_collect()
        .await
}

/// Mis en cache (1h, par mois) : même borné en concurrence, calculer ceci
/// pour chaque recette reste coûteux (un appel détail par recette, la liste
/// Mealie ne renvoie pas les ingrédients) pour être refait à chaque affichage
/// d'un écran qui en a besoin (Liste, Planificateur). Le cache vit dans le
/// navigateur : il survit aux rechargements de la page, mais est propre à
/// chaque appareil.
pub async fn list_recipes_with_season() -> Result<Rc<Vec<SeasonalRecipe>>, ApiError> {
    let month = Local::now().month();
    if let Some(recipes) = fresh_seasonal_cache(month) {
        return Ok(recipes);
    }

    let lock = SEASONAL_LOCK.with(Rc::clone);
    let _guard = lock.lock().await;
    // Un autre appel a pu remplir le cache pendant qu'on attendait le verrou.
    if let Some(recipes) = fresh_seasonal_cache(month) {
        return Ok(recipes);
    }
    let recipes = Rc::new(compute_recipes_with_season().await?);
    store_seasonal_cache(month, &recipes);
    Ok(recipes)
}

/// Les endpoints /api/foods et /api/units n'acceptent pas de doublon de nom
/// (contrainte unique côté Mealie) : on cherche d'abord une correspondance
/// exacte avant de créer. On ne peut pas non plus se contenter de laisser
/// Mealie créer le food/unit à la volée depuis recipeIngredient : passer un
/// nom sans id existant y fait planter l'API (ValueError) sur cette version.
async fn get_or_create_id(endpoint: &str, name: &str) -> Result<Value, ApiError> {
    let data = send_json(http::get(endpoint).query([("search", name)])).await?;
    let wanted = py::strip(name).to_lowercase();
    for item in py::list(py::at(&data, "items")?)? {
        if py::strip(py::as_str(py::at(item, "name")?)?).to_lowercase() == wanted {
            return Ok(py::at(item, "id")?.clone());
        }
    }

    let created = send_with_json(http::post(endpoint), &json!({"name": name})).await?;
    Ok(py::at(&created, "id")?.clone())
}

async fn apply_draft(mut recipe: Value, draft: &RecipeDraft) -> Result<Value, ApiError> {
    let mut recipe_ingredients = Vec::with_capacity(draft.ingredients.len());
    for ingredient in &draft.ingredients {
        let food_id = get_or_create_id("/api/foods", &ingredient.aliment).await?;
        let unit = match ingredient.unite.as_deref() {
            Some(unite) if !unite.is_empty() => {
                let unit_id = get_or_create_id("/api/units", unite).await?;
                json!({"id": unit_id, "name": unite})
            }
            _ => Value::Null,
        };
        recipe_ingredients.push(json!({
            "quantity": ingredient.quantite,
            "unit": unit,
            "food": {"id": food_id, "name": ingredient.aliment},
            "note": "",
        }));
    }
    let Value::Object(map) = &mut recipe else {
        return Err(ApiError::Data(
            "object does not support item assignment".to_owned(),
        ));
    };
    map.insert("name".to_owned(), json!(draft.nom));
    map.insert(
        "recipeIngredient".to_owned(),
        Value::Array(recipe_ingredients),
    );
    map.insert(
        "recipeInstructions".to_owned(),
        draft
            .etapes
            .iter()
            .map(|etape| json!({"text": etape}))
            .collect(),
    );
    Ok(recipe)
}

/// Crée la recette dans Mealie et retourne son slug.
///
/// Mealie crée d'abord la recette avec son nom, puis on la met à jour avec les
/// ingrédients et étapes. L'API PUT/PATCH de Mealie n'applique pas de fusion
/// partielle : le corps envoyé remplace l'objet recette en entier (les champs
/// absents repartent à leur valeur par défaut), donc on récupère d'abord la
/// recette telle que créée par Mealie et on ne modifie que ce qui nous
/// intéresse avant de la renvoyer complète.
pub async fn create_recipe(draft: &RecipeDraft) -> Result<String, ApiError> {
    let created = send_with_json(http::post("/api/recipes"), &json!({"name": draft.nom})).await?;
    let slug = py::string(&created)?;

    let recipe = fetch_recipe(&slug).await?;
    let recipe = apply_draft(recipe, draft).await?;

    send_with(http::put(&format!("/api/recipes/{slug}")), &recipe).await?;
    Ok(slug)
}

fn to_draft(item: &Value) -> Result<RecipeDraft, ApiError> {
    let mut ingredients = Vec::new();
    for ing in py::list(py::at(item, "recipeIngredient")?)? {
        let food = py::get(ing, "food")?;
        let unit = py::get(ing, "unit")?;
        let aliment = if py::truthy(food) {
            py::string(py::at(food, "name")?)?
        } else {
            let note = py::get(ing, "note")?;
            py::string_or_empty(if py::truthy(note) {
                note
            } else {
                py::get(ing, "display")?
            })?
        };
        ingredients.push(DraftIngredient {
            quantite: py::opt_float(py::get(ing, "quantity")?)?,
            unite: if py::truthy(unit) {
                py::opt_string(py::at(unit, "name")?)?
            } else {
                None
            },
            aliment,
        });
    }

    Ok(RecipeDraft {
        nom: py::string(py::at(item, "name")?)?,
        ingredients,
        etapes: non_blank_step_texts(item)?,
    })
}

/// Forme éditable (quantite/unite/aliment séparés) pour le formulaire de
/// modification — à la différence de `RecipeDetail`, qui combine quantité et
/// unité en une seule chaîne d'affichage impossible à re-découper de façon
/// fiable (ex: "1,5 kg" -> 1.5 + "kg" n'est pas trivial à inverser).
pub async fn get_recipe_draft(slug: &str) -> Result<RecipeDraft, ApiError> {
    to_draft(&fetch_recipe(slug).await?)
}

/// Met à jour une recette existante et retourne son slug à jour — Mealie
/// re-génère le slug à partir du nom quand celui-ci change (vérifié
/// empiriquement), donc le slug de retour peut différer de celui d'entrée.
pub async fn update_recipe(slug: &str, draft: &RecipeDraft) -> Result<String, ApiError> {
    let recipe = fetch_recipe(slug).await?;
    let recipe = apply_draft(recipe, draft).await?;

    let updated = send_with_json(http::put(&format!("/api/recipes/{slug}")), &recipe).await?;
    py::string(py::at(&updated, "slug")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_color_matches_zlib_crc32() {
        // zlib.crc32(slug.encode()) % 8 sous CPython.
        assert_eq!(placeholder_color("salade-nicoise"), PLACEHOLDER_COLORS[2]);
        assert_eq!(placeholder_color("poulet-roti"), PLACEHOLDER_COLORS[0]);
        assert_eq!(placeholder_color(""), PLACEHOLDER_COLORS[0]);
        assert_eq!(placeholder_color("tarte-aux-pommes"), PLACEHOLDER_COLORS[3]);
        assert_eq!(placeholder_color("crêpes"), PLACEHOLDER_COLORS[2]);
    }

    #[test]
    fn id_list_filters_round_trip() {
        let a = "11111111-1111-1111-1111-111111111111".to_owned();
        let b = "22222222-2222-2222-2222-222222222222".to_owned();
        let filter = build_id_list_filter(&[a.clone(), b.clone()]);
        assert_eq!(filter, format!(r#"id IN ["{a}", "{b}"]"#));
        assert_eq!(
            parse_id_list_filter(&json!(filter)).unwrap(),
            Some(vec![a, b])
        );
    }

    #[test]
    fn empty_cookbooks_use_the_sentinel() {
        let filter = build_id_list_filter(&[]);
        assert_eq!(filter, r#"id IN ["00000000-0000-0000-0000-000000000000"]"#);
        assert_eq!(parse_id_list_filter(&json!(filter)).unwrap(), Some(vec![]));
        assert_eq!(parse_id_list_filter(&json!("  ")).unwrap(), Some(vec![]));
        assert_eq!(parse_id_list_filter(&Value::Null).unwrap(), Some(vec![]));
    }

    #[test]
    fn other_filters_are_not_manageable() {
        let filter = json!(r#"recipe_category.id IN ["11111111-1111-1111-1111-111111111111"]"#);
        assert_eq!(parse_id_list_filter(&filter).unwrap(), None);
    }

    #[test]
    fn summary_falls_back_from_category_to_tag() {
        let item = json!({
            "id": "r1", "slug": "soupe", "name": "Soupe", "image": "x",
            "recipeCategory": [{"name": ""}], "tags": [{"name": "Hiver"}],
            "totalTime": "", "prepTime": null,
        });
        let summary = to_summary(&item, &HashSet::from(["r1".to_owned()])).unwrap();
        assert_eq!(summary.tag.as_deref(), Some("Hiver"));
        assert_eq!(summary.time, None);
        assert_eq!(
            summary.image_url.as_deref(),
            Some("/api/media/recipes/r1/images/min-original.webp")
        );
        assert!(summary.is_favorite);
    }

    #[test]
    fn ingredients_fall_back_to_their_note() {
        let structured = json!({"food": {"name": "tomate"}, "unit": {"name": ""}, "quantity": 0});
        let ingredient = to_ingredient(&structured).unwrap();
        assert_eq!(
            (
                ingredient.quantity,
                ingredient.unit,
                ingredient.food.as_str()
            ),
            (None, None, "tomate")
        );

        let raw = json!({"food": null, "note": "", "display": "2 pincées de sel"});
        assert_eq!(to_ingredient(&raw).unwrap().food, "2 pincées de sel");
    }

    #[test]
    fn detail_and_draft_follow_the_python_mapping() {
        let item = json!({
            "id": "r1", "slug": "soupe", "name": "Soupe", "description": null, "image": null,
            "recipeCategory": [{"name": "Soupe"}], "tags": [{"name": "Soupe"}, {"name": "Hiver"}],
            "totalTime": null, "prepTime": "20 min", "recipeServings": 4,
            "recipeIngredient": [
                {"quantity": 2, "unit": {"name": "kg"}, "food": {"name": "Poireaux"}},
                {"quantity": 0, "unit": null, "food": null, "note": "sel", "display": "sel"},
            ],
            "recipeInstructions": [{"text": "Laver"}, {"text": "  "}, {"text": "Cuire"}],
        });
        let detail = to_detail(&item).unwrap();
        assert_eq!(detail.tags, ["Soupe", "Hiver"]);
        assert_eq!(detail.time.as_deref(), Some("20 min"));
        assert_eq!(detail.servings, Some(4.0));
        assert_eq!(detail.description, "");
        assert_eq!(detail.image_url, None);
        assert_eq!(detail.steps, ["Laver", "Cuire"]);

        let draft = to_draft(&item).unwrap();
        assert_eq!(draft.ingredients[0].quantite, Some(2.0));
        assert_eq!(draft.ingredients[0].unite.as_deref(), Some("kg"));
        // `quantite` garde le 0 (pas de `or None` dans la forme éditable).
        assert_eq!(draft.ingredients[1].quantite, Some(0.0));
        assert_eq!(draft.ingredients[1].aliment, "sel");
    }

    #[test]
    fn seasonal_cache_expires_and_tracks_the_month() {
        let recipes = Rc::new(vec![SeasonalRecipe {
            slug: "soupe".into(),
            name: "Soupe".into(),
            in_season: true,
            category: None,
        }]);
        store_seasonal_cache(10, &recipes);
        assert_eq!(fresh_seasonal_cache(10).as_deref(), Some(recipes.as_ref()));
        assert_eq!(fresh_seasonal_cache(11), None);

        // Rechargement de la page : seul le localStorage reste.
        SEASONAL_CACHE.with(|cache| *cache.borrow_mut() = None);
        assert_eq!(fresh_seasonal_cache(10).as_deref(), Some(recipes.as_ref()));

        let stale = SeasonalCacheEntry {
            month: 10,
            cached_at_ms: storage::now_ms() - SEASONAL_CACHE_TTL_MS - 1.0,
            recipes: recipes.as_ref().clone(),
        };
        storage::set(SEASONAL_CACHE_KEY, &serde_json::to_string(&stale).unwrap()).unwrap();
        SEASONAL_CACHE.with(|cache| *cache.borrow_mut() = None);
        assert_eq!(fresh_seasonal_cache(10), None);
    }
}
