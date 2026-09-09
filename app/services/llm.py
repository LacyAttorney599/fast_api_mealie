import json

import httpx

from app.config import settings
from app.models.recipe import RecipeDraft

PROMPT_TEMPLATE = """Tu structures une recette de cuisine extraite par OCR d'une photo. \
Le texte est souvent bruité : mots mal reconnus, symboles isolés issus d'icônes, fragments \
de colonnes différentes (étapes/ingrédients) mis bout à bout. Corrige les erreurs évidentes \
et ignore les symboles isolés sans rapport avec la recette, mais n'invente jamais d'information.

Règles pour "unite" et "aliment" :
- "unite" est UNIQUEMENT une unité de mesure standard : g, kg, ml, cl, L, c. à soupe, c. à café, \
pincée, tranche, gousse, botte... Si l'ingrédient est juste compté (ex: "1 orange", "2 œufs"), \
laisse "unite" à null et mets le nom complet dans "aliment" (ex: aliment="orange", pas unite="orange").
- "aliment" contient le nom de l'aliment et ses éventuelles précisions (ex: "oignon rouge émincé"), \
jamais une unité de mesure.
- Ne recopie JAMAIS le même texte dans "unite" et "aliment". Si tu hésites à remplir "unite", \
laisse-le à null plutôt que d'y répéter le nom de l'aliment.
- Ne mets un nombre dans "quantite" que s'il est clairement lisible dans le texte. En cas de doute \
(chiffre isolé suspect, symbole mal reconnu comme "½" ou "1/2"), laisse "quantite" à null plutôt que \
de deviner.
- Inclus TOUS les ingrédients mentionnés, y compris le sel, le poivre, les épices, et les \
ingrédients listés comme optionnels ou en variante (ex: "ou 1 boîte de thon nature").

Règles pour "etapes" :
- Une étape par instruction numérotée ou listée dans le texte, dans l'ordre.
- N'inclus PAS les métadonnées de la fiche (nombre de personnes, temps de préparation, prix) \
dans les étapes.

Exemple (texte OCR bruité en entrée → JSON attendu en sortie) :
Texte : "2 personnes\\n- 1 boîte de flageolets\\n- 1/2 concombre\\nEtapes\\n1.Eplucher le concombre\\n2.Melanger"
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

Rappel : la liste "ingredients" doit contenir TOUS les ingrédients du texte ci-dessus, sans exception \
(y compris sel, poivre, épices, condiments et ingrédients optionnels/variantes). Vérifie ta réponse \
avant de la donner : si un ingrédient du texte manque, ajoute-le.
"""


UNITES_RECONNUES = {
    "g", "kg", "mg",
    "ml", "cl", "l",
    "c. à soupe", "c à soupe", "cuillère à soupe", "cuillère a soupe",
    "c. à café", "c à café", "cuillère à café", "cuillère a café",
    "boîte", "boite", "sachet", "gousse", "pincée", "tranche",
    "botte", "brin", "feuille", "branche", "paquet", "pot", "verre",
}


def _nettoyer_unite(unite: str | None, aliment: str) -> str | None:
    """Le LLM (qwen2.5:3b) recopie parfois le nom de l'aliment dans "unite" au lieu de
    laisser le champ vide pour les ingrédients simplement comptés (ex: "1 orange"). On
    n'a pas réussi à éliminer ce biais par le prompt, donc on filtre après coup contre
    une liste d'unités de mesure reconnues plutôt que de faire confiance au modèle."""
    if not unite:
        return None
    normalized = unite.strip().lower()
    if normalized == aliment.strip().lower() or normalized not in UNITES_RECONNUES:
        return None
    return unite.strip()


async def structure_recipe(texte_ocr: str) -> RecipeDraft:
    prompt = PROMPT_TEMPLATE.format(texte=texte_ocr)

    async with httpx.AsyncClient(base_url=settings.ollama_base_url, timeout=120) as client:
        response = await client.post(
            "/api/generate",
            json={
                "model": settings.ollama_model,
                "prompt": prompt,
                "format": "json",
                "stream": False,
                "options": {"temperature": 0.2},
            },
        )
        response.raise_for_status()
        raw = response.json()["response"]

    data = json.loads(raw)
    draft = RecipeDraft.model_validate(data)
    for ingredient in draft.ingredients:
        ingredient.unite = _nettoyer_unite(ingredient.unite, ingredient.aliment)
    return draft
