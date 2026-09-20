from fastapi import APIRouter

from app.models.recipe import BulkImportResult, RecipeDetail, RecipeDraft, RecipeSummary, SeasonalRecipe
from app.services import mealie

router = APIRouter(prefix="/api")


@router.get("/recipes")
async def list_recipes(search: str = "", cookbook: str = "") -> list[RecipeSummary]:
    return await mealie.list_recipe_summaries(search, cookbook)


# Doit être déclaré avant /recipes/{slug} : sinon FastAPI matcherait
# "seasonal" comme une valeur de slug.
@router.get("/recipes/seasonal")
async def list_recipes_seasonal() -> list[SeasonalRecipe]:
    return await mealie.list_recipes_with_season()


@router.get("/recipes/{slug}")
async def get_recipe(slug: str) -> RecipeDetail:
    return await mealie.get_recipe_detail(slug)


@router.get("/recipes/{slug}/edit")
async def get_recipe_for_edit(slug: str) -> RecipeDraft:
    return await mealie.get_recipe_draft(slug)


@router.post("/recipes", status_code=201)
async def create_recipe(draft: RecipeDraft) -> dict[str, str]:
    slug = await mealie.create_recipe(draft)
    return {"slug": slug}


@router.put("/recipes/{slug}")
async def update_recipe(slug: str, draft: RecipeDraft) -> dict[str, str]:
    new_slug = await mealie.update_recipe(slug, draft)
    return {"slug": new_slug}


@router.post("/recipes/bulk")
async def create_recipes_bulk(drafts: list[RecipeDraft]) -> list[BulkImportResult]:
    """Import groupé (ex: livre de recettes extrait en JSON par Claude).
    Séquentiel plutôt que parallèle : la résolution food/unit de chaque
    recette fait plusieurs allers-retours vers Mealie, et des créations
    concurrentes du même food/unit inconnu risqueraient de se marcher
    dessus (contrainte d'unicité déjà rencontrée en Phase 1). Une recette
    en échec n'interrompt pas les suivantes.
    """
    results = []
    for draft in drafts:
        try:
            slug = await mealie.create_recipe(draft)
            results.append(BulkImportResult(nom=draft.nom, success=True, slug=slug))
        except Exception as exc:
            results.append(BulkImportResult(nom=draft.nom, success=False, error=str(exc)))
    return results
