from pydantic import BaseModel


class ShoppingItem(BaseModel):
    id: str
    quantity: str
    food: str
    checked: bool
    category: str


class ShoppingCategory(BaseModel):
    name: str
    items: list[ShoppingItem]


class ShoppingList(BaseModel):
    categories: list[ShoppingCategory]
    total_items: int
    recipe_count: int


class CreateShoppingItem(BaseModel):
    text: str


class ToggleShoppingItem(BaseModel):
    checked: bool


class GenerateShoppingList(BaseModel):
    start: str
    end: str
