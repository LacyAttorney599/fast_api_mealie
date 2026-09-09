import httpx

from app.config import settings
from app.models.recipe import RecipeDraft


def _client() -> httpx.AsyncClient:
    return httpx.AsyncClient(
        base_url=settings.mealie_base_url,
        headers={"Authorization": f"Bearer {settings.mealie_api_token}"},
        timeout=30,
    )


async def search_recipes(query: str = "") -> list[dict]:
    async with _client() as client:
        response = await client.get("/api/recipes", params={"search": query} if query else None)
        response.raise_for_status()
        return response.json().get("items", [])


async def _get_or_create_id(client: httpx.AsyncClient, endpoint: str, name: str) -> str:
    """Les endpoints /api/foods et /api/units n'acceptent pas de doublon de nom
    (contrainte unique côté Mealie) : on cherche d'abord une correspondance
    exacte avant de créer. On ne peut pas non plus se contenter de laisser
    Mealie créer le food/unit à la volée depuis recipeIngredient : passer un
    nom sans id existant y fait planter l'API (ValueError) sur cette version."""
    response = await client.get(endpoint, params={"search": name})
    response.raise_for_status()
    for item in response.json()["items"]:
        if item["name"].strip().lower() == name.strip().lower():
            return item["id"]

    response = await client.post(endpoint, json={"name": name})
    response.raise_for_status()
    return response.json()["id"]


async def create_recipe(draft: RecipeDraft) -> str:
    """Crée la recette dans Mealie et retourne son slug.

    Mealie crée d'abord la recette avec son nom, puis on la met à jour avec les
    ingrédients et étapes. L'API PUT/PATCH de Mealie n'applique pas de fusion
    partielle : le corps envoyé remplace l'objet recette en entier (les champs
    absents repartent à leur valeur par défaut), donc on récupère d'abord la
    recette telle que créée par Mealie et on ne modifie que ce qui nous
    intéresse avant de la renvoyer complète.
    """
    async with _client() as client:
        response = await client.post("/api/recipes", json={"name": draft.nom})
        response.raise_for_status()
        slug = response.json()

        response = await client.get(f"/api/recipes/{slug}")
        response.raise_for_status()
        recipe = response.json()

        recipe_ingredients = []
        for ingredient in draft.ingredients:
            food_id = await _get_or_create_id(client, "/api/foods", ingredient.aliment)
            unit = None
            if ingredient.unite:
                unit_id = await _get_or_create_id(client, "/api/units", ingredient.unite)
                unit = {"id": unit_id, "name": ingredient.unite}
            recipe_ingredients.append(
                {
                    "quantity": ingredient.quantite,
                    "unit": unit,
                    "food": {"id": food_id, "name": ingredient.aliment},
                    "note": "",
                }
            )
        recipe["recipeIngredient"] = recipe_ingredients
        recipe["recipeInstructions"] = [{"text": etape} for etape in draft.etapes]

        response = await client.put(f"/api/recipes/{slug}", json=recipe)
        response.raise_for_status()
        return slug
