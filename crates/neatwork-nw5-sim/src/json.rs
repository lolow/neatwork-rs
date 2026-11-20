//! JSON values and the writer of python's `json.dumps(indent=2,
//! ensure_ascii=False, allow_nan=False)` (spec section 5.3).

/// A JSON value (failure details, issue details, payload).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `null` (also written for NaN and infinite floats).
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A float, written as python `repr`.
    Float(f64),
    /// A string.
    Str(String),
    /// A list.
    List(Vec<Value>),
    /// An object, in insertion order.
    Map(Vec<(String, Value)>),
}

impl Value {
    /// Text of the value with python's `json.dumps(..., indent=2)` layout.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        write_value(&mut out, self, 0);
        out
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Value {
        Value::Float(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Value {
        Value::Int(v)
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Value {
        Value::Int(i64::try_from(v).unwrap_or(i64::MAX))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Value {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Value {
        Value::Str(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Value {
        Value::Str(v)
    }
}

impl From<Vec<String>> for Value {
    fn from(v: Vec<String>) -> Value {
        Value::List(v.into_iter().map(Value::Str).collect())
    }
}

fn indent(out: &mut String, level: usize) {
    out.push('\n');
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_value(out: &mut String, v: &Value, level: usize) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Float(f) => out.push_str(&py_float(*f)),
        Value::Str(s) => write_str(out, s),
        Value::List(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (k, item) in items.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                indent(out, level + 1);
                write_value(out, item, level + 1);
            }
            indent(out, level);
            out.push(']');
        }
        Value::Map(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (k, (key, item)) in entries.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                indent(out, level + 1);
                write_str(out, key);
                out.push_str(": ");
                write_value(out, item, level + 1);
            }
            indent(out, level);
            out.push('}');
        }
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Python `repr(float)` (shortest round-trip digits); NaN and infinities
/// give `null`, as python-core's exporter does.
pub fn py_float(x: f64) -> String {
    if !x.is_finite() {
        return "null".to_string();
    }
    py_repr(x)
}

/// Python `repr` of a finite float: fixed notation for decimal exponents in
/// [-4, 16), scientific otherwise (`1e-05`, `1.5e+16`).
pub(crate) fn py_repr(x: f64) -> String {
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    // shortest round-trip digits from Rust's `{:e}`: "-1.2345e-7"
    let sci = format!("{x:e}");
    let (mant, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let negative = mant.starts_with('-');
    let digits: String = mant.chars().filter(char::is_ascii_digit).collect();
    let sign = if negative { "-" } else { "" };
    if !(-4..16).contains(&exp) {
        let (first, rest) = digits.split_at(1);
        let frac = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let esign = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{first}{frac}e{esign}{:02}", exp.abs());
    }
    let point = exp + 1; // digits before the decimal point
    let body = if point <= 0 {
        format!("0.{}{}", "0".repeat(point.unsigned_abs() as usize), digits)
    } else {
        let p = point as usize;
        if digits.len() <= p {
            format!("{}{}.0", digits, "0".repeat(p - digits.len()))
        } else {
            format!("{}.{}", &digits[..p], &digits[p..])
        }
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_repr() {
        assert_eq!(py_repr(1.0), "1.0");
        assert_eq!(py_repr(0.0001), "0.0001");
        assert_eq!(py_repr(1e-5), "1e-05");
        assert_eq!(py_repr(1.3129139494292002e-19), "1.3129139494292002e-19");
        assert_eq!(py_repr(-3.517220266147092e-07), "-3.517220266147092e-07");
        assert_eq!(py_repr(9.92), "9.92");
        assert_eq!(py_repr(1e16), "1e+16");
        assert_eq!(py_repr(9999999999999998.0), "9999999999999998.0");
        assert_eq!(py_repr(123456.789), "123456.789");
        assert_eq!(py_repr(-0.5), "-0.5");
        assert_eq!(py_repr(1e22), "1e+22");
        assert_eq!(py_repr(1.5e300), "1.5e+300");
        assert_eq!(py_float(f64::NAN), "null");
    }

    #[test]
    fn layout_like_python() {
        let v = Value::Map(vec![
            (
                "a".into(),
                Value::List(vec![Value::Int(1), Value::Bool(true)]),
            ),
            ("b".into(), Value::Map(vec![])),
            ("c".into(), Value::List(vec![])),
            ("d".into(), Value::Str("x\"é\n".into())),
        ]);
        assert_eq!(
            v.to_json(),
            "{\n  \"a\": [\n    1,\n    true\n  ],\n  \"b\": {},\n  \"c\": [],\n  \"d\": \"x\\\"é\\n\"\n}"
        );
    }
}
