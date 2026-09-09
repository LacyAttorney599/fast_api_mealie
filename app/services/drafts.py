import uuid

from app.models.recipe import RecipeDraft

_STORE: dict[str, RecipeDraft] = {}


def save_draft(draft: RecipeDraft) -> str:
    draft_id = uuid.uuid4().hex
    _STORE[draft_id] = draft
    return draft_id


def pop_draft(draft_id: str) -> RecipeDraft | None:
    return _STORE.pop(draft_id, None)
