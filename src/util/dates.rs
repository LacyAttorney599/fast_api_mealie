//! Dates de la semaine affichée (ex-`frontend/src/lib/dates.ts`), en heure locale.

use chrono::{Datelike, Days, Local, NaiveDate};

const DAY_LABELS: [&str; 7] = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];
const MONTH_NAMES: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre",
    "novembre", "décembre",
];

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// Lundi de la semaine de `date` (le dimanche appartient à la semaine qui finit).
pub fn get_monday(date: NaiveDate) -> NaiveDate {
    date - Days::new(u64::from(date.weekday().num_days_from_monday()))
}

pub fn add_days(date: NaiveDate, amount: i64) -> NaiveDate {
    if amount >= 0 {
        date + Days::new(amount.unsigned_abs())
    } else {
        date - Days::new(amount.unsigned_abs())
    }
}

/// `YYYY-MM-DD`.
pub fn to_iso_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Date `YYYY-MM-DD` (paramètres d'URL) ; `None` si invalide.
pub fn parse_iso_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeekDay {
    pub date: NaiveDate,
    pub iso: String,
    /// « Lun 19 ».
    pub label: String,
}

pub fn get_week_days(monday: NaiveDate) -> Vec<WeekDay> {
    (0..7)
        .map(|i| {
            let date = add_days(monday, i);
            WeekDay {
                date,
                iso: to_iso_date(date),
                label: format!("{} {}", DAY_LABELS[i as usize], date.day()),
            }
        })
        .collect()
}

/// « 19 – 25 octobre 2026 », « 27 octobre – 2 novembre 2026 » ou, à cheval
/// sur deux années, « 29 décembre 2025 – 4 janvier 2026 ».
pub fn format_week_range(monday: NaiveDate) -> String {
    let sunday = add_days(monday, 6);
    let month = |date: NaiveDate| MONTH_NAMES[date.month0() as usize];
    if monday.year() != sunday.year() {
        return format!(
            "{} {} {} – {} {} {}",
            monday.day(),
            month(monday),
            monday.year(),
            sunday.day(),
            month(sunday),
            sunday.year()
        );
    }
    if monday.month() != sunday.month() {
        return format!(
            "{} {} – {} {} {}",
            monday.day(),
            month(monday),
            sunday.day(),
            month(sunday),
            sunday.year()
        );
    }
    format!("{} – {} {} {}", monday.day(), sunday.day(), month(monday), monday.year())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        parse_iso_date(text).unwrap()
    }

    #[test]
    fn monday_of_the_week() {
        assert_eq!(get_monday(date("2026-10-23")), date("2026-10-19"));
        assert_eq!(get_monday(date("2026-10-25")), date("2026-10-19"));
        assert_eq!(get_monday(date("2026-10-19")), date("2026-10-19"));
    }

    #[test]
    fn week_days_and_labels() {
        let days = get_week_days(date("2026-10-26"));
        assert_eq!(days[0].label, "Lun 26");
        assert_eq!(days[6].iso, "2026-11-01");
        assert_eq!(days[6].label, "Dim 1");
    }

    #[test]
    fn week_ranges() {
        assert_eq!(format_week_range(date("2026-10-19")), "19 – 25 octobre 2026");
        assert_eq!(format_week_range(date("2026-10-26")), "26 octobre – 1 novembre 2026");
        assert_eq!(format_week_range(date("2025-12-29")), "29 décembre 2025 – 4 janvier 2026");
    }
}
