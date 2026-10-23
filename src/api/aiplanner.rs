//! Génération du planning de la semaine par le LLM.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use chrono::{Datelike, NaiveDate};
use regex::Regex;
use serde_json::{Value, json};

use crate::api::course_types::{self, CourseTypeMap};
use crate::api::error::ApiError;
use crate::api::models::{MealPlanEntry, RecipeSummary};
use crate::api::{mealie, mealplan, ollama};
use crate::py;
use crate::pyrandom::PyRandom;

const MEAL_TYPES: [&str; 2] = ["lunch", "dinner"];

/// Limite le nombre de recettes envoyées au LLM pour garder un prompt
/// raisonnable sur les petits modèles (qwen2.5:3b, etc.).
const MAX_RECIPES_IN_PROMPT: usize = 40;

fn build_prompt(recipes: &str, slots: &str, n_slots: usize) -> String {
    format!(
        r#"Tu planifies les repas d'une semaine. Réponds UNIQUEMENT avec du JSON valide.

Recettes disponibles :
{recipes}

Créneaux à remplir ({n_slots} au total) :
{slots}

Règles :
- Utilise UNIQUEMENT les noms listés ci-dessus, copiés à l'identique.
- Remplis TOUS les créneaux.
- Ne répète pas la même recette deux fois dans la même journée.
- Varie les catégories [entre crochets] : évite la même catégorie deux repas de suite sur le même créneau.
- Chaque recette maximum 2 fois dans la semaine.

Format de réponse (JSON uniquement, rien d'autre) :
{{"assignments": [{{"date": "YYYY-MM-DD", "entry_type": "lunch", "recipe": "Nom exact"}}, ...]}}

entry_type vaut "lunch" ou "dinner" uniquement.
"#
    )
}

async fn call_ollama(prompt: &str) -> Result<Value, ApiError> {
    ollama::generate_json(prompt, json!({"temperature": 0.3, "num_ctx": 8192}), 240).await
}

fn parse_date(text: &str) -> Result<NaiveDate, ApiError> {
    py::date_fromisoformat(text).ok_or_else(|| {
        ApiError::Data(format!("Invalid isoformat string: {}", py::repr_str(text)))
    })
}

fn empty_slots(
    start: &str,
    end: &str,
    existing: &[MealPlanEntry],
) -> Result<Vec<(String, String)>, ApiError> {
    let filled: HashSet<(&str, &str)> = existing
        .iter()
        .map(|entry| (entry.date.as_str(), entry.entry_type.as_str()))
        .collect();
    let start_date = parse_date(start)?;
    let end_date = parse_date(end)?;

    let mut slots = Vec::new();
    let mut current = start_date;
    while current <= end_date {
        let iso = current.format("%Y-%m-%d").to_string();
        for meal_type in MEAL_TYPES {
            if !filled.contains(&(iso.as_str(), meal_type)) {
                slots.push((iso.clone(), meal_type.to_owned()));
            }
        }
        current = current
            .succ_opt()
            .filter(|next| next.year() <= 9999)
            .ok_or_else(|| ApiError::Data("date value out of range".to_owned()))?;
    }
    Ok(slots)
}

static CATEGORY_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\[.*?\]\s*$").expect("regex valide"));

/// Remplit les créneaux vides de la période avec des recettes choisies par
/// le LLM. Utilise `list_recipe_summaries` (1 seul appel API) plutôt que
/// `list_recipes_with_season` (N appels) pour éviter les timeouts sur la
/// première génération. Un nom halluciné ou un créneau invalide dans la
/// réponse du modèle est simplement ignoré.
pub async fn generate_plan(start: &str,
    end: &str,
) -> Result<Vec<MealPlanEntry>, ApiError> {
    let summaries = mealie::list_recipe_summaries("", "").await?;
    if summaries.is_empty() {
        return Ok(Vec::new());
    }

    let existing = mealplan::list_entries(start, end).await?;
    let slots = empty_slots(start, end, &existing)?;
    if slots.is_empty() {
        return Ok(Vec::new());
    }

    // Filtre les recettes non planifiables (desserts, accompagnements).
    let course_types = if summaries.iter().any(|recipe| recipe.tag.is_some()) {
        course_types::get_all()?
    } else {
        CourseTypeMap::new()
    };
    let mut available: Vec<RecipeSummary> = summaries
        .into_iter()
        .filter(|recipe| {
            recipe
                .tag
                .as_deref()
                .is_none_or(|tag| course_types::is_plannable(&course_types, tag))
        })
        .collect();

    // Échantillon aléatoire mais stable pour la semaine courante.
    // On en prend suffisamment pour couvrir les créneaux avec de la variété.
    let n_needed = MAX_RECIPES_IN_PROMPT.min((slots.len() * 2).max(20));
    if available.len() > n_needed {
        let seed = py::year_week_number(parse_date(start)?);
        available = PyRandom::new(seed).sample(&available, n_needed);
    }
    available.sort_by(|a, b| a.name.cmp(&b.name));

    let recipes_by_name: HashMap<&str, &RecipeSummary> = available
        .iter()
        .map(|recipe| (recipe.name.as_str(), recipe))
        .collect();
    let recipe_lines: Vec<String> = available
        .iter()
        .map(|recipe| match recipe.tag.as_deref() {
            Some(tag) if !tag.is_empty() => format!("- {} [{tag}]", recipe.name),
            _ => format!("- {}", recipe.name),
        })
        .collect();
    let slot_lines: Vec<String> = slots
        .iter()
        .map(|(iso, meal_type)| format!("- {iso} {meal_type}"))
        .collect();

    let prompt = build_prompt(
        &recipe_lines.join("\n"),
        &slot_lines.join("\n"),
        slots.len(),
    );
    let data = call_ollama(&prompt).await?;

    let mut created = Vec::new();
    let mut remaining_slots: HashSet<(String, String)> = slots.into_iter().collect();
    for assignment in py::list_or_empty(&data, "assignments")? {
        // Accepte snake_case ("entry_type") et camelCase ("entryType")
        let entry_type = py::or(
            py::get(assignment, "entry_type")?,
            py::get(assignment, "entryType")?,
        );
        let date = py::get(assignment, "date")?;
        if [date, entry_type]
            .iter()
            .any(|value| matches!(value, Value::Array(_) | Value::Object(_)))
        {
            return Err(ApiError::Data("unhashable type".to_owned()));
        }
        let (Value::String(date), Value::String(entry_type)) = (date, entry_type) else {
            continue;
        };
        let slot = (date.clone(), entry_type.clone());
        if !remaining_slots.contains(&slot) {
            continue;
        }

        // Le LLM répète parfois le suffixe "[Catégorie]" qu'on a mis dans la
        // liste — on le retire avant la lookup.
        let raw_name = match assignment.get("recipe") {
            None => "",
            Some(Value::String(name)) => name.as_str(),
            Some(_) => {
                return Err(ApiError::Data(
                    "expected string or bytes-like object".to_owned(),
                ));
            }
        };
        let without_suffix = CATEGORY_SUFFIX.replace_all(raw_name, "");
        let Some(recipe) = recipes_by_name.get(py::strip(&without_suffix)) else {
            continue;
        };

        let entry = mealplan::create_entry(&slot.0, &slot.1, &recipe.slug).await?;
        created.push(entry);
        remaining_slots.remove(&slot);
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(date: &str, entry_type: &str) -> MealPlanEntry {
        MealPlanEntry {
            id: 1,
            date: date.to_owned(),
            entry_type: entry_type.to_owned(),
            name: String::new(),
            recipe_slug: None,
        }
    }

    #[test]
    fn only_empty_slots_are_listed() {
        let slots =
            empty_slots("2026-10-19", "2026-10-20", &[entry("2026-10-19", "dinner")]).unwrap();
        let slots: Vec<(&str, &str)> = slots
            .iter()
            .map(|(d, t)| (d.as_str(), t.as_str()))
            .collect();
        assert_eq!(
            slots,
            [
                ("2026-10-19", "lunch"),
                ("2026-10-20", "lunch"),
                ("2026-10-20", "dinner")
            ]
        );
        assert!(
            empty_slots("2026-10-20", "2026-10-19", &[])
                .unwrap()
                .is_empty()
        );
        assert!(empty_slots("2026-10-xx", "2026-10-19", &[]).is_err());
    }

    #[test]
    fn category_suffix_is_removed_like_re_sub() {
        let clean = |name: &str| py::strip(&CATEGORY_SUFFIX.replace_all(name, "")).to_owned();
        assert_eq!(clean("Soupe de poireaux [Soupe]"), "Soupe de poireaux");
        assert_eq!(clean("Gratin  [Plat] \n"), "Gratin");
        assert_eq!(clean("A [B] C [D]"), "A");
        assert_eq!(clean("Chili [sin] carne"), "Chili [sin] carne");
    }
}
