import httpx

from app.models.shoppinglist import ShoppingCategory, ShoppingItem, ShoppingList
from app.services import mealplan
from app.services.mealie_client import get_client


async def _get_or_create_list_id(client: httpx.AsyncClient) -> str:
    response = await client.get("/api/households/shopping/lists")
    response.raise_for_status()
    items = response.json()["items"]
    if items:
        return items[0]["id"]

    response = await client.post("/api/households/shopping/lists", json={"name": "Courses"})
    response.raise_for_status()
    return response.json()["id"]


def _format_quantity(item: dict) -> str:
    quantity = item.get("quantity") or 0
    if not quantity:
        return ""
    qty_str = f"{quantity:g}".replace(".", ",")
    unit = item.get("unit")
    if unit:
        name = unit.get("pluralName") if quantity > 1 and unit.get("pluralName") else unit.get("name")
        if name:
            return f"{qty_str} {name}"
    return qty_str


def _to_shopping_item(item: dict) -> ShoppingItem:
    food = item.get("food")
    name = food["name"] if food else (item.get("note") or item.get("display") or "")
    label = item.get("label")
    return ShoppingItem(
        id=item["id"],
        quantity=_format_quantity(item),
        food=name,
        checked=item["checked"],
        category=label["name"] if label else "Autres",
    )


async def get_shopping_list() -> ShoppingList:
    async with get_client() as client:
        list_id = await _get_or_create_list_id(client)
        response = await client.get(f"/api/households/shopping/lists/{list_id}")
        response.raise_for_status()
        data = response.json()

    categories: dict[str, list[ShoppingItem]] = {}
    for raw in data["listItems"]:
        item = _to_shopping_item(raw)
        categories.setdefault(item.category, []).append(item)

    recipe_ids = {ref["recipeId"] for ref in data["recipeReferences"]}

    return ShoppingList(
        categories=[ShoppingCategory(name=name, items=items) for name, items in categories.items()],
        total_items=len(data["listItems"]),
        recipe_count=len(recipe_ids),
    )


async def toggle_item(item_id: str, checked: bool) -> None:
    async with get_client() as client:
        response = await client.get(f"/api/households/shopping/items/{item_id}")
        response.raise_for_status()
        item = response.json()
        item["checked"] = checked
        response = await client.put(f"/api/households/shopping/items/{item_id}", json=item)
        response.raise_for_status()


async def add_item(text: str) -> None:
    async with get_client() as client:
        list_id = await _get_or_create_list_id(client)
        response = await client.post(
            "/api/households/shopping/items",
            json={"shoppingListId": list_id, "note": text, "display": text},
        )
        response.raise_for_status()


async def clear_checked() -> None:
    async with get_client() as client:
        list_id = await _get_or_create_list_id(client)
        response = await client.get(f"/api/households/shopping/lists/{list_id}")
        response.raise_for_status()
        checked_ids = [item["id"] for item in response.json()["listItems"] if item["checked"]]

        for item_id in checked_ids:
            response = await client.delete(f"/api/households/shopping/items/{item_id}")
            response.raise_for_status()


async def generate_from_mealplan(start: str, end: str) -> None:
    """Ajoute à la liste les ingrédients de chaque recette planifiée sur la
    période (une seule fois par recette même si planifiée plusieurs fois dans
    la semaine). Mealie fusionne lui-même les quantités des ingrédients déjà
    présents dans la liste, donc pas de logique d'agrégation à réécrire ici.
    """
    entries = await mealplan.list_entries(start, end)
    recipe_slugs = {entry.recipe_slug for entry in entries if entry.recipe_slug}

    async with get_client() as client:
        list_id = await _get_or_create_list_id(client)

        list_response = await client.get(f"/api/households/shopping/lists/{list_id}")
        list_response.raise_for_status()
        already_added = {ref["recipeId"] for ref in list_response.json()["recipeReferences"]}

        for slug in recipe_slugs:
            recipe_response = await client.get(f"/api/recipes/{slug}")
            recipe_response.raise_for_status()
            recipe_id = recipe_response.json()["id"]
            if recipe_id in already_added:
                continue

            response = await client.post(f"/api/households/shopping/lists/{list_id}/recipe/{recipe_id}", json={})
            response.raise_for_status()
