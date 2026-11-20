//! Number formatting and parsing with the exact output of the Java runtime.
//!
//! NeatWork writes numbers with `Double.toString` (string concatenation) and
//! with `DecimalFormat` patterns such as `0.##`. The output files must match
//! the Java files, so these functions copy the Java rules.

/// Java `Double.toString(d)` (JDK 19 and later: shortest decimal that rounds
/// to `d`; at least two significant digits are considered).
pub fn double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    let (digits, exp) = shortest_digits(d.abs());
    let mut out = String::new();
    if d < 0.0 {
        out.push('-');
    }
    let a = d.abs();
    if (1e-3..1e7).contains(&a) {
        // Plain notation. `exp` is the power of ten of the first digit.
        if exp >= 0 {
            let int_len = (exp + 1) as usize;
            if digits.len() <= int_len {
                out.push_str(&digits);
                out.push_str(&"0".repeat(int_len - digits.len()));
                out.push_str(".0");
            } else {
                out.push_str(&digits[..int_len]);
                out.push('.');
                out.push_str(&digits[int_len..]);
            }
        } else {
            out.push_str("0.");
            out.push_str(&"0".repeat((-exp - 1) as usize));
            out.push_str(&digits);
        }
    } else {
        // Computerized scientific notation: d.dddE[-]n
        out.push_str(&digits[..1]);
        out.push('.');
        if digits.len() > 1 {
            out.push_str(&digits[1..]);
        } else {
            out.push('0');
        }
        out.push('E');
        out.push_str(&exp.to_string());
    }
    out
}

/// Shortest decimal digits (no trailing zeros) and the decimal exponent of
/// the first digit. Follows the JDK rule: when one digit is enough, the
/// closest two-digit decimal is used (so `4.9E-324`, not `5.0E-324`).
fn shortest_digits(a: f64) -> (String, i32) {
    let s = format!("{a:e}");
    let (m, e) = split_exp(&s);
    let mut digits: String = m.chars().filter(|c| *c != '.').collect();
    let mut exp = e;
    if digits.len() == 1 {
        let s2 = format!("{a:.1e}");
        let (m2, e2) = split_exp(&s2);
        digits = m2.chars().filter(|c| *c != '.').collect();
        exp = e2;
    }
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    (digits, exp)
}

fn split_exp(s: &str) -> (&str, i32) {
    match s.split_once('e') {
        Some((m, e)) => (m, e.parse().unwrap_or(0)),
        None => (s, 0),
    }
}

/// Java `new DecimalFormat(mask)` with `Locale.US` symbols and the default
/// HALF_EVEN rounding, for the simple masks NeatWork uses
/// (`0.##`, `0.####`, `000`, ...). No grouping, no exponent.
pub fn decimal_format(mask: &str, d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    let (int_part, frac_part) = match mask.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mask, ""),
    };
    let min_int = int_part.chars().filter(|c| *c == '0').count();
    let max_frac = frac_part.chars().filter(|c| *c == '0' || *c == '#').count();
    let min_frac = frac_part.chars().filter(|c| *c == '0').count();
    let negative = d.is_sign_negative();
    if d.is_infinite() {
        return if negative { "-\u{221e}" } else { "\u{221e}" }.to_string();
    }
    let (int_digits, frac_digits) = round_half_even(d.abs(), max_frac);
    let mut frac = frac_digits;
    while frac.len() > min_frac && frac.ends_with('0') {
        frac.pop();
    }
    let mut int = int_digits.trim_start_matches('0').to_string();
    if int.len() < min_int {
        int = format!("{}{}", "0".repeat(min_int - int.len()), int);
    }
    if int.is_empty() && frac.is_empty() {
        int.push('0');
    }
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&int);
    if !frac.is_empty() {
        out.push('.');
        out.push_str(&frac);
    }
    out
}

/// Round a non-negative finite value to `scale` decimals, half to even, on
/// its exact binary value. Returns the integer digits and `scale` fraction
/// digits.
fn round_half_even(a: f64, scale: usize) -> (String, String) {
    // 1100 decimals hold the exact expansion of any f64.
    let exact = format!("{a:.1100}");
    let (int_s, frac_s) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let mut digits: Vec<u8> = int_s.bytes().map(|b| b - b'0').collect();
    let int_len = digits.len();
    let frac_bytes = frac_s.as_bytes();
    digits.extend(frac_bytes[..scale].iter().map(|b| b - b'0'));
    let rest = &frac_bytes[scale..];
    let round_up = match rest.first() {
        Some(b) if *b > b'5' => true,
        Some(b'5') => {
            if rest[1..].iter().any(|b| *b != b'0') {
                true
            } else {
                digits.last().is_some_and(|l| l % 2 == 1)
            }
        }
        _ => false,
    };
    if round_up {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, 1);
                break;
            }
            i -= 1;
            if digits[i] == 9 {
                digits[i] = 0;
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let int_count = digits.len() - scale;
    let _ = int_len;
    let to_s = |v: &[u8]| v.iter().map(|d| (d + b'0') as char).collect::<String>();
    (to_s(&digits[..int_count]), to_s(&digits[int_count..]))
}

/// Java `Double.parseDouble`: surrounding whitespace (chars <= ' ') is
/// ignored, an `f`/`F`/`d`/`D` suffix is allowed, `NaN` and `Infinity` are
/// the only named values.
pub fn parse_double(s: &str) -> Option<f64> {
    let t = s.trim_matches(|c: char| c <= ' ');
    if t.is_empty() {
        return None;
    }
    let (sign, body) = match t.as_bytes()[0] {
        b'-' => (-1.0, &t[1..]),
        b'+' => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    match body {
        "NaN" => return Some(f64::NAN),
        "Infinity" => return Some(sign * f64::INFINITY),
        _ => {}
    }
    let body = body.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(body);
    let ok = !body.is_empty()
        && body
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'))
        && body.bytes().any(|b| b.is_ascii_digit());
    if !ok {
        return None;
    }
    body.parse::<f64>().ok().map(|v| sign * v)
}

/// Java `Integer.parseInt`: optional sign, ASCII digits, 32-bit range.
pub fn parse_int(s: &str) -> Option<i32> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    let digits = if b[0] == b'-' || b[0] == b'+' {
        &s[1..]
    } else {
        s
    };
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse::<i32>().ok()
}

/// Java `Math.round(double)`: floor(x + 0.5) with the JDK edge cases.
pub fn math_round(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    // JDK 7+ semantics: round half up, exact for all doubles.
    let f = x.floor();
    let diff = x - f;
    let r = if diff >= 0.5 { f + 1.0 } else { f };
    if r >= i64::MAX as f64 {
        i64::MAX
    } else if r <= i64::MIN as f64 {
        i64::MIN
    } else {
        r as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_to_string_matches_java() {
        let cases = [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (200.0, "200.0"),
            (0.0015, "0.0015"),
            (1e-6, "1.0E-6"),
            (1e9, "1.0E9"),
            (1e7, "1.0E7"),
            (9999999.0, "9999999.0"),
            (0.001, "0.001"),
            (0.0009, "9.0E-4"),
            (1.0e30, "1.0E30"),
            (-1.0e30, "-1.0E30"),
            (2e-8, "2.0E-8"),
            (0.00384204, "0.00384204"),
            (f64::MIN_POSITIVE * f64::EPSILON, "4.9E-324"),
            (123456.789, "123456.789"),
            (0.1 + 0.2, "0.30000000000000004"),
        ];
        for (v, s) in cases {
            assert_eq!(double_to_string(v), s, "{v:e}");
        }
    }

    #[test]
    fn decimal_format_matches_java() {
        assert_eq!(decimal_format("0.##", 123456.701), "123456.7");
        assert_eq!(decimal_format("0.##", 0.125), "0.12");
        assert_eq!(decimal_format("0.##", 0.375), "0.38");
        assert_eq!(decimal_format("0.##", -0.001), "-0");
        assert_eq!(decimal_format("0.##", -0.0), "-0");
        assert_eq!(decimal_format("000", 7.0), "007");
        assert_eq!(decimal_format("0.####", 0.0037), "0.0037");
        assert_eq!(decimal_format("0.##", f64::NAN), "NaN");
        assert_eq!(decimal_format("0.##", f64::INFINITY), "\u{221e}");
        assert_eq!(decimal_format("0.#", 2.0), "2");
        assert_eq!(decimal_format("0.##", 9.999), "10");
    }

    #[test]
    fn parse_double_like_java() {
        assert_eq!(parse_double(" 1.5 "), Some(1.5));
        assert_eq!(parse_double("1.0E-6"), Some(1e-6));
        assert_eq!(parse_double("2d"), Some(2.0));
        assert_eq!(parse_double("inf"), None);
        assert_eq!(parse_double("N"), None);
        assert_eq!(parse_double(""), None);
        assert_eq!(parse_int("12"), Some(12));
        assert_eq!(parse_int(" 12"), None);
        assert_eq!(math_round(2.5), 3);
        assert_eq!(math_round(-2.5), -2);
    }
}
