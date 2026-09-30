from contextlib import asynccontextmanager

import httpx

from app.config import settings

_active_base_url: str | None = None


async def get_active_base_url() -> str:
    """Retourne l'URL de base active pour Mealie.

    Au premier appel, teste si MEALIE_BASE_URL est joignable. Si la connexion
    échoue et qu'une MEALIE_FALLBACK_URL est configurée, bascule dessus et met
    le résultat en cache pour les appels suivants.
    """
    global _active_base_url
    if _active_base_url is not None:
        return _active_base_url

    if settings.mealie_fallback_url:
        try:
            async with httpx.AsyncClient(timeout=5) as probe:
                await probe.get(f"{settings.mealie_base_url}/api/about")
            _active_base_url = settings.mealie_base_url
        except (httpx.ConnectError, httpx.TimeoutException):
            _active_base_url = settings.mealie_fallback_url
    else:
        _active_base_url = settings.mealie_base_url

    return _active_base_url


@asynccontextmanager
async def get_client():
    base_url = await get_active_base_url()
    async with httpx.AsyncClient(
        base_url=base_url,
        headers={"Authorization": f"Bearer {settings.mealie_api_token}"},
        timeout=30,
    ) as client:
        yield client
