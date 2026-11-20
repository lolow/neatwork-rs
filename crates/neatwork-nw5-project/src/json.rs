//! JSON helpers over `neatwork_nw5_io::json::Json`.
//!
//! The crate stores its files with the JSON value of `neatwork-nw5-io`: it
//! has a parser and keeps the key order. The value of `neatwork-nw5-sim`
//! has a writer only, so the statistics payload is converted once, when a
//! run ends.

pub use neatwork_nw5_io::json::{Json, n, obj, s};
use neatwork_nw5_sim::Value;

/// A simulation JSON value as a stored JSON value (NaN and infinities
/// become `null`).
pub fn from_sim(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Int(i) => Json::Number(*i as f64),
        Value::Float(f) => n(*f),
        Value::Str(t) => Json::String(t.clone()),
        Value::List(items) => Json::Array(items.iter().map(from_sim).collect()),
        Value::Map(m) => Json::Object(m.iter().map(|(k, v)| (k.clone(), from_sim(v))).collect()),
    }
}

/// One-line JSON text (`JSON.stringify` layout).
pub fn compact(v: &Json) -> String {
    match v {
        Json::Array(items) => format!(
            "[{}]",
            items.iter().map(compact).collect::<Vec<_>>().join(",")
        ),
        Json::Object(m) => format!(
            "{{{}}}",
            m.iter()
                .map(|(k, v)| format!(
                    "{}:{}",
                    Json::String(k.clone()).pretty().trim_end(),
                    compact(v)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        scalar => scalar.pretty().trim_end().to_string(),
    }
}

/// Text of a scalar for a table cell: a string as is, a number in its
/// shortest form, `true`/`false`; `None` for `null`.
pub fn scalar_text(v: &Json) -> Option<String> {
    match v {
        Json::Null => None,
        Json::String(t) => Some(t.clone()),
        Json::Number(x) => Some(format!("{x}")),
        Json::Bool(b) => Some(b.to_string()),
        other => Some(compact(other)),
    }
}

/// Optional number: `null` for `None` or a non-finite value.
pub fn opt_n(x: Option<f64>) -> Json {
    x.map_or(Json::Null, n)
}

/// Optional string.
pub fn opt_s(x: Option<&str>) -> Json {
    x.map_or(Json::Null, s)
}

/// String member.
pub fn get_str(v: &Json, key: &str) -> Option<String> {
    match v.get(key) {
        Some(Json::String(t)) => Some(t.clone()),
        _ => None,
    }
}

/// Number member (`null` gives `None`).
pub fn get_f64(v: &Json, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Json::Number(x)) => Some(*x),
        _ => None,
    }
}

/// Non-negative integer member.
pub fn get_u64(v: &Json, key: &str) -> Option<u64> {
    get_f64(v, key)
        .filter(|x| *x >= 0.0 && x.fract() == 0.0)
        .map(|x| x as u64)
}

/// Boolean member.
pub fn get_bool(v: &Json, key: &str) -> Option<bool> {
    match v.get(key) {
        Some(Json::Bool(b)) => Some(*b),
        _ => None,
    }
}

/// Array member (empty when absent).
pub fn get_array<'a>(v: &'a Json, key: &str) -> &'a [Json] {
    match v.get(key) {
        Some(Json::Array(items)) => items,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_simulation_values() {
        let v = Value::Map(vec![
            ("a".into(), Value::Int(3)),
            ("b".into(), Value::Float(f64::NAN)),
            (
                "c".into(),
                Value::List(vec![Value::Bool(true), Value::Str("x".into())]),
            ),
        ]);
        let j = from_sim(&v);
        assert_eq!(get_u64(&j, "a"), Some(3));
        assert_eq!(j.get("b"), Some(&Json::Null));
        assert_eq!(get_array(&j, "c").len(), 2);
        assert_eq!(get_array(&j, "zz").len(), 0);
    }
}
