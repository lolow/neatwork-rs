//! A small JSON value with a parser and a pretty printer. Object keys keep
//! their order, so a written file has the documented key order.

use neatwork_nw5::read::g15;
use neatwork_nw5::{Error, Result};

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// Members in file order.
    Object(Vec<(String, Json)>),
}

/// Object from `(key, value)` pairs.
pub fn obj(members: Vec<(&str, Json)>) -> Json {
    Json::Object(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// String value.
pub fn s(text: &str) -> Json {
    Json::String(text.to_string())
}

/// Number value. Non-finite numbers become `null` (JSON has no `Inf`).
pub fn n(x: f64) -> Json {
    if x.is_finite() {
        Json::Number(x)
    } else {
        Json::Null
    }
}

/// Nesting depth accepted by the parser.
const MAX_DEPTH: usize = 256;

impl Json {
    /// Parse a JSON text (a UTF-8 BOM is accepted).
    pub fn parse(text: &str) -> Result<Json> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut p = Parser {
            b: text.as_bytes(),
            i: 0,
        };
        let v = p.value(0)?;
        p.ws();
        if p.i < p.b.len() {
            return Err(p.err("unexpected text after the JSON value"));
        }
        Ok(v)
    }

    /// Member of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// First present, non-null member among `keys`.
    pub fn first(&self, keys: &[&str]) -> Option<&Json> {
        keys.iter()
            .find_map(|k| self.get(k).filter(|v| !matches!(v, Json::Null)))
    }

    /// Number value (a numeric string is accepted).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(x) => Some(*x),
            Json::String(t) => neatwork_nw5::read::num(t),
            Json::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Text of a scalar: a string as is, a number as `%.15g`, a boolean as
    /// `true`/`false`. `None` for null, arrays and objects.
    pub fn text(&self) -> Option<String> {
        match self {
            Json::String(t) => Some(t.trim().to_string()),
            Json::Number(x) => Some(g15(*x)),
            Json::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Rows of a table: the items of an array, or a single object (MATLAB
    /// `jsonencode` writes a one-row table as an object).
    pub fn rows(&self) -> Vec<&Json> {
        match self {
            Json::Array(v) => v.iter().collect(),
            Json::Object(_) => vec![self],
            _ => Vec::new(),
        }
    }

    /// Pretty text with two-space indentation and a final newline.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Number(x) => out.push_str(&number(*x)),
            Json::String(t) => string(out, t),
            Json::Array(v) if v.is_empty() => out.push_str("[]"),
            Json::Object(m) if m.is_empty() => out.push_str("{}"),
            Json::Array(v) => {
                out.push_str("[\n");
                for (k, item) in v.iter().enumerate() {
                    pad(out, indent + 1);
                    item.write(out, indent + 1);
                    out.push_str(if k + 1 < v.len() { ",\n" } else { "\n" });
                }
                pad(out, indent);
                out.push(']');
            }
            Json::Object(m) => {
                out.push_str("{\n");
                for (k, (key, item)) in m.iter().enumerate() {
                    pad(out, indent + 1);
                    string(out, key);
                    out.push_str(": ");
                    item.write(out, indent + 1);
                    out.push_str(if k + 1 < m.len() { ",\n" } else { "\n" });
                }
                pad(out, indent);
                out.push('}');
            }
        }
    }
}

fn pad(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

/// Shortest text that reads back to the same number: integers without a
/// fraction, an exponent only for very small or large values.
fn number(x: f64) -> String {
    if !x.is_finite() {
        "null".into()
    } else if x == x.trunc() && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else if (1e-5..1e15).contains(&x.abs()) {
        format!("{x}")
    } else {
        format!("{x:e}")
    }
}

fn string(out: &mut String, t: &str) {
    out.push('"');
    for c in t.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, what: &str) -> Error {
        Error::Format(format!("JSON: {what} at byte {}", self.i))
    }

    fn ws(&mut self) {
        while self.b.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json> {
        if depth > MAX_DEPTH {
            return Err(self.err("nesting too deep"));
        }
        self.ws();
        match self.b.get(self.i) {
            None => Err(self.err("unexpected end")),
            Some(b'{') => {
                self.i += 1;
                let mut m = Vec::new();
                self.ws();
                if self.eat("}") {
                    return Ok(Json::Object(m));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return Err(self.err("expected a key"));
                    }
                    let k = self.string()?;
                    self.ws();
                    if !self.eat(":") {
                        return Err(self.err("expected `:`"));
                    }
                    let v = self.value(depth + 1)?;
                    m.push((k, v));
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("}") {
                        return Ok(Json::Object(m));
                    }
                    return Err(self.err("expected `,` or `}`"));
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.eat("]") {
                    return Ok(Json::Array(v));
                }
                loop {
                    v.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("]") {
                        return Ok(Json::Array(v));
                    }
                    return Err(self.err("expected `,` or `]`"));
                }
            }
            Some(b'"') => self.string().map(Json::String),
            Some(_) if self.eat("null") => Ok(Json::Null),
            Some(_) if self.eat("true") => Ok(Json::Bool(true)),
            Some(_) if self.eat("false") => Ok(Json::Bool(false)),
            Some(_) => {
                let start = self.i;
                while self
                    .b
                    .get(self.i)
                    .is_some_and(|c| matches!(c, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[start..self.i])
                    .ok()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(Json::Number)
                    .ok_or_else(|| self.err("invalid value"))
            }
        }
    }

    fn hex4(&mut self) -> Result<u32> {
        let h = self
            .b
            .get(self.i..self.i + 4)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .ok_or_else(|| self.err("invalid \\u escape"))?;
        self.i += 4;
        Ok(h)
    }

    fn string(&mut self) -> Result<String> {
        self.i += 1; // opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.b.get(self.i) {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(String::from_utf8_lossy(&out).into_owned());
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = self
                        .b
                        .get(self.i)
                        .copied()
                        .ok_or_else(|| self.err("unterminated string"))?;
                    self.i += 1;
                    let ch = match c {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) && self.eat("\\u") {
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                hi
                            };
                            char::from_u32(code).unwrap_or('\u{FFFD}')
                        }
                        _ => return Err(self.err("invalid escape")),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                Some(&c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_print() -> Result<()> {
        let v = Json::parse(
            " {\"a\": [1, -2.5e-3, true, null, \"x\\\"\\u00e9\\ud83d\\ude00\"], \"b\": {}} ",
        )?;
        assert_eq!(v.get("a").map(Json::rows).map(|r| r.len()), Some(5));
        assert_eq!(
            v.pretty(),
            "{\n  \"a\": [\n    1,\n    -0.0025,\n    true,\n    null,\n    \"x\\\"é😀\"\n  ],\n  \"b\": {}\n}\n"
        );
        assert_eq!(Json::parse(&v.pretty())?, v);
        assert!(Json::parse("[1,]").is_err());
        assert!(Json::parse("{\"a\" 1}").is_err());
        assert!(Json::parse(&"[".repeat(1000)).is_err());
        Ok(())
    }

    #[test]
    fn numbers() {
        assert_eq!(number(1e9), "1000000000");
        assert_eq!(number(1.87e-8), "1.87e-8");
        assert_eq!(number(0.0182), "0.0182");
        assert_eq!(number(1e300), "1e300");
        assert_eq!(number(f64::INFINITY), "null");
    }
}
