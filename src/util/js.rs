//! Sémantique JavaScript dont dépendait l'ancienne interface React :
//! `String.prototype.trim`, `Number(…)`, `Math.round` et l'affichage des
//! nombres (`String(n)`).

use serde_json::Value;

/// Blancs de JavaScript : ceux d'Unicode, plus le BOM, mais sans U+0085 (NEL),
/// que Rust compte comme blanc et JavaScript non.
fn is_js_whitespace(c: char) -> bool {
    c != '\u{85}' && (c.is_whitespace() || c == '\u{feff}')
}

/// `text.trim()` (espaces Unicode, fins de ligne et BOM).
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

/// `Number(text)` : `NaN` si le texte n'est pas un nombre JavaScript valide
/// (décimal, `Infinity`, ou entier `0x`/`0o`/`0b`) ; chaîne vide ou blanche → 0.
pub fn string_to_number(text: &str) -> f64 {
    let text = trim(text);
    if text.is_empty() {
        return 0.0;
    }
    let radix = match text.get(..2) {
        Some("0x" | "0X") => Some(16),
        Some("0o" | "0O") => Some(8),
        Some("0b" | "0B") => Some(2),
        _ => None,
    };
    if let Some(radix) = radix {
        let digits = &text[2..];
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return f64::NAN;
        }
        return digits.chars().fold(0.0, |acc, c| {
            acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
        });
    }
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if unsigned == "Infinity" {
        return if text.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    // Grammaire décimale de JavaScript : chiffres, point, exposant — pas de
    // `inf`, `nan` ni séparateurs que `str::parse` accepterait.
    let valid = unsigned
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
        && unsigned
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '.');
    if !valid {
        return f64::NAN;
    }
    text.parse().unwrap_or(f64::NAN)
}

/// `Number(value)` pour une valeur JSON.
pub fn value_to_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(text) => string_to_number(text),
        // `Number([x])` passe par `String([x])` ; un objet donne `NaN`.
        Value::Array(items) => match items.as_slice() {
            [] => 0.0,
            [single] if !single.is_array() && !single.is_object() => match single {
                Value::Null => 0.0,
                Value::String(text) => string_to_number(text),
                other => value_to_number(other),
            },
            _ => f64::NAN,
        },
        Value::Object(_) => f64::NAN,
    }
}

/// `Math.round(x)` : à mi-chemin, arrondi vers +∞.
pub fn math_round(x: f64) -> f64 {
    if x - x.floor() == 0.5 {
        x.ceil()
    } else {
        x.round()
    }
}

/// `String(n)` pour un nombre.
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    let magnitude = n.abs();
    if (1e-6..1e21).contains(&magnitude) {
        // Même représentation décimale la plus courte que JavaScript.
        return n.to_string();
    }
    let formatted = format!("{n:e}");
    let (mantissa, exponent) = formatted.split_once('e').unwrap_or((&formatted, "0"));
    let sign = if exponent.starts_with('-') { "" } else { "+" };
    format!("{mantissa}e{sign}{exponent}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn number_conversion_follows_javascript() {
        assert_eq!(string_to_number("12"), 12.0);
        assert_eq!(string_to_number(" 1.5 "), 1.5);
        assert_eq!(string_to_number("   "), 0.0);
        assert_eq!(string_to_number(".5"), 0.5);
        assert_eq!(string_to_number("5."), 5.0);
        assert_eq!(string_to_number("1e3"), 1000.0);
        assert_eq!(string_to_number("0x10"), 16.0);
        assert_eq!(string_to_number("-Infinity"), f64::NEG_INFINITY);
        for invalid in ["1,5", "1/2", "abc", "inf", "NaN", "1_000", "0x", "--1"] {
            assert!(string_to_number(invalid).is_nan(), "{invalid}");
        }
        assert_eq!(value_to_number(&json!(true)), 1.0);
        assert_eq!(value_to_number(&json!([])), 0.0);
        assert_eq!(value_to_number(&json!(["7"])), 7.0);
        assert!(value_to_number(&json!([1, 2])).is_nan());
        assert!(value_to_number(&json!({})).is_nan());
    }

    #[test]
    fn rounding_and_display_follow_javascript() {
        assert_eq!(math_round(2.5), 3.0);
        assert_eq!(math_round(-2.5), -2.0);
        assert_eq!(math_round(1.49), 1.0);
        assert_eq!(number_to_string(2.0), "2");
        assert_eq!(number_to_string(1.5), "1.5");
        assert_eq!(number_to_string(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(1.5e-8), "1.5e-8");
        assert_eq!(number_to_string(0.000001), "0.000001");
        assert_eq!(number_to_string(9e-7), "9e-7");
        assert_eq!(number_to_string(-0.0), "0");
    }

    #[test]
    fn trim_keeps_nel_like_javascript() {
        assert_eq!(trim("\u{85}lait\u{85}"), "\u{85}lait\u{85}");
        assert!(string_to_number("\u{85}2").is_nan());
    }

    #[test]
    fn trim_also_removes_the_bom() {
        assert_eq!(trim("\u{feff} lait \n"), "lait");
    }
}
