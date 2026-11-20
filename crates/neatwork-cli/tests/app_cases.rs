#![expect(
    clippy::unwrap_used,
    reason = "test code: a failure is the test result"
)]

//! Replays the cases recorded from the Java reference (`rust/app-cases`,
//! made by `rust-anything app record`) against the `neatwork` binary.
//! Every file the Java program wrote must be byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rust/app-cases")
}

/// Parse the `args = [...]` and `status.code = N` lines of a trycmd file.
fn read_case(toml: &str) -> (Vec<String>, i32) {
    let mut args = Vec::new();
    let mut code = 0;
    for line in toml.lines() {
        if let Some(rest) = line.strip_prefix("args = [") {
            let body = rest.trim_end_matches(']');
            let mut cur = String::new();
            let mut inside = false;
            let mut escaped = false;
            for c in body.chars() {
                match (inside, escaped, c) {
                    (true, true, _) => {
                        cur.push(c);
                        escaped = false;
                    }
                    (true, false, '\\') => escaped = true,
                    (true, false, '"') => {
                        args.push(std::mem::take(&mut cur));
                        inside = false;
                    }
                    (true, false, _) => cur.push(c),
                    (false, _, '"') => inside = true,
                    _ => {}
                }
            }
        }
        if let Some(rest) = line.strip_prefix("status.code = ") {
            code = rest.trim().parse().unwrap_or(0);
        }
    }
    (args, code)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

#[test]
fn recorded_java_cases_match() {
    let dir = cases_dir();
    let mut tomls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    tomls.sort();
    assert!(!tomls.is_empty(), "no cases in {}", dir.display());
    let work = std::env::temp_dir().join(format!("neatwork-app-cases-{}", std::process::id()));
    let mut failures = Vec::new();
    for toml in &tomls {
        let name = toml.file_stem().unwrap().to_string_lossy().into_owned();
        let (args, code) = read_case(&std::fs::read_to_string(toml).unwrap());
        let sandbox = work.join(&name);
        let _ = std::fs::remove_dir_all(&sandbox);
        copy_dir(&dir.join(format!("{name}.in")), &sandbox);
        let status = Command::new(env!("CARGO_BIN_EXE_neatwork"))
            .args(&args)
            .current_dir(&sandbox)
            .status()
            .unwrap();
        if status.code() != Some(code) {
            failures.push(format!("{name}: exit {:?}, expected {code}", status.code()));
            continue;
        }
        let expected = dir.join(format!("{name}.out"));
        for e in std::fs::read_dir(&expected).unwrap().flatten() {
            let want = std::fs::read(e.path()).unwrap();
            let got = std::fs::read(sandbox.join(e.file_name())).unwrap_or_default();
            if want != got {
                failures.push(format!(
                    "{name}: {} differs",
                    e.file_name().to_string_lossy()
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    assert!(
        failures.is_empty(),
        "{} of {} cases failed:\n{}",
        failures.len(),
        tomls.len(),
        failures.join("\n")
    );
}
