from fastapi import APIRouter

from app.models.mealplan import CreateMealPlanEntry, MealPlanEntry
from app.services import mealplan

router = APIRouter(prefix="/api/mealplan")


@router.get("")
async def list_mealplan(start: str, end: str) -> list[MealPlanEntry]:
    return await mealplan.list_entries(start, end)


@router.post("")
async def create_mealplan_entry(payload: CreateMealPlanEntry) -> MealPlanEntry:
    return await mealplan.create_entry(payload)


@router.delete("/{entry_id}", status_code=204)
async def delete_mealplan_entry(entry_id: int) -> None:
    await mealplan.delete_entry(entry_id)
