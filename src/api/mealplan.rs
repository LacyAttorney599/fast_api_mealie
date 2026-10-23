//! Planning des repas (Mealie : `/api/households/mealplans`).

use serde_json::{Value, json};

use crate::api::error::ApiError;
use crate::api::http::{self, send, send_json, send_with_json};
use crate::api::models::MealPlanEntry;
use crate::py;

fn to_entry(item: &Value) -> Result<MealPlanEntry, ApiError> {
    let recipe = py::get(item, "recipe")?;
    let name = if py::truthy(recipe) {
        py::string(py::at(recipe, "name")?)?
    } else {
        let title = py::get(item, "title")?;
        py::string_or_empty(if py::truthy(title) {
            title
        } else {
            py::get(item, "text")?
        })?
    };
    Ok(MealPlanEntry {
        id: py::int(py::at(item, "id")?)?,
        date: py::string(py::at(item, "date")?)?,
        entry_type: py::string(py::at(item, "entryType")?)?,
        name,
        recipe_slug: if py::truthy(recipe) {
            py::opt_string(py::at(recipe, "slug")?)?
        } else {
            None
        },
    })
}

pub async fn list_entries(start: &str, end: &str) -> Result<Vec<MealPlanEntry>, ApiError> {
    let data = send_json(http::get("/api/households/mealplans").query([
        ("start_date", start),
        ("end_date", end),
        ("perPage", "100"),
    ]))
    .await?;
    py::list(py::at(&data, "items")?)?
        .iter()
        .map(to_entry)
        .collect()
}

pub async fn create_entry(date: &str, entry_type: &str, recipe_slug: &str) -> Result<MealPlanEntry, ApiError> {
    let recipe = send_json(http::get(&format!("/api/recipes/{recipe_slug}"))).await?;
    let recipe_id = py::at(&recipe, "id")?;

    let created = send_with_json(
        http::post("/api/households/mealplans"),
        &json!({"date": date, "entryType": entry_type, "recipeId": recipe_id}),
    )
    .await?;
    to_entry(&created)
}

pub async fn delete_entry(entry_id: i64) -> Result<(), ApiError> {
    send(http::delete(&format!("/api/households/mealplans/{entry_id}"))).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_without_recipe_use_their_title() {
        let entry = to_entry(&json!({
            "id": 3, "date": "2026-10-19", "entryType": "dinner", "recipe": null,
            "title": "", "text": "Restes",
        }))
        .unwrap();
        assert_eq!(entry.name, "Restes");
        assert_eq!(entry.recipe_slug, None);

        let entry = to_entry(&json!({
            "id": 4, "date": "2026-10-19", "entryType": "lunch",
            "recipe": {"name": "Soupe", "slug": "soupe"},
        }))
        .unwrap();
        assert_eq!((entry.name.as_str(), entry.recipe_slug.as_deref()), ("Soupe", Some("soupe")));
    }
}
