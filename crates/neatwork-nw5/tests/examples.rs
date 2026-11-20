#![expect(
    clippy::unwrap_used,
    reason = "test code: a failure is the test result"
)]

//! Every example `.tpo` and `.dsg` of NW5 and NW4 reads without error.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else if matches!(p.extension().and_then(|e| e.to_str()), Some("tpo" | "dsg")) {
            out.push(p);
        }
    }
}

#[test]
fn all_examples_read() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference");
    let mut list = Vec::new();
    files(&root.join("neatwork5/examples"), &mut list);
    files(&root.join("neatwork2024/sample_projects"), &mut list);
    list.sort();
    let mut failures = Vec::new();
    for p in &list {
        match neatwork_nw5::read::read_file(p) {
            Ok(r) => {
                let n = &r.network;
                println!(
                    "{:60} nodes {:4} arcs {:4} clients {:4} bornes {:3} pipes {:3} orif {:2} params {:2} design {}",
                    p.strip_prefix(&root).unwrap().display(),
                    n.nodes.len(),
                    n.arcs.len(),
                    n.clients.len(),
                    n.terminals.len(),
                    n.pipes.len(),
                    n.orifices.len(),
                    n.params.entries.len(),
                    n.is_design()
                );
                if n.nodes.is_empty() || n.arcs.is_empty() {
                    failures.push(format!("{}: empty network", p.display()));
                }
            }
            Err(e) => failures.push(format!("{}: {e}", p.display())),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
