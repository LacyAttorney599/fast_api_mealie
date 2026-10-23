use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{Datelike, NaiveDate};
use regex::Regex;

/// Calendrier des fruits et légumes de saison en France, par mois (janvier
/// en premier). Volontairement simple (liste de noms usuels) : sert à un
/// matching par mot contre les noms d'aliments Mealie, pas à un référentiel
/// botanique précis.
#[rustfmt::skip]
const SEASONAL_PRODUCE: [&[&str]; 12] = [
    &["chou", "poireau", "endive", "mâche", "épinard", "panais", "topinambour",
      "carotte", "pomme", "poire", "clémentine", "orange", "kiwi"],
    &["chou", "poireau", "endive", "mâche", "épinard", "panais",
      "carotte", "pomme", "poire", "clémentine", "orange", "kiwi"],
    &["poireau", "épinard", "radis", "carotte", "pomme", "poire", "kiwi", "rhubarbe"],
    &["asperge", "radis", "épinard", "petit pois", "carotte", "rhubarbe", "fraise"],
    &["asperge", "radis", "petit pois", "épinard", "fraise", "rhubarbe", "cerise"],
    &["courgette", "tomate", "concombre", "haricot vert", "petit pois",
      "fraise", "cerise", "abricot", "pêche"],
    &["tomate", "courgette", "aubergine", "poivron", "concombre", "haricot vert",
      "maïs", "abricot", "pêche", "prune", "melon", "framboise", "myrtille"],
    &["tomate", "courgette", "aubergine", "poivron", "concombre", "haricot vert",
      "maïs", "melon", "pêche", "prune", "raisin", "figue", "framboise"],
    &["tomate", "aubergine", "poivron", "courge", "champignon", "raisin",
      "figue", "prune", "pomme", "poire", "mûre"],
    &["courge", "potiron", "champignon", "chou", "poireau", "carotte",
      "pomme", "poire", "raisin", "coing", "châtaigne"],
    &["chou", "poireau", "endive", "courge", "potiron", "carotte", "panais",
      "pomme", "poire", "châtaigne", "coing"],
    &["chou", "poireau", "endive", "mâche", "panais", "topinambour",
      "carotte", "pomme", "poire", "clémentine", "orange", "kiwi"],
];

pub fn current_month_produce(today: NaiveDate) -> &'static [&'static str] {
    SEASONAL_PRODUCE[today.month0() as usize]
}

fn word_pattern(item: &str) -> Regex {
    Regex::new(&format!(r"\b{}s?\b", regex::escape(item))).expect("regex valide")
}

/// Motifs précompilés de tous les produits du calendrier.
static PATTERNS: LazyLock<HashMap<&'static str, Regex>> = LazyLock::new(|| {
    SEASONAL_PRODUCE
        .iter()
        .flat_map(|month| month.iter())
        .map(|item| (*item, word_pattern(item)))
        .collect()
});

/// Matching par mot entier, pas par sous-chaîne brute : un simple "in"
/// ferait matcher "poireau" avec "poire" (les deux partagent le préfixe),
/// ce qui classerait à tort le poireau (légume d'hiver) comme "de saison"
/// en septembre à cause de la poire.
pub fn is_seasonal(food_name: &str, produce: &[&str]) -> bool {
    let normalized = food_name.to_lowercase();
    produce.iter().any(|item| match PATTERNS.get(item) {
        Some(pattern) => pattern.is_match(&normalized),
        None => word_pattern(item).is_match(&normalized),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poireau_is_not_matched_by_poire() {
        let september = current_month_produce(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap());
        assert!(is_seasonal("Poires williams", september));
        assert!(!is_seasonal("poireau", september));
        assert!(is_seasonal("Tomates cerises", september));
    }

    #[test]
    fn multi_word_and_accented_items_match() {
        let june = current_month_produce(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        assert!(is_seasonal("Petit pois frais", june));
        assert!(is_seasonal("Pêches jaunes", june));
        assert!(!is_seasonal("pêcheur", june));
    }
}
