//! Number formats (`specs/app.md` F54) and display units (F55).
//!
//! Stored values keep full precision in canonical SI units (lengths m,
//! velocities m/s, flows L/s). The functions here only produce text, or
//! convert between the canonical unit and the display unit of a project.

/// Text of a missing or non-finite value.
pub const MISSING: &str = "—";

/// Below this magnitude a value uses scientific notation.
pub const SCIENTIFIC_NOTATION_THRESHOLD: f64 = 1.0e-4;
/// Below this magnitude a value gets four decimals.
pub const EXTRA_DECIMAL_THRESHOLD: f64 = 1.0e-2;

/// JavaScript `Number.prototype.toFixed`: `digits` decimals, an exact tie
/// rounds away from zero (Rust's `{:.N}` rounds a tie to even).
pub fn fixed(value: f64, digits: usize) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }
    let plain = format!("{value:.digits$}");
    // A tie at `digits` decimals is an odd multiple of 2^-(digits+1): the
    // product below is exact (power of two).
    let Ok(exp) = i32::try_from(digits + 1) else {
        return plain;
    };
    let t = value.abs() * 2f64.powi(exp);
    if t.fract() != 0.0 || t >= 9.0e15 || (t as u64) % 2 == 0 {
        return plain;
    }
    let Some(scale) = u32::try_from(digits)
        .ok()
        .and_then(|d| 5u128.checked_pow(d))
    else {
        return plain;
    };
    let Some(n) = (t as u128).checked_mul(scale).map(|k| k.div_ceil(2)) else {
        return plain;
    };
    let digits_text = format!("{n:0>width$}", width = digits + 1);
    let (int, frac) = digits_text.split_at(digits_text.len() - digits);
    let sign = if value < 0.0 { "-" } else { "" };
    if digits == 0 {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac}")
    }
}

/// Adaptive format (`formatAdaptiveNumber`): "—" for a non-finite value,
/// "0.00" for zero, 2 decimals from 1e-2, 4 decimals from 1e-4, else
/// scientific with 2 decimals and no "+" in the exponent. A non-zero value
/// never shows as zero.
pub fn adaptive(value: f64) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }
    if value == 0.0 {
        return "0.00".to_string();
    }
    let absolute = value.abs();
    if absolute >= EXTRA_DECIMAL_THRESHOLD {
        fixed(value, 2)
    } else if absolute >= SCIENTIFIC_NOTATION_THRESHOLD {
        fixed(value, 4)
    } else {
        // Rust writes `3.10e-5` (no "+", no padding), as JavaScript
        // `toExponential(2)` once "e+" is replaced by "e".
        format!("{value:.2e}")
    }
}

/// Ratio as a percentage (`formatRatioAsPercent`): `adaptive(ratio × 100)`
/// and "%". Ratios above 1 give more than 100%.
pub fn ratio_percent(ratio: f64) -> String {
    if !ratio.is_finite() {
        return MISSING.to_string();
    }
    format!("{}%", adaptive(ratio * 100.0))
}

/// Integer (`formatInteger`): truncated, with "," as thousands separator.
// PROPOSAL: NW5 uses the browser locale (`toLocaleString`); the port uses
// the English grouping "1,234" until the interface has a locale setting.
pub fn integer(value: f64) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }
    let t = value.trunc();
    let digits = format!("{:.0}", t.abs());
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if t < 0.0 { format!("-{out}") } else { out }
}

/// Signed adaptive format (`formatSignedNumber`): "+" before a positive
/// value.
pub fn signed(value: f64) -> String {
    let text = adaptive(value);
    if text == MISSING || value <= 0.0 {
        text
    } else {
        format!("+{text}")
    }
}

/// Signed percentage of a value already in percent: `signed(v) + "%"`
/// (the numeric comparison shows a change of +10 % as "+10.00%", D4).
pub fn signed_percent(percent: f64) -> String {
    let text = signed(percent);
    if text == MISSING {
        text
    } else {
        format!("{text}%")
    }
}

/// Length unit used to display lengths, elevations, coordinates, pressure
/// heads and velocities (F53).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayUnit {
    /// Metre (canonical).
    #[default]
    Meter,
    /// Foot.
    Feet,
    /// Inch.
    Inches,
}

/// Metres to feet.
pub const METERS_TO_FEET: f64 = 3.280839895013123;
/// Metres to inches.
pub const METERS_TO_INCHES: f64 = 39.37007874015748;

impl DisplayUnit {
    /// All units in the order of the settings dialog.
    pub const ALL: [DisplayUnit; 3] = [DisplayUnit::Meter, DisplayUnit::Feet, DisplayUnit::Inches];

    /// Parse a unit name: m/meter/metre, ft/feet/foot, in/inch/inches
    /// (case-insensitive, `BE enums/ProjectDisplayLengthUnit.java`).
    pub fn parse(text: &str) -> Option<DisplayUnit> {
        match text.trim().to_ascii_lowercase().as_str() {
            "m" | "meter" | "metre" => Some(DisplayUnit::Meter),
            "ft" | "feet" | "foot" => Some(DisplayUnit::Feet),
            "in" | "inch" | "inches" => Some(DisplayUnit::Inches),
            _ => None,
        }
    }

    /// Symbol: `m`, `ft` or `in` (also the stored form).
    pub fn symbol(self) -> &'static str {
        match self {
            DisplayUnit::Meter => "m",
            DisplayUnit::Feet => "ft",
            DisplayUnit::Inches => "in",
        }
    }

    /// Label of the settings dialog: "Meter (m)", "Feet (ft)", "Inches (in)".
    pub fn label(self) -> &'static str {
        match self {
            DisplayUnit::Meter => "Meter (m)",
            DisplayUnit::Feet => "Feet (ft)",
            DisplayUnit::Inches => "Inches (in)",
        }
    }

    /// Velocity symbol `<u>/s`.
    pub fn velocity_symbol(self) -> String {
        format!("{}/s", self.symbol())
    }

    /// Factor from metres to this unit.
    pub fn factor(self) -> f64 {
        match self {
            DisplayUnit::Meter => 1.0,
            DisplayUnit::Feet => METERS_TO_FEET,
            DisplayUnit::Inches => METERS_TO_INCHES,
        }
    }

    /// Canonical metres (or m/s) to this unit, at display time.
    pub fn from_meters(self, meters: f64) -> f64 {
        match self {
            DisplayUnit::Meter => meters,
            _ => meters * self.factor(),
        }
    }

    /// A value typed in this unit back to metres, at input time.
    pub fn to_meters(self, value: f64) -> f64 {
        match self {
            DisplayUnit::Meter => value,
            _ => value / self.factor(),
        }
    }

    /// Unit of orifice diameters: mm in a metre project, else the project
    /// unit.
    pub fn orifice_symbol(self) -> &'static str {
        match self {
            DisplayUnit::Meter => "mm",
            _ => self.symbol(),
        }
    }

    /// An orifice diameter in m to the orifice display unit.
    pub fn orifice_from_meters(self, meters: f64) -> f64 {
        match self {
            DisplayUnit::Meter => meters * 1000.0,
            _ => self.from_meters(meters),
        }
    }
}

/// Storage unit of a table column that holds a length (F55).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnUnit {
    /// Stored in metres.
    Meter,
    /// Stored in millimetres (`diameter_mm`).
    Millimeter,
}

/// Length columns of the domain tables, by normalized key
/// (`FE features/domain/unitSystem.ts:45-96`).
const LENGTH_COLUMNS: &[&str] = &[
    "haut",
    "elevation",
    "elevation_m",
    "xdata",
    "ydata",
    "x",
    "y",
    "lon",
    "lon1",
    "lon2",
    "length",
    "length_m",
    "pipe_length",
    "pipe_length_m",
    "diam",
    "diameter",
    "diameter_m",
    "diam1",
    "diam2",
    "orif_ideal",
    "orif_com",
    "ideal_orifice_diameter_m",
    "commercial_orifice_diameter_m",
    "pressure",
    "pressure_m",
    "required_pressure_m",
    "pipelength",
    "commercial_pipe_length_m",
    "commercialpipelengthm",
    "requiredpressurem",
];

/// Normalized column key: trimmed, lower case, runs of spaces and hyphens
/// as `_`.
pub fn normalize_column_key(key: &str) -> String {
    let mut out = String::new();
    let mut sep = false;
    for c in key.trim().chars() {
        if c == '-' || c.is_whitespace() {
            sep = true;
        } else {
            if sep {
                out.push('_');
                sep = false;
            }
            out.extend(c.to_lowercase());
        }
    }
    if sep {
        out.push('_');
    }
    out
}

/// Storage unit of a column, `None` for a column that is not a length.
pub fn column_unit(key: &str) -> Option<ColumnUnit> {
    let k = normalize_column_key(key);
    if k == "diameter_mm" {
        Some(ColumnUnit::Millimeter)
    } else if LENGTH_COLUMNS.contains(&k.as_str()) {
        Some(ColumnUnit::Meter)
    } else {
        None
    }
}

/// A stored table value to the display unit. `None` for a non-length
/// column (shown unchanged).
pub fn table_value_to_display(key: &str, value: f64, unit: DisplayUnit) -> Option<f64> {
    match column_unit(key)? {
        ColumnUnit::Meter => Some(unit.from_meters(value)),
        ColumnUnit::Millimeter => Some(unit.from_meters(value / 1000.0)),
    }
}

/// A value typed in the display unit back to the stored unit of the column.
/// `None` for a non-length column (stored unchanged).
pub fn display_to_table_value(key: &str, value: f64, unit: DisplayUnit) -> Option<f64> {
    match column_unit(key)? {
        ColumnUnit::Meter => Some(unit.to_meters(value)),
        ColumnUnit::Millimeter => Some(unit.to_meters(value) * 1000.0),
    }
}

/// Natural, case-insensitive order of two texts: digit runs compare by
/// value (`Intl.Collator(undefined, {numeric: true, sensitivity: "base"})`).
// PROPOSAL: accents are not folded (the collator's base sensitivity also
// ignores them); letters compare by their lower-case form.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let si = i;
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            let sj = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let da: String = a[si..i].iter().collect();
            let db: String = b[sj..j].iter().collect();
            let (ta, tb) = (da.trim_start_matches('0'), db.trim_start_matches('0'));
            let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
            if ord != Ordering::Equal {
                return ord;
            }
        } else {
            let ca: String = a[i].to_lowercase().collect();
            let cb: String = b[j].to_lowercase().collect();
            let ord = ca.cmp(&cb);
            if ord != Ordering::Equal {
                return ord;
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ported from `FE features/simulation/resultNumberFormatting.test.ts`.

    #[test]
    fn keeps_ordinary_values_compact() {
        assert_eq!(adaptive(12.3467), "12.35");
    }

    #[test]
    fn does_not_hide_a_small_non_zero_value() {
        assert_eq!(adaptive(0.0047), "0.0047");
    }

    #[test]
    fn uses_scientific_notation_for_very_small_values() {
        assert_eq!(adaptive(0.000031), "3.10e-5");
    }

    #[test]
    fn keeps_real_zero_readable() {
        assert_eq!(adaptive(0.0), "0.00");
    }

    #[test]
    fn does_not_turn_small_negative_values_into_zero() {
        assert_eq!(adaptive(-0.0047), "-0.0047");
    }

    #[test]
    fn converts_a_satisfaction_ratio_to_percent() {
        assert_eq!(ratio_percent(0.95), "95.00%");
    }

    #[test]
    fn renders_full_satisfaction_as_one_hundred_percent() {
        assert_eq!(ratio_percent(1.0), "100.00%");
    }

    // Further rules of F54.

    #[test]
    fn adaptive_edge_cases() {
        assert_eq!(adaptive(-0.0), "0.00");
        assert_eq!(adaptive(f64::NAN), "—");
        assert_eq!(adaptive(f64::INFINITY), "—");
        assert_eq!(ratio_percent(1.23), "123.00%");
        assert_eq!(ratio_percent(f64::NAN), "—");
        assert_eq!(adaptive(-0.000031), "-3.10e-5");
        assert_eq!(adaptive(1.5e-12), "1.50e-12");
        assert_eq!(adaptive(0.01), "0.01");
        assert_eq!(adaptive(0.0001), "0.0001");
    }

    #[test]
    fn ties_round_away_from_zero_as_javascript() {
        assert_eq!(fixed(0.125, 2), "0.13");
        assert_eq!(fixed(-0.125, 2), "-0.13");
        assert_eq!(fixed(0.625, 2), "0.63");
        assert_eq!(fixed(2.5, 0), "3");
        assert_eq!(fixed(1.005, 2), "1.00");
        assert_eq!(fixed(0.25, 2), "0.25");
        assert_eq!(fixed(12.0, 2), "12.00");
    }

    #[test]
    fn integers_signed_and_percent_change() {
        assert_eq!(integer(1234.9), "1,234");
        assert_eq!(integer(-1234567.0), "-1,234,567");
        assert_eq!(integer(12.0), "12");
        assert_eq!(signed(1.5), "+1.50");
        assert_eq!(signed(-1.5), "-1.50");
        assert_eq!(signed(0.0), "0.00");
        assert_eq!(signed_percent(10.0), "+10.00%");
        assert_eq!(signed_percent(f64::NAN), "—");
    }

    #[test]
    fn units_convert_at_display_and_input_time() {
        assert_eq!(DisplayUnit::parse(" Metre "), Some(DisplayUnit::Meter));
        assert_eq!(DisplayUnit::parse("foot"), Some(DisplayUnit::Feet));
        assert_eq!(DisplayUnit::parse("INCHES"), Some(DisplayUnit::Inches));
        assert_eq!(DisplayUnit::parse("yd"), None);
        let ft = DisplayUnit::Feet;
        assert!((ft.from_meters(1.0) - 3.280839895013123).abs() < 1e-15);
        assert!((ft.to_meters(ft.from_meters(12.5)) - 12.5).abs() < 1e-12);
        assert_eq!(DisplayUnit::Inches.velocity_symbol(), "in/s");
        assert_eq!(DisplayUnit::Meter.orifice_symbol(), "mm");
        assert_eq!(DisplayUnit::Meter.orifice_from_meters(0.005), 5.0);
        assert_eq!(DisplayUnit::Feet.orifice_symbol(), "ft");
        assert_eq!(column_unit("Elevation M"), Some(ColumnUnit::Meter));
        assert_eq!(column_unit("diameter_mm"), Some(ColumnUnit::Millimeter));
        assert_eq!(column_unit("cible"), None);
        assert_eq!(
            table_value_to_display("diameter_mm", 1000.0, DisplayUnit::Meter),
            Some(1.0)
        );
        assert_eq!(
            display_to_table_value("diameter_mm", 1.0, DisplayUnit::Meter),
            Some(1000.0)
        );
        assert_eq!(table_value_to_display("proba", 0.5, ft), None);
    }

    #[test]
    fn natural_order() {
        use std::cmp::Ordering::*;
        assert_eq!(natural_cmp("P2", "P10"), Less);
        assert_eq!(natural_cmp("p10", "P10"), Equal);
        assert_eq!(natural_cmp("A", "b"), Less);
        assert_eq!(natural_cmp("N007", "N7"), Equal);
        assert_eq!(natural_cmp("N7a", "N7"), Greater);
    }
}
