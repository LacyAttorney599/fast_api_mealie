from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles

from app.routers import api_mealplan, api_recipes, api_shoppinglist, photo, recipes

app = FastAPI(title="Recettes → Mealie")

app.mount("/static", StaticFiles(directory="app/static"), name="static")

app.include_router(api_recipes.router)
app.include_router(api_mealplan.router)
app.include_router(api_shoppinglist.router)
app.include_router(recipes.router)
app.include_router(photo.router)
