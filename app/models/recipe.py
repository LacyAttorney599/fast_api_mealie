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
    is_favorite: bool


class IngredientDisplay(BaseModel):
    """Quantité et unité brutes (pas de texte déjà formaté) : le SPA doit
    pouvoir recalculer l'affichage quand l'utilisateur change le nombre de
    parts."""

    quantity: float | None
    unit: str | None
    food: str


class RecipeDetail(BaseModel):
    """Forme renvoyée par le BFF au SPA pour l'écran Détail."""

    slug: str
    name: str
    description: str
    image_url: str | None
    time: str | None
    servings: float | None
    tags: list[str]
    ingredients: list[IngredientDisplay]
    steps: list[str]
    in_season: bool


class BulkImportResult(BaseModel):
    nom: str
    success: bool
    slug: str | None = None
    error: str | None = None


class SeasonalRecipe(BaseModel):
    slug: str
    name: str
    in_season: bool


class Cookbook(BaseModel):
    slug: str
    name: str
    manual: bool = True


class CreateCookbook(BaseModel):
    name: str
    category_id: str | None = None


class RecipeCategory(BaseModel):
    id: str
    name: str
