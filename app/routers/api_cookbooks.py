from fastapi import APIRouter, HTTPException

from app.models.recipe import Cookbook, CreateCookbook
from app.services import mealie

router = APIRouter(prefix="/api/cookbooks")


@router.get("")
async def list_cookbooks() -> list[Cookbook]:
    return await mealie.list_cookbooks()


@router.post("", status_code=201)
async def create_cookbook(payload: CreateCookbook) -> Cookbook:
    return await mealie.create_cookbook(payload.name)


@router.post("/{cookbook_slug}/recipes/{recipe_slug}", status_code=204)
async def add_recipe_to_cookbook(cookbook_slug: str, recipe_slug: str) -> None:
    try:
        await mealie.add_recipe_to_cookbook(cookbook_slug, recipe_slug)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@router.delete("/{cookbook_slug}/recipes/{recipe_slug}", status_code=204)
async def remove_recipe_from_cookbook(cookbook_slug: str, recipe_slug: str) -> None:
    try:
        await mealie.remove_recipe_from_cookbook(cookbook_slug, recipe_slug)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
