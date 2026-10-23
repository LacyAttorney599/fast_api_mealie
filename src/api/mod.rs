//! Accès aux données, depuis le navigateur : Mealie (via `/api`), Ollama (via
//! `/ollama`) et stockage local. Reprend ce que faisait l'ancien backend
//! Python ; les fonctions publiques portent les noms de l'ancien `api.ts`.

mod aiplanner;
mod config;
mod course_types;
mod error;
mod http;
mod llm;
mod mealie;
mod mealplan;
mod models;
mod ocr;
mod ollama;
mod seasons;
mod shoppinglist;
mod storage;

use std::rc::Rc;

use chrono::{Datelike, Local};

pub use course_types::{COURSE_TYPES, CourseTypeMap};
pub use error::ApiError;
pub use models::*;

pub type ApiResult<T> = Result<T, ApiError>;

pub async fn fetch_recipes(search: &str, cookbook: &str) -> ApiResult<Vec<RecipeSummary>> {
    mealie::list_recipe_summaries(search, cookbook).await
}

pub async fn add_favorite(slug: &str) -> ApiResult<()> {
    mealie::set_favorite(slug, true).await
}

pub async fn remove_favorite(slug: &str) -> ApiResult<()> {
    mealie::set_favorite(slug, false).await
}

pub async fn fetch_cookbooks() -> ApiResult<Vec<Cookbook>> {
    mealie::list_cookbooks().await
}

pub async fn fetch_cookbook_categories() -> ApiResult<Vec<RecipeCategory>> {
    mealie::list_recipe_categories().await
}

/// `category_id` : `None` pour un livre manuel, l'id d'une catégorie pour un
/// livre automatique.
pub async fn create_cookbook(name: &str, category_id: Option<&str>) -> ApiResult<Cookbook> {
    mealie::create_cookbook(name, category_id).await
}

/// Échoue avec [`ApiError::Invalid`] (message à afficher) si le livre utilise
/// un filtre que l'application ne sait pas modifier.
pub async fn add_recipe_to_cookbook(cookbook_slug: &str, recipe_slug: &str) -> ApiResult<()> {
    mealie::add_recipe_to_cookbook(cookbook_slug, recipe_slug).await
}

pub async fn remove_recipe_from_cookbook(cookbook_slug: &str, recipe_slug: &str) -> ApiResult<()> {
    mealie::remove_recipe_from_cookbook(cookbook_slug, recipe_slug).await
}

pub async fn fetch_recipe(slug: &str) -> ApiResult<RecipeDetail> {
    mealie::get_recipe_detail(slug).await
}

pub async fn fetch_meal_plan(start: &str, end: &str) -> ApiResult<Vec<MealPlanEntry>> {
    mealplan::list_entries(start, end).await
}

/// `entry_type` : "lunch" ou "dinner".
pub async fn create_meal_plan_entry(date: &str, entry_type: &str, recipe_slug: &str) -> ApiResult<MealPlanEntry> {
    mealplan::create_entry(date, entry_type, recipe_slug).await
}

pub async fn delete_meal_plan_entry(id: i64) -> ApiResult<()> {
    mealplan::delete_entry(id).await
}

pub async fn fetch_shopping_list() -> ApiResult<ShoppingList> {
    shoppinglist::get_shopping_list().await
}

pub async fn toggle_shopping_item(id: &str, checked: bool) -> ApiResult<()> {
    shoppinglist::toggle_item(id, checked).await
}

pub async fn add_shopping_item(text: &str) -> ApiResult<()> {
    shoppinglist::add_item(text).await
}

pub async fn clear_checked_shopping_items() -> ApiResult<()> {
    shoppinglist::clear_checked().await
}

pub async fn clear_all_shopping_items() -> ApiResult<()> {
    shoppinglist::clear_all().await
}

pub async fn generate_shopping_list(start: &str, end: &str) -> ApiResult<()> {
    shoppinglist::generate_from_mealplan(start, end).await
}

/// Renvoie le slug de la recette créée.
pub async fn create_recipe(draft: &RecipeDraft) -> ApiResult<String> {
    mealie::create_recipe(draft).await
}

pub async fn fetch_recipe_draft(slug: &str) -> ApiResult<RecipeDraft> {
    mealie::get_recipe_draft(slug).await
}

/// Renvoie le slug à jour (Mealie le régénère si le nom change).
pub async fn update_recipe(slug: &str, draft: &RecipeDraft) -> ApiResult<String> {
    mealie::update_recipe(slug, draft).await
}

/// Import d'une photo de recette : OCR dans le navigateur, puis structuration
/// du texte par le LLM.
pub async fn import_photo(file: &web_sys::File) -> ApiResult<RecipeDraft> {
    let texte = ocr::extract_text(file).await?;
    llm::structure_recipe(&texte).await
}

/// Import groupé (ex: livre de recettes extrait en JSON par Claude).
/// Séquentiel plutôt que parallèle : la résolution food/unit de chaque
/// recette fait plusieurs allers-retours vers Mealie, et des créations
/// concurrentes du même food/unit inconnu risqueraient de se marcher
/// dessus (contrainte d'unicité déjà rencontrée en Phase 1). Une recette
/// en échec n'interrompt pas les suivantes.
pub async fn create_recipes_bulk(drafts: &[RecipeDraft]) -> Vec<BulkImportResult> {
    let mut results = Vec::with_capacity(drafts.len());
    for draft in drafts {
        let result = match mealie::create_recipe(draft).await {
            Ok(slug) => BulkImportResult {
                nom: draft.nom.clone(),
                success: true,
                slug: Some(slug),
                error: None,
            },
            Err(err) => BulkImportResult {
                nom: draft.nom.clone(),
                success: false,
                slug: None,
                error: Some(err.to_string()),
            },
        };
        results.push(result);
    }
    results
}

/// Recettes et indicateur « de saison », calculé à partir de leurs
/// ingrédients (cache d'une heure, voir `mealie::list_recipes_with_season`).
pub async fn fetch_seasonal_recipes() -> ApiResult<Rc<Vec<SeasonalRecipe>>> {
    mealie::list_recipes_with_season().await
}

pub fn fetch_current_season() -> CurrentSeason {
    let today = Local::now().date_naive();
    CurrentSeason {
        month: today.month(),
        produce: seasons::current_month_produce(today),
    }
}

/// Type de service (`COURSE_TYPES`) par nom de catégorie, stocké dans le navigateur.
pub fn fetch_course_types() -> ApiResult<CourseTypeMap> {
    course_types::get_all()
}

/// `course_type` : `None` pour revenir au type par défaut (plat). Renvoie la
/// configuration complète à jour.
pub fn set_course_type(category_name: &str, course_type: Option<&str>) -> ApiResult<CourseTypeMap> {
    course_types::set_category_type(category_name, course_type)
}

/// Remplit les créneaux vides de la semaine avec des recettes choisies par le
/// LLM ; renvoie les entrées créées.
pub async fn generate_ai_meal_plan(start: &str, end: &str) -> ApiResult<Vec<MealPlanEntry>> {
    aiplanner::generate_plan(start, end).await
}
