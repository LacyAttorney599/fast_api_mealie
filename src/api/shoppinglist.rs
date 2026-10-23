//! Liste de courses (Mealie : `/api/households/shopping/…`).

use std::collections::HashSet;

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::api::error::ApiError;
use crate::api::http::{self, send, send_json, send_with, send_with_json};
use crate::api::mealplan;
use crate::api::models::{ShoppingCategory, ShoppingItem, ShoppingList};
use crate::py;

async fn get_or_create_list_id() -> Result<Value, ApiError> {
    let data = send_json(http::get("/api/households/shopping/lists")).await?;
    let items = py::at(&data, "items")?;
    if py::truthy(items) {
        return Ok(py::at(py::first(items)?, "id")?.clone());
    }

    let created = send_with_json(
        http::post("/api/households/shopping/lists"),
        &json!({"name": "Courses"}),
    )
    .await?;
    Ok(py::at(&created, "id")?.clone())
}

fn format_quantity(item: &Value) -> Result<String, ApiError> {
    let quantity = py::get(item, "quantity")?;
    if !py::truthy(quantity) {
        return Ok(String::new());
    }
    let quantity = match quantity {
        Value::Number(n) => n.as_f64().unwrap_or_default(),
        Value::Bool(_) => 1.0,
        _ => {
            return Err(ApiError::Data(
                "Unknown format code 'g' for object of type 'str'".to_owned(),
            ));
        }
    };
    let qty_str = py::format_g(quantity).replace('.', ",");
    let unit = py::get(item, "unit")?;
    if py::truthy(unit) {
        let plural_name = py::get(unit, "pluralName")?;
        let name = if quantity > 1.0 && py::truthy(plural_name) {
            plural_name
        } else {
            py::get(unit, "name")?
        };
        if py::truthy(name) {
            return Ok(format!("{qty_str} {}", py::fstr(name)));
        }
    }
    Ok(qty_str)
}

fn to_shopping_item(item: &Value) -> Result<ShoppingItem, ApiError> {
    let food = py::get(item, "food")?;
    let name = if py::truthy(food) {
        py::string(py::at(food, "name")?)?
    } else {
        let note = py::get(item, "note")?;
        py::string_or_empty(if py::truthy(note) {
            note
        } else {
            py::get(item, "display")?
        })?
    };
    let label = py::get(item, "label")?;
    Ok(ShoppingItem {
        id: py::string(py::at(item, "id")?)?,
        quantity: format_quantity(item)?,
        food: name,
        checked: py::lax_bool(py::at(item, "checked")?)
            .ok_or_else(|| ApiError::Data("Input should be a valid boolean".to_owned()))?,
        category: if py::truthy(label) {
            py::string(py::at(label, "name")?)?
        } else {
            "Autres".to_owned()
        },
    })
}

/// Identifiants des recettes déjà ajoutées à la liste (`recipeReferences`).
fn recipe_reference_ids(list: &Value) -> Result<HashSet<String>, ApiError> {
    py::list(py::at(list, "recipeReferences")?)?
        .iter()
        .map(|reference| Ok(py::fstr(py::at(reference, "recipeId")?)))
        .collect()
}

pub async fn get_shopping_list() -> Result<ShoppingList, ApiError> {
    let list_id = get_or_create_list_id().await?;
    let data = send_json(http::get(&format!(
        "/api/households/shopping/lists/{}",
        py::fstr(&list_id)
    )))
    .await?;

    let list_items = py::list(py::at(&data, "listItems")?)?;
    let mut categories: IndexMap<String, Vec<ShoppingItem>> = IndexMap::new();
    for raw in list_items {
        let item = to_shopping_item(raw)?;
        categories
            .entry(item.category.clone())
            .or_default()
            .push(item);
    }

    let recipe_ids = recipe_reference_ids(&data)?;

    Ok(ShoppingList {
        categories: categories
            .into_iter()
            .map(|(name, items)| ShoppingCategory { name, items })
            .collect(),
        total_items: list_items.len(),
        recipe_count: recipe_ids.len(),
    })
}

pub async fn toggle_item(item_id: &str,
    checked: bool,
) -> Result<(), ApiError> {
    let path = format!("/api/households/shopping/items/{item_id}");
    let mut item = send_json(http::get(&path)).await?;
    let Value::Object(map) = &mut item else {
        return Err(ApiError::Data(
            "object does not support item assignment".to_owned(),
        ));
    };
    map.insert("checked".to_owned(), Value::Bool(checked));
    send_with(http::put(&path), &item).await?;
    Ok(())
}

pub async fn add_item(text: &str) -> Result<(), ApiError> {
    let list_id = get_or_create_list_id().await?;
    send_with(
        http::post("/api/households/shopping/items"),
        &json!({"shoppingListId": list_id, "note": text, "display": text}),
    )
    .await?;
    Ok(())
}

/// Supprime un à un les articles de la liste retenus par `keep`.
async fn delete_items(keep: impl Fn(&Value) -> Result<bool, ApiError>,
) -> Result<(), ApiError> {
    let list_id = get_or_create_list_id().await?;
    let data = send_json(http::get(&format!(
        "/api/households/shopping/lists/{}",
        py::fstr(&list_id)
    )))
    .await?;
    let mut item_ids = Vec::new();
    for item in py::list(py::at(&data, "listItems")?)? {
        if keep(item)? {
            item_ids.push(py::fstr(py::at(item, "id")?));
        }
    }

    for item_id in item_ids {
        send(http::delete(&format!("/api/households/shopping/items/{item_id}"))).await?;
    }
    Ok(())
}

pub async fn clear_checked() -> Result<(), ApiError> {
    delete_items(|item| Ok(py::truthy(py::at(item, "checked")?))).await
}

pub async fn clear_all() -> Result<(), ApiError> {
    delete_items(|_| Ok(true)).await
}

/// Ajoute à la liste les ingrédients de chaque recette planifiée sur la
/// période (une seule fois par recette même si planifiée plusieurs fois dans
/// la semaine). Mealie fusionne lui-même les quantités des ingrédients déjà
/// présents dans la liste, donc pas de logique d'agrégation à réécrire ici.
pub async fn generate_from_mealplan(start: &str,
    end: &str,
) -> Result<(), ApiError> {
    let entries = mealplan::list_entries(start, end).await?;
    let mut recipe_slugs: Vec<&str> = Vec::new();
    for slug in entries
        .iter()
        .filter_map(|entry| entry.recipe_slug.as_deref())
    {
        if !slug.is_empty() && !recipe_slugs.contains(&slug) {
            recipe_slugs.push(slug);
        }
    }

    let list_id = py::fstr(&get_or_create_list_id().await?);

    let list = send_json(http::get(&format!("/api/households/shopping/lists/{list_id}"))).await?;
    let already_added = recipe_reference_ids(&list)?;

    for slug in recipe_slugs {
        let recipe = send_json(http::get(&format!("/api/recipes/{slug}"))).await?;
        let recipe_id = py::fstr(py::at(&recipe, "id")?);
        if already_added.contains(&recipe_id) {
            continue;
        }

        send_with(
            http::post(&format!("/api/households/shopping/lists/{list_id}/recipe/{recipe_id}")),
            &json!({}),
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities_are_formatted_like_python() {
        let cases = [
            (json!({"quantity": 0}), ""),
            (json!({"quantity": null}), ""),
            (json!({"quantity": 1.5}), "1,5"),
            (
                json!({"quantity": 2.0, "unit": {"name": "gousse", "pluralName": "gousses"}}),
                "2 gousses",
            ),
            (
                json!({"quantity": 1.0, "unit": {"name": "gousse", "pluralName": "gousses"}}),
                "1 gousse",
            ),
            (
                json!({"quantity": 3, "unit": {"name": "kg", "pluralName": null}}),
                "3 kg",
            ),
            (
                json!({"quantity": 0.333333333, "unit": {"name": ""}}),
                "0,333333",
            ),
            (json!({"quantity": 250, "unit": null}), "250"),
        ];
        for (item, expected) in cases {
            assert_eq!(format_quantity(&item).unwrap(), expected, "{item}");
        }
    }

    #[test]
    fn unstructured_items_use_their_note() {
        let item = json!({"id": "i1", "food": null, "note": "", "display": "Pain", "checked": false, "label": null});
        let parsed = to_shopping_item(&item).unwrap();
        assert_eq!(
            (parsed.food.as_str(), parsed.category.as_str()),
            ("Pain", "Autres")
        );
    }
}
