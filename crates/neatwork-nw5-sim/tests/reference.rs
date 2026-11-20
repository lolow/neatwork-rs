//! Reproduction of the python-core reference runs of
//! `reference/harness-nw5/out` (seed 42, 100 scenarios): every statistic of
//! the schema 1.1 payload and every raw per-scenario array within
//! `|a-b| <= 1e-10 + 1e-8 |b|`, integer outputs exactly.

use std::path::{Path, PathBuf};

use neatwork_nw5::read::read_file;
use neatwork_nw5_sim::{SimulationOptions, simulate, statistics_json};

/// Parsed JSON.
#[derive(Clone, Debug, PartialEq)]
enum J {
    Null,
    Bool(bool),
    /// Number and whether its text was an integer.
    Num(f64, bool),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn get(&self, key: &str) -> &J {
        match self {
            J::Obj(m) => m
                .iter()
                .find(|(k, _)| k == key)
                .map_or(&J::Null, |(_, v)| v),
            _ => &J::Null,
        }
    }

    fn arr(&self) -> &[J] {
        match self {
            J::Arr(v) => v,
            _ => &[],
        }
    }

    fn num(&self) -> f64 {
        match self {
            J::Num(v, _) => *v,
            _ => f64::NAN,
        }
    }

    fn str(&self) -> &str {
        match self {
            J::Str(s) => s,
            _ => "",
        }
    }

    fn strings(&self) -> Vec<String> {
        self.arr().iter().map(|v| v.str().to_string()).collect()
    }

    fn nums(&self) -> Vec<f64> {
        self.arr().iter().map(J::num).collect()
    }

    fn matrix(&self) -> Vec<Vec<f64>> {
        self.arr().iter().map(J::nums).collect()
    }
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Result<J, String> {
        self.ws();
        let c = *self.s.get(self.i).ok_or("unexpected end")?;
        match c {
            b'{' => {
                self.i += 1;
                let mut m = Vec::new();
                self.ws();
                if self.s[self.i] == b'}' {
                    self.i += 1;
                    return Ok(J::Obj(m));
                }
                loop {
                    self.ws();
                    let J::Str(k) = self.value()? else {
                        return Err("key".into());
                    };
                    self.ws();
                    self.i += 1; // ':'
                    let v = self.value()?;
                    m.push((k, v));
                    self.ws();
                    let c = self.s[self.i];
                    self.i += 1;
                    if c == b'}' {
                        return Ok(J::Obj(m));
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.s[self.i] == b']' {
                    self.i += 1;
                    return Ok(J::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    let c = self.s[self.i];
                    self.i += 1;
                    if c == b']' {
                        return Ok(J::Arr(v));
                    }
                }
            }
            b'"' => {
                self.i += 1;
                let mut out = String::new();
                loop {
                    let c = self.s[self.i];
                    self.i += 1;
                    match c {
                        b'"' => return Ok(J::Str(out)),
                        b'\\' => {
                            let e = self.s[self.i];
                            self.i += 1;
                            match e {
                                b'n' => out.push('\n'),
                                b't' => out.push('\t'),
                                b'r' => out.push('\r'),
                                b'b' => out.push('\u{8}'),
                                b'f' => out.push('\u{c}'),
                                b'u' => {
                                    let h = std::str::from_utf8(&self.s[self.i..self.i + 4])
                                        .map_err(|e| e.to_string())?;
                                    let code =
                                        u32::from_str_radix(h, 16).map_err(|e| e.to_string())?;
                                    out.push(char::from_u32(code).unwrap_or('?'));
                                    self.i += 4;
                                }
                                other => out.push(other as char),
                            }
                        }
                        _ => {
                            // copy a UTF-8 sequence
                            let start = self.i - 1;
                            let mut end = self.i;
                            while end < self.s.len() && (self.s[end] & 0xC0) == 0x80 {
                                end += 1;
                            }
                            out.push_str(
                                std::str::from_utf8(&self.s[start..end])
                                    .map_err(|e| e.to_string())?,
                            );
                            self.i = end;
                        }
                    }
                }
            }
            b't' => {
                self.i += 4;
                Ok(J::Bool(true))
            }
            b'f' => {
                self.i += 5;
                Ok(J::Bool(false))
            }
            b'n' => {
                self.i += 4;
                Ok(J::Null)
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len()
                    && matches!(
                        self.s[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[start..self.i]).map_err(|e| e.to_string())?;
                let v: f64 = t.parse().map_err(|_| format!("bad number {t}"))?;
                Ok(J::Num(v, !t.contains(['.', 'e', 'E'])))
            }
        }
    }
}

fn parse(text: &str) -> J {
    let mut p = P {
        s: text.as_bytes(),
        i: 0,
    };
    match p.value() {
        Ok(v) => v,
        Err(e) => panic!("JSON parse error: {e}"),
    }
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn find_dsg(dir: &Path, stem: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(found) = find_dsg(&p, stem) {
                return Some(found);
            }
        } else if p.extension().is_some_and(|x| x == "dsg")
            && p.file_stem().is_some_and(|s| s == stem)
        {
            return Some(p);
        }
    }
    None
}

/// Worst deviation of a case.
#[derive(Default)]
struct Acc {
    compared: usize,
    worst_ratio: f64,
    worst_abs: f64,
    worst_rel: f64,
    worst_at: String,
    violations: Vec<String>,
}

impl Acc {
    fn num(&mut self, a: f64, b: f64, at: &str) {
        self.compared += 1;
        if a.is_nan() && b.is_nan() {
            return;
        }
        let abs = (a - b).abs();
        let ratio = abs / (1e-10 + 1e-8 * b.abs());
        if ratio.is_nan() || ratio > 1.0 {
            self.violations.push(format!("{at}: {a:e} vs python {b:e}"));
        }
        if ratio > self.worst_ratio {
            self.worst_ratio = ratio;
            self.worst_at = at.to_string();
        }
        self.worst_abs = self.worst_abs.max(abs);
        if b != 0.0 {
            self.worst_rel = self.worst_rel.max(abs / b.abs());
        }
    }

    fn exact<T: PartialEq + std::fmt::Debug>(&mut self, a: T, b: T, at: &str) {
        self.compared += 1;
        if a != b {
            self.violations
                .push(format!("{at}: {a:?} vs python {b:?} (must be exact)"));
        }
    }
}

/// Compare our payload with python's (python's has no `durationSeconds`).
fn compare_json(mine: &J, theirs: &J, at: &str, acc: &mut Acc) {
    match (mine, theirs) {
        (J::Obj(m), J::Obj(t)) => {
            let keys_m: Vec<&str> = m
                .iter()
                .map(|(k, _)| k.as_str())
                .filter(|k| *k != "durationSeconds")
                .collect();
            let keys_t: Vec<&str> = t.iter().map(|(k, _)| k.as_str()).collect();
            acc.exact(keys_m, keys_t, &format!("{at} keys"));
            for (k, v) in t {
                compare_json(mine.get(k), v, &format!("{at}.{k}"), acc);
            }
        }
        (J::Arr(m), J::Arr(t)) => {
            acc.exact(m.len(), t.len(), &format!("{at} length"));
            for (i, (a, b)) in m.iter().zip(t).enumerate() {
                compare_json(a, b, &format!("{at}[{i}]"), acc);
            }
        }
        (J::Num(a, ai), J::Num(b, bi)) => {
            if *bi {
                acc.exact((*a, *ai), (*b, *bi), at);
            } else {
                acc.exact(*ai, false, &format!("{at} is a float"));
                acc.num(*a, *b, at);
            }
        }
        (a, b) => acc.exact(a, b, at),
    }
}

fn matrix_nan(j: &J) -> Vec<Vec<f64>> {
    j.arr()
        .iter()
        .map(|r| r.arr().iter().map(J::num).collect())
        .collect()
}

fn run_case(prefix: &str) -> Result<Acc, String> {
    let out = repo().join("reference/harness-nw5/out");
    let read = |s: &str| {
        std::fs::read_to_string(out.join(format!("{prefix}.{s}.json"))).map_err(|e| e.to_string())
    };
    let stats = parse(&read("stats")?);
    let raw = parse(&read("raw")?);
    let name = stats.get("network").get("name").str().to_string();
    let examples = repo().join("reference/neatwork5/examples/dsg_examples");
    let dsg = find_dsg(&examples, &name).ok_or_else(|| format!("no input {name}.dsg"))?;
    let network = read_file(&dsg).map_err(|e| e.to_string())?.network;
    let meta = raw.get("scenario_meta");
    let options = SimulationOptions::new(
        meta.get("scenarioCount").num() as usize,
        meta.get("seed").num() as u64,
    );
    let r = simulate(&network, &options).map_err(|e| e.to_string())?;
    let mut acc = Acc::default();

    // identifiers and orders
    acc.exact(
        r.pipes.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        raw.get("edge_ids").strings(),
        "edge_ids",
    );
    acc.exact(
        r.pipes.iter().map(|p| p.from.clone()).collect::<Vec<_>>(),
        raw.get("pipe_pred").strings(),
        "pipe_pred",
    );
    acc.exact(
        r.pipes.iter().map(|p| p.to.clone()).collect::<Vec<_>>(),
        raw.get("pipe_succ").strings(),
        "pipe_succ",
    );
    acc.exact(
        r.terminals.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        raw.get("terminal_ids").strings(),
        "terminal_ids",
    );
    acc.exact(
        r.transit_nodes
            .iter()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>(),
        raw.get("transit_node_ids").strings(),
        "transit_node_ids",
    );
    // static coefficients
    for (key, get) in [
        (
            "beta",
            (|p: &neatwork_nw5_sim::PipeInfo| p.beta) as fn(&neatwork_nw5_sim::PipeInfo) -> f64,
        ),
        ("gravite", |p| p.gravity),
        ("alpha_ideal", |p| p.alpha_ideal),
        ("alpha_com", |p| p.alpha_com),
    ] {
        for (j, b) in raw.get(key).nums().iter().enumerate() {
            acc.num(get(&r.pipes[j]), *b, &format!("{key}[{j}]"));
        }
    }
    // scenario generation: bit-exact
    acc.exact(
        r.generation.generated_draw_count as f64,
        meta.get("generatedDrawCount").num(),
        "generatedDrawCount",
    );
    acc.exact(
        r.generation.generation_batch_count as f64,
        meta.get("generationBatchCount").num(),
        "generationBatchCount",
    );
    acc.exact(
        r.generation.valid_scenario_probability,
        meta.get("validScenarioProbability").num(),
        "validScenarioProbability",
    );
    let demand = raw.get("demand_by_terminal_lps").matrix();
    let counts = raw.get("open_client_count_by_terminal").matrix();
    let iterations = raw.get("iterations").nums();
    let status = raw.get("status").strings();
    acc.exact(r.scenarios.len(), iterations.len(), "scenario count");
    let per_pipe = [
        ("flow_by_pipe_m3s", matrix_nan(raw.get("flow_by_pipe_m3s"))),
        ("charge_by_pipe_m", matrix_nan(raw.get("charge_by_pipe_m"))),
        ("alpha_by_pipe", matrix_nan(raw.get("alpha_by_pipe"))),
    ];
    let dual = matrix_nan(raw.get("node_dual"));
    let sat = matrix_nan(raw.get("satisfaction"));
    for (k, s) in r.scenarios.iter().enumerate() {
        for t in 0..r.terminals.len() {
            acc.exact(
                s.demand_lps[t].to_bits(),
                demand[t][k].to_bits(),
                &format!("demand[{t}][{k}]"),
            );
            acc.exact(
                f64::from(s.open_clients[t]),
                counts[t][k],
                &format!("count[{t}][{k}]"),
            );
            acc.num(
                s.satisfaction[t],
                sat[t][k],
                &format!("satisfaction[{t}][{k}]"),
            );
        }
        acc.exact(
            s.iterations as f64,
            iterations[k],
            &format!("iterations[{k}]"),
        );
        acc.exact(
            s.status.as_str(),
            status[k].as_str(),
            &format!("status[{k}]"),
        );
        for (key, m) in &per_pipe {
            let mine = match *key {
                "flow_by_pipe_m3s" => &s.flow,
                "charge_by_pipe_m" => &s.charge,
                _ => &s.alpha,
            };
            for j in 0..r.pipes.len() {
                acc.num(mine[j], m[j][k], &format!("{key}[{j}][{k}]"));
            }
        }
        for (i, row) in dual.iter().enumerate() {
            acc.num(s.head[i], row[k], &format!("node_dual[{i}][{k}]"));
        }
    }
    // statistics payload
    let mine = parse(&statistics_json(&r).map_err(|e| e.to_string())?);
    compare_json(&mine, &stats, "$", &mut acc);
    Ok(acc)
}

#[test]
fn reproduces_python_core_references() -> Result<(), String> {
    let out = repo().join("reference/harness-nw5/out");
    let mut cases: Vec<String> = std::fs::read_dir(&out)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".stats.json"))
                .map(str::to_string)
        })
        .collect();
    cases.sort();
    assert!(cases.len() >= 11, "reference cases missing: {cases:?}");
    let mut failed = Vec::new();
    for case in &cases {
        let acc = run_case(case)?;
        eprintln!(
            "{case:28} values {:7}  max |a-b|/(1e-10+1e-8|b|) {:.3e}  max abs {:.3e}  max rel {:.3e}  at {}",
            acc.compared, acc.worst_ratio, acc.worst_abs, acc.worst_rel, acc.worst_at
        );
        if !acc.violations.is_empty() {
            failed.push(format!(
                "{case}: {} violations, first: {:?}",
                acc.violations.len(),
                &acc.violations[..acc.violations.len().min(8)]
            ));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
    Ok(())
}
