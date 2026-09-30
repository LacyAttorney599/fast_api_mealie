"""Persistance locale du type de service (plat, dessert, accompagnement, entrée)
par nom de catégorie Mealie. Stocké dans app/data/course_types.json."""

import json
from pathlib import Path

_DATA_FILE = Path(__file__).parent.parent / "data" / "course_types.json"

# Types valides
COURSE_TYPES = {"plat", "entrée", "dessert", "accompagnement"}

# Types exclus de la génération IA (ne doivent pas être mis en plat principal)
EXCLUDED_FROM_PLANNER = {"dessert", "accompagnement"}


def _load() -> dict[str, str]:
    try:
        return json.loads(_DATA_FILE.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def _save(data: dict[str, str]) -> None:
    _DATA_FILE.parent.mkdir(parents=True, exist_ok=True)
    _DATA_FILE.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def get_all() -> dict[str, str]:
    return _load()


def set_category_type(category_name: str, course_type: str | None) -> None:
    """Associe un type de service à une catégorie. Si course_type est None,
    retire l'entrée (la catégorie revient au défaut 'plat')."""
    data = _load()
    if course_type is None:
        data.pop(category_name, None)
    else:
        if course_type not in COURSE_TYPES:
            raise ValueError(f"Type invalide : {course_type!r}")
        data[category_name] = course_type
    _save(data)


def is_plannable(category_name: str | None) -> bool:
    """Renvoie True si une recette de cette catégorie peut être planifiée
    en repas principal (lunch/dinner). Les catégories non configurées
    sont traitées comme 'plat' par défaut."""
    if category_name is None:
        return True
    data = _load()
    return data.get(category_name, "plat") not in EXCLUDED_FROM_PLANNER
