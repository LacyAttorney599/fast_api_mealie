from pydantic import BaseModel


class Ingredient(BaseModel):
    quantite: float | None = None
    unite: str | None = None
    aliment: str


class RecipeDraft(BaseModel):
    nom: str
    ingredients: list[Ingredient] = []
    etapes: list[str] = []
