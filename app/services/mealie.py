import asyncio
import re
import time
import zlib
from datetime import date

import httpx

from app.config import settings
from app.models.recipe import (
    Cookbook,
    Ingredient,
    IngredientDisplay,
    RecipeCategory,
    RecipeDetail,
    RecipeDraft,
    RecipeSummary,
    SeasonalRecipe,
)
from app.services import seasons
from app.services.mealie_client import get_client as _client

# Mealie n'a pas de notion de couleur par recette : ces teintes reprennent
# la palette de la maquette pour les cartes sans photo, choisies par recette
# via un hash stable (pas de random : la couleur ne doit pas changer à
# chaque rechargement).
_PLACEHOLDER_COLORS = [
    "#E3B9A4", "#E7C7A6", "#C9D1B8", "#D9C9A0",
    "#B9C9B0", "#E4CDBF", "#DEC2A4", "#C7CDB0",
]


async def search_recipes(query: str = "", cookbook: str = "") -> list[dict]:
    params = {}
    if query:
        params["search"] = query
    if cookbook:
        params["cookbook"] = cookbook
    async with _client() as client:
        response = await client.get("/api/recipes", params=params or None)
        response.raise_for_status()
        return response.json().get("items", [])


async def list_cookbooks() -> list[Cookbook]:
    async with _client() as client:
        response = await client.get("/api/households/cookbooks")
        response.raise_for_status()
        items = response.json()["items"]
    return [
        Cookbook(
            slug=item["slug"],
            name=item["name"],
            manual=_parse_id_list_filter(item.get("queryFilterString", "")) is not None,
        )
        for item in items
    ]


async def list_recipe_categories() -> list[RecipeCategory]:
    async with _client() as client:
        response = await client.get("/api/organizers/categories")
        response.raise_for_status()
        items = response.json()["items"]
    return [RecipeCategory(id=item["id"], name=item["name"]) for item in items]


async def create_cookbook(name: str, category_id: str | None = None) -> Cookbook:
    """Deux types de livre : manuel (l'utilisateur choisit les recettes une à
    une, stocké comme un filtre `id IN [...]`) ou automatique par catégorie
    (filtre `recipe_category.id IN [...]`, se met à jour tout seul quand la
    catégorie d'une recette change). Un livre manuel démarre toujours avec un
    filtre explicite plutôt que vide — voir _build_id_list_filter."""
    query_filter = f'recipe_category.id IN ["{category_id}"]' if category_id else _build_id_list_filter([])

    async with _client() as client:
        response = await client.post(
            "/api/households/cookbooks",
            json={"name": name, "queryFilterString": query_filter},
        )
        response.raise_for_status()
        data = response.json()
    return Cookbook(slug=data["slug"], name=data["name"], manual=category_id is None)


_ID_LIST_PATTERN = re.compile(r"^id IN \[(.*)\]$")

# Mealie traite un `queryFilterString` vide comme "pas de filtre" = TOUTES les
# recettes (bug constaté : un livre manuel fraîchement créé sans recette
# affichait les 50 recettes de la base). Et `id IN []` (tableau vide) est
# rejeté par Mealie avec un 422. Un livre manuel vide utilise donc un id
# sentinelle qui ne correspond à aucune vraie recette.
_EMPTY_COOKBOOK_SENTINEL_ID = "00000000-0000-0000-0000-000000000000"


def _parse_id_list_filter(query_filter_string: str) -> list[str] | None:
    """Un livre géré par l'app utilise toujours un filtre `id IN [...]`
    (liste explicite de recettes). Si le filtre est vide, la liste est vide.
    Si c'est autre chose (catégorie, tag... comme "Répertoire des sauces",
    déjà présent côté foyer, ou un livre créé par catégorie depuis l'app), on
    ne sait pas le modifier sans risquer de le casser : on retourne None pour
    signaler "non gérable manuellement depuis l'app"."""
    stripped = (query_filter_string or "").strip()
    if not stripped:
        return []
    match = _ID_LIST_PATTERN.match(stripped)
    if not match:
        return None
    ids = re.findall(r'"([0-9a-fA-F-]{36})"', match.group(1))
    return [i for i in ids if i != _EMPTY_COOKBOOK_SENTINEL_ID]


def _build_id_list_filter(ids: list[str]) -> str:
    if not ids:
        return f'id IN ["{_EMPTY_COOKBOOK_SENTINEL_ID}"]'
    return "id IN [" + ", ".join(f'"{i}"' for i in ids) + "]"


async def _update_cookbook_recipe_ids(cookbook_slug: str, recipe_slug: str, *, add: bool) -> None:
    async with _client() as client:
        response = await client.get(f"/api/households/cookbooks/{cookbook_slug}")
        response.raise_for_status()
        cookbook = response.json()

        ids = _parse_id_list_filter(cookbook["queryFilterString"])
        if ids is None:
            raise ValueError(
                "Ce livre utilise un filtre personnalisé (catégorie, tag…) "
                "non modifiable depuis l'application."
            )

        recipe_response = await client.get(f"/api/recipes/{recipe_slug}")
        recipe_response.raise_for_status()
        recipe_id = recipe_response.json()["id"]

        if add:
            if recipe_id not in ids:
                ids.append(recipe_id)
        else:
            ids = [i for i in ids if i != recipe_id]

        cookbook["queryFilterString"] = _build_id_list_filter(ids)
        # PUT (et DELETE) exigent l'id UUID réel du cookbook dans l'URL, pas
        # son slug — contrairement à GET, qui accepte les deux. Utiliser le
        # slug ici fait planter Mealie avec un 500 (vérifié empiriquement :
        # erreur Postgres "invalid input syntax for type uuid").
        response = await client.put(f"/api/households/cookbooks/{cookbook['id']}", json=cookbook)
        response.raise_for_status()


async def add_recipe_to_cookbook(cookbook_slug: str, recipe_slug: str) -> None:
    await _update_cookbook_recipe_ids(cookbook_slug, recipe_slug, add=True)


async def remove_recipe_from_cookbook(cookbook_slug: str, recipe_slug: str) -> None:
    await _update_cookbook_recipe_ids(cookbook_slug, recipe_slug, add=False)


def _placeholder_color(slug: str) -> str:
    return _PLACEHOLDER_COLORS[zlib.crc32(slug.encode()) % len(_PLACEHOLDER_COLORS)]


def _to_summary(item: dict) -> RecipeSummary:
    category = item["recipeCategory"][0]["name"] if item["recipeCategory"] else None
    tag = category or (item["tags"][0]["name"] if item["tags"] else None)

    image_url = None
    if item["image"]:
        image_url = f"{settings.mealie_base_url}/api/media/recipes/{item['id']}/images/min-original.webp"

    return RecipeSummary(
        slug=item["slug"],
        name=item["name"],
        time=item["totalTime"] or item["prepTime"],
        tag=tag,
        image_url=image_url,
        color=_placeholder_color(item["slug"]),
    )


async def list_recipe_summaries(query: str = "", cookbook: str = "") -> list[RecipeSummary]:
    items = await search_recipes(query, cookbook)
    return [_to_summary(item) for item in items]


async def fetch_recipe(slug: str) -> dict:
    async with _client() as client:
        response = await client.get(f"/api/recipes/{slug}")
        response.raise_for_status()
        return response.json()


def _format_quantity(quantity: float | None, unit: dict | None) -> str:
    if not quantity:
        return ""
    qty_str = f"{quantity:g}".replace(".", ",")
    if unit and unit.get("name"):
        return f"{qty_str} {unit['name']}"
    return qty_str


def _to_ingredient_display(ing: dict) -> IngredientDisplay:
    food = ing.get("food")
    if food:
        return IngredientDisplay(qty=_format_quantity(ing.get("quantity"), ing.get("unit")), food=food["name"])
    # Ingrédient non structuré (ex: recette importée par URL sans parsing fiable) :
    # pas de food/unit à afficher séparément, on retombe sur le texte brut.
    return IngredientDisplay(qty="", food=ing.get("note") or ing.get("display") or "")


def _format_servings(value: float | None) -> str | None:
    if not value:
        return None
    n = int(value) if value == int(value) else value
    return f"{n} personne{'s' if n != 1 else ''}"


async def get_recipe_detail(slug: str) -> RecipeDetail:
    item = await fetch_recipe(slug)

    image_url = None
    if item["image"]:
        image_url = f"{settings.mealie_base_url}/api/media/recipes/{item['id']}/images/original.webp"

    all_tags = [c["name"] for c in item["recipeCategory"]] + [t["name"] for t in item["tags"]]
    tags = list(dict.fromkeys(all_tags))  # dédoublonne en gardant l'ordre (catégorie et tag peuvent se recouper)

    ingredients = [_to_ingredient_display(ing) for ing in item["recipeIngredient"]]
    produce = seasons.current_month_produce()
    in_season = any(seasons.is_seasonal(ing.food, produce) for ing in ingredients)

    return RecipeDetail(
        slug=item["slug"],
        name=item["name"],
        description=item["description"] or "",
        image_url=image_url,
        time=item["totalTime"] or item["prepTime"],
        servings=_format_servings(item["recipeServings"]),
        tags=tags,
        ingredients=ingredients,
        steps=[step["text"] for step in item["recipeInstructions"] if step["text"].strip()],
        in_season=in_season,
    )


_SEASONAL_CACHE_TTL = 3600  # secondes
_seasonal_cache: dict[int, list[SeasonalRecipe]] = {}
_seasonal_cache_at: dict[int, float] = {}
_seasonal_cache_lock = asyncio.Lock()
_SEASONAL_CONCURRENCY = 6


async def _compute_recipes_with_season() -> list[SeasonalRecipe]:
    """Un appel détail par recette (la liste Mealie ne renvoie pas les
    ingrédients). Vérifié empiriquement : tirer les appels tous en même temps
    (asyncio.gather sans limite) fait planter Mealie sur ~36 recettes — une
    requête finit par expirer (httpx.ReadTimeout) et fait échouer tout le lot.
    D'où le sémaphore, qui borne le nombre de requêtes concurrentes envoyées
    à Mealie."""
    summaries = await list_recipe_summaries()
    semaphore = asyncio.Semaphore(_SEASONAL_CONCURRENCY)

    async def check(summary: RecipeSummary) -> SeasonalRecipe:
        async with semaphore:
            detail = await get_recipe_detail(summary.slug)
        return SeasonalRecipe(slug=summary.slug, name=summary.name, in_season=detail.in_season)

    return list(await asyncio.gather(*(check(s) for s in summaries)))


async def list_recipes_with_season() -> list[SeasonalRecipe]:
    """Mis en cache (1h, par mois) : même borné en concurrence, calculer ceci
    pour chaque recette reste coûteux (un appel détail par recette, la liste
    Mealie ne renvoie pas les ingrédients) pour être refait à chaque affichage
    d'un écran qui en a besoin (Liste, Planificateur, génération IA)."""
    month = date.today().month
    now = time.monotonic()
    cached_at = _seasonal_cache_at.get(month)
    if cached_at is not None and (now - cached_at) < _SEASONAL_CACHE_TTL:
        return _seasonal_cache[month]

    async with _seasonal_cache_lock:
        # Un autre appel a pu remplir le cache pendant qu'on attendait le verrou.
        cached_at = _seasonal_cache_at.get(month)
        if cached_at is not None and (time.monotonic() - cached_at) < _SEASONAL_CACHE_TTL:
            return _seasonal_cache[month]

        result = await _compute_recipes_with_season()
        _seasonal_cache[month] = result
        _seasonal_cache_at[month] = time.monotonic()
        return result


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


async def _apply_draft(client: httpx.AsyncClient, recipe: dict, draft: RecipeDraft) -> dict:
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
    recipe["name"] = draft.nom
    recipe["recipeIngredient"] = recipe_ingredients
    recipe["recipeInstructions"] = [{"text": etape} for etape in draft.etapes]
    return recipe


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
        recipe = await _apply_draft(client, response.json(), draft)

        response = await client.put(f"/api/recipes/{slug}", json=recipe)
        response.raise_for_status()
        return slug


async def get_recipe_draft(slug: str) -> RecipeDraft:
    """Forme éditable (quantite/unite/aliment séparés) pour le formulaire de
    modification — à la différence de RecipeDetail, qui combine quantité et
    unité en une seule chaîne d'affichage impossible à re-découper de façon
    fiable (ex: "1,5 kg" -> 1.5 + "kg" n'est pas trivial à inverser)."""
    item = await fetch_recipe(slug)
    ingredients = []
    for ing in item["recipeIngredient"]:
        food = ing.get("food")
        unit = ing.get("unit")
        aliment = food["name"] if food else (ing.get("note") or ing.get("display") or "")
        ingredients.append(Ingredient(quantite=ing.get("quantity"), unite=unit["name"] if unit else None, aliment=aliment))

    return RecipeDraft(
        nom=item["name"],
        ingredients=ingredients,
        etapes=[step["text"] for step in item["recipeInstructions"] if step["text"].strip()],
    )


async def update_recipe(slug: str, draft: RecipeDraft) -> str:
    """Met à jour une recette existante et retourne son slug à jour — Mealie
    re-génère le slug à partir du nom quand celui-ci change (vérifié
    empiriquement), donc le slug de retour peut différer de celui d'entrée."""
    async with _client() as client:
        response = await client.get(f"/api/recipes/{slug}")
        response.raise_for_status()
        recipe = await _apply_draft(client, response.json(), draft)

        response = await client.put(f"/api/recipes/{slug}", json=recipe)
        response.raise_for_status()
        return response.json()["slug"]
