from fastapi import FastAPI

from app.routers import api_import, api_mealplan, api_recipes, api_seasons, api_shoppinglist

app = FastAPI(title="Recettes → Mealie")

app.include_router(api_recipes.router)
app.include_router(api_mealplan.router)
app.include_router(api_shoppinglist.router)
app.include_router(api_import.router)
app.include_router(api_seasons.router)
