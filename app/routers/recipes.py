from fastapi import APIRouter, Form, Request
from fastapi.responses import RedirectResponse

from app.models.recipe import Ingredient, RecipeDraft
from app.services import mealie
from app.services.drafts import pop_draft
from app.templating import templates

router = APIRouter()


@router.get("/")
async def liste(request: Request, q: str = ""):
    recettes = await mealie.search_recipes(q)
    return templates.TemplateResponse(
        request, "liste.html", {"recettes": recettes, "q": q}
    )


@router.get("/ajouter")
async def ajouter_formulaire(request: Request, draft_id: str | None = None):
    draft = pop_draft(draft_id) if draft_id else None
    return templates.TemplateResponse(request, "ajouter.html", {"draft": draft})


@router.post("/recipes")
async def envoyer_recette(
    request: Request,
    nom: str = Form(...),
    quantite: list[str] = Form([]),
    unite: list[str] = Form([]),
    aliment: list[str] = Form([]),
    etape: list[str] = Form([]),
):
    ingredients = [
        Ingredient(
            quantite=float(q) if q else None,
            unite=u or None,
            aliment=a,
        )
        for q, u, a in zip(quantite, unite, aliment)
        if a.strip()
    ]
    etapes = [e for e in etape if e.strip()]

    draft = RecipeDraft(nom=nom, ingredients=ingredients, etapes=etapes)
    slug = await mealie.create_recipe(draft)

    return RedirectResponse(url=f"/?ajoute={slug}", status_code=303)
