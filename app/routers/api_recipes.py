from fastapi import APIRouter

from app.models.recipe import RecipeDetail, RecipeDraft, RecipeSummary
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
