//! From a `Network` to the sorted simulation tables (spec sections 1.2 to
//! 1.6): parameter normalization, client demand rule, preprocessing.

use std::collections::HashMap;

use neatwork_nw5::Network;

use crate::error::{SimError, ValidationIssue};
use crate::json::py_repr;
use crate::options::{OrificeMode, SimulationOptions};
use crate::result::PipeKind;

/// Normalized simulation parameters (`inst.param`).
pub(crate) type ParamMap = HashMap<String, f64>;

/// `DEFAULT_SIMULATION_PARAMS` (`sim/constants.py:22-46`). The DSG reader
/// default `lambda = 0` is not used (ADR-021).
const CORE_DEFAULTS: &[(&str, f64)] = &[
    ("bescal", 8.9396e-4),
    ("bounded_flow", 0.0),
    ("faucetcoef", 1.0),
    ("fusion", 0.0),
    ("lambda", 1.781),
    ("limitbudget", f64::INFINITY),
    ("nbsim", 100.0),
    ("nu", 4.781),
    ("opentaps", 1.0),
    ("orif_com", 1.0),
    ("orifcoef", 1.0),
    ("pipelength", 6.0),
    ("pressure", 2.0),
    ("secu", 1.0),
    ("servicequal", 0.65),
    ("seuil", 0.0),
    ("split", 0.0),
    ("targetflow", 0.0),
    ("uniform_dem", 1.0),
    ("use_efficient_pipes", 0.0),
    ("vmax", 1.5),
    ("vmax_secu", 0.95),
    ("watertemp", 20.0),
];

/// `INTEGER_SIMULATION_PARAMS`: values truncated to integers.
const INTEGER_PARAMS: &[&str] = &[
    "bounded_flow",
    "fusion",
    "nbsim",
    "orif_com",
    "split",
    "uniform_dem",
    "use_efficient_pipes",
];

/// `PARAMETER_ALIASES` after snake-case conversion.
const PARAM_ALIASES: &[(&str, &str)] = &[
    ("serviceequal", "servicequal"),
    ("bounded_flow_enabled", "bounded_flow"),
    ("open_taps", "opentaps"),
    ("orifice_commercial", "orif_com"),
    ("orifice_coefficient", "orifcoef"),
    ("faucet_coefficient", "faucetcoef"),
    ("pipe_length", "pipelength"),
    ("service_quality", "servicequal"),
    ("target_flow", "targetflow"),
    ("uniform_demand", "uniform_dem"),
    ("maximum_velocity", "vmax"),
    ("maximum_safe_velocity", "vmax_secu"),
    ("water_temperature", "watertemp"),
];

/// Canonical parameter name (`canonical_parameter_name`): `-` and spaces to
/// `_`, `_` before inner capitals, `__` collapsed, lower case, aliases.
pub(crate) fn canonical_param(name: &str) -> String {
    let n = name.trim().replace(['-', ' '], "_");
    let mut s = String::new();
    for (i, c) in n.chars().enumerate() {
        if i > 0 && c.is_ascii_uppercase() {
            s.push('_');
        }
        s.push(c);
    }
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    let s = s.to_lowercase();
    PARAM_ALIASES
        .iter()
        .find(|(a, _)| *a == s)
        .map_or(s, |(_, c)| (*c).to_string())
}

/// python `float(text)`.
pub(crate) fn py_float_parse(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok()
}

/// Identifier normalization of the python DSG reader (`to_number` then
/// `str`): `"007"` -> `"7"`, `"1.50"` -> `"1.5"`, `"1e3"` -> `"1000"`. Used as
/// the sort key so that the order (and the random draws) equal python-core.
pub(crate) fn py_id(id: &str) -> String {
    let t = id.trim();
    if t.is_empty() {
        return t.to_string();
    }
    let Some(f) = py_float_parse(t) else {
        return t.to_string();
    };
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if f == f.trunc() {
        if f == 0.0 {
            return "0".into();
        }
        return format!("{f:.0}");
    }
    py_repr(f)
}

/// Node row (`nodes` table).
#[derive(Clone, Debug)]
pub(crate) struct NodeRow {
    pub(crate) id: String,
    pub(crate) haut: f64,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) orif_ideal: f64,
    pub(crate) orif_com: f64,
    pub(crate) statut: i32,
    /// Index in `Network::nodes`.
    pub(crate) index: usize,
}

/// Pipe row (`pipes` table).
#[derive(Clone, Debug)]
pub(crate) struct PipeRow {
    pub(crate) eid: String,
    pub(crate) pred: String,
    pub(crate) succ: String,
    pub(crate) lon: f64,
    pub(crate) lon1: f64,
    pub(crate) id1: Option<String>,
    pub(crate) lon2: f64,
    pub(crate) id2: Option<String>,
    pub(crate) open: bool,
    /// Index in `Network::arcs`.
    pub(crate) index: usize,
}

/// Client row.
#[derive(Clone, Debug)]
pub(crate) struct ClientRow {
    pub(crate) id: String,
    pub(crate) borne: String,
    pub(crate) proba: f64,
    pub(crate) cible: f64,
}

/// Borne row.
#[derive(Clone, Debug)]
pub(crate) struct BorneRow {
    pub(crate) id: String,
    pub(crate) pressure: f64,
}

/// Diameter row.
#[derive(Clone, Debug)]
pub(crate) struct DiamRow {
    pub(crate) id: String,
    pub(crate) diam: f64,
    pub(crate) cost: f64,
    pub(crate) pmax: f64,
    pub(crate) roughness: f64,
}

/// The five tables and the parameters, before validation.
#[derive(Clone, Debug)]
pub(crate) struct Tables {
    pub(crate) nodes: Vec<NodeRow>,
    pub(crate) pipes: Vec<PipeRow>,
    pub(crate) clients: Vec<ClientRow>,
    pub(crate) bornes: Vec<BorneRow>,
    pub(crate) diameters: Vec<DiamRow>,
    pub(crate) params: ParamMap,
}

fn diameter_ref(id: &Option<String>) -> Option<String> {
    match id.as_deref().map(str::trim) {
        None | Some("") | Some("0") => None,
        Some(s) => Some(s.to_string()),
    }
}

/// Parameters of the network after normalization (1.4.1), overrides and the
/// client demand rule (1.4 steps 1, 2 and 5).
fn normalize_params(net: &Network, options: &SimulationOptions) -> ParamMap {
    let mut p: ParamMap = CORE_DEFAULTS
        .iter()
        .map(|(k, v)| ((*k).to_string(), *v))
        .collect();
    for (name, text) in &net.params.entries {
        let key = canonical_param(name);
        let value = match py_float_parse(text) {
            Some(v) => v,
            // python raises ValueError; a text value of a parameter the
            // simulation uses becomes NaN (validation reports it), others
            // are ignored
            None if p.contains_key(&key) => f64::NAN,
            None => continue,
        };
        let value = if INTEGER_PARAMS.contains(&key.as_str()) {
            value.trunc()
        } else {
            value
        };
        p.insert(key, value);
    }
    let o = &options.overrides;
    if let Some(m) = o.orifice_mode {
        p.insert(
            "orif_com".into(),
            if m == OrificeMode::Commercial {
                1.0
            } else {
                0.0
            },
        );
    }
    if let Some(b) = o.bounded_flow {
        p.insert("bounded_flow".into(), if b { 1.0 } else { 0.0 });
    }
    if let Some(v) = o.required_pressure_m {
        p.insert("pressure".into(), v);
    }
    if let Some(v) = o.orifice_coefficient {
        p.insert("orifcoef".into(), v);
    }
    if let Some(v) = o.faucet_coefficient {
        p.insert("faucetcoef".into(), v);
    }
    if let Some(first) = net.clients.first() {
        let uniform = net
            .clients
            .iter()
            .all(|c| c.probability == first.probability && c.target_flow == first.target_flow);
        if uniform {
            p.insert("uniform_dem".into(), 1.0);
            p.insert("opentaps".into(), first.probability);
            p.insert("targetflow".into(), first.target_flow);
        } else {
            p.insert("uniform_dem".into(), 0.0);
            p.insert("bounded_flow".into(), 1.0);
        }
    }
    p
}

/// Build the tables from a network and the run options.
pub(crate) fn build_tables(net: &Network, options: &SimulationOptions) -> Result<Tables, SimError> {
    // edge overrides (simulation_service.py:43-131)
    let mut open: Vec<bool> = net.arcs.iter().map(|a| a.open).collect();
    if !options.arc_open.is_empty() {
        let mut issues = Vec::new();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for a in &net.arcs {
            *seen.entry(a.id.trim()).or_default() += 1;
        }
        let mut dup: Vec<&str> = seen
            .iter()
            .filter(|(_, n)| **n > 1)
            .map(|(k, _)| *k)
            .collect();
        dup.sort_unstable();
        for d in dup {
            issues.push(
                ValidationIssue::new(
                    "DUPLICATE_EDGE_ID",
                    format!("Edge ID '{d}' is duplicated in the network."),
                )
                .entity("pipe", d),
            );
        }
        let mut given: Vec<&str> = Vec::new();
        for (id, state) in &options.arc_open {
            let id = id.trim();
            if given.contains(&id) {
                issues.push(
                    ValidationIssue::new(
                        "DUPLICATE_EDGE_OVERRIDE",
                        format!("Edge '{id}' is overridden twice."),
                    )
                    .entity("pipe", id),
                );
                continue;
            }
            given.push(id);
            match net.arcs.iter().position(|a| a.id.trim() == id) {
                Some(k) => open[k] = *state,
                None => issues.push(
                    ValidationIssue::new(
                        "UNKNOWN_EDGE_OVERRIDE",
                        format!("Edge override '{id}' does not match any pipe."),
                    )
                    .entity("pipe", id),
                ),
            }
        }
        if !issues.is_empty() {
            return Err(SimError::Validation(issues));
        }
    }
    let params = normalize_params(net, options);
    let nodes = net
        .nodes
        .iter()
        .enumerate()
        .map(|(index, n)| NodeRow {
            id: n.id.trim().to_string(),
            haut: n.elevation,
            x: n.x,
            y: n.y,
            orif_ideal: n.orifice_ideal,
            orif_com: n.orifice_commercial,
            statut: n.status.code(),
            index,
        })
        .collect();
    let pipes = net
        .arcs
        .iter()
        .enumerate()
        .map(|(index, a)| PipeRow {
            eid: a.id.trim().to_string(),
            pred: a.from.trim().to_string(),
            succ: a.to.trim().to_string(),
            lon: a.length,
            lon1: a.length1,
            id1: diameter_ref(&a.pipe1),
            lon2: a.length2,
            id2: diameter_ref(&a.pipe2),
            open: open[index],
            index,
        })
        .collect();
    let clients = net
        .clients
        .iter()
        .map(|c| ClientRow {
            id: c.id.trim().to_string(),
            borne: c.terminal.trim().to_string(),
            proba: c.probability,
            cible: c.target_flow,
        })
        .collect();
    let pressure_override = options.overrides.required_pressure_m;
    let bornes = net
        .terminals
        .iter()
        .map(|t| BorneRow {
            id: t.id.trim().to_string(),
            pressure: pressure_override.unwrap_or(t.pressure),
        })
        .collect();
    let diameters = net
        .pipes
        .iter()
        .map(|d| DiamRow {
            id: d.id.trim().to_string(),
            diam: d.diameter,
            cost: d.cost,
            pmax: d.max_pressure,
            roughness: d.roughness,
        })
        .collect();
    Ok(Tables {
        nodes,
        pipes,
        clients,
        bornes,
        diameters,
        params,
    })
}

/// Prepared hydraulic data (`HydraulicPreprocessor`), every vector in sorted
/// order.
#[derive(Clone, Debug)]
pub(crate) struct Model {
    pub(crate) nodes: Vec<NodeRow>,
    pub(crate) pipes: Vec<PipeRow>,
    pub(crate) kind: Vec<PipeKind>,
    pub(crate) gravity: Vec<f64>,
    pub(crate) beta: Vec<f64>,
    pub(crate) diam1: Vec<f64>,
    pub(crate) diam2: Vec<f64>,
    pub(crate) alpha_ideal: Vec<f64>,
    pub(crate) alpha_com: Vec<f64>,
    /// Transit row of `pred` and of `succ` (`None` for a source or terminal).
    pub(crate) rows: Vec<(Option<usize>, Option<usize>)>,
    /// Indices in `nodes` of the transit, terminal and source nodes.
    pub(crate) transit: Vec<usize>,
    pub(crate) terminals: Vec<usize>,
    pub(crate) sources: Vec<usize>,
    /// Terminal pipe of each terminal (aligned by sorting).
    pub(crate) terminal_pipe: Vec<usize>,
    pub(crate) clients: Vec<ClientRow>,
    /// Terminal index of each client.
    pub(crate) client_terminal: Vec<usize>,
    /// Required pressure of each terminal, m.
    pub(crate) preq: Vec<f64>,
    pub(crate) params: ParamMap,
}

impl Model {
    pub(crate) fn param(&self, name: &str) -> f64 {
        self.params.get(name).copied().unwrap_or(f64::NAN)
    }
}

fn plain(message: impl Into<String>) -> SimError {
    SimError::Preprocessing(message.into())
}

/// `HydraulicPreprocessor.prepare` (spec 1.6), on validated tables.
pub(crate) fn preprocess(t: Tables) -> Result<Model, SimError> {
    let Tables {
        mut nodes,
        mut pipes,
        mut clients,
        mut bornes,
        diameters,
        params,
    } = t;
    let p = |k: &str| params.get(k).copied().unwrap_or(f64::NAN);
    let status: HashMap<String, i32> = nodes.iter().map(|n| (n.id.clone(), n.statut)).collect();
    let haut: HashMap<String, f64> = nodes.iter().map(|n| (n.id.clone(), n.haut)).collect();
    let orifices: HashMap<String, (f64, f64)> = nodes
        .iter()
        .map(|n| (n.id.clone(), (n.orif_ideal, n.orif_com)))
        .collect();
    let diam: HashMap<&str, f64> = diameters.iter().map(|d| (d.id.as_str(), d.diam)).collect();
    // sorting (1.6 step 8), all stable, with python's identifier keys
    nodes.sort_by_cached_key(|n| (n.statut, py_id(&n.id)));
    let pipe_status = |r: &PipeRow| -> i32 {
        if status.get(&r.succ) == Some(&2) {
            2
        } else if status.get(&r.pred) == Some(&0) {
            0
        } else {
            1
        }
    };
    pipes.sort_by_cached_key(|r| {
        (
            pipe_status(r),
            py_id(&r.succ),
            py_id(&r.pred),
            py_id(&r.eid),
        )
    });
    clients.sort_by_cached_key(|c| (py_id(&c.borne), py_id(&c.id)));
    bornes.sort_by_cached_key(|b| py_id(&b.id));

    let bescal = p("bescal");
    let nu = p("nu");
    let faucetcoef = p("faucetcoef");
    let orifcoef = p("orifcoef");
    let faucet_alpha = if faucetcoef > 0.0 {
        1.0 / faucetcoef
    } else {
        0.0
    };
    let n = pipes.len();
    let mut kind = Vec::with_capacity(n);
    let mut gravity = Vec::with_capacity(n);
    let mut beta = Vec::with_capacity(n);
    let mut diam1 = Vec::with_capacity(n);
    let mut diam2 = Vec::with_capacity(n);
    let mut alpha_ideal = Vec::with_capacity(n);
    let mut alpha_com = Vec::with_capacity(n);
    for r in &pipes {
        let k = match pipe_status(r) {
            0 => PipeKind::Source,
            2 => PipeKind::Terminal,
            _ => PipeKind::Transit,
        };
        let z = |id: &str| haut.get(id).copied().unwrap_or(f64::NAN);
        gravity.push(match k {
            PipeKind::Source => -z(&r.pred),
            PipeKind::Terminal => z(&r.succ),
            PipeKind::Transit => 0.0,
        });
        let segment = |len: f64, id: &Option<String>| -> Result<f64, SimError> {
            if len > 0.0 {
                let d = id
                    .as_deref()
                    .and_then(|i| diam.get(i).copied())
                    .ok_or_else(|| {
                        plain(format!("Pipe '{}' references an unknown diameter.", r.eid))
                    })?;
                if !d.is_finite() || d <= 0.0 {
                    return Err(plain(format!(
                        "Pipe '{}' has an invalid active diameter.",
                        r.eid
                    )));
                }
                Ok(d)
            } else {
                Ok(1.0)
            }
        };
        let d1 = segment(r.lon1, &r.id1)?;
        let d2 = segment(r.lon2, &r.id2)?;
        let mut b = 0.0;
        if r.lon1 > 0.0 {
            b = bescal * r.lon1 / d1.powf(nu);
        }
        if r.lon2 > 0.0 {
            b += bescal * r.lon2 / d2.powf(nu);
        }
        if !b.is_finite() || b <= 0.0 {
            return Err(plain(format!(
                "Pipe '{}' has an invalid beta coefficient.",
                r.eid
            )));
        }
        beta.push(b);
        diam1.push(d1);
        diam2.push(d2);
        let (oi, oc) = if k == PipeKind::Terminal {
            orifices.get(&r.succ).copied().unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };
        alpha_ideal.push(if oi > 0.0 {
            (orifcoef / oi).powf(4.0) + faucet_alpha
        } else {
            0.0
        });
        alpha_com.push(if oc > 0.0 {
            (orifcoef / oc).powf(4.0) + faucet_alpha
        } else {
            0.0
        });
        kind.push(k);
    }
    let transit: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].statut == 1).collect();
    let terminals: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].statut == 2).collect();
    let sources: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].statut == 0).collect();
    let row_of: HashMap<&str, usize> = transit
        .iter()
        .enumerate()
        .map(|(r, &i)| (nodes[i].id.as_str(), r))
        .collect();
    let rows = pipes
        .iter()
        .map(|r| {
            (
                row_of.get(r.pred.as_str()).copied(),
                row_of.get(r.succ.as_str()).copied(),
            )
        })
        .collect();
    let terminal_pipe: Vec<usize> = (0..n).filter(|&j| kind[j] == PipeKind::Terminal).collect();
    // consistency checks (1.6 step 11)
    if terminal_pipe.len() != terminals.len() || terminals.len() != bornes.len() {
        return Err(plain(
            "Terminal pipes, terminal nodes and bornes are not aligned.",
        ));
    }
    for (t, &j) in terminal_pipe.iter().enumerate() {
        if pipes[j].succ != nodes[terminals[t]].id {
            return Err(plain("Terminal pipes are not aligned with terminal nodes."));
        }
    }
    if !kind.contains(&PipeKind::Source) {
        return Err(plain("The network has no source pipe."));
    }
    if gravity
        .iter()
        .chain(&beta)
        .chain(&alpha_ideal)
        .chain(&alpha_com)
        .any(|v| !v.is_finite())
    {
        return Err(plain("Hydraulic coefficients contain non-finite values."));
    }
    let terminal_index: HashMap<&str, usize> = terminals
        .iter()
        .enumerate()
        .map(|(t, &i)| (nodes[i].id.as_str(), t))
        .collect();
    let mut client_terminal = Vec::with_capacity(clients.len());
    let mut unknown: Vec<String> = Vec::new();
    for c in &clients {
        match terminal_index.get(c.borne.as_str()) {
            Some(&t) => client_terminal.push(t),
            None => unknown.push(c.borne.clone()),
        }
    }
    if !unknown.is_empty() {
        unknown.sort();
        unknown.dedup();
        return Err(SimError::ScenarioGeneration {
            message: "Some clients cannot be mapped to terminal nodes.".into(),
            details: vec![("unknownTerminalIds".into(), unknown.into())],
        });
    }
    let pressure: HashMap<&str, f64> = bornes.iter().map(|b| (b.id.as_str(), b.pressure)).collect();
    let mut preq = Vec::with_capacity(terminals.len());
    for &i in &terminals {
        match pressure.get(nodes[i].id.as_str()) {
            Some(v) if v.is_finite() => preq.push(*v),
            _ => {
                return Err(SimError::ScenarioPreparation {
                    message: "Required terminal pressures contain missing or invalid values."
                        .into(),
                    details: Vec::new(),
                });
            }
        }
    }
    Ok(Model {
        nodes,
        pipes,
        kind,
        gravity,
        beta,
        diam1,
        diam2,
        alpha_ideal,
        alpha_com,
        rows,
        transit,
        terminals,
        sources,
        terminal_pipe,
        clients,
        client_terminal,
        preq,
        params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_names() {
        assert_eq!(canonical_param("nScenLoad"), "n_scen_load");
        assert_eq!(canonical_param("bounded-flow"), "bounded_flow");
        assert_eq!(canonical_param("serviceequal"), "servicequal");
        assert_eq!(canonical_param("uniform_dem"), "uniform_dem");
        assert_eq!(canonical_param("targetFlow"), "targetflow");
        assert_eq!(canonical_param("vmax_secu"), "vmax_secu");
    }

    #[test]
    fn identifiers_like_python() {
        assert_eq!(py_id("007"), "7");
        assert_eq!(py_id("1.50"), "1.5");
        assert_eq!(py_id("1e3"), "1000");
        assert_eq!(py_id("166T"), "166T");
        assert_eq!(py_id("-0"), "0");
        assert_eq!(py_id("P1"), "P1");
    }
}
