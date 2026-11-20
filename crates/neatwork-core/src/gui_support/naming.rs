//! Free project names: `ProjectManager.isAlreadyLoaded` and
//! `AbstractFileManager.getFirstNameCompatible`.

use crate::java::parse_int;

/// `ProjectManager.isAlreadyLoaded`: a name that no open project of the
/// same type uses. A name that ends with `(n)` continues from `n`.
///
/// `name (n)` (or `name(n)`) gives `name (n)`, then `name (n+1)`... Java
/// drops the character before the `(` whatever it is, so `a(5)` gives
/// ` (5)`; this is kept. When nothing is before the `(`, Java throws
/// `StringIndexOutOfBoundsException`; here the name is then kept as it is.
pub fn already_loaded_name(name: &str, open_names_of_same_type: &[String]) -> String {
    let in_use = |n: &String| open_names_of_same_type.contains(n);
    let mut oldname = name.to_string();
    let mut current = name.to_string();
    let mut cpt: i64 = 2;
    if let (Some(i), Some(j)) = (name.rfind('('), name.rfind(')'))
        && i < j
        && let Some(n) = parse_int(&name[i + 1..j])
    {
        let mut before = name[..i].chars();
        if before.next_back().is_some() {
            oldname = before.as_str().to_string();
            cpt = i64::from(n);
            current = format!("{oldname} ({cpt})");
            cpt += 1;
        }
    }
    while in_use(&current) {
        current = format!("{oldname} ({cpt})");
        cpt += 1;
    }
    current
}

/// `AbstractFileManager.getFirstNameCompatible`: `name`, or `name (2)`,
/// `name (3)`, ... the first one that is not a file name of the same type.
pub fn first_compatible_name(name: &str, existing_file_names: &[String]) -> String {
    let mut current = name.to_string();
    let mut cpt = 2;
    while existing_file_names.contains(&current) {
        current = format!("{name} ({cpt})");
        cpt += 1;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cases of the recording: the name, then the names in use.
    const CASES: [&[&str]; 8] = [
        &["toy"],
        &["toy", "toy"],
        &["toy", "toy", "toy (2)"],
        &["toy (2)", "toy (2)"],
        &["toy (2)", "toy (3)"],
        &["toy (x)", "toy (x)"],
        &["a(5)", "a(5)", "a (5)"],
        &["toy)(", "toy)("],
    ];

    #[test]
    fn names_match_java() {
        let mut out = String::new();
        for c in CASES {
            let used: Vec<String> = c[1..].iter().map(|s| s.to_string()).collect();
            out.push_str(&format!(
                "{} -> {}\n",
                c[0],
                already_loaded_name(c[0], &used)
            ));
        }
        for c in CASES {
            let used: Vec<String> = c[1..].iter().map(|s| s.to_string()).collect();
            out.push_str(&format!(
                "{} => {}\n",
                c[0],
                first_compatible_name(c[0], &used)
            ));
        }
        assert_eq!(out, include_str!("testdata/naming.txt"));
    }

    #[test]
    fn index_without_text_before() {
        // Java throws StringIndexOutOfBoundsException on "(3)"; the name
        // is then kept as it is.
        assert_eq!(already_loaded_name("(3)", &[]), "(3)");
        assert_eq!(already_loaded_name("(3)", &["(3)".to_string()]), "(3) (2)");
        // Only one character before the parenthesis is dropped.
        assert_eq!(already_loaded_name("é(3)", &[]), " (3)");
        assert_eq!(already_loaded_name("x (+4)", &[]), "x (4)");
    }
}
