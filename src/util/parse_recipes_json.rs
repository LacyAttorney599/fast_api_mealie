//! Lecture du JSON de recettes collé dans l'écran d'import (ex-
//! `frontend/src/lib/parseRecipesJson.ts`) : tolérant, les entrées invalides
//! sont ignorées.

use serde_json::Value;

use crate::api::{DraftIngredient, RecipeDraft};
use crate::util::js;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub recipes: Vec<RecipeDraft>,
    pub error: Option<&'static str>,
}

/// Chaîne non vide une fois `trim()`, comme le test `typeof x === "string" && x.trim()`.
fn non_blank_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|text| !js::trim(text).is_empty())
}

fn parse_ingredient(raw: &Value) -> Option<DraftIngredient> {
    let item = raw.as_object()?;
    let aliment = non_blank_string(item.get("aliment"))?;

    let quantite = match item.get("quantite") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) if text.is_empty() => None,
        Some(raw_qty) => Some(js::value_to_number(raw_qty)).filter(|n| !n.is_nan()),
    };

    let unite = non_blank_string(item.get("unite"))
        .filter(|unit| *unit != "—")
        .map(|unit| js::trim(unit).to_owned());

    Some(DraftIngredient {
        quantite,
        unite,
        aliment: js::trim(aliment).to_owned(),
    })
}

pub fn parse_recipes_json(text: &str) -> ParseResult {
    let failure = |error| ParseResult {
        recipes: Vec::new(),
        error: Some(error),
    };
    if js::trim(text).is_empty() {
        return ParseResult {
            recipes: Vec::new(),
            error: None,
        };
    }

    let Ok(data) = serde_json::from_str::<Value>(text) else {
        return failure("JSON invalide — vérifiez la syntaxe.");
    };
    let Value::Array(items) = data else {
        return failure("Le JSON doit être un tableau de recettes.");
    };

    let mut recipes = Vec::new();
    for raw in &items {
        let Some(item) = raw.as_object() else { continue };
        let Some(nom) = non_blank_string(item.get("nom")) else {
            continue;
        };

        let ingredients = match item.get("ingredients") {
            Some(Value::Array(ingredients)) => ingredients.iter().filter_map(parse_ingredient).collect(),
            _ => Vec::new(),
        };

        let etapes = match item.get("etapes") {
            Some(Value::Array(steps)) => steps
                .iter()
                .filter_map(|step| non_blank_string(Some(step)))
                .map(str::to_owned)
                .collect(),
            _ => Vec::new(),
        };

        recipes.push(RecipeDraft {
            nom: js::trim(nom).to_owned(),
            ingredients,
            etapes,
        });
    }

    if recipes.is_empty() {
        return failure("Aucune recette valide trouvée dans ce JSON (chaque objet doit avoir un champ \"nom\").");
    }

    ParseResult { recipes, error: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_json_is_parsed() {
        let result = parse_recipes_json(
            r#"[
              {"nom": " Tarte aux pommes ", "ingredients": [
                {"quantite": "6", "unite": "—", "aliment": "pommes"},
                {"quantite": "200", "unite": " g ", "aliment": "farine"},
                {"quantite": "1/2", "unite": "", "aliment": "citron"},
                {"quantite": 2, "aliment": "  "},
                "oeuf"
              ], "etapes": ["Éplucher ", "", 3, "Cuire"]},
              {"nom": "  "},
              [1, 2]
            ]"#,
        );
        assert_eq!(result.error, None);
        assert_eq!(result.recipes.len(), 1);
        let recipe = &result.recipes[0];
        assert_eq!(recipe.nom, "Tarte aux pommes");
        assert_eq!(recipe.etapes, ["Éplucher ", "Cuire"]);
        let ingredients: Vec<_> = recipe
            .ingredients
            .iter()
            .map(|ing| (ing.quantite, ing.unite.as_deref(), ing.aliment.as_str()))
            .collect();
        assert_eq!(
            ingredients,
            [
                (Some(6.0), None, "pommes"),
                (Some(200.0), Some("g"), "farine"),
                (None, None, "citron")
            ]
        );
    }

    #[test]
    fn errors_match_the_react_messages() {
        assert_eq!(parse_recipes_json("  ").error, None);
        assert_eq!(parse_recipes_json("{").error, Some("JSON invalide — vérifiez la syntaxe."));
        assert_eq!(parse_recipes_json("{}").error, Some("Le JSON doit être un tableau de recettes."));
        assert!(parse_recipes_json("[{}]").error.unwrap().starts_with("Aucune recette valide"));
    }
}
