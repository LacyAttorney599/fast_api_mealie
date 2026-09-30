from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from app.services import course_types as svc

router = APIRouter(prefix="/api/course-types")


class SetCourseTypePayload(BaseModel):
    category_name: str
    course_type: str | None  # None = reset to default


@router.get("")
def get_course_types() -> dict[str, str]:
    return svc.get_all()


@router.patch("")
def set_course_type(payload: SetCourseTypePayload) -> dict[str, str]:
    try:
        svc.set_category_type(payload.category_name, payload.course_type)
    except ValueError as e:
        raise HTTPException(status_code=422, detail=str(e))
    return svc.get_all()
