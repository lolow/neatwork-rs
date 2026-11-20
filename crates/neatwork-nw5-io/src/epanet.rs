//! EPANET 2.2 `.inp` files (`specs/formats.md` section 8).
//!
//! - [`read_inp`] ports the python-core import prototype
//!   (`APP:neatwork/epanet/`): syntax, typed sections, unit conversion and
//!   mapping to a NeatWork network, with the same issue codes.
//! - [`write_inp`] implements the export proposal of section 8.3: a
//!   deterministic snapshot of one design, SI units (`LPS`), H-W head loss
//!   with `C = (10.667 / bescal)^(1/1.852)`, PDA for bounded flow and
//!   emitters for free flow.

use crate::fmt::fmt_g;
use crate::sha1::sha1_hex;
use neatwork_nw5::read::g15;
use neatwork_nw5::{Arc, ArcState, Client, Error, Network, Node, NodeStatus, Pipe, Result, Terminal};

// ---------------------------------------------------------------- export

/// Result of [`write_inp`]: the `.inp` text and the warnings of the export.
#[derive(Clone, Debug)]
pub struct InpExport {
    /// `.inp` text.
    pub text: String,
    /// Approximations and data that EPANET cannot represent.
    pub warnings: Vec<String>,
}

/// An EPANET ID has 1 to 31 characters, no white space, no `;`, no `"`.
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().count() <= 31 && !id.chars().any(|c| c.is_whitespace() || c == ';' || c == '"')
}

/// Hazen-Williams coefficient equivalent to `bescal` (exact for the H-W
/// exponents `lambda = 1.852`, `nu = 4.871`).
pub fn hazen_williams_c(bescal: f64) -> f64 {
    (10.667 / bescal).powf(1.0 / 1.852)
}

/// Orifice of a terminal used by the simulation: `orif_com` or `orif_ideal`
/// per the `orif_com` parameter, the other one when it is 0.
fn orifice(node: &Node, use_commercial: bool) -> f64 {
    let (first, second) = if use_commercial {
        (node.orifice_commercial, node.orifice_ideal)
    } else {
        (node.orifice_ideal, node.orifice_commercial)
    };
    if first > 0.0 { first } else { second }
}

struct Segment<'a> {
    id: String,
    from: String,
    to: String,
    length: f64,
    pipe: &'a Pipe,
    open: bool,
}

fn segment<'a>((length, pipe): (f64, &'a Pipe), id: String, from: &str, to: &str, open: bool) -> Segment<'a> {
    Segment {
        id,
        from: from.to_string(),
        to: to.to_string(),
        length,
        pipe,
        open,
    }
}

/// Export a design to EPANET 2.2 (section 8.3 proposal). Every segment with
/// a length must use a pipe of the catalogue, else an error is returned.
pub fn write_inp(net: &Network) -> Result<InpExport> {
    let mut warnings = Vec::new();
    let p = &net.params;
    let (bescal, lambda, nu) = (p.get("bescal"), p.get("lambda"), p.get("nu"));
    let c = hazen_williams_c(bescal);
    if lambda != 1.852 || nu != 4.871 {
        warnings.push(format!(
            "EPANET export is approximate: NeatWork head loss uses lambda = {}, nu = {}, EPANET H-W uses 1.852 and 4.871",
            g15(lambda),
            g15(nu)
        ));
    }
    let bounded = p.get("bounded_flow") == 1.0;
    for n in &net.nodes {
        if !valid_id(&n.id) {
            warnings.push(format!("node `{}` is not a valid EPANET ID", n.id));
        }
    }

    // pipes and intermediate junctions
    let mut used: Vec<String> = net.nodes.iter().map(|n| n.id.clone()).collect();
    let mut pipe_ids: Vec<String> = Vec::new();
    let mut splits: Vec<(String, f64, f64, f64)> = Vec::new(); // id, elevation, x, y
    let mut segments: Vec<Segment> = Vec::new();
    for (k, a) in net.arcs.iter().enumerate() {
        let mut segs = Vec::new();
        for (part, (len, id)) in [(a.length1, &a.pipe1), (a.length2, &a.pipe2)].into_iter().enumerate() {
            if len > 0.0 {
                let id = id.as_deref().unwrap_or("");
                let pipe = net.pipe(id).ok_or_else(|| {
                    Error::Validation(format!(
                        "arc {} -> {}: segment {} uses `{id}`, which is not in the pipe catalogue",
                        a.from,
                        a.to,
                        part + 1
                    ))
                })?;
                segs.push((len, pipe));
            }
        }
        if segs.is_empty() {
            warnings.push(format!("arc {} -> {} has no pipe and is not exported", a.from, a.to));
            continue;
        }
        let parallel = net.arcs.iter().enumerate().any(|(j, b)| j != k && b.from == a.from && b.to == a.to);
        let mut base = format!("{}-{}", a.from, a.to);
        if parallel && a.state == ArcState::Reinforcement {
            base.push_str("#R");
        }
        let suffix = if segs.len() == 2 { "#1" } else { "" };
        if !valid_id(&format!("{base}{suffix}")) || pipe_ids.contains(&base) {
            base = if a.id.is_empty() { format!("A{}", k + 1) } else { a.id.clone() };
        }
        pipe_ids.push(base.clone());
        match segs.as_slice() {
            [one] => segments.push(segment(*one, base, &a.from, &a.to, a.open)),
            [s1, s2] => {
                let mut mid = format!("{}_{}_s", a.from, a.to);
                if !valid_id(&mid) || used.contains(&mid) {
                    mid = format!("{base}_s");
                }
                used.push(mid.clone());
                let t = s1.0 / (s1.0 + s2.0);
                let (from, to) = (net.node(&a.from), net.node(&a.to));
                let lerp = |f: fn(&Node) -> f64| match (from, to) {
                    (Some(x), Some(y)) => f(x) + (f(y) - f(x)) * t,
                    _ => 0.0,
                };
                splits.push((mid.clone(), lerp(|n| n.elevation), lerp(|n| n.x), lerp(|n| n.y)));
                segments.push(segment(*s1, format!("{base}#1"), &a.from, &mid, a.open));
                segments.push(segment(*s2, format!("{base}#2"), &mid, &a.to, a.open));
            }
            _ => {}
        }
    }

    // emitters (free flow) and demands
    let use_commercial = p.get("orif_com") != 0.0;
    let (orifcoef, faucetcoef) = (p.get("orifcoef"), p.get("faucetcoef"));
    let mut emitters: Vec<(String, f64)> = Vec::new();
    if !bounded {
        for n in net.nodes.iter().filter(|n| n.status == NodeStatus::Terminal) {
            let count = net.clients.iter().filter(|c| c.terminal == n.id).count();
            if count == 0 {
                continue;
            }
            let d = orifice(n, use_commercial);
            let orifice_loss = if d > 0.0 {
                (orifcoef / d).powi(4)
            } else {
                warnings.push(format!("terminal {} has no orifice diameter", n.id));
                0.0
            };
            let alpha = orifice_loss + 1.0 / faucetcoef;
            emitters.push((n.id.clone(), count as f64 * 1000.0 / alpha.sqrt()));
        }
    }
    let required = if net.terminals.is_empty() {
        p.get("pressure")
    } else {
        net.terminals.iter().map(|t| t.pressure).fold(f64::NEG_INFINITY, f64::max)
    };
    if bounded && net.terminals.iter().any(|t| t.pressure != required) {
        warnings.push(format!(
            "EPANET has one required pressure: the bornes have different pressures, the maximum {} m is written",
            g15(required)
        ));
    }

    let mut out = String::new();
    let mut line = |cols: &[&str]| {
        out.push_str(&cols.join("\t"));
        out.push('\n');
    };
    line(&["[TITLE]"]);
    if !net.name.is_empty() {
        line(&[&net.name]);
    }
    for c in &net.comments {
        line(&[c.as_str()]);
    }
    line(&[]);
    line(&["[JUNCTIONS]"]);
    line(&[";ID", "Elev", "Demand", "Pattern"]);
    for n in net.nodes.iter().filter(|n| n.status != NodeStatus::Source) {
        line(&[&n.id, &g15(n.elevation), "0"]);
    }
    for (id, z, _, _) in &splits {
        line(&[id, &g15(*z), "0"]);
    }
    line(&[]);
    line(&["[RESERVOIRS]"]);
    line(&[";ID", "Head"]);
    for n in net.nodes.iter().filter(|n| n.status == NodeStatus::Source) {
        line(&[&n.id, &g15(n.elevation + p.get("pump_pressure"))]);
    }
    line(&[]);
    line(&["[PIPES]"]);
    line(&[";ID", "Node1", "Node2", "Length", "Diameter", "Roughness", "MinorLoss", "Status"]);
    let c_text = g15(c);
    for s in &segments {
        let status = if s.open { "Open" } else { "Closed" };
        line(&[&s.id, &s.from, &s.to, &g15(s.length), &g15(s.pipe.diameter * 1000.0), &c_text, "0", status]);
    }
    line(&[]);
    if !emitters.is_empty() {
        line(&["[EMITTERS]"]);
        line(&[";Junction", "Coefficient"]);
        for (id, k) in &emitters {
            line(&[id, &g15(*k)]);
        }
        line(&[]);
    }
    line(&["[DEMANDS]"]);
    line(&[";Junction", "Demand", "Pattern", ";Category"]);
    for cl in &net.clients {
        if emitters.iter().any(|(id, _)| *id == cl.terminal) {
            continue;
        }
        let comment = format!(";{} p={}", cl.id, g15(cl.probability));
        line(&[&cl.terminal, &g15(cl.target_flow), "", &comment]);
    }
    line(&[]);
    line(&["[OPTIONS]"]);
    line(&[&format!(
        "; NeatWork head loss: bescal = {}, lambda = {}, nu = {}; H-W C = (10.667 / bescal)^(1/1.852)",
        g15(bescal),
        g15(lambda),
        g15(nu)
    )]);
    line(&["UNITS", "LPS"]);
    line(&["HEADLOSS", "H-W"]);
    if bounded {
        line(&["DEMAND MODEL", "PDA"]);
        line(&["MINIMUM PRESSURE", "0"]);
        line(&["REQUIRED PRESSURE", &g15(required)]);
        line(&["PRESSURE EXPONENT", "0.5"]);
    } else {
        line(&["DEMAND MODEL", "DDA"]);
        line(&["EMITTER EXPONENT", "0.5"]);
    }
    line(&[]);
    line(&["[TIMES]"]);
    line(&["DURATION", "0"]);
    line(&[]);
    line(&["[REPORT]"]);
    line(&["STATUS", "NO"]);
    line(&[]);
    line(&["[COORDINATES]"]);
    line(&[";Node", "X-Coord", "Y-Coord"]);
    for n in &net.nodes {
        line(&[&n.id, &g15(n.x), &g15(n.y)]);
    }
    for (id, _, x, y) in &splits {
        line(&[id, &g15(*x), &g15(*y)]);
    }
    line(&[]);
    line(&["[END]"]);
    Ok(InpExport { text: out, warnings })
}

// ---------------------------------------------------------------- import

/// How EPANET demands become NeatWork clients.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DemandPolicy {
    /// Keep the demands unresolved: no client, blocking issue
    /// `INTERMITTENT_PROBABILITY_UNRESOLVED` (default).
    #[default]
    PreserveUnresolved,
    /// One always-open client `EP_<node>_<nn>` per positive demand.
    DeterministicAlwaysOpen,
}

/// Options of [`read_inp`].
#[derive(Clone, Debug, Default)]
pub struct InpImportOptions {
    /// Network name (default `EPANET import prototype`).
    pub name: String,
    /// How demands become clients.
    pub demand_policy: DemandPolicy,
    /// Required pressure of the bornes when the file has no
    /// `REQUIRED PRESSURE`, m.
    pub default_required_pressure_m: Option<f64>,
    /// Map one H-W roughness to `lambda 1.852, nu 4.871, bescal =
    /// 10.667 C^-1.852` (off by default, as in python-core).
    pub enable_hw_param_mapping: bool,
}

/// Severity of an import issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Information only.
    Info,
    /// Possible loss of meaning.
    Warning,
    /// The network is not simulation-ready.
    Blocking,
}

/// An import issue (codes of section 13.4).
#[derive(Clone, Debug, PartialEq)]
pub struct InpIssue {
    /// Code, for example `DDA_NOT_SIMULATION_EQUIVALENT`.
    pub code: String,
    /// Severity.
    pub severity: Severity,
    /// Message of the python-core adapter.
    pub message: String,
    /// EPANET section, upper case.
    pub section: Option<String>,
    /// 1-based line of the file.
    pub line_number: Option<usize>,
}

/// Result of [`read_inp`].
#[derive(Clone, Debug)]
pub struct InpImport {
    /// The imported network.
    pub network: Network,
    /// Mapping issues.
    pub issues: Vec<InpIssue>,
    /// No blocking issue.
    pub simulation_ready: bool,
    /// Flow units (`UNITS`) and head-loss model (`HEADLOSS`).
    pub units: String,
    /// Head-loss model (`H-W`, `D-W`, `C-M`).
    pub headloss: String,
    /// `[OPTIONS]` with their keys upper-cased.
    pub options: Vec<(String, String)>,
    /// `[PATTERNS]` multipliers by ID.
    pub patterns: Vec<(String, Vec<f64>)>,
    /// `[EMITTERS]` coefficients (not mapped to orifices).
    pub emitters: Vec<(String, f64)>,
    /// Raw lines of the sections the import does not map, by name.
    pub preserved_sections: Vec<(String, Vec<String>)>,
    /// Warnings of the client data rules (`apply_client_data`).
    pub warnings: Vec<String>,
}

struct DataLine {
    number: usize,
    tokens: Vec<String>,
}

#[derive(Default)]
struct InpSection {
    name: String,
    lines: Vec<DataLine>,
    raw: Vec<String>,
}

/// EPANET syntax: `;` comments, `[NAME]` headers (upper-cased), `[END]`
/// stops, repeated sections merged.
fn parse_sections(text: &str) -> Vec<InpSection> {
    let mut sections: Vec<InpSection> = Vec::new();
    let mut current: Option<usize> = None;
    for (k, original) in text.lines().enumerate() {
        let raw = original.trim_start_matches('\u{feff}').trim_end_matches('\r');
        let data = raw.split(';').next().unwrap_or("").trim();
        if data.starts_with('[') && data.ends_with(']') && data.len() > 2 && !data[1..data.len() - 1].contains(']') {
            let name = data[1..data.len() - 1].trim().to_uppercase();
            if name == "END" {
                break;
            }
            current = match sections.iter().position(|s| s.name == name) {
                Some(i) => Some(i),
                None => {
                    sections.push(InpSection {
                        name,
                        ..InpSection::default()
                    });
                    Some(sections.len() - 1)
                }
            };
            continue;
        }
        let Some(sec) = current.and_then(|i| sections.get_mut(i)) else {
            continue;
        };
        sec.raw.push(raw.to_string());
        if data.is_empty() {
            continue;
        }
        sec.lines.push(DataLine {
            number: k + 1,
            tokens: data.split_whitespace().map(str::to_string).collect(),
        });
    }
    sections
}

const TYPED_SECTIONS: &[&str] = &[
    "TITLE",
    "JUNCTIONS",
    "RESERVOIRS",
    "PIPES",
    "DEMANDS",
    "EMITTERS",
    "COORDINATES",
    "PATTERNS",
    "OPTIONS",
];

const OPTION_KEYS: &[&str] = &[
    "MINIMUM PRESSURE",
    "REQUIRED PRESSURE",
    "PRESSURE EXPONENT",
    "DEMAND MULTIPLIER",
    "EMITTER EXPONENT",
    "DEMAND MODEL",
    "SPECIFIC GRAVITY",
    "UNITS",
    "HEADLOSS",
    "PATTERN",
];

/// Flow unit conversion factor to L/s (`APP:neatwork/epanet/units.py`).
fn flow_to_lps(units: &str) -> Option<f64> {
    Some(match units {
        "CFS" => 28.316846592,
        "GPM" => 0.0630901964,
        "MGD" => 43.81263638888889,
        "IMGD" => 52.61678240740741,
        "AFD" => 14.2764101568,
        "LPS" => 1.0,
        "LPM" => 1.0 / 60.0,
        "MLD" => 11.574074074074074,
        "CMH" => 1.0 / 3.6,
        "CMD" => 1.0 / 86.4,
        _ => return None,
    })
}

/// Order-independent catalogue ID of an EPANET diameter:
/// `EPD_` + SHA-1 of `"%.12g(diameter_m)|%.12g(roughness)|HEADLOSS"`.
pub fn diameter_catalog_id(diameter_m: f64, roughness: f64, headloss: &str) -> String {
    let identity = format!("{}|{}|{}", fmt_g(diameter_m, 12), fmt_g(roughness, 12), headloss.to_uppercase());
    format!("EPD_{}", sha1_hex(identity.as_bytes())[..10].to_uppercase())
}

struct Issues(Vec<InpIssue>);

impl Issues {
    fn add(&mut self, code: &str, message: String, section: Option<&str>, line: Option<usize>) {
        self.0.push(InpIssue {
            code: code.into(),
            severity: Severity::Blocking,
            message,
            section: section.map(str::to_string),
            line_number: line,
        });
    }

    fn float(&mut self, text: &str, field: &str, section: &str, line: usize) -> Option<f64> {
        let v = text.parse::<f64>().ok();
        if v.is_none() {
            self.add(
                "INVALID_NUMBER",
                format!("Invalid numeric value for {field}: '{text}'"),
                Some(section),
                Some(line),
            );
        }
        v
    }
}

struct Demand {
    node: String,
    base: f64,
    pattern: Option<String>,
    line: usize,
}

/// Import an EPANET `.inp` file (section 8.2). An unsupported `UNITS` is an
/// error; every other problem is an issue of the result.
pub fn read_inp(text: &str, opts: &InpImportOptions) -> Result<InpImport> {
    let sections = parse_sections(text);
    let sec = |name: &str| sections.iter().find(|s| s.name == name);
    let mut issues = Issues(Vec::new());

    // options
    let mut options: Vec<(String, String)> = vec![("UNITS".into(), "GPM".into()), ("HEADLOSS".into(), "H-W".into())];
    for l in sec("OPTIONS").map(|s| s.lines.as_slice()).unwrap_or_default() {
        let upper: Vec<String> = l.tokens.iter().map(|t| t.to_uppercase()).collect();
        let (key, skip) = OPTION_KEYS
            .iter()
            .find_map(|k| {
                let kt: Vec<&str> = k.split(' ').collect();
                (upper.len() >= kt.len() && upper[..kt.len()] == kt[..]).then(|| (k.to_string(), kt.len()))
            })
            .unwrap_or_else(|| (upper.first().cloned().unwrap_or_default(), 1));
        let value = l.tokens.get(skip..).map(|v| v.join(" ")).unwrap_or_default();
        match options.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = value,
            None => options.push((key, value)),
        }
    }
    let option = |k: &str| options.iter().find(|(o, _)| o == k).map(|(_, v)| v.trim().to_string());
    let units = option("UNITS").unwrap_or_default().to_uppercase();
    let flow = flow_to_lps(&units).ok_or_else(|| Error::Format(format!("Unsupported EPANET flow units: {units}")))?;
    let us = matches!(units.as_str(), "CFS" | "GPM" | "MGD" | "IMGD" | "AFD");
    let length_m = |v: f64| if us { v * 0.3048 } else { v };
    let headloss = option("HEADLOSS").unwrap_or_default().to_uppercase();

    // typed sections
    // title lines without their comments (python-core keeps `line.data`)
    let title: Vec<String> = sec("TITLE")
        .map(|s| {
            s.raw
                .iter()
                .map(|r| r.split(';').next().unwrap_or("").trim().to_string())
                .filter(|r| !r.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let lines = |name: &str| sec(name).map(|s| s.lines.as_slice()).unwrap_or_default();
    let coordinate = |id: &str| -> (f64, f64) {
        lines("COORDINATES")
            .iter()
            .filter(|l| l.tokens.len() >= 3 && l.tokens[0] == id)
            .filter_map(|l| Some((l.tokens[1].parse().ok()?, l.tokens[2].parse().ok()?)))
            .next()
            .unwrap_or((0.0, 0.0))
    };
    for l in lines("COORDINATES").iter().filter(|l| l.tokens.len() >= 3) {
        issues.float(&l.tokens[1], "X coordinate", "COORDINATES", l.number);
        issues.float(&l.tokens[2], "Y coordinate", "COORDINATES", l.number);
    }

    let mut net = Network {
        name: if opts.name.is_empty() { "EPANET import prototype".into() } else { opts.name.clone() },
        comments: title,
        ..Network::default()
    };
    let mut junction_demands: Vec<Demand> = Vec::new();
    for l in lines("JUNCTIONS") {
        if l.tokens.len() < 2 {
            issues.add(
                "INVALID_JUNCTION_ROW",
                "Junction rows require at least ID and elevation.".into(),
                Some("JUNCTIONS"),
                Some(l.number),
            );
            continue;
        }
        let Some(elevation) = issues.float(&l.tokens[1], "junction elevation", "JUNCTIONS", l.number) else {
            continue;
        };
        let demand = match l.tokens.get(2) {
            Some(t) => match issues.float(t, "junction demand", "JUNCTIONS", l.number) {
                Some(d) => d,
                None => continue,
            },
            None => 0.0,
        };
        let (x, y) = coordinate(&l.tokens[0]);
        net.nodes.push(Node {
            id: l.tokens[0].clone(),
            elevation: length_m(elevation),
            x,
            y,
            taps: None,
            orifice_ideal: 0.0,
            orifice_commercial: 0.0,
            status: NodeStatus::Junction,
        });
        if demand != 0.0 {
            junction_demands.push(Demand {
                node: l.tokens[0].clone(),
                base: demand,
                pattern: l.tokens.get(3).cloned(),
                line: l.number,
            });
        }
    }
    for l in lines("RESERVOIRS").iter().filter(|l| l.tokens.len() >= 2) {
        let Some(head) = issues.float(&l.tokens[1], "reservoir head", "RESERVOIRS", l.number) else {
            continue;
        };
        let (x, y) = coordinate(&l.tokens[0]);
        net.nodes.push(Node {
            id: l.tokens[0].clone(),
            elevation: length_m(head),
            x,
            y,
            taps: None,
            orifice_ideal: 0.0,
            orifice_commercial: 0.0,
            status: NodeStatus::Source,
        });
        if l.tokens.len() >= 3 {
            issues.add(
                "TIME_VARYING_RESERVOIR_HEAD",
                "Reservoir head patterns are not supported.".into(),
                Some("RESERVOIRS"),
                Some(l.number),
            );
        }
    }

    // head-loss model
    let mut roughness_values: Vec<f64> = Vec::new();
    match headloss.as_str() {
        "H-W" => {}
        "D-W" => issues.add(
            "DW_HEADLOSS_MODEL_NOT_VALIDATED",
            "Darcy-Weisbach headloss model is not yet validated.".into(),
            Some("OPTIONS"),
            None,
        ),
        "C-M" => issues.add(
            "CM_HEADLOSS_MODEL_NOT_VALIDATED",
            "Chezy-Manning headloss model is not yet validated.".into(),
            Some("OPTIONS"),
            None,
        ),
        other => issues.add(
            "UNKNOWN_HEADLOSS_MODEL",
            format!("Unsupported headloss model: '{other}'."),
            Some("OPTIONS"),
            None,
        ),
    }

    // pipes and diameter catalogue
    let mut minor_loss = false;
    for l in lines("PIPES") {
        if l.tokens.len() < 6 {
            issues.add(
                "INVALID_PIPE_ROW",
                "Pipe rows require ID, two nodes, length, diameter and roughness.".into(),
                Some("PIPES"),
                Some(l.number),
            );
            continue;
        }
        let length = issues.float(&l.tokens[3], "pipe length", "PIPES", l.number);
        let diameter = issues.float(&l.tokens[4], "pipe diameter", "PIPES", l.number);
        let roughness = issues.float(&l.tokens[5], "pipe roughness", "PIPES", l.number);
        let (Some(length), Some(diameter), Some(roughness)) = (length, diameter, roughness) else {
            continue;
        };
        let minor = match l.tokens.get(6) {
            Some(t) => match issues.float(t, "minor loss coefficient", "PIPES", l.number) {
                Some(m) => m,
                None => continue,
            },
            None => 0.0,
        };
        let status = l.tokens.get(7).map_or("OPEN".to_string(), |s| s.to_uppercase());
        if !matches!(status.as_str(), "OPEN" | "CLOSED" | "CV") {
            issues.add(
                "UNKNOWN_PIPE_STATUS",
                format!("Unsupported pipe status '{status}'."),
                Some("PIPES"),
                Some(l.number),
            );
        }
        if status == "CV" {
            issues.add(
                "CHECK_VALVE_NOT_SUPPORTED",
                "Check-valve pipe semantics are not supported.".into(),
                Some("PIPES"),
                Some(l.number),
            );
        }
        minor_loss |= minor.abs() > 1e-12;
        if headloss == "H-W" && !roughness_values.contains(&roughness) {
            roughness_values.push(roughness);
        }
        let diameter_m = if us { diameter * 0.0254 } else { diameter / 1000.0 };
        let catalogue_id = diameter_catalog_id(diameter_m, roughness, &headloss);
        if net.pipe(&catalogue_id).is_none() {
            net.pipes.push(Pipe {
                id: catalogue_id.clone(),
                nominal: format!("{} {}", fmt_g(diameter, 6), if us { "in" } else { "mm" }),
                sdr: "0".into(),
                diameter: diameter_m,
                cost: 0.0,
                max_pressure: 0.0,
                kind: "EPANET".into(),
                roughness,
            });
        }
        let length = length_m(length);
        net.arcs.push(Arc {
            id: l.tokens[0].clone(),
            from: l.tokens[1].clone(),
            to: l.tokens[2].clone(),
            length,
            length1: length,
            pipe1: Some(catalogue_id),
            length2: 0.0,
            pipe2: None,
            state: ArcState::New,
            open: status != "CLOSED",
        });
    }
    if roughness_values.len() > 1 {
        issues.add(
            "VARIABLE_HW_ROUGHNESS_UNSUPPORTED",
            "NeatWork uses one global friction scale and cannot exactly represent multiple H-W roughness values."
                .into(),
            Some("PIPES"),
            None,
        );
    }

    // demand model
    let demand_model = option("DEMAND MODEL").unwrap_or_else(|| "DDA".into()).to_uppercase();
    let option_num = |k: &str, default: f64| option(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(default);
    match demand_model.as_str() {
        "PDA" | "PDD" => {
            if option_num("MINIMUM PRESSURE", 0.0).abs() > 1e-12 {
                issues.add(
                    "PDA_MINIMUM_PRESSURE_UNSUPPORTED",
                    "NeatWork bounded flow matches PDA only when minimum pressure is zero.".into(),
                    Some("OPTIONS"),
                    None,
                );
            }
            if (option_num("PRESSURE EXPONENT", 0.5) - 0.5).abs() > 1e-12 {
                issues.add(
                    "PDA_PRESSURE_EXPONENT_UNSUPPORTED",
                    "NeatWork uses a fixed quadratic exponent; custom PDA pressure exponent is not supported.".into(),
                    Some("OPTIONS"),
                    None,
                );
            }
        }
        "DDA" => issues.add(
            "DDA_NOT_SIMULATION_EQUIVALENT",
            "DDA (fixed demand) is not equivalent to NeatWork bounded flow. Topology may be imported; simulation equivalence is not declared.".into(),
            Some("OPTIONS"),
            None,
        ),
        other => issues.add(
            "UNKNOWN_DEMAND_MODEL",
            format!("Unsupported demand model: '{other}'."),
            Some("OPTIONS"),
            None,
        ),
    }
    if minor_loss {
        issues.add(
            "EPANET_MINOR_LOSS_UNSUPPORTED",
            "Minor-loss coefficients are not represented by NeatWork.".into(),
            Some("PIPES"),
            None,
        );
    }

    // demands: [DEMANDS] rows, then junction base demands
    let mut demands: Vec<Demand> = Vec::new();
    for l in lines("DEMANDS").iter().filter(|l| l.tokens.len() >= 2) {
        if let Some(base) = issues.float(&l.tokens[1], "base demand", "DEMANDS", l.number) {
            demands.push(Demand {
                node: l.tokens[0].clone(),
                base,
                pattern: l.tokens.get(2).cloned(),
                line: l.number,
            });
        }
    }
    demands.extend(junction_demands);
    let mut terminals: Vec<String> = Vec::new();
    if !demands.is_empty() && opts.demand_policy == DemandPolicy::PreserveUnresolved {
        issues.add(
            "INTERMITTENT_PROBABILITY_UNRESOLVED",
            format!("{} demands preserved, but EPANET has no client opening probability.", demands.len()),
            Some("DEMANDS"),
            None,
        );
    }
    if opts.demand_policy == DemandPolicy::DeterministicAlwaysOpen {
        let multiplier = option_num("DEMAND MULTIPLIER", 1.0);
        let default_pattern = option("PATTERN");
        let mut nodes: Vec<&str> = Vec::new();
        for d in &demands {
            if !nodes.contains(&d.node.as_str()) {
                nodes.push(&d.node);
            }
        }
        for node in nodes {
            for (k, d) in demands.iter().filter(|d| d.node == node).enumerate() {
                if d.base < 0.0 {
                    issues.add(
                        "NEGATIVE_DEMAND_NOT_CLIENT",
                        "Negative demand (inflow) cannot become a client.".into(),
                        Some("DEMANDS"),
                        Some(d.line),
                    );
                    continue;
                }
                if d.base == 0.0 {
                    continue;
                }
                if d.pattern.is_some() || default_pattern.is_some() {
                    issues.add(
                        "TIME_PATTERN_NOT_MAPPED",
                        format!("Demand at {node} uses a time pattern; static client cannot preserve semantics."),
                        Some("DEMANDS"),
                        Some(d.line),
                    );
                }
                net.clients.push(Client {
                    id: format!("EP_{node}_{:02}", k + 1),
                    terminal: node.to_string(),
                    probability: 1.0,
                    target_flow: d.base * multiplier * flow,
                });
                if !terminals.iter().any(|t| t == node) {
                    terminals.push(node.to_string());
                }
            }
        }
        for n in &mut net.nodes {
            if terminals.contains(&n.id) && n.status != NodeStatus::Source {
                n.status = NodeStatus::Terminal;
            }
        }
    }

    // bornes
    let required = option("REQUIRED PRESSURE")
        .and_then(|v| v.parse::<f64>().ok())
        .map(|p| if us { p * 0.703249614902 } else { p })
        .or(opts.default_required_pressure_m);
    match required {
        None if !terminals.is_empty() => issues.add(
            "TERMINAL_PRESSURE_UNRESOLVED",
            "Required pressure cannot be inferred; provide adapter default or EPANET REQUIRED PRESSURE.".into(),
            None,
            None,
        ),
        Some(pressure) => {
            terminals.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            net.terminals = terminals.iter().map(|id| Terminal { id: id.clone(), pressure }).collect();
        }
        None => {}
    }

    // emitters, patterns, preserved sections
    let mut emitters = Vec::new();
    for l in lines("EMITTERS").iter().filter(|l| l.tokens.len() >= 2) {
        if let Some(k) = issues.float(&l.tokens[1], "emitter coefficient", "EMITTERS", l.number) {
            emitters.push((l.tokens[0].clone(), k));
        }
    }
    if !emitters.is_empty() {
        issues.add(
            "EMITTER_ORIFICE_MAPPING_UNRESOLVED",
            "Emitter coefficients are not mapped to orifice diameters.".into(),
            Some("EMITTERS"),
            None,
        );
    }
    let mut patterns: Vec<(String, Vec<f64>)> = Vec::new();
    for l in lines("PATTERNS") {
        let Some(id) = l.tokens.first() else { continue };
        let mut values = Vec::new();
        let mut valid = true;
        for t in &l.tokens[1..] {
            match issues.float(t, "pattern multiplier", "PATTERNS", l.number) {
                Some(v) => values.push(v),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }
        match patterns.iter_mut().find(|(p, _)| p == id) {
            Some(e) => e.1.extend(values),
            None => patterns.push((id.clone(), values)),
        }
    }
    let mut preserved = Vec::new();
    for s in sections.iter().filter(|s| !TYPED_SECTIONS.contains(&s.name.as_str())) {
        let active = s.raw.iter().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with(';')).count();
        if matches!(s.name.as_str(), "TANKS" | "PUMPS" | "VALVES") && active > 0 {
            issues.add(
                &format!("UNSUPPORTED_{}", s.name),
                format!("[{}] has {active} active record(s) not representable by NeatWork.", s.name),
                Some(&s.name),
                None,
            );
        }
        preserved.push((s.name.clone(), s.raw.clone()));
    }

    // H-W parameters
    if opts.enable_hw_param_mapping && headloss == "H-W" && roughness_values.len() == 1 && !minor_loss {
        let c = roughness_values[0];
        net.params.set("lambda", "1.852");
        net.params.set("nu", "4.871");
        net.params.set("bescal", &format!("{}", 10.667 * c.powf(-1.852)));
    }
    let warnings = net.apply_client_data();
    let issues = issues.0;
    Ok(InpImport {
        network: net,
        simulation_ready: !issues.iter().any(|i| i.severity == Severity::Blocking),
        issues,
        units,
        headloss,
        options,
        patterns,
        emitters,
        preserved_sections: preserved,
        warnings,
    })
}
