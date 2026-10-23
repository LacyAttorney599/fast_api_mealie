//! Structuration par le LLM du texte OCR d'une photo de recette.

use serde::Deserialize;
use serde_json::json;

use crate::api::error::ApiError;
use crate::api::models::RecipeDraft;
use crate::api::ollama;
use crate::py;

fn build_prompt(texte: &str) -> String {
    format!(
        r#"Tu structures une recette de cuisine extraite par OCR d'une photo. Le texte est souvent bruité : mots mal reconnus, symboles isolés issus d'icônes, fragments de colonnes différentes (étapes/ingrédients) mis bout à bout. Corrige les erreurs évidentes et ignore les symboles isolés sans rapport avec la recette, mais n'invente jamais d'information.

Règles pour "unite" et "aliment" :
- "unite" est UNIQUEMENT une unité de mesure standard : g, kg, ml, cl, L, c. à soupe, c. à café, pincée, tranche, gousse, botte... Si l'ingrédient est juste compté (ex: "1 orange", "2 œufs"), laisse "unite" à null et mets le nom complet dans "aliment" (ex: aliment="orange", pas unite="orange").
- "aliment" contient le nom de l'aliment et ses éventuelles précisions (ex: "oignon rouge émincé"), jamais une unité de mesure.
- Ne recopie JAMAIS le même texte dans "unite" et "aliment". Si tu hésites à remplir "unite", laisse-le à null plutôt que d'y répéter le nom de l'aliment.
- Ne mets un nombre dans "quantite" que s'il est clairement lisible dans le texte. En cas de doute (chiffre isolé suspect, symbole mal reconnu comme "½" ou "1/2"), laisse "quantite" à null plutôt que de deviner.
- Inclus TOUS les ingrédients mentionnés, y compris le sel, le poivre, les épices, et les ingrédients listés comme optionnels ou en variante (ex: "ou 1 boîte de thon nature").

Règles pour "etapes" :
- Une étape par instruction numérotée ou listée dans le texte, dans l'ordre.
- N'inclus PAS les métadonnées de la fiche (nombre de personnes, temps de préparation, prix) dans les étapes.

Exemple (texte OCR bruité en entrée → JSON attendu en sortie) :
Texte : "2 personnes\n- 1 boîte de flageolets\n- 1/2 concombre\nEtapes\n1.Eplucher le concombre\n2.Melanger"
JSON : {{"nom": "Salade de flageolets", "ingredients": [{{"quantite": 1, "unite": "boîte", "aliment": "flageolets"}}, {{"quantite": null, "unite": null, "aliment": "concombre"}}], "etapes": ["Eplucher le concombre", "Melanger"]}}

Réponds UNIQUEMENT avec un JSON valide de cette forme, sans texte autour :
{{
  "nom": "string",
  "ingredients": [{{"quantite": number ou null, "unite": "string ou null", "aliment": "string"}}],
  "etapes": ["string", ...]
}}

Texte brut extrait par OCR :
---
{texte}
---

Rappel : la liste "ingredients" doit contenir TOUS les ingrédients du texte ci-dessus, sans exception (y compris sel, poivre, épices, condiments et ingrédients optionnels/variantes). Vérifie ta réponse avant de la donner : si un ingrédient du texte manque, ajoute-le.
"#
    )
}

#[rustfmt::skip]
const UNITES_RECONNUES: [&str; 27] = [
    "g", "kg", "mg",
    "ml", "cl", "l",
    "c. à soupe", "c à soupe", "cuillère à soupe", "cuillère a soupe",
    "c. à café", "c à café", "cuillère à café", "cuillère a café",
    "boîte", "boite", "sachet", "gousse", "pincée", "tranche",
    "botte", "brin", "feuille", "branche", "paquet", "pot", "verre",
];

/// Le LLM (qwen2.5:3b) recopie parfois le nom de l'aliment dans "unite" au lieu de
/// laisser le champ vide pour les ingrédients simplement comptés (ex: "1 orange"). On
/// n'a pas réussi à éliminer ce biais par le prompt, donc on filtre après coup contre
/// une liste d'unités de mesure reconnues plutôt que de faire confiance au modèle.
fn nettoyer_unite(unite: Option<&str>, aliment: &str) -> Option<String> {
    let unite = unite.filter(|unite| !unite.is_empty())?;
    let normalized = py::strip(unite).to_lowercase();
    if normalized == py::strip(aliment).to_lowercase()
        || !UNITES_RECONNUES.contains(&normalized.as_str())
    {
        return None;
    }
    Some(py::strip(unite).to_owned())
}

pub async fn structure_recipe(texte_ocr: &str) -> Result<RecipeDraft, ApiError> {
    let data = ollama::generate_json(&build_prompt(texte_ocr), json!({"temperature": 0.2}), 120).await?;
    let mut draft =
        RecipeDraft::deserialize(&data).map_err(|err| ApiError::Data(err.to_string()))?;
    for ingredient in &mut draft.ingredients {
        ingredient.unite = nettoyer_unite(ingredient.unite.as_deref(), &ingredient.aliment);
    }
    Ok(draft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_are_kept_only_when_recognised() {
        assert_eq!(nettoyer_unite(Some(" g "), "farine").as_deref(), Some("g"));
        assert_eq!(
            nettoyer_unite(Some("C. à soupe"), "huile").as_deref(),
            Some("C. à soupe")
        );
        assert_eq!(nettoyer_unite(Some("orange"), "orange"), None);
        assert_eq!(nettoyer_unite(Some("boîte"), " Boîte "), None);
        assert_eq!(nettoyer_unite(Some("poignée"), "persil"), None);
        assert_eq!(nettoyer_unite(Some(""), "sel"), None);
        assert_eq!(nettoyer_unite(None, "sel"), None);
    }

    #[test]
    fn llm_output_is_validated_like_pydantic() {
        let body = json!({"response": r#"{"nom": "Soupe", "ingredients": [{"quantite": "2", "aliment": "poireau"}], "extra": 1}"#});
        let draft = RecipeDraft::deserialize(&ollama::parse_response(&body).unwrap()).unwrap();
        assert_eq!(draft.ingredients[0].quantite, Some(2.0));
        assert!(draft.etapes.is_empty());

        let invalid = json!({"response": r#"{"nom": "Soupe", "ingredients": [{"quantite": "1/2", "aliment": "x"}]}"#});
        assert!(RecipeDraft::deserialize(&ollama::parse_response(&invalid).unwrap()).is_err());
    }
}
