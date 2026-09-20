from app.models.mealplan import CreateMealPlanEntry, MealPlanEntry
from app.services.mealie_client import get_client


def _to_entry(item: dict) -> MealPlanEntry:
    recipe = item.get("recipe")
    name = recipe["name"] if recipe else (item.get("title") or item.get("text") or "")
    return MealPlanEntry(
        id=item["id"],
        date=item["date"],
        entry_type=item["entryType"],
        name=name,
        recipe_slug=recipe["slug"] if recipe else None,
    )


async def list_entries(start: str, end: str) -> list[MealPlanEntry]:
    async with get_client() as client:
        response = await client.get(
            "/api/households/mealplans",
            params={"start_date": start, "end_date": end, "perPage": 100},
        )
        response.raise_for_status()
        items = response.json()["items"]
    return [_to_entry(item) for item in items]


async def create_entry(payload: CreateMealPlanEntry) -> MealPlanEntry:
    async with get_client() as client:
        recipe_response = await client.get(f"/api/recipes/{payload.recipe_slug}")
        recipe_response.raise_for_status()
        recipe_id = recipe_response.json()["id"]

        response = await client.post(
            "/api/households/mealplans",
            json={"date": payload.date, "entryType": payload.entry_type, "recipeId": recipe_id},
        )
        response.raise_for_status()
        return _to_entry(response.json())


async def delete_entry(entry_id: int) -> None:
    async with get_client() as client:
        response = await client.delete(f"/api/households/mealplans/{entry_id}")
        response.raise_for_status()
