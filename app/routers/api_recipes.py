from fastapi import APIRouter

from app.models.recipe import RecipeSummary
from app.services import mealie

router = APIRouter(prefix="/api")


@router.get("/recipes")
async def list_recipes(search: str = "") -> list[RecipeSummary]:
    return await mealie.list_recipe_summaries(search)
