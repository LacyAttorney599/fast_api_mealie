from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles

from app.routers import photo, recipes

app = FastAPI(title="Recettes → Mealie")

app.mount("/static", StaticFiles(directory="app/static"), name="static")

app.include_router(recipes.router)
app.include_router(photo.router)
