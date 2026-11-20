//! Helpers shared by the integration tests.

use neatwork_nw5::{Network, Params};
use std::path::{Path, PathBuf};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `reference/` folder of the repository.
pub fn reference() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference")
}

fn collect(dir: &Path, ext: &[&str], out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            collect(&p, ext, out)?;
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ext.contains(&e))
        {
            out.push(p);
        }
    }
    Ok(())
}

/// Every example `.tpo` and `.dsg` of NW5 and NW2024.
pub fn example_files() -> std::io::Result<Vec<PathBuf>> {
    let root = reference();
    let mut list = Vec::new();
    collect(&root.join("neatwork5/examples"), &["tpo", "dsg"], &mut list)?;
    collect(
        &root.join("neatwork2024/sample_projects"),
        &["tpo", "dsg"],
        &mut list,
    )?;
    list.sort();
    Ok(list)
}

pub fn read(path: &Path) -> Result<Network, String> {
    neatwork_nw5::read::read_file(path)
        .map(|r| r.network)
        .map_err(|e| e.to_string())
}

pub fn close(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
}

fn num(s: &str) -> Option<f64> {
    neatwork_nw5::read::num(s)
}

fn same_text_or_number(a: &str, b: &str) -> bool {
    match (num(a), num(b)) {
        (Some(x), Some(y)) => close(x, y),
        _ => a.trim() == b.trim(),
    }
}

fn param_names(p: &Params) -> Vec<String> {
    let mut v: Vec<String> = p
        .entries
        .iter()
        .map(|(k, _)| Params::canonical(k).to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Options of [`compare`].
#[derive(Clone, Copy, Default)]
pub struct Skip {
    /// Ignore `Arc::open` (formats that cannot store it).
    pub open: bool,
    /// Ignore the order and header text of extra sections.
    pub extra_order: bool,
    /// Ignore the network name.
    pub name: bool,
}

/// Differences between two networks (empty = equal tables).
pub fn compare(a: &Network, b: &Network, skip: Skip) -> Vec<String> {
    let mut d = Vec::new();
    let mut f = |c: bool, what: String| {
        if !c {
            d.push(what);
        }
    };
    if !skip.name {
        f(a.name == b.name, format!("name {} / {}", a.name, b.name));
    }
    f(
        a.nodes.len() == b.nodes.len(),
        format!("node count {} / {}", a.nodes.len(), b.nodes.len()),
    );
    for (x, y) in a.nodes.iter().zip(&b.nodes) {
        let ok = x.id == y.id
            && close(x.elevation, y.elevation)
            && close(x.x, y.x)
            && close(x.y, y.y)
            && close(x.orifice_ideal, y.orifice_ideal)
            && close(x.orifice_commercial, y.orifice_commercial)
            && x.status == y.status
            && close(x.taps.unwrap_or(-1.0), y.taps.unwrap_or(-1.0));
        f(ok, format!("node {x:?} / {y:?}"));
    }
    f(
        a.arcs.len() == b.arcs.len(),
        format!("arc count {} / {}", a.arcs.len(), b.arcs.len()),
    );
    for (x, y) in a.arcs.iter().zip(&b.arcs) {
        let ok = x.id == y.id
            && x.from == y.from
            && x.to == y.to
            && close(x.length, y.length)
            && close(x.length1, y.length1)
            && close(x.length2, y.length2)
            && x.pipe1 == y.pipe1
            && x.pipe2 == y.pipe2
            && x.state == y.state
            && (skip.open || x.open == y.open);
        f(ok, format!("arc {x:?} / {y:?}"));
    }
    let sorted = |n: &Network| {
        let mut c = n.clients.clone();
        c.sort_by(|p, q| (&p.terminal, &p.id).cmp(&(&q.terminal, &q.id)));
        c
    };
    let (ca, cb) = (sorted(a), sorted(b));
    f(
        ca.len() == cb.len(),
        format!("client count {} / {}", ca.len(), cb.len()),
    );
    for (x, y) in ca.iter().zip(&cb) {
        let ok = x.id == y.id
            && x.terminal == y.terminal
            && close(x.probability, y.probability)
            && close(x.target_flow, y.target_flow);
        f(ok, format!("client {x:?} / {y:?}"));
    }
    f(
        a.terminals.len() == b.terminals.len(),
        "terminal count".into(),
    );
    for (x, y) in a.terminals.iter().zip(&b.terminals) {
        f(
            x.id == y.id && close(x.pressure, y.pressure),
            format!("terminal {x:?} / {y:?}"),
        );
    }
    f(
        a.pipes.len() == b.pipes.len(),
        format!("pipe count {} / {}", a.pipes.len(), b.pipes.len()),
    );
    for (x, y) in a.pipes.iter().zip(&b.pipes) {
        let ok = x.id == y.id
            && x.nominal == y.nominal
            && same_text_or_number(&x.sdr, &y.sdr)
            && close(x.diameter, y.diameter)
            && close(x.cost, y.cost)
            && close(x.max_pressure, y.max_pressure)
            && same_text_or_number(&x.kind, &y.kind)
            && close(x.roughness, y.roughness);
        f(ok, format!("pipe {x:?} / {y:?}"));
    }
    f(
        a.orifices.len() == b.orifices.len()
            && a.orifices
                .iter()
                .zip(&b.orifices)
                .all(|(x, y)| close(*x, *y)),
        format!("orifices {:?} / {:?}", a.orifices, b.orifices),
    );
    let names = param_names(&a.params);
    f(
        names == param_names(&b.params),
        format!("param names {names:?} / {:?}", param_names(&b.params)),
    );
    for n in &names {
        let (x, y) = (a.params.raw(n).unwrap_or(""), b.params.raw(n).unwrap_or(""));
        f(same_text_or_number(x, y), format!("param {n}: {x} / {y}"));
    }
    f(
        a.loadflow.len() == b.loadflow.len(),
        "loadflow count".into(),
    );
    for (x, y) in a.loadflow.iter().zip(&b.loadflow) {
        f(
            x.0 == y.0 && close(x.1, y.1) && close(x.2, y.2),
            format!("loadflow {x:?} / {y:?}"),
        );
    }
    f(
        a.loadfactor_raw == b.loadfactor_raw,
        "loadfactor rows".into(),
    );
    f(
        a.comments == b.comments,
        format!("comments {:?} / {:?}", a.comments, b.comments),
    );
    if skip.extra_order {
        let canon = |n: &Network| {
            let mut v: Vec<(String, Vec<String>)> = n
                .extra_sections
                .iter()
                .map(|(h, l)| (neatwork_nw5::read::canonical_section(h), l.clone()))
                .collect();
            v.sort();
            v
        };
        f(canon(a) == canon(b), "extra sections".into());
    } else {
        f(
            a.extra_sections == b.extra_sections,
            format!(
                "extra sections {:?} / {:?}",
                a.extra_sections, b.extra_sections
            ),
        );
    }
    d
}
