from fastapi import APIRouter

from app.models.recipe import BulkImportResult, RecipeDetail, RecipeDraft, RecipeSummary
from app.services import mealie

router = APIRouter(prefix="/api")


@router.get("/recipes")
async def list_recipes(search: str = "") -> list[RecipeSummary]:
    return await mealie.list_recipe_summaries(search)


@router.get("/recipes/{slug}")
async def get_recipe(slug: str) -> RecipeDetail:
    return await mealie.get_recipe_detail(slug)


@router.post("/recipes", status_code=201)
async def create_recipe(draft: RecipeDraft) -> dict[str, str]:
    slug = await mealie.create_recipe(draft)
    return {"slug": slug}


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
