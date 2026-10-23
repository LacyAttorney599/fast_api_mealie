//! Données échangées entre la couche d'accès à Mealie et les écrans. Noms et
//! champs reprennent ceux de l'ancien `frontend/src/lib/api.ts`.

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::py;

/// Carte de l'écran Liste : sous-ensemble aplati de l'objet recette Mealie.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecipeSummary {
    pub slug: String,
    pub name: String,
    pub time: Option<String>,
    pub tag: Option<String>,
    pub image_url: Option<String>,
    pub color: String,
    pub is_favorite: bool,
}

/// Quantité et unité brutes (pas de texte déjà formaté) : l'écran Détail
/// recalcule l'affichage quand l'utilisateur change le nombre de parts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ingredient {
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub food: String,
}

/// Recette de l'écran Détail.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecipeDetail {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub time: Option<String>,
    pub servings: Option<f64>,
    pub tags: Vec<String>,
    pub ingredients: Vec<Ingredient>,
    pub steps: Vec<String>,
    pub in_season: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cookbook {
    pub slug: String,
    pub name: String,
    pub manual: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecipeCategory {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MealPlanEntry {
    pub id: i64,
    pub date: String,
    /// "lunch" ou "dinner" (d'autres valeurs restent possibles côté Mealie).
    pub entry_type: String,
    pub name: String,
    pub recipe_slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShoppingItem {
    pub id: String,
    /// Quantité déjà formatée (« 1,5 kg »), vide si aucune.
    pub quantity: String,
    pub food: String,
    pub checked: bool,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShoppingCategory {
    pub name: String,
    pub items: Vec<ShoppingItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShoppingList {
    pub categories: Vec<ShoppingCategory>,
    pub total_items: usize,
    pub recipe_count: usize,
}

/// Ingrédient d'une recette à créer ou modifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftIngredient {
    #[serde(default, deserialize_with = "lax_opt_float")]
    pub quantite: Option<f64>,
    #[serde(default)]
    pub unite: Option<String>,
    pub aliment: String,
}

/// Recette à créer ou modifier (formulaire, import photo, import JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDraft {
    pub nom: String,
    #[serde(default)]
    pub ingredients: Vec<DraftIngredient>,
    #[serde(default)]
    pub etapes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BulkImportResult {
    pub nom: String,
    pub success: bool,
    pub slug: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonalRecipe {
    pub slug: String,
    pub name: String,
    pub in_season: bool,
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CurrentSeason {
    pub month: u32,
    pub produce: &'static [&'static str],
}

/// `float | None` de pydantic, en mode « lax » : nombre, booléen ou chaîne
/// numérique (la sortie du LLM contient parfois `"quantite": "2"`).
fn lax_opt_float<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(None),
        value => py::lax_float(&value)
            .map(Some)
            .ok_or_else(|| D::Error::custom("Input should be a valid number")),
    }
}
