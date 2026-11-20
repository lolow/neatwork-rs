//! Input validation (`validate_simulation_data`, spec section 6.1) and
//! option resolution (`resolve_simulation_options`, spec 2.2 and 6.2).

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use crate::error::ValidationIssue as Issue;
use crate::json::Value;
use crate::options::{SimulationOptions, SolverOptions};
use crate::prep::{ParamMap, Tables};

fn sorted_duplicates<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut count: HashMap<&str, usize> = HashMap::new();
    for id in ids {
        if !id.is_empty() {
            *count.entry(id).or_default() += 1;
        }
    }
    let mut d: Vec<String> = count
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(k, _)| k.to_string())
        .collect();
    d.sort();
    d
}

fn validate_nodes(t: &Tables, out: &mut Vec<Issue>) {
    if t.nodes.is_empty() {
        out.push(
            Issue::new("EMPTY_NODE_TABLE", "The simulation network has no nodes.")
                .entity("table", "nodes"),
        );
        return;
    }
    for (k, n) in t.nodes.iter().enumerate() {
        if n.id.is_empty() {
            out.push(
                Issue::new("EMPTY_NODE_ID", format!("Node at row {k} has an empty ID."))
                    .entity("node", format!("row-{k}"))
                    .field("noeud"),
            );
        }
    }
    for id in sorted_duplicates(t.nodes.iter().map(|n| n.id.as_str())) {
        out.push(
            Issue::new(
                "DUPLICATE_NODE_ID",
                format!("Node ID '{id}' is duplicated."),
            )
            .entity("node", id)
            .field("noeud"),
        );
    }
    for (k, n) in t.nodes.iter().enumerate() {
        let id = if n.id.is_empty() {
            format!("row-{k}")
        } else {
            n.id.clone()
        };
        for (column, v) in [
            ("haut", n.haut),
            ("xdata", n.x),
            ("ydata", n.y),
            ("orif_ideal", n.orif_ideal),
            ("orif_com", n.orif_com),
        ] {
            if !v.is_finite() {
                out.push(
                    Issue::new(
                        "INVALID_NODE_NUMBER",
                        format!("Node '{id}' has an invalid numeric value for '{column}'."),
                    )
                    .entity("node", id.clone())
                    .field(column),
                );
            }
        }
    }
    if !t.nodes.iter().any(|n| n.statut == 0) {
        out.push(
            Issue::new(
                "MISSING_SOURCE_NODE",
                "The network must contain at least one source node.",
            )
            .entity("table", "nodes"),
        );
    }
    if !t.nodes.iter().any(|n| n.statut == 2) {
        out.push(
            Issue::new(
                "MISSING_TERMINAL_NODE",
                "The network must contain at least one terminal node.",
            )
            .entity("table", "nodes"),
        );
    }
}

fn validate_pipes(t: &Tables, out: &mut Vec<Issue>) {
    if t.pipes.is_empty() {
        out.push(
            Issue::new("EMPTY_PIPE_TABLE", "The simulation network has no pipes.")
                .entity("table", "pipes"),
        );
        return;
    }
    let node_ids: HashSet<&str> = t.nodes.iter().map(|n| n.id.as_str()).collect();
    let diameter_ids: HashSet<&str> = t.diameters.iter().map(|d| d.id.as_str()).collect();
    for id in sorted_duplicates(t.pipes.iter().map(|p| p.eid.as_str())) {
        out.push(
            Issue::new(
                "DUPLICATE_PIPE_ID",
                format!("Pipe ID '{id}' is duplicated."),
            )
            .entity("pipe", id)
            .field("external_edge_id"),
        );
    }
    for (k, p) in t.pipes.iter().enumerate() {
        let id = if p.eid.is_empty() {
            format!("row-{k}")
        } else {
            p.eid.clone()
        };
        if p.eid.is_empty() {
            out.push(
                Issue::new("EMPTY_PIPE_ID", format!("Pipe at row {k} has an empty ID."))
                    .entity("pipe", format!("row-{k}"))
                    .field("external_edge_id"),
            );
        }
        for (field, node, end) in [
            ("pred", &p.pred, "predecessor"),
            ("succ", &p.succ, "successor"),
        ] {
            if !node_ids.contains(node.as_str()) {
                out.push(
                    Issue::new(
                        "UNKNOWN_PIPE_NODE",
                        format!("Pipe '{id}' references unknown {end} node '{node}'."),
                    )
                    .entity("pipe", id.clone())
                    .field(field)
                    .detail("nodeId", node.as_str()),
                );
            }
        }
        if p.pred == p.succ {
            out.push(
                Issue::new(
                    "PIPE_SELF_LOOP",
                    format!("Pipe '{id}' connects node '{}' to itself.", p.pred),
                )
                .entity("pipe", id.clone()),
            );
        }
        let mut valid = true;
        for (column, v) in [("lon", p.lon), ("lon1", p.lon1), ("lon2", p.lon2)] {
            if !v.is_finite() {
                valid = false;
                out.push(
                    Issue::new(
                        "INVALID_PIPE_LENGTH",
                        format!("Pipe '{id}' has an invalid value for '{column}'."),
                    )
                    .entity("pipe", id.clone())
                    .field(column),
                );
            } else if v < 0.0 {
                valid = false;
                out.push(
                    Issue::new(
                        "NEGATIVE_PIPE_LENGTH",
                        format!("Pipe '{id}' has a negative value for '{column}'."),
                    )
                    .entity("pipe", id.clone())
                    .field(column),
                );
            }
        }
        if !valid {
            continue;
        }
        let segments = p.lon1 + p.lon2;
        if segments <= 0.0 {
            out.push(
                Issue::new(
                    "ZERO_PIPE_LENGTH",
                    format!("Pipe '{id}' must have a strictly positive segment length."),
                )
                .entity("pipe", id.clone())
                .field("lon1"),
            );
        }
        // numpy isclose(lon, lon1+lon2, rtol=1e-6, atol=1e-9)
        if (p.lon - segments).abs() > 1e-9 + 1e-6 * segments.abs() {
            out.push(
                Issue::new(
                    "PIPE_LENGTH_MISMATCH",
                    format!("Pipe '{id}' total length does not match lon1 + lon2."),
                )
                .entity("pipe", id.clone())
                .field("lon")
                .detail("lon", p.lon)
                .detail("lon1PlusLon2", segments),
            );
        }
        for (field, len, did) in [("ID1", p.lon1, &p.id1), ("ID2", p.lon2, &p.id2)] {
            if len <= 0.0 {
                continue;
            }
            match did {
                None => out.push(
                    Issue::new(
                        "MISSING_PIPE_DIAMETER",
                        format!(
                            "Pipe '{id}' requires {field} because its segment length is positive."
                        ),
                    )
                    .entity("pipe", id.clone())
                    .field(field),
                ),
                Some(d) if !diameter_ids.contains(d.as_str()) => out.push(
                    Issue::new(
                        "UNKNOWN_PIPE_DIAMETER",
                        format!("Pipe '{id}' references unknown diameter '{d}'."),
                    )
                    .entity("pipe", id.clone())
                    .field(field)
                    .detail("diameterId", d.as_str()),
                ),
                Some(_) => {}
            }
        }
    }
}

fn validate_clients(t: &Tables, terminals: &BTreeSet<&str>, out: &mut Vec<Issue>) {
    if t.clients.is_empty() {
        out.push(
            Issue::new(
                "EMPTY_CLIENT_TABLE",
                "The network must contain at least one client.",
            )
            .entity("table", "clients"),
        );
        return;
    }
    for id in sorted_duplicates(t.clients.iter().map(|c| c.id.as_str())) {
        out.push(
            Issue::new(
                "DUPLICATE_CLIENT_ID",
                format!("Client ID '{id}' is duplicated."),
            )
            .entity("client", id)
            .field("client"),
        );
    }
    for (k, c) in t.clients.iter().enumerate() {
        let id = if c.id.is_empty() {
            format!("row-{k}")
        } else {
            c.id.clone()
        };
        if c.id.is_empty() {
            out.push(
                Issue::new(
                    "EMPTY_CLIENT_ID",
                    format!("Client at row {k} has an empty ID."),
                )
                .entity("client", format!("row-{k}"))
                .field("client"),
            );
        }
        if !terminals.contains(c.borne.as_str()) {
            out.push(
                Issue::new(
                    "UNKNOWN_CLIENT_TERMINAL",
                    format!("Client '{id}' references unknown terminal '{}'.", c.borne),
                )
                .entity("client", id.clone())
                .field("borne"),
            );
        }
        if !c.proba.is_finite() {
            out.push(
                Issue::new(
                    "INVALID_CLIENT_PROBABILITY",
                    format!("Client '{id}' has an invalid opening probability."),
                )
                .entity("client", id.clone())
                .field("proba"),
            );
        } else if !(0.0..=1.0).contains(&c.proba) {
            out.push(
                Issue::new(
                    "CLIENT_PROBABILITY_OUT_OF_RANGE",
                    format!("Client '{id}' probability must be between 0 and 1."),
                )
                .entity("client", id.clone())
                .field("proba")
                .detail("value", c.proba),
            );
        }
        if !c.cible.is_finite() || c.cible <= 0.0 {
            out.push(
                Issue::new(
                    "INVALID_CLIENT_TARGET_FLOW",
                    format!("Client '{id}' target flow must be strictly positive."),
                )
                .entity("client", id.clone())
                .field("cible"),
            );
        }
    }
    let used: HashSet<&str> = t.clients.iter().map(|c| c.borne.as_str()).collect();
    for term in terminals.iter().filter(|x| !used.contains(*x)) {
        out.push(
            Issue::new(
                "TERMINAL_WITHOUT_CLIENT",
                format!("Terminal '{term}' has no associated client."),
            )
            .entity("node", *term),
        );
    }
    let p = &t.params;
    if p.get("uniform_dem").copied().unwrap_or(1.0) == 1.0 {
        let open = p.get("opentaps").copied().unwrap_or(f64::NAN);
        if !open.is_finite() {
            return;
        }
        if open <= 0.0 {
            out.push(
                Issue::new("NO_ACTIVE_CLIENT_PROBABILITY", "Parameter 'opentaps' must be greater than zero for uniform-demand simulations.")
                    .entity("parameter", "opentaps"),
            );
        } else if !t.clients.iter().any(|c| c.proba > 0.0) {
            out.push(
                Issue::new(
                    "NO_ACTIVE_CLIENT_PROBABILITY",
                    "At least one client must have a positive opening probability.",
                )
                .entity("table", "clients")
                .field("proba"),
            );
        }
    }
}

fn validate_bornes(t: &Tables, terminals: &BTreeSet<&str>, out: &mut Vec<Issue>) {
    if t.bornes.is_empty() {
        out.push(
            Issue::new("EMPTY_BORNE_TABLE", "The borne table cannot be empty.")
                .entity("table", "bornes"),
        );
        return;
    }
    for id in sorted_duplicates(t.bornes.iter().map(|b| b.id.as_str())) {
        out.push(
            Issue::new(
                "DUPLICATE_BORNE_ID",
                format!("Borne ID '{id}' is duplicated."),
            )
            .entity("borne", id)
            .field("borne"),
        );
    }
    let ids: BTreeSet<&str> = t.bornes.iter().map(|b| b.id.as_str()).collect();
    for term in terminals.difference(&ids) {
        out.push(
            Issue::new(
                "MISSING_TERMINAL_BORNE",
                format!("Terminal '{term}' is missing from the borne table."),
            )
            .entity("node", *term),
        );
    }
    for b in ids.difference(terminals) {
        out.push(
            Issue::new(
                "UNKNOWN_BORNE_TERMINAL",
                format!("Borne '{b}' is not a terminal node."),
            )
            .entity("borne", *b),
        );
    }
    for (k, b) in t.bornes.iter().enumerate() {
        if !(b.pressure.is_finite() && b.pressure >= 0.0) {
            let id = if b.id.is_empty() {
                format!("row-{k}")
            } else {
                b.id.clone()
            };
            out.push(
                Issue::new(
                    "INVALID_BORNE_PRESSURE",
                    format!("Borne '{id}' pressure must be finite and non-negative."),
                )
                .entity("borne", id)
                .field("pressure"),
            );
        }
    }
}

fn validate_diameters(t: &Tables, out: &mut Vec<Issue>) {
    if t.diameters.is_empty() {
        out.push(
            Issue::new(
                "EMPTY_DIAMETER_TABLE",
                "The diameter table cannot be empty.",
            )
            .entity("table", "diameters"),
        );
        return;
    }
    for id in sorted_duplicates(t.diameters.iter().map(|d| d.id.as_str())) {
        out.push(
            Issue::new(
                "DUPLICATE_DIAMETER_ID",
                format!("Diameter ID '{id}' is duplicated."),
            )
            .entity("diameter", id)
            .field("ID"),
        );
    }
    type Getter = fn(&crate::prep::DiamRow) -> f64;
    type Rule = fn(f64) -> bool;
    let rules: [(&str, Getter, Rule); 4] = [
        ("diam", |d| d.diam, |v| v > 0.0),
        ("cost", |d| d.cost, |v| v >= 0.0),
        ("Pmax", |d| d.pmax, |v| v >= 0.0),
        ("Roughness", |d| d.roughness, |v| v >= 0.0),
    ];
    for (column, get, rule) in rules {
        for (k, d) in t.diameters.iter().enumerate() {
            let id = if d.id.is_empty() {
                format!("row-{k}")
            } else {
                d.id.clone()
            };
            let v = get(d);
            if !v.is_finite() {
                out.push(
                    Issue::new(
                        "INVALID_DIAMETER_VALUE",
                        format!("Diameter '{id}' has an invalid value for '{column}'."),
                    )
                    .entity("diameter", id)
                    .field(column),
                );
            } else if !rule(v) {
                out.push(
                    Issue::new(
                        "INVALID_DIAMETER_VALUE",
                        format!("Diameter '{id}' has an unsupported value for '{column}'."),
                    )
                    .entity("diameter", id)
                    .field(column)
                    .detail("value", v),
                );
            }
        }
    }
}

fn validate_params(t: &Tables, out: &mut Vec<Issue>) {
    let p: &ParamMap = &t.params;
    let get = |k: &str| p.get(k).copied().unwrap_or(f64::NAN);
    let required = [
        "bescal",
        "bounded_flow",
        "faucetcoef",
        "lambda",
        "nbsim",
        "nu",
        "opentaps",
        "orif_com",
        "orifcoef",
        "pressure",
        "servicequal",
        "targetflow",
        "uniform_dem",
        "vmax",
        "vmax_secu",
    ];
    for name in required {
        if !p.contains_key(name) {
            out.push(
                Issue::new(
                    "MISSING_SIMULATION_PARAMETER",
                    format!("Required simulation parameter '{name}' is missing."),
                )
                .entity("parameter", name),
            );
        }
    }
    for name in [
        "bounded_flow",
        "fusion",
        "orif_com",
        "split",
        "uniform_dem",
        "use_efficient_pipes",
    ] {
        if let Some(&v) = p.get(name)
            && v != 0.0
            && v != 1.0
        {
            out.push(
                Issue::new(
                    "INVALID_BINARY_PARAMETER",
                    format!("Parameter '{name}' must be 0 or 1."),
                )
                .entity("parameter", name)
                .detail("value", v),
            );
        }
    }
    let nbsim = get("nbsim");
    if !nbsim.is_finite() || nbsim <= 0.0 || nbsim != nbsim.trunc() {
        out.push(
            Issue::new(
                "INVALID_SCENARIO_COUNT",
                "Parameter 'nbsim' must be a positive integer.",
            )
            .entity("parameter", "nbsim"),
        );
    }
    for name in ["bescal", "lambda", "nu", "orifcoef", "vmax", "vmax_secu"] {
        let v = get(name);
        if !v.is_finite() || v <= 0.0 {
            out.push(
                Issue::new(
                    "INVALID_POSITIVE_PARAMETER",
                    format!("Parameter '{name}' must be strictly positive."),
                )
                .entity("parameter", name)
                .detail("value", v),
            );
        }
    }
    for name in ["faucetcoef", "pressure"] {
        let v = get(name);
        if !v.is_finite() || v < 0.0 {
            out.push(
                Issue::new(
                    "INVALID_NON_NEGATIVE_PARAMETER",
                    format!("Parameter '{name}' must be non-negative."),
                )
                .entity("parameter", name)
                .detail("value", v),
            );
        }
    }
    for name in ["opentaps", "servicequal"] {
        let v = get(name);
        if !v.is_finite() || !(0.0..=1.0).contains(&v) {
            out.push(
                Issue::new(
                    "PARAMETER_OUT_OF_RANGE",
                    format!("Parameter '{name}' must be between 0 and 1."),
                )
                .entity("parameter", name)
                .detail("value", v),
            );
        }
    }
    let bounded = get("bounded_flow");
    let uniform = get("uniform_dem");
    if bounded == 0.0 && uniform == 0.0 {
        out.push(
            Issue::new(
                "UNSUPPORTED_SIMULATION_CASE",
                "The combination uniform_dem=0 and bounded_flow=0 is not supported.",
            )
            .entity("parameter", "uniform_dem"),
        );
    }
    if bounded == 0.0 {
        let q = get("targetflow");
        if !q.is_finite() || q <= 0.0 {
            out.push(
                Issue::new("INVALID_TARGET_FLOW_PARAMETER", "Parameter 'targetflow' must be strictly positive for unbounded-flow simulation.")
                    .entity("parameter", "targetflow")
                    .detail("value", q),
            );
        }
        let commercial = get("orif_com") == 1.0;
        let column = if commercial { "orif_com" } else { "orif_ideal" };
        for n in t.nodes.iter().filter(|n| n.statut == 2) {
            let v = if commercial { n.orif_com } else { n.orif_ideal };
            if v.is_finite() && v > 0.0 {
                continue;
            }
            out.push(
                Issue::new(
                    "INVALID_TERMINAL_ORIFICE",
                    format!("Terminal '{}' must have a positive '{column}' value for the selected orifice model (unbounded-flow simulation).", n.id),
                )
                .entity("node", n.id.clone())
                .field(column)
                .detail("orificeMode", if commercial { "COMMERCIAL" } else { "IDEAL" }),
            );
        }
    }
}

fn validate_topology(t: &Tables, out: &mut Vec<Issue>) {
    let node_ids: BTreeSet<&str> = t.nodes.iter().map(|n| n.id.as_str()).collect();
    let sources: BTreeSet<&str> = t
        .nodes
        .iter()
        .filter(|n| n.statut == 0)
        .map(|n| n.id.as_str())
        .collect();
    let terminals: BTreeSet<&str> = t
        .nodes
        .iter()
        .filter(|n| n.statut == 2)
        .map(|n| n.id.as_str())
        .collect();
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, usize> = HashMap::new();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for p in &t.pipes {
        if sources.contains(p.pred.as_str()) && terminals.contains(p.succ.as_str()) {
            out.push(
                Issue::new(
                    "DIRECT_SOURCE_TO_TERMINAL_PIPE",
                    format!("Pipe '{}' connects a source directly to a terminal.", p.eid),
                )
                .entity("pipe", p.eid.clone()),
            );
        }
        if !node_ids.contains(p.pred.as_str()) || !node_ids.contains(p.succ.as_str()) {
            continue;
        }
        *outgoing.entry(&p.pred).or_default() += 1;
        *incoming.entry(&p.succ).or_default() += 1;
        adjacency.entry(&p.pred).or_default().push(&p.succ);
    }
    for s in &sources {
        if incoming.get(s).copied().unwrap_or(0) > 0 {
            out.push(
                Issue::new(
                    "SOURCE_HAS_INCOMING_PIPE",
                    format!("Source node '{s}' cannot have incoming pipes."),
                )
                .entity("node", *s),
            );
        }
        if outgoing.get(s).copied().unwrap_or(0) == 0 {
            out.push(
                Issue::new(
                    "SOURCE_WITHOUT_OUTGOING_PIPE",
                    format!("Source node '{s}' has no outgoing pipe."),
                )
                .entity("node", *s),
            );
        }
    }
    for term in &terminals {
        if outgoing.get(term).copied().unwrap_or(0) > 0 {
            out.push(
                Issue::new(
                    "TERMINAL_HAS_OUTGOING_PIPE",
                    format!("Terminal node '{term}' cannot have outgoing pipes."),
                )
                .entity("node", *term),
            );
        }
        let n = incoming.get(term).copied().unwrap_or(0);
        if n != 1 {
            out.push(
                Issue::new(
                    "INVALID_TERMINAL_IN_DEGREE",
                    format!("Terminal node '{term}' must have exactly one incoming pipe."),
                )
                .entity("node", *term)
                .detail("incomingPipeCount", n),
            );
        }
    }
    if sources.is_empty() {
        return;
    }
    // reachability ignores the open state, as python-core
    let mut reached: HashSet<&str> = sources.iter().copied().collect();
    let mut queue: VecDeque<&str> = sources.iter().copied().collect();
    while let Some(v) = queue.pop_front() {
        for &w in adjacency.get(v).map(Vec::as_slice).unwrap_or(&[]) {
            if reached.insert(w) {
                queue.push_back(w);
            }
        }
    }
    for n in node_ids.iter().filter(|n| !reached.contains(*n)) {
        out.push(
            Issue::new(
                "NODE_UNREACHABLE_FROM_SOURCE",
                format!("Node '{n}' is not reachable from any source node."),
            )
            .entity("node", *n),
        );
    }
}

/// All input issues, in python-core order.
pub(crate) fn validate_tables(t: &Tables) -> Vec<Issue> {
    let mut out = Vec::new();
    validate_nodes(t, &mut out);
    validate_pipes(t, &mut out);
    let terminals: BTreeSet<&str> = t
        .nodes
        .iter()
        .filter(|n| n.statut == 2)
        .map(|n| n.id.as_str())
        .collect();
    validate_clients(t, &terminals, &mut out);
    validate_bornes(t, &terminals, &mut out);
    validate_diameters(t, &mut out);
    validate_params(t, &mut out);
    validate_topology(t, &mut out);
    out
}

/// Resolved run options (`ResolvedSimulationOptions`).
#[derive(Clone, Debug)]
pub(crate) struct Resolved {
    pub(crate) scenario_count: usize,
    pub(crate) uniform: bool,
    pub(crate) open_taps_probability: Option<f64>,
    pub(crate) target_flow_lps: Option<f64>,
}

fn solver_issues(s: &SolverOptions, out: &mut Vec<Issue>) {
    let positive = [
        ("flow_tolerance", s.flow_tolerance),
        ("pressure_tolerance", s.pressure_tolerance),
        ("centering_tolerance", s.centering_tolerance),
        ("centering_exponent", s.centering_exponent),
    ];
    for (name, v) in positive {
        if !v.is_finite() || v <= 0.0 {
            out.push(
                Issue::new(
                    "INVALID_SOLVER_OPTION",
                    format!("Solver option '{name}' must be finite and strictly positive."),
                )
                .entity("option", name),
            );
        }
    }
    if s.max_iterations == 0 {
        out.push(
            Issue::new(
                "INVALID_SOLVER_OPTION",
                "Solver option 'max_iterations' must be strictly positive.",
            )
            .entity("option", "max_iterations"),
        );
    }
    for (name, v) in [
        ("barrier_reduction", s.barrier_reduction),
        ("step_safety", s.step_safety),
    ] {
        if !(v > 0.0 && v < 1.0) {
            out.push(
                Issue::new(
                    "INVALID_SOLVER_OPTION",
                    format!("Solver option '{name}' must be between 0 and 1."),
                )
                .entity("option", name),
            );
        }
    }
}

/// `resolve_simulation_options` and `resolve_solver_options`.
pub(crate) fn resolve_options(
    params: &ParamMap,
    clients: &[crate::prep::ClientRow],
    o: &SimulationOptions,
) -> Result<Resolved, Vec<Issue>> {
    let mut out = Vec::new();
    let get = |k: &str, d: f64| params.get(k).copied().unwrap_or(d);
    let uniform = get("uniform_dem", 1.0) != 0.0;
    let count = o.scenario_count.unwrap_or_else(|| {
        let n = get("nbsim", 100.0);
        if n.is_finite() && n > 0.0 {
            n as usize
        } else {
            0
        }
    });
    if count == 0 {
        out.push(
            Issue::new(
                "INVALID_SCENARIO_COUNT",
                "The number of scenarios must be strictly positive.",
            )
            .entity("option", "scenario_count")
            .detail("value", Value::Int(0)),
        );
    }
    if o.max_generation_batches == 0 {
        out.push(
            Issue::new(
                "INVALID_GENERATION_BATCH_COUNT",
                "max_generation_batches must be strictly positive.",
            )
            .entity("option", "max_generation_batches"),
        );
    }
    if o.oversampling_factor.is_nan() || o.oversampling_factor <= 1.0 {
        out.push(
            Issue::new(
                "INVALID_OVERSAMPLING_FACTOR",
                "oversampling_factor must be greater than 1.",
            )
            .entity("option", "oversampling_factor")
            .detail("value", o.oversampling_factor),
        );
    }
    if o.max_draws_per_batch == 0 {
        out.push(
            Issue::new(
                "INVALID_MAX_DRAWS_PER_BATCH",
                "max_draws_per_batch must be strictly positive.",
            )
            .entity("option", "max_draws_per_batch"),
        );
    }
    let (mut p, mut q) = (None, None);
    if uniform {
        let open = get("opentaps", 1.0);
        if !(0.0..=1.0).contains(&open) {
            out.push(
                Issue::new(
                    "OPEN_TAPS_PROBABILITY_OUT_OF_RANGE",
                    "open_taps_probability must be between 0 and 1.",
                )
                .entity("option", "open_taps_probability")
                .detail("value", open),
            );
        }
        if open == 0.0 {
            out.push(
                Issue::new("ZERO_OPEN_TAPS_PROBABILITY", "open_taps_probability must be greater than zero because empty scenarios are not retained.")
                    .entity("option", "open_taps_probability"),
            );
        }
        p = Some(open);
        // _resolve_uniform_target_flow: the parameter, else the single
        // positive client target
        let param = get("targetflow", f64::NAN);
        q = if param > 0.0 {
            Some(param)
        } else {
            let mut targets: Vec<f64> = clients
                .iter()
                .map(|c| c.cible)
                .filter(|v| v.is_finite() && *v > 0.0)
                .collect();
            targets.sort_by(f64::total_cmp);
            targets.dedup();
            if targets.len() == 1 {
                Some(targets[0])
            } else {
                None
            }
        };
        if q.is_none() {
            out.push(
                Issue::new(
                    "INVALID_UNIFORM_TARGET_FLOW",
                    "A strictly positive target flow is required for uniform-demand simulation.",
                )
                .entity("option", "target_flow_lps"),
            );
        }
    } else {
        if o.open_taps_probability.is_some() {
            out.push(
                Issue::new(
                    "UNSUPPORTED_UNIFORM_PROBABILITY_OVERRIDE",
                    "A global opening probability cannot override heterogeneous client demands.",
                )
                .entity("option", "open_taps_probability"),
            );
        }
        if o.target_flow_lps.is_some() {
            out.push(
                Issue::new(
                    "UNSUPPORTED_UNIFORM_FLOW_OVERRIDE",
                    "A global target flow cannot override heterogeneous client demands.",
                )
                .entity("option", "target_flow_lps"),
            );
        }
    }
    solver_issues(&o.solver, &mut out);
    if !out.is_empty() {
        return Err(out);
    }
    Ok(Resolved {
        scenario_count: count,
        uniform,
        open_taps_probability: p,
        target_flow_lps: q,
    })
}
