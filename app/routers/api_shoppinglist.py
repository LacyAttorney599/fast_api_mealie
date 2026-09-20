from fastapi import APIRouter

from app.models.shoppinglist import CreateShoppingItem, GenerateShoppingList, ShoppingList, ToggleShoppingItem
from app.services import shoppinglist

router = APIRouter(prefix="/api/shoppinglist")


@router.get("")
async def get_shopping_list() -> ShoppingList:
    return await shoppinglist.get_shopping_list()


@router.post("/items", status_code=201)
async def add_item(payload: CreateShoppingItem) -> None:
    await shoppinglist.add_item(payload.text)


@router.patch("/items/{item_id}", status_code=204)
async def toggle_item(item_id: str, payload: ToggleShoppingItem) -> None:
    await shoppinglist.toggle_item(item_id, payload.checked)


@router.post("/clear-checked", status_code=204)
async def clear_checked() -> None:
    await shoppinglist.clear_checked()


@router.post("/generate", status_code=204)
async def generate(payload: GenerateShoppingList) -> None:
    await shoppinglist.generate_from_mealplan(payload.start, payload.end)
