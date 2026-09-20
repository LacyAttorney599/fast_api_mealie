import json
from datetime import date, timedelta

import httpx

from app.config import settings
from app.models.mealplan import CreateMealPlanEntry, MealPlanEntry
from app.services import mealie, mealplan

MEAL_TYPES = ["lunch", "dinner"]

PROMPT_TEMPLATE = """Tu planifies les repas d'une semaine à partir d'une liste de recettes existantes.

Recettes disponibles (🌱 = de saison ce mois-ci, à privilégier si pertinent) :
{recipes}

Créneaux à remplir (une recette par créneau) :
{slots}

Règles :
- Utilise UNIQUEMENT les noms de recettes listés ci-dessus, exactement tels qu'écrits.
- Remplis TOUS les créneaux listés.
- Varie les recettes : évite de répéter la même recette plus de 2 fois dans la semaine si le nombre de recettes disponibles le permet.
- Privilégie les recettes de saison quand plusieurs choix sont raisonnables, sans que ce soit une contrainte stricte.

Réponds UNIQUEMENT avec un JSON de cette forme, sans texte autour :
{{"assignments": [{{"date": "YYYY-MM-DD", "entryType": "lunch ou dinner", "recipe": "nom exact d'une recette ci-dessus"}}]}}
"""


async def _call_ollama(prompt: str) -> dict:
    async with httpx.AsyncClient(base_url=settings.ollama_base_url, timeout=180) as client:
        response = await client.post(
            "/api/generate",
            json={
                "model": settings.ollama_model,
                "prompt": prompt,
                "format": "json",
                "stream": False,
                "options": {"temperature": 0.4},
            },
        )
        response.raise_for_status()
        raw = response.json()["response"]
    return json.loads(raw)


def _empty_slots(start: str, end: str, existing: list[MealPlanEntry]) -> list[tuple[str, str]]:
    filled = {(entry.date, entry.entry_type) for entry in existing}
    start_date = date.fromisoformat(start)
    end_date = date.fromisoformat(end)

    slots = []
    current = start_date
    while current <= end_date:
        iso = current.isoformat()
        for meal_type in MEAL_TYPES:
            if (iso, meal_type) not in filled:
                slots.append((iso, meal_type))
        current += timedelta(days=1)
    return slots


async def generate_plan(start: str, end: str) -> list[MealPlanEntry]:
    """Remplit les créneaux vides de la période avec des recettes existantes,
    choisies par le LLM en tenant compte de la saison et de la variété.
    Ne touche jamais un créneau déjà rempli. Un nom de recette halluciné (hors
    de la liste fournie) ou un créneau invalide dans la réponse du modèle est
    simplement ignoré plutôt que de faire planter la génération.
    """
    recipes = await mealie.list_recipes_with_season()
    if not recipes:
        return []

    existing = await mealplan.list_entries(start, end)
    slots = _empty_slots(start, end, existing)
    if not slots:
        return []

    recipes_by_name = {recipe.name: recipe for recipe in recipes}
    recipe_lines = "\n".join(f"- {'🌱 ' if recipe.in_season else ''}{recipe.name}" for recipe in recipes)
    slot_lines = "\n".join(f"- {iso} ({meal_type})" for iso, meal_type in slots)

    prompt = PROMPT_TEMPLATE.format(recipes=recipe_lines, slots=slot_lines)
    data = await _call_ollama(prompt)

    created: list[MealPlanEntry] = []
    remaining_slots = set(slots)
    for assignment in data.get("assignments", []):
        slot = (assignment.get("date"), assignment.get("entryType"))
        if slot not in remaining_slots:
            continue

        recipe = recipes_by_name.get(assignment.get("recipe"))
        if not recipe:
            continue

        entry = await mealplan.create_entry(
            CreateMealPlanEntry(date=slot[0], entry_type=slot[1], recipe_slug=recipe.slug)
        )
        created.append(entry)
        remaining_slots.discard(slot)

    return created
