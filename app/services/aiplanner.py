import json
import random
import re
from datetime import date, timedelta

import httpx

from app.config import settings
from app.models.mealplan import CreateMealPlanEntry, MealPlanEntry
from app.services import mealie, mealplan
from app.services.course_types import is_plannable

MEAL_TYPES = ["lunch", "dinner"]

# Limite le nombre de recettes envoyées au LLM pour garder un prompt
# raisonnable sur les petits modèles (qwen2.5:3b, etc.).
_MAX_RECIPES_IN_PROMPT = 40

PROMPT_TEMPLATE = """Tu planifies les repas d'une semaine. Réponds UNIQUEMENT avec du JSON valide.

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
"""


async def _call_ollama(prompt: str) -> dict:
    async with httpx.AsyncClient(base_url=settings.ollama_base_url, timeout=240) as client:
        response = await client.post(
            "/api/generate",
            json={
                "model": settings.ollama_model,
                "prompt": prompt,
                "format": "json",
                "stream": False,
                "options": {
                    "temperature": 0.3,
                    "num_ctx": 8192,
                },
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
    """Remplit les créneaux vides de la période avec des recettes choisies par
    le LLM. Utilise list_recipe_summaries (1 seul appel API) plutôt que
    list_recipes_with_season (N appels) pour éviter les timeouts sur la première
    génération. Un nom halluciné ou un créneau invalide dans la réponse du
    modèle est simplement ignoré."""
    summaries = await mealie.list_recipe_summaries()
    if not summaries:
        return []

    existing = await mealplan.list_entries(start, end)
    slots = _empty_slots(start, end, existing)
    if not slots:
        return []

    # Filtre les recettes non planifiables (desserts, accompagnements).
    summaries = [r for r in summaries if is_plannable(r.tag)]

    # Échantillon aléatoire mais stable pour la semaine courante.
    # On en prend suffisamment pour couvrir les créneaux avec de la variété.
    available = list(summaries)
    n_needed = min(_MAX_RECIPES_IN_PROMPT, max(len(slots) * 2, 20))
    if len(available) > n_needed:
        seed = int(date.fromisoformat(start).strftime("%Y%W"))
        rng = random.Random(seed)
        available = rng.sample(available, n_needed)
    available.sort(key=lambda r: r.name)

    recipes_by_name = {r.name: r for r in available}
    recipe_lines = "\n".join(
        f"- {r.name}" + (f" [{r.tag}]" if r.tag else "")
        for r in available
    )
    slot_lines = "\n".join(f"- {iso} {meal_type}" for iso, meal_type in slots)

    prompt = PROMPT_TEMPLATE.format(
        recipes=recipe_lines,
        slots=slot_lines,
        n_slots=len(slots),
    )
    data = await _call_ollama(prompt)

    created: list[MealPlanEntry] = []
    remaining_slots = set(slots)
    for assignment in data.get("assignments", []):
        # Accepte snake_case ("entry_type") et camelCase ("entryType")
        entry_type = assignment.get("entry_type") or assignment.get("entryType")
        slot = (assignment.get("date"), entry_type)
        if slot not in remaining_slots:
            continue

        # Le LLM répète parfois le suffixe "[Catégorie]" qu'on a mis dans la
        # liste — on le retire avant la lookup.
        raw_name = assignment.get("recipe", "")
        clean_name = re.sub(r"\s*\[.*?\]\s*$", "", raw_name).strip()
        recipe = recipes_by_name.get(clean_name)
        if not recipe:
            continue

        entry = await mealplan.create_entry(
            CreateMealPlanEntry(date=slot[0], entry_type=slot[1], recipe_slug=recipe.slug)
        )
        created.append(entry)
        remaining_slots.discard(slot)

    return created
