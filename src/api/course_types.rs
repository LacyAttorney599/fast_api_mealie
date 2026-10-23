//! Type de service (plat, dessert, accompagnement, entrée) par nom de catégorie
//! Mealie. Stocké dans le navigateur (`localStorage`), donc propre à chaque
//! appareil ; au premier lancement, la configuration livrée avec l'application
//! (`data/course_types.json`) sert de point de départ.

use indexmap::IndexMap;
use serde_json::Value;

use crate::api::error::ApiError;
use crate::api::storage;
use crate::py;

/// Types valides.
pub const COURSE_TYPES: [&str; 4] = ["plat", "entrée", "dessert", "accompagnement"];

/// Types exclus de la génération IA (ne doivent pas être mis en plat principal).
const EXCLUDED_FROM_PLANNER: [&str; 2] = ["dessert", "accompagnement"];

const STORAGE_KEY: &str = "ma-cuisine.types-de-service";
const DEFAULT_COURSE_TYPES: &str = include_str!("../../data/course_types.json");

pub type CourseTypeMap = IndexMap<String, String>;

/// Contenu JSON d'une configuration ; un JSON illisible vaut une configuration
/// vide (comme l'ancien fichier corrompu côté serveur).
fn parse(text: &str) -> Result<CourseTypeMap, ApiError> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Ok(CourseTypeMap::new());
    };
    serde_json::from_value(value).map_err(|err| ApiError::Data(err.to_string()))
}

pub fn get_all() -> Result<CourseTypeMap, ApiError> {
    match storage::get(STORAGE_KEY) {
        Some(text) => parse(&text),
        None => parse(DEFAULT_COURSE_TYPES),
    }
}

fn save(data: &CourseTypeMap) -> Result<(), ApiError> {
    let text = serde_json::to_string(data).map_err(|err| ApiError::Internal(err.to_string()))?;
    storage::set(STORAGE_KEY, &text).map_err(ApiError::Internal)
}

/// Associe un type de service à une catégorie. Si `course_type` est `None`,
/// retire l'entrée (la catégorie revient au défaut 'plat'). Renvoie la
/// configuration complète mise à jour.
pub fn set_category_type(category_name: &str, course_type: Option<&str>) -> Result<CourseTypeMap, ApiError> {
    let mut data = get_all()?;
    match course_type {
        None => {
            data.shift_remove(category_name);
        }
        Some(course_type) => {
            if !COURSE_TYPES.contains(&course_type) {
                return Err(ApiError::Invalid(format!(
                    "Type invalide : {}",
                    py::repr_str(course_type)
                )));
            }
            data.insert(category_name.to_owned(), course_type.to_owned());
        }
    }
    save(&data)?;
    Ok(data)
}

/// Renvoie `true` si une recette de cette catégorie peut être planifiée
/// en repas principal (lunch/dinner). Les catégories non configurées
/// sont traitées comme 'plat' par défaut.
pub fn is_plannable(data: &CourseTypeMap, category_name: &str) -> bool {
    let course_type = data.get(category_name).map_or("plat", String::as_str);
    !EXCLUDED_FROM_PLANNER.contains(&course_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        storage::set(STORAGE_KEY, "{}").unwrap();
    }

    #[test]
    fn the_shipped_configuration_is_the_default() {
        let defaults = parse(DEFAULT_COURSE_TYPES).unwrap();
        assert_eq!(defaults.get("Desserts").map(String::as_str), Some("dessert"));
        assert_eq!(defaults.get("Sauce").map(String::as_str), Some("accompagnement"));
    }

    #[test]
    fn invalid_json_is_an_empty_configuration() {
        storage::set(STORAGE_KEY, "{oops").unwrap();
        assert!(get_all().unwrap().is_empty());
    }

    #[test]
    fn set_and_reset_keep_insertion_order() {
        reset();
        set_category_type("Soupe", Some("plat")).unwrap();
        set_category_type("Desserts", Some("dessert")).unwrap();
        set_category_type("Salade", Some("entrée")).unwrap();
        set_category_type("Soupe", Some("entrée")).unwrap();
        let keys: Vec<_> = get_all().unwrap().into_keys().collect();
        assert_eq!(keys, ["Soupe", "Desserts", "Salade"]);

        let data = set_category_type("Desserts", None).unwrap();
        assert_eq!(data.into_keys().collect::<Vec<_>>(), ["Soupe", "Salade"]);
    }

    #[test]
    fn invalid_types_are_rejected_with_the_python_message() {
        reset();
        assert_eq!(
            set_category_type("Soupe", Some("goûter")),
            Err(ApiError::Invalid("Type invalide : 'goûter'".to_owned()))
        );
    }

    #[test]
    fn unconfigured_categories_are_plannable() {
        let data = CourseTypeMap::from([("Desserts".to_owned(), "dessert".to_owned())]);
        assert!(!is_plannable(&data, "Desserts"));
        assert!(is_plannable(&data, "Soupe"));
    }
}
