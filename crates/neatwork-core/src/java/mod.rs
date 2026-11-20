//! Behavior of the Java runtime that reaches NeatWork output files.

pub mod collections;
pub mod format;

pub use collections::{JConcurrentHashMap, JHashtable, JProperties, JavaHash};
pub use format::{decimal_format, double_to_string, math_round, parse_double, parse_int};

/// `Tools.readCSV`: lines split on `\n` and `\r`, fields on `,`. Empty lines
/// and empty fields disappear (`StringTokenizer` semantics).
pub fn read_csv(content: &str) -> Vec<Vec<String>> {
    content
        .split(['\n', '\r'])
        .filter(|l| !l.is_empty())
        .map(|l| {
            l.split(',')
                .filter(|f| !f.is_empty())
                .map(str::to_string)
                .collect()
        })
        .collect()
}

/// `String.equalsIgnoreCase` (per character upper then lower case check).
pub fn eq_ignore_case(a: &str, b: &str) -> bool {
    let mut x = a.chars();
    let mut y = b.chars();
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(c1), Some(c2)) => {
                if c1 == c2 {
                    continue;
                }
                let u1 = upper(c1);
                let u2 = upper(c2);
                if u1 == u2 || lower(u1) == lower(u2) {
                    continue;
                }
                return false;
            }
            _ => return false,
        }
    }
}

fn upper(c: char) -> char {
    let mut u = c.to_uppercase();
    match (u.next(), u.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

fn lower(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

/// `String.compareTo`: lexicographic on UTF-16 code units.
pub fn compare_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// `FileManagerDisk.writeFile`: drop empty lines, end every line with `\n`.
pub fn project_file_text(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.split(['\n', '\r']).filter(|l| !l.is_empty()) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_csv_drops_empty_tokens() {
        let v = read_csv("!Nodes\n\nA,,1\r\nB,2\n");
        assert_eq!(v, vec![vec!["!Nodes"], vec!["A", "1"], vec!["B", "2"]]);
    }

    #[test]
    fn ignore_case() {
        assert!(eq_ignore_case("Source", "SOURCE"));
        assert!(!eq_ignore_case("A", "B"));
    }
}
