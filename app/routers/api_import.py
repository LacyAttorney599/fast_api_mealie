from fastapi import APIRouter, UploadFile

from app.models.recipe import RecipeDraft
from app.services import llm, ocr

router = APIRouter(prefix="/api/import")


@router.post("/photo")
async def import_photo(image: UploadFile) -> RecipeDraft:
    image_bytes = await image.read()
    texte = ocr.extract_text(image_bytes)
    return await llm.structure_recipe(texte)
