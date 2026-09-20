from pydantic import BaseModel


class MealPlanEntry(BaseModel):
    id: int
    date: str
    entry_type: str
    name: str
    recipe_slug: str | None


class CreateMealPlanEntry(BaseModel):
    date: str
    entry_type: str
    recipe_slug: str
