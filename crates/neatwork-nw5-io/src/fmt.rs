//! Number formatting shared by the writers.

/// Text of a number as C `printf("%.<precision>g", x)` writes it, with
/// `Inf`, `-Inf` and `NaN` for non-finite values (MATLAB spelling).
///
/// The exponent is chosen after rounding, as in C.
pub fn fmt_g(x: f64, precision: usize) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Inf".into() } else { "-Inf".into() };
    }
    if x == 0.0 {
        return "0".into();
    }
    let p = precision.max(1);
    let sci = format!("{:.*e}", p - 1, x);
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i64 = exp.parse().unwrap_or(0);
    if exp < -4 || exp >= p as i64 {
        let m = trim_zeros(mantissa);
        format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        let decimals = (p as i64 - 1 - exp).max(0) as usize;
        trim_zeros(&format!("{x:.decimals$}"))
    }
}

/// Shortest text that reads back to the same `f64`, never with an exponent
/// (Python `format(Decimal(str(v)).normalize(), 'f')`).
pub fn plain_decimal(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    format!("{x}")
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printf_g() {
        assert_eq!(fmt_g(0.063, 12), "0.063");
        assert_eq!(fmt_g(100.0, 12), "100");
        assert_eq!(fmt_g(45.300000000000004, 6), "45.3");
        assert_eq!(fmt_g(1.87e-8, 15), "1.87e-08");
        assert_eq!(fmt_g(999999.5, 6), "1e+06");
        assert_eq!(fmt_g(0.0001, 6), "0.0001");
        assert_eq!(fmt_g(-2.5e20, 6), "-2.5e+20");
        assert_eq!(fmt_g(f64::NEG_INFINITY, 6), "-Inf");
    }

    #[test]
    fn plain() {
        assert_eq!(plain_decimal(0.6), "0.6");
        assert_eq!(plain_decimal(1.0), "1");
        assert_eq!(plain_decimal(10.0), "10");
        assert_eq!(plain_decimal(1e-8), "0.00000001");
        assert_eq!(plain_decimal(-0.0), "0");
    }
}
