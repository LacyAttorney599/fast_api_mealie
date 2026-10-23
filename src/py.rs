//! Sémantique Python dont dépend le comportement de l'app d'origine, reproduite
//! à l'identique : vérité des valeurs, `dict[clé]` (KeyError) contre
//! `dict.get(clé)` (None), `str.strip()`, conversions « lax » de pydantic,
//! `f"{x:g}"`, `repr()` d'une chaîne, `date.fromisoformat`.

use chrono::{Datelike, NaiveDate, Weekday};
use serde_json::Value;

use crate::api::ApiError;

fn data_error(message: impl Into<String>) -> ApiError {
    ApiError::Data(message.into())
}

/// Nom du type Python d'une valeur JSON, pour les messages d'erreur.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// `bool(value)`.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

/// `a or b`.
pub fn or<'a>(a: &'a Value, b: &'a Value) -> &'a Value {
    if truthy(a) { a } else { b }
}

/// `value[key]` : KeyError si la clé manque.
pub fn at<'a>(value: &'a Value, key: &str) -> Result<&'a Value, ApiError> {
    match value {
        Value::Object(map) => map.get(key).ok_or_else(|| data_error(format!("'{key}'"))),
        Value::Array(_) => Err(data_error(
            "list indices must be integers or slices, not str",
        )),
        Value::String(_) => Err(data_error("string indices must be integers, not 'str'")),
        other => Err(data_error(format!(
            "'{}' object is not subscriptable",
            type_name(other)
        ))),
    }
}

/// `value.get(key)` : null si la clé manque.
pub fn get<'a>(value: &'a Value, key: &str) -> Result<&'a Value, ApiError> {
    match value {
        Value::Object(map) => Ok(map.get(key).unwrap_or(&Value::Null)),
        other => Err(data_error(format!(
            "'{}' object has no attribute 'get'",
            type_name(other)
        ))),
    }
}

/// `value[0]`.
pub fn first(value: &Value) -> Result<&Value, ApiError> {
    match value {
        Value::Array(items) => items
            .first()
            .ok_or_else(|| data_error("list index out of range")),
        Value::Object(_) => Err(data_error("0")),
        Value::String(_) => Err(data_error("string indices must be integers, not 'str'")),
        other => Err(data_error(format!(
            "'{}' object is not subscriptable",
            type_name(other)
        ))),
    }
}

/// Itération `for item in value` sur une liste.
pub fn list(value: &Value) -> Result<&[Value], ApiError> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(data_error(format!(
            "'{}' object is not iterable",
            type_name(other)
        ))),
    }
}

/// Itération sur `value.get(key, [])` : liste vide si la clé manque, mais
/// erreur si elle vaut `null`.
pub fn list_or_empty<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], ApiError> {
    match get(value, key)? {
        Value::Null if value.get(key).is_none() => Ok(&[]),
        items => list(items),
    }
}

/// Champ pydantic `str`.
pub fn string(value: &Value) -> Result<String, ApiError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        _ => Err(data_error("Input should be a valid string")),
    }
}

/// Champ pydantic `str | None`.
pub fn opt_string(value: &Value) -> Result<Option<String>, ApiError> {
    match value {
        Value::Null => Ok(None),
        other => string(other).map(Some),
    }
}

/// `value or ""` affecté à un champ `str`.
pub fn string_or_empty(value: &Value) -> Result<String, ApiError> {
    if truthy(value) {
        string(value)
    } else {
        Ok(String::new())
    }
}

/// Une chaîne Python à laquelle on applique une méthode (`.strip()`…).
pub fn as_str(value: &Value) -> Result<&str, ApiError> {
    match value {
        Value::String(s) => Ok(s),
        other => Err(data_error(format!(
            "'{}' object has no attribute 'strip'",
            type_name(other)
        ))),
    }
}

/// Champ pydantic `float | None`.
pub fn opt_float(value: &Value) -> Result<Option<f64>, ApiError> {
    match value {
        Value::Null => Ok(None),
        other => lax_float(other)
            .map(Some)
            .ok_or_else(|| data_error("Input should be a valid number")),
    }
}

/// Conversion « lax » de pydantic vers `float` : nombres, booléens et chaînes
/// numériques sont acceptés.
pub fn lax_float(value: &Value) -> Option<f64> {
    let parsed = match value {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::String(s) => strip(s).parse().ok(),
        _ => None,
    };
    parsed.filter(|f: &f64| f.is_finite())
}

/// Champ pydantic `int`.
pub fn int(value: &Value) -> Result<i64, ApiError> {
    let parsed = match value {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)),
        Value::Bool(b) => Some(i64::from(*b)),
        Value::String(s) => parse_int(s),
        _ => None,
    };
    parsed.ok_or_else(|| data_error("Input should be a valid integer"))
}

/// Entier écrit en texte, comme l'accepte pydantic (`" 12 "`, `"12.0"`).
pub fn parse_int(text: &str) -> Option<i64> {
    let text = strip(text);
    let text = match text.split_once('.') {
        Some((int, fraction)) if !fraction.is_empty() && fraction.bytes().all(|b| b == b'0') => int,
        Some(_) => return None,
        None => text,
    };
    text.parse().ok()
}

/// Conversion « lax » de pydantic vers `bool`.
pub fn lax_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => match n.as_f64() {
            Some(0.0) => Some(false),
            Some(1.0) => Some(true),
            _ => None,
        },
        Value::String(s) => match s.to_lowercase().as_str() {
            "0" | "off" | "f" | "false" | "n" | "no" => Some(false),
            "1" | "on" | "t" | "true" | "y" | "yes" => Some(true),
            _ => None,
        },
        _ => None,
    }
}

/// `str(value)`, tel qu'interpolé dans une f-string (ids dans les URL).
pub fn fstr(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        other => other.to_string(),
    }
}

/// `str.strip()` : `str::trim`, plus les séparateurs ASCII 0x1C à 0x1F que
/// Python compte aussi parmi les espaces.
pub fn strip(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c))
}

/// `f"{x:g}"` : 6 chiffres significatifs, zéros finaux retirés, notation
/// scientifique quand l'exposant est inférieur à -4 ou supérieur à 5.
pub fn format_g(x: f64) -> String {
    const PRECISION: i32 = 6;
    if x.is_nan() {
        return "nan".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.to_owned();
    }
    // L'exposant après arrondi à 6 chiffres décide de la notation.
    let scientific = format!("{:.*e}", (PRECISION - 1) as usize, x);
    let (mantissa, exponent) = scientific.split_once('e').expect("format {:e}");
    let exponent: i32 = exponent.parse().expect("exposant entier");
    if (-4..PRECISION).contains(&exponent) {
        trim_fraction_zeros(&format!("{:.*}", (PRECISION - 1 - exponent) as usize, x)).to_owned()
    } else {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!(
            "{}e{sign}{:02}",
            trim_fraction_zeros(mantissa),
            exponent.abs()
        )
    }
}

fn trim_fraction_zeros(number: &str) -> &str {
    if number.contains('.') {
        number.trim_end_matches('0').trim_end_matches('.')
    } else {
        number
    }
}

/// `repr()` d'une chaîne : guillemets simples, sauf si elle en contient sans
/// contenir de guillemets doubles.
pub fn repr_str(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `date.fromisoformat(text)` (implémentation C de CPython 3.12) : formats
/// `YYYY-MM-DD`, `YYYYMMDD`, et semaines ISO `YYYY-Www[-D]` / `YYYYWww[D]`.
pub fn date_fromisoformat(text: &str) -> Option<NaiveDate> {
    let bytes = text.as_bytes();
    if !matches!(bytes.len(), 7 | 8 | 10) {
        return None;
    }
    // `parse_digits` : exactement `count` chiffres ASCII.
    let digits = |pos: usize, count: usize| -> Option<u32> {
        let part = bytes.get(pos..pos + count)?;
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0, |acc, b| acc * 10 + u32::from(b - b'0')))
    };

    let year = i32::try_from(digits(0, 4)?).ok()?;
    let uses_separator = bytes[4] == b'-';
    let mut pos = 4 + usize::from(uses_separator);

    let date = if bytes.get(pos) == Some(&b'W') {
        pos += 1;
        let week = digits(pos, 2)?;
        pos += 2;
        let day = if pos < bytes.len() {
            if uses_separator {
                if bytes[pos] != b'-' {
                    return None;
                }
                pos += 1;
            }
            digits(pos, 1)?
        } else {
            1
        };
        let weekday = Weekday::try_from(u8::try_from(day.checked_sub(1)?).ok()?).ok()?;
        NaiveDate::from_isoywd_opt(year, week, weekday)?
    } else {
        let month = digits(pos, 2)?;
        pos += 2;
        if uses_separator {
            if bytes.get(pos) != Some(&b'-') {
                return None;
            }
            pos += 1;
        }
        let day = digits(pos, 2)?;
        NaiveDate::from_ymd_opt(year, month, day)?
    };
    (1..=9999).contains(&date.year()).then_some(date)
}

/// `int(date.strftime("%Y%W"))` : année suivie du numéro de semaine (semaines
/// commençant le lundi, la semaine 0 précédant le premier lundi de l'année).
pub fn year_week_number(date: NaiveDate) -> u64 {
    let week = (date.ordinal0() + 7 - date.weekday().num_days_from_monday()) / 7;
    // Année > 0 : garantie par `date_fromisoformat`.
    u64::try_from(date.year()).unwrap_or(0) * 100 + u64::from(week)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn truthiness_follows_python() {
        for falsy in [
            json!(null),
            json!(false),
            json!(0),
            json!(0.0),
            json!(""),
            json!([]),
            json!({}),
        ] {
            assert!(!truthy(&falsy), "{falsy}");
        }
        for truthy_value in [
            json!(true),
            json!(1),
            json!(-0.5),
            json!(" "),
            json!([0]),
            json!({"a": null}),
        ] {
            assert!(truthy(&truthy_value), "{truthy_value}");
        }
    }

    #[test]
    fn key_error_text_matches_python() {
        let err = at(&json!({}), "id").unwrap_err();
        assert_eq!(err.to_string(), "'id'");
    }

    #[test]
    fn strip_also_removes_ascii_separators() {
        assert_eq!(strip("\u{1c}\t abc \u{a0}\n\u{1f}"), "abc");
    }

    #[test]
    fn format_g_matches_cpython() {
        let cases = [
            (1.0, "1"),
            (2.0, "2"),
            (1.5, "1.5"),
            (0.5, "0.5"),
            (0.25, "0.25"),
            (0.125, "0.125"),
            (0.1, "0.1"),
            (0.333333333, "0.333333"),
            (2.0 / 3.0, "0.666667"),
            (1234567.0, "1.23457e+06"),
            (123456.0, "123456"),
            (100000.0, "100000"),
            (999999.5, "1e+06"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (1.23456789e-7, "1.23457e-07"),
            (12.5, "12.5"),
            (250.0, "250"),
            (1e16, "1e+16"),
            (1.23456789, "1.23457"),
            (99.99999, "100"),
            (0.000123456, "0.000123456"),
            (131072.5, "131072"),
            (1024.0625, "1024.06"),
            (1e100, "1e+100"),
            (-2.5, "-2.5"),
            (7.0000005, "7"),
        ];
        for (value, expected) in cases {
            assert_eq!(format_g(value), expected, "{value}");
        }
    }

    #[test]
    fn repr_matches_cpython() {
        assert_eq!(repr_str("foo"), "'foo'");
        assert_eq!(repr_str("l'entrée"), "\"l'entrée\"");
        assert_eq!(repr_str("a\"b"), "'a\"b'");
        assert_eq!(repr_str("a'b\"c"), "'a\\'b\"c'");
        assert_eq!(repr_str("tab\there"), "'tab\\there'");
        assert_eq!(repr_str("x\u{1}y"), "'x\\x01y'");
        assert_eq!(repr_str("é"), "'é'");
    }

    #[test]
    fn fromisoformat_matches_cpython() {
        let ok = [
            ("2026-10-23", "2026-10-23"),
            ("20261023", "2026-10-23"),
            ("2026-W43", "2026-10-19"),
            ("2026W43", "2026-10-19"),
            ("2026W435", "2026-10-23"),
            ("2026-W43-5", "2026-10-23"),
            ("2026102399", "2026-10-23"),
            ("2026-W53", "2026-12-28"),
            ("2020-W53-7", "2021-01-03"),
        ];
        for (input, expected) in ok {
            assert_eq!(
                date_fromisoformat(input).map(|d| d.to_string()).as_deref(),
                Some(expected),
                "{input}"
            );
        }
        for input in [
            "2026-13-01",
            "2026-10-2",
            "  2026-10-23",
            "2026/10/23",
            "2026-1023",
            "2026-W43-0",
            "2026-W00",
        ] {
            assert_eq!(date_fromisoformat(input), None, "{input}");
        }
    }

    #[test]
    fn year_week_number_matches_strftime() {
        let cases = [
            ("2026-10-23", 202642),
            ("2026-01-01", 202600),
            ("2026-01-05", 202601),
            ("2027-01-03", 202700),
            ("2024-12-30", 202453),
            ("2026-12-31", 202652),
            ("0999-06-15", 99923),
        ];
        for (input, expected) in cases {
            assert_eq!(
                year_week_number(date_fromisoformat(input).unwrap()),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn lax_conversions_follow_pydantic() {
        assert_eq!(lax_float(&json!(" 1.5 ")), Some(1.5));
        assert_eq!(lax_float(&json!(true)), Some(1.0));
        assert_eq!(lax_float(&json!("1/2")), None);
        assert_eq!(lax_bool(&json!("Yes")), Some(true));
        assert_eq!(lax_bool(&json!(0)), Some(false));
        assert_eq!(lax_bool(&json!(2)), None);
        assert_eq!(parse_int(" 12 "), Some(12));
        assert_eq!(parse_int("12.0"), Some(12));
        assert_eq!(parse_int("12.5"), None);
    }
}
