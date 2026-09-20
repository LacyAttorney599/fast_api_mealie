from datetime import date

from fastapi import APIRouter
from pydantic import BaseModel

from app.services import seasons

router = APIRouter(prefix="/api/seasons")


class CurrentSeason(BaseModel):
    month: int
    produce: list[str]


@router.get("/current")
async def get_current_season() -> CurrentSeason:
    today = date.today()
    return CurrentSeason(month=today.month, produce=seasons.current_month_produce(today))
