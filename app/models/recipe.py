from pydantic import BaseModel


class Ingredient(BaseModel):
    quantite: float | None = None
    unite: str | None = None
    aliment: str


class RecipeDraft(BaseModel):
    nom: str
    ingredients: list[Ingredient] = []
    etapes: list[str] = []


class RecipeSummary(BaseModel):
    """Forme renvoyée par le BFF au SPA pour l'écran Liste — un sous-ensemble
    aplati de l'objet recette Mealie, adapté à la carte de la maquette."""

    slug: str
    name: str
    time: str | None
    tag: str | None
    image_url: str | None
    color: str
