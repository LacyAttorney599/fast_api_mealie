from fastapi import APIRouter, Request, Response, UploadFile

from app.services import llm, ocr
from app.services.drafts import save_draft
from app.templating import templates

router = APIRouter()


@router.get("/photo")
async def photo_formulaire(request: Request):
    return templates.TemplateResponse(request, "photo.html", {})


@router.post("/import/photo")
async def importer_photo(request: Request, image: UploadFile):
    image_bytes = await image.read()

    texte = ocr.extract_text(image_bytes)
    draft = await llm.structure_recipe(texte)
    draft_id = save_draft(draft)

    # 200 + HX-Redirect (pas un vrai 3xx) : sinon le navigateur suit la
    # redirection avant que htmx ne puisse lire l'en-tête, et le HTML de
    # la page suivie finit injecté dans le petit div de statut.
    response = Response(status_code=200)
    response.headers["HX-Redirect"] = f"/ajouter?draft_id={draft_id}"
    return response
