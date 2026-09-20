import re
from datetime import date

# Calendrier des fruits et légumes de saison en France, par mois. Volontairement
# simple (liste de noms usuels) : sert à un matching par sous-chaîne contre les
# noms d'aliments Mealie, pas à un référentiel botanique précis.
SEASONAL_PRODUCE: dict[int, list[str]] = {
    1: ["chou", "poireau", "endive", "mâche", "épinard", "panais", "topinambour",
        "carotte", "pomme", "poire", "clémentine", "orange", "kiwi"],
    2: ["chou", "poireau", "endive", "mâche", "épinard", "panais",
        "carotte", "pomme", "poire", "clémentine", "orange", "kiwi"],
    3: ["poireau", "épinard", "radis", "carotte", "pomme", "poire", "kiwi", "rhubarbe"],
    4: ["asperge", "radis", "épinard", "petit pois", "carotte", "rhubarbe", "fraise"],
    5: ["asperge", "radis", "petit pois", "épinard", "fraise", "rhubarbe", "cerise"],
    6: ["courgette", "tomate", "concombre", "haricot vert", "petit pois",
        "fraise", "cerise", "abricot", "pêche"],
    7: ["tomate", "courgette", "aubergine", "poivron", "concombre", "haricot vert",
        "maïs", "abricot", "pêche", "prune", "melon", "framboise", "myrtille"],
    8: ["tomate", "courgette", "aubergine", "poivron", "concombre", "haricot vert",
        "maïs", "melon", "pêche", "prune", "raisin", "figue", "framboise"],
    9: ["tomate", "aubergine", "poivron", "courge", "champignon", "raisin",
        "figue", "prune", "pomme", "poire", "mûre"],
    10: ["courge", "potiron", "champignon", "chou", "poireau", "carotte",
         "pomme", "poire", "raisin", "coing", "châtaigne"],
    11: ["chou", "poireau", "endive", "courge", "potiron", "carotte", "panais",
         "pomme", "poire", "châtaigne", "coing"],
    12: ["chou", "poireau", "endive", "mâche", "panais", "topinambour",
         "carotte", "pomme", "poire", "clémentine", "orange", "kiwi"],
}


def current_month_produce(today: date | None = None) -> list[str]:
    month = (today or date.today()).month
    return SEASONAL_PRODUCE[month]


def is_seasonal(food_name: str, produce: list[str]) -> bool:
    """Matching par mot entier, pas par sous-chaîne brute : un simple "in"
    ferait matcher "poireau" avec "poire" (les deux partagent le préfixe),
    ce qui classerait à tort le poireau (légume d'hiver) comme "de saison"
    en septembre à cause de la poire."""
    normalized = food_name.lower()
    return any(re.search(rf"\b{re.escape(item)}s?\b", normalized) for item in produce)
