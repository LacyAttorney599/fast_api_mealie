import httpx

from app.config import settings


def get_client() -> httpx.AsyncClient:
    return httpx.AsyncClient(
        base_url=settings.mealie_base_url,
        headers={"Authorization": f"Bearer {settings.mealie_api_token}"},
        timeout=30,
    )
