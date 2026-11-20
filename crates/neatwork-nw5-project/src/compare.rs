//! Comparison of two runs (`specs/app.md` F48–F51): choice of the two
//! runs, revision compatibility, the unified comparison table and the
//! numeric comparison panel.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use neatwork_nw5::Network;

use crate::format::{
    DisplayUnit, MISSING, adaptive, natural_cmp, ratio_percent, signed, signed_percent,
};
use crate::json::{Json, get_str, scalar_text};
use crate::review::{
    NodeRole, SortDirection, SortValue, TableKind, display_unit_label, table_rows, to_display,
};
use crate::run::RunRecord;

// ----- sources (F48) -----------------------------------------------------

/// Completed runs in the order of the selectors: runs of the active
/// revision first, then by run number, newest first.
pub fn comparison_sources<'a>(
    runs: &'a [RunRecord],
    active_revision: Option<&str>,
) -> Vec<&'a RunRecord> {
    let mut out: Vec<&RunRecord> = runs
        .iter()
        .filter(|r| r.status.is_completed() && r.statistics.is_some())
        .collect();
    out.sort_by(|a, b| {
        let aa = Some(a.revision.as_str()) == active_revision;
        let ba = Some(b.revision.as_str()) == active_revision;
        ba.cmp(&aa).then_with(|| b.run_number.cmp(&a.run_number))
    });
    out
}

/// Initial left and right runs: the saved choice when both runs still
/// exist, else the two newest completed runs, the older on the left (D5).
/// `None` with fewer than two completed runs.
// PROPOSAL: "the two newest" is read as the two highest run numbers, the
// newest on the right.
pub fn default_pair(runs: &[RunRecord], saved: Option<(u32, u32)>) -> Option<(u32, u32)> {
    let completed: Vec<u32> = runs
        .iter()
        .filter(|r| r.status.is_completed() && r.statistics.is_some())
        .map(|r| r.run_number)
        .collect();
    if let Some((l, r)) = saved
        && l != r
        && completed.contains(&l)
        && completed.contains(&r)
    {
        return Some((l, r));
    }
    let mut sorted = completed;
    sorted.sort_unstable();
    match sorted.as_slice() {
        [.., a, b] => Some((*a, *b)),
        _ => None,
    }
}

/// Tables both runs have (F48).
pub fn common_tables(left: &RunRecord, right: &RunRecord) -> Vec<TableKind> {
    if left.statistics.is_none() || right.statistics.is_none() {
        return Vec::new();
    }
    TableKind::ALL.to_vec()
}

// ----- compatibility (F49) -----------------------------------------------

/// Compatibility level of two revisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompatibilityLevel {
    /// The same revision file.
    SameSnapshot,
    /// Different revisions, same effective input.
    EquivalentInput,
    /// Same nodes and pipes, other input values.
    SameTopology,
    /// Nodes or pipes changed.
    TopologyChanged,
}

impl CompatibilityLevel {
    /// Code.
    pub fn as_str(self) -> &'static str {
        match self {
            CompatibilityLevel::SameSnapshot => "SAME_SNAPSHOT",
            CompatibilityLevel::EquivalentInput => "EQUIVALENT_INPUT",
            CompatibilityLevel::SameTopology => "SAME_TOPOLOGY",
            CompatibilityLevel::TopologyChanged => "TOPOLOGY_CHANGED",
        }
    }

    /// Banner title and text.
    pub fn banner(self) -> (&'static str, &'static str) {
        match self {
            CompatibilityLevel::SameSnapshot => (
                "Exact same input snapshot",
                "Both simulations were generated from exactly the same immutable network input.",
            ),
            CompatibilityLevel::EquivalentInput => (
                "Equivalent network input",
                "The simulations belong to different revisions, but their effective hydraulic input is identical.",
            ),
            CompatibilityLevel::SameTopology => (
                "Same topology — revised input",
                "Node and pipe structure is unchanged, but hydraulic input values differ.",
            ),
            CompatibilityLevel::TopologyChanged => (
                "Topology changed",
                "Only entities that exist with the same identity in both revisions are compared directly.",
            ),
        }
    }
}

/// Matching of one kind of entity (terminals, sources, transit nodes or
/// pipes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntityMatch {
    /// Same identity and context on both sides (sorted).
    pub matched: Vec<String>,
    /// Only on the left (or context changed).
    pub left_only: Vec<String>,
    /// Only on the right (or context changed).
    pub right_only: Vec<String>,
    /// Same ID, other incident pipes.
    pub context_changed: Vec<String>,
}

/// Structural changes between two revisions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChangeSummary {
    /// Nodes on the left.
    pub left_nodes: usize,
    /// Nodes on the right.
    pub right_nodes: usize,
    /// Same ID and role.
    pub matched_nodes: usize,
    /// Added nodes.
    pub added_nodes: Vec<String>,
    /// Removed nodes.
    pub removed_nodes: Vec<String>,
    /// Nodes whose role changed.
    pub changed_roles: Vec<String>,
    /// Pipes on the left.
    pub left_pipes: usize,
    /// Pipes on the right.
    pub right_pipes: usize,
    /// Same ID and endpoints.
    pub matched_pipes: usize,
    /// Added pipes.
    pub added_pipes: Vec<String>,
    /// Removed pipes.
    pub removed_pipes: Vec<String>,
    /// Pipes whose endpoints changed.
    pub changed_endpoints: Vec<String>,
    /// Terminals on the left.
    pub left_terminals: usize,
    /// Terminals on the right.
    pub right_terminals: usize,
    /// Terminals on both sides.
    pub matched_terminals: usize,
    /// Added terminals.
    pub added_terminals: Vec<String>,
    /// Removed terminals.
    pub removed_terminals: Vec<String>,
}

impl ChangeSummary {
    /// Summary lines "Nodes L → R, Matched m · +a · -r" for nodes, pipes and
    /// terminals.
    pub fn lines(&self) -> Vec<String> {
        let line = |what: &str, l: usize, r: usize, m: usize, a: usize, d: usize| {
            format!("{what} {l} → {r}, Matched {m} · +{a} · -{d}")
        };
        vec![
            line(
                "Nodes",
                self.left_nodes,
                self.right_nodes,
                self.matched_nodes,
                self.added_nodes.len(),
                self.removed_nodes.len(),
            ),
            line(
                "Pipes",
                self.left_pipes,
                self.right_pipes,
                self.matched_pipes,
                self.added_pipes.len(),
                self.removed_pipes.len(),
            ),
            line(
                "Terminals",
                self.left_terminals,
                self.right_terminals,
                self.matched_terminals,
                self.added_terminals.len(),
                self.removed_terminals.len(),
            ),
        ]
    }
}

/// Compatibility of the revisions of two runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Compatibility {
    /// Level.
    pub level: CompatibilityLevel,
    /// Same nodes (ID, role) and pipes (ID, endpoints).
    pub same_topology: bool,
    /// Same clients (ID, terminal).
    pub same_demand: bool,
    /// Same effective input.
    pub same_input: bool,
    /// Level ≠ TOPOLOGY_CHANGED.
    pub direct_comparison_allowed: bool,
    /// Always true.
    pub partial_comparison_allowed: bool,
    /// Structural changes.
    pub changes: ChangeSummary,
    /// Warning texts.
    pub warnings: Vec<String>,
    /// Terminals (role 2).
    pub terminals: EntityMatch,
    /// Sources (role 0).
    pub sources: EntityMatch,
    /// Transit nodes (role 1).
    pub transit_nodes: EntityMatch,
    /// Pipes.
    pub pipes: EntityMatch,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeId {
    id: String,
    from: String,
    to: String,
}

struct Profile {
    nodes: BTreeMap<String, i32>,
    edges: BTreeMap<String, EdgeId>,
    edge_set: BTreeSet<EdgeId>,
    clients: BTreeSet<(String, String)>,
    effective: String,
}

/// Canonical number text: shortest form, so `2`, `2.0` and `2.00` agree.
fn num(x: f64) -> String {
    if x == 0.0 { "0".into() } else { format!("{x}") }
}

fn text_or_num(t: &str) -> String {
    t.trim()
        .parse::<f64>()
        .map_or_else(|_| t.trim().to_string(), num)
}

/// Pipe identity is the arc ID (`A<n>` in pipe order) and its endpoints
/// (D3: NW5 pipe rows had no ID, so no pipe ever matched).
fn profile(net: &Network) -> Profile {
    let nodes = net
        .nodes
        .iter()
        .map(|nd| (nd.id.trim().to_string(), nd.status.code()))
        .collect();
    let edge_list: Vec<EdgeId> = net
        .arcs
        .iter()
        .map(|a| EdgeId {
            id: a.id.trim().to_string(),
            from: a.from.trim().to_string(),
            to: a.to.trim().to_string(),
        })
        .collect();
    let edges = edge_list
        .iter()
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    let clients = net
        .clients
        .iter()
        .map(|c| (c.id.trim().to_string(), c.terminal.trim().to_string()))
        .collect();
    // Effective input: parameters and every table without the x/y
    // coordinates, clients with their resolved demand; numbers in canonical
    // form, rows sorted.
    // PROPOSAL: the canonical forms are compared directly instead of their
    // SHA-256 digests (same decision, no hash code needed).
    let mut parts: Vec<String> = Vec::new();
    let mut params: Vec<String> = neatwork_nw5_io::text::canonical_params(&net.params)
        .into_iter()
        .map(|(k, v)| format!("{k}={}", text_or_num(&v)))
        .collect();
    params.sort();
    parts.push(format!("params[{}]", params.join(",")));
    let table = |name: &str, mut rows: Vec<String>| {
        rows.sort();
        format!("{name}[{}]", rows.join(";"))
    };
    parts.push(table(
        "nodes",
        net.nodes
            .iter()
            .map(|nd| {
                format!(
                    "{}|{}|{}|{}|{}|{}",
                    nd.id,
                    num(nd.elevation),
                    nd.taps.map_or(String::new(), num),
                    num(nd.orifice_ideal),
                    num(nd.orifice_commercial),
                    nd.status.code()
                )
            })
            .collect(),
    ));
    parts.push(table(
        "pipes",
        net.arcs
            .iter()
            .map(|a| {
                format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}|{}",
                    a.id,
                    a.from,
                    a.to,
                    num(a.length),
                    num(a.length1),
                    a.pipe1.as_deref().unwrap_or(""),
                    num(a.length2),
                    a.pipe2.as_deref().unwrap_or(""),
                    a.state.code()
                )
            })
            .collect(),
    ));
    parts.push(table(
        "clients",
        net.clients
            .iter()
            .map(|c| {
                format!(
                    "{}|{}|{}|{}",
                    c.id,
                    c.terminal,
                    num(c.probability),
                    num(c.target_flow)
                )
            })
            .collect(),
    ));
    parts.push(table(
        "bornes",
        net.terminals
            .iter()
            .map(|t| format!("{}|{}", t.id, num(t.pressure)))
            .collect(),
    ));
    parts.push(table(
        "diameters",
        net.pipes
            .iter()
            .map(|p| {
                format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}",
                    p.id,
                    p.nominal,
                    p.sdr,
                    num(p.diameter),
                    num(p.cost),
                    num(p.max_pressure),
                    p.kind,
                    num(p.roughness)
                )
            })
            .collect(),
    ));
    parts.push(table(
        "orifices",
        net.orifices.iter().map(|d| num(*d)).collect(),
    ));
    parts.push(table(
        "loadflow",
        net.loadflow
            .iter()
            .map(|(s, q, m)| format!("{s}|{}|{}", num(*q), num(*m)))
            .collect(),
    ));
    Profile {
        nodes,
        edge_set: edge_list.into_iter().collect(),
        edges,
        clients,
        effective: parts.join("\n"),
    }
}

fn incident(edges: &BTreeSet<EdgeId>) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in edges {
        let key = format!("{}|{}|{}", e.id, e.from, e.to);
        out.entry(e.from.clone()).or_default().push(key.clone());
        out.entry(e.to.clone()).or_default().push(key);
    }
    for v in out.values_mut() {
        v.sort();
    }
    out
}

fn compare_nodes(l: &Profile, r: &Profile, role: i32) -> EntityMatch {
    let (li, ri) = (incident(&l.edge_set), incident(&r.edge_set));
    let empty: Vec<String> = Vec::new();
    let mut m = EntityMatch::default();
    for (id, _) in l.nodes.iter().filter(|(_, x)| **x == role) {
        if r.nodes.get(id) != Some(&role) {
            m.left_only.push(id.clone());
        } else if li.get(id).unwrap_or(&empty) == ri.get(id).unwrap_or(&empty) {
            m.matched.push(id.clone());
        } else {
            m.context_changed.push(id.clone());
            m.left_only.push(id.clone());
            m.right_only.push(id.clone());
        }
    }
    for (id, _) in r.nodes.iter().filter(|(_, x)| **x == role) {
        if l.nodes.get(id) != Some(&role) {
            m.right_only.push(id.clone());
        }
    }
    m.left_only.sort();
    m.right_only.sort();
    m
}

fn compare_pipes(l: &Profile, r: &Profile) -> EntityMatch {
    let mut m = EntityMatch::default();
    for (id, e) in &l.edges {
        match r.edges.get(id) {
            Some(x) if x == e => m.matched.push(id.clone()),
            Some(_) => {
                m.context_changed.push(id.clone());
                m.left_only.push(id.clone());
                m.right_only.push(id.clone());
            }
            None => m.left_only.push(id.clone()),
        }
    }
    for id in r.edges.keys() {
        if !l.edges.contains_key(id) {
            m.right_only.push(id.clone());
        }
    }
    m.right_only.sort();
    m
}

fn change_summary(l: &Profile, r: &Profile) -> ChangeSummary {
    let terminals = |p: &Profile| -> BTreeSet<String> {
        p.nodes
            .iter()
            .filter(|(_, x)| **x == 2)
            .map(|(k, _)| k.clone())
            .collect()
    };
    let (lt, rt) = (terminals(l), terminals(r));
    ChangeSummary {
        left_nodes: l.nodes.len(),
        right_nodes: r.nodes.len(),
        matched_nodes: l
            .nodes
            .iter()
            .filter(|(k, x)| r.nodes.get(*k) == Some(x))
            .count(),
        added_nodes: r
            .nodes
            .keys()
            .filter(|k| !l.nodes.contains_key(*k))
            .cloned()
            .collect(),
        removed_nodes: l
            .nodes
            .keys()
            .filter(|k| !r.nodes.contains_key(*k))
            .cloned()
            .collect(),
        changed_roles: l
            .nodes
            .iter()
            .filter(|(k, x)| r.nodes.get(*k).is_some_and(|y| y != *x))
            .map(|(k, _)| k.clone())
            .collect(),
        left_pipes: l.edges.len(),
        right_pipes: r.edges.len(),
        matched_pipes: l
            .edges
            .iter()
            .filter(|(k, e)| r.edges.get(*k) == Some(e))
            .count(),
        added_pipes: r
            .edges
            .keys()
            .filter(|k| !l.edges.contains_key(*k))
            .cloned()
            .collect(),
        removed_pipes: l
            .edges
            .keys()
            .filter(|k| !r.edges.contains_key(*k))
            .cloned()
            .collect(),
        changed_endpoints: l
            .edges
            .iter()
            .filter(|(k, e)| r.edges.get(*k).is_some_and(|x| x != *e))
            .map(|(k, _)| k.clone())
            .collect(),
        left_terminals: lt.len(),
        right_terminals: rt.len(),
        matched_terminals: lt.intersection(&rt).count(),
        added_terminals: rt.difference(&lt).cloned().collect(),
        removed_terminals: lt.difference(&rt).cloned().collect(),
    }
}

/// Compatibility of two revisions (`DomainSnapshotCompatibilityService`),
/// from the networks of the two revision files.
pub fn compatibility(
    left: &Network,
    left_revision: &str,
    right: &Network,
    right_revision: &str,
) -> Compatibility {
    let l = profile(left);
    if left_revision == right_revision {
        let all = |role: i32| EntityMatch {
            matched: l
                .nodes
                .iter()
                .filter(|(_, x)| **x == role)
                .map(|(k, _)| k.clone())
                .collect(),
            ..EntityMatch::default()
        };
        return Compatibility {
            level: CompatibilityLevel::SameSnapshot,
            same_topology: true,
            same_demand: true,
            same_input: true,
            direct_comparison_allowed: true,
            partial_comparison_allowed: true,
            changes: change_summary(&l, &l),
            warnings: Vec::new(),
            terminals: all(2),
            sources: all(0),
            transit_nodes: all(1),
            pipes: EntityMatch {
                matched: l.edges.keys().cloned().collect(),
                ..EntityMatch::default()
            },
        };
    }
    let r = profile(right);
    let same_topology = l.nodes == r.nodes && l.edge_set == r.edge_set;
    let same_demand = l.clients == r.clients;
    let same_input = l.effective == r.effective;
    let level = if same_input {
        CompatibilityLevel::EquivalentInput
    } else if same_topology {
        CompatibilityLevel::SameTopology
    } else {
        CompatibilityLevel::TopologyChanged
    };
    let mut warnings = Vec::new();
    if level == CompatibilityLevel::SameTopology {
        warnings.push(
            "Input revision differs. Metric changes may reflect network parameters as well as simulation settings."
                .to_string(),
        );
    }
    if !same_demand {
        warnings.push(
            "Client-to-terminal allocation changed between revisions. Terminal statistics may reflect demand redistribution."
                .to_string(),
        );
    }
    if level == CompatibilityLevel::TopologyChanged {
        warnings.push(
            "Network topology changed. Only matched entities should be compared directly."
                .to_string(),
        );
    }
    Compatibility {
        level,
        same_topology,
        same_demand,
        same_input,
        direct_comparison_allowed: level != CompatibilityLevel::TopologyChanged,
        partial_comparison_allowed: true,
        changes: change_summary(&l, &r),
        warnings,
        terminals: compare_nodes(&l, &r, 2),
        sources: compare_nodes(&l, &r, 0),
        transit_nodes: compare_nodes(&l, &r, 1),
        pipes: compare_pipes(&l, &r),
    }
}

// ----- columns of the comparison tables ----------------------------------

/// A column of a comparison table (`comparisonDatasets.ts`).
#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonColumn {
    /// Key.
    pub key: &'static str,
    /// Label.
    pub label: &'static str,
    /// Canonical unit: `ratio`, `L/s`, `m`, `m/s`.
    pub unit: Option<&'static str>,
    /// Identity column.
    pub identity: bool,
    /// Context shown once for both sides (From, To, Type).
    pub shared_context: bool,
    /// Numeric metric of the numeric panel.
    pub comparable: bool,
    /// Key of the row value that holds the node role of this column.
    pub role_key: Option<&'static str>,
    /// Fixed node role of this column.
    pub role: Option<NodeRole>,
}

fn col(key: &'static str, label: &'static str) -> ComparisonColumn {
    ComparisonColumn {
        key,
        label,
        unit: None,
        identity: false,
        shared_context: false,
        comparable: false,
        role_key: None,
        role: None,
    }
}

fn metric(key: &'static str, label: &'static str, unit: &'static str) -> ComparisonColumn {
    ComparisonColumn {
        unit: Some(unit),
        comparable: true,
        ..col(key, label)
    }
}

/// Columns of a comparison table.
pub fn comparison_columns(kind: TableKind) -> Vec<ComparisonColumn> {
    match kind {
        TableKind::TerminalSatisfaction => vec![
            ComparisonColumn {
                identity: true,
                role: Some(NodeRole::Terminal),
                ..col("terminalId", "Terminal")
            },
            metric("minSat", "Min", "ratio"),
            metric("moySat", "Mean", "ratio"),
            metric("maxSat", "Max", "ratio"),
            metric("cVarSat5", "CVaR 5%", "ratio"),
            metric("cVarSat20", "CVaR 20%", "ratio"),
        ],
        TableKind::SourceFlow => vec![
            ComparisonColumn {
                identity: true,
                role: Some(NodeRole::Source),
                ..col("sourceId", "Source")
            },
            metric("minSt", "Min", "L/s"),
            metric("moySt", "Mean", "L/s"),
            metric("maxSt", "Max", "L/s"),
            metric("cVarSt5", "CVaR 5%", "L/s"),
            metric("cVarSt95", "CVaR 95%", "L/s"),
        ],
        TableKind::PipeVelocity => vec![
            ComparisonColumn {
                identity: true,
                ..col("edgeId", "Edge")
            },
            ComparisonColumn {
                shared_context: true,
                role_key: Some("startNodeType"),
                ..col("startNodeId", "From")
            },
            ComparisonColumn {
                shared_context: true,
                role_key: Some("endNodeType"),
                ..col("endNodeId", "To")
            },
            metric("minVelocityMps", "Min", "m/s"),
            metric("meanVelocityMps", "Mean", "m/s"),
            metric("maxVelocityMps", "Max", "m/s"),
        ],
        TableKind::Pressure => vec![
            ComparisonColumn {
                identity: true,
                role_key: Some("nodeType"),
                ..col("nodeId", "Node")
            },
            ComparisonColumn {
                shared_context: true,
                role_key: Some("nodeType"),
                ..col("nodeType", "Type")
            },
            metric("minPressureM", "Minimum", "m"),
            metric("meanPressureM", "Mean", "m"),
            metric("maxPressureM", "Maximum", "m"),
        ],
    }
}

/// Identity columns of a table (the row identity joins their values with
/// `::`).
pub fn identity_columns(kind: TableKind) -> Vec<&'static str> {
    match kind {
        TableKind::TerminalSatisfaction => vec!["terminalId"],
        TableKind::SourceFlow => vec!["sourceId"],
        TableKind::PipeVelocity => vec!["edgeId", "startNodeId", "endNodeId"],
        TableKind::Pressure => vec!["nodeId"],
    }
}

/// Header label with the display unit: ratio → "(%)", m → "(`<u>`)", m/s →
/// "(`<u>`/s)", L/s as is.
pub fn column_label(c: &ComparisonColumn, unit: DisplayUnit) -> String {
    match c.unit {
        Some("ratio") => format!("{} (%)", c.label),
        Some(u) => format!("{} ({})", c.label, display_unit_label(u, unit)),
        None => c.label.to_string(),
    }
}

/// Text of a value of a comparison column.
pub fn value_text(v: &Json, c: &ComparisonColumn, unit: DisplayUnit) -> String {
    match v {
        Json::Null => MISSING.to_string(),
        Json::Number(x) if !x.is_finite() => MISSING.to_string(),
        Json::Number(x) => {
            let shown = to_display(*x, c.unit, unit);
            if c.unit == Some("ratio") {
                ratio_percent(shown)
            } else {
                adaptive(shown)
            }
        }
        Json::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
        other => {
            let t = scalar_text(other).unwrap_or_default();
            if c.key == "nodeType" {
                NodeRole::parse(&t).map_or(t, |r| r.label().to_string())
            } else {
                t
            }
        }
    }
}

// ----- row alignment and unified table (F50) ------------------------------

/// Status of an aligned row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchStatus {
    /// Both sides.
    Matched,
    /// Left only.
    LeftOnly,
    /// Right only.
    RightOnly,
}

impl MatchStatus {
    /// "Matched", "Left only", "Right only".
    pub fn label(self) -> &'static str {
        match self {
            MatchStatus::Matched => "Matched",
            MatchStatus::LeftOnly => "Left only",
            MatchStatus::RightOnly => "Right only",
        }
    }
}

/// Entities allowed to match, from the compatibility (F50 step 4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchPolicy {
    /// IDs allowed to match.
    pub allowed: BTreeSet<String>,
    /// IDs forced to left-only and right-only rows.
    pub forced_unmatched: BTreeSet<String>,
}

/// Policy of a table: Pressure uses terminals and transit nodes, Terminal
/// Satisfaction terminals, Source Flow sources, Pipe Velocity pipes.
pub fn match_policy(kind: TableKind, c: &Compatibility) -> MatchPolicy {
    let sets: Vec<&EntityMatch> = match kind {
        TableKind::Pressure => vec![&c.terminals, &c.transit_nodes],
        TableKind::TerminalSatisfaction => vec![&c.terminals],
        TableKind::SourceFlow => vec![&c.sources],
        TableKind::PipeVelocity => vec![&c.pipes],
    };
    MatchPolicy {
        allowed: sets
            .iter()
            .flat_map(|m| m.matched.iter().cloned())
            .collect(),
        forced_unmatched: sets
            .iter()
            .flat_map(|m| m.context_changed.iter().cloned())
            .collect(),
    }
}

/// A row of the aligned stream.
#[derive(Clone, Debug, PartialEq)]
pub struct AlignedRow {
    /// Identity (identity values joined with `::`).
    pub identity: String,
    /// Status.
    pub status: MatchStatus,
    /// Index of the left row.
    pub left: Option<usize>,
    /// Index of the right row.
    pub right: Option<usize>,
}

fn identity_of(row: &Json, cols: &[&str], index: usize) -> String {
    let v = cols
        .iter()
        .map(|c| row.get(c).and_then(scalar_text).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("::");
    if v.is_empty() {
        format!("row:{index}")
    } else {
        v
    }
}

/// Align the rows of two tables (`comparisonRowAlignment.ts`): rows group
/// by identity, identities in first-seen order (left first); the k-th left
/// and right rows of a group pair up; a policy can force rows apart. Rows
/// are never paired by position without an identity.
pub fn align_rows(
    left: &[Json],
    right: &[Json],
    identity: &[&str],
    policy: Option<&MatchPolicy>,
) -> Vec<AlignedRow> {
    if identity.is_empty() {
        let mut out: Vec<AlignedRow> = (0..left.len())
            .map(|k| AlignedRow {
                identity: format!("Left row {}", k + 1),
                status: MatchStatus::LeftOnly,
                left: Some(k),
                right: None,
            })
            .collect();
        out.extend((0..right.len()).map(|k| AlignedRow {
            identity: format!("Right row {}", k + 1),
            status: MatchStatus::RightOnly,
            left: None,
            right: Some(k),
        }));
        return out;
    }
    let mut order: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, (Vec<usize>, Vec<usize>)> = BTreeMap::new();
    for (side, rows) in [(0, left), (1, right)] {
        for (k, row) in rows.iter().enumerate() {
            let id = identity_of(row, identity, k);
            let g = groups.entry(id.clone()).or_insert_with(|| {
                order.push(id.clone());
                (Vec::new(), Vec::new())
            });
            if side == 0 { g.0.push(k) } else { g.1.push(k) }
        }
    }
    let mut out = Vec::new();
    for id in order {
        let Some((ls, rs)) = groups.get(&id) else {
            continue;
        };
        let rep = ls
            .first()
            .map(|&k| &left[k])
            .or_else(|| rs.first().map(|&k| &right[k]));
        let policy_id = rep
            .and_then(|r| r.get(identity[0]))
            .and_then(scalar_text)
            .unwrap_or_else(|| id.clone());
        let apart = policy.is_some_and(|p| {
            p.forced_unmatched.contains(&policy_id) || !p.allowed.contains(&policy_id)
        });
        if apart {
            out.extend(ls.iter().map(|&k| AlignedRow {
                identity: id.clone(),
                status: MatchStatus::LeftOnly,
                left: Some(k),
                right: None,
            }));
            out.extend(rs.iter().map(|&k| AlignedRow {
                identity: id.clone(),
                status: MatchStatus::RightOnly,
                left: None,
                right: Some(k),
            }));
            continue;
        }
        for k in 0..ls.len().max(rs.len()) {
            let (l, r) = (ls.get(k).copied(), rs.get(k).copied());
            out.push(AlignedRow {
                identity: id.clone(),
                status: match (l, r) {
                    (Some(_), Some(_)) => MatchStatus::Matched,
                    (Some(_), None) => MatchStatus::LeftOnly,
                    _ => MatchStatus::RightOnly,
                },
                left: l,
                right: r,
            });
        }
    }
    out
}

/// What a header of the unified table sorts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortTarget {
    /// The Match column: status, then identity.
    Match,
    /// A shared context column (left value, else right).
    Shared(&'static str),
    /// A value column of the left run.
    Left(&'static str),
    /// A value column of the right run.
    Right(&'static str),
}

/// A header of the unified table.
#[derive(Clone, Debug, PartialEq)]
pub struct UnifiedHeader {
    /// Sort target of the header.
    pub target: SortTarget,
    /// Label (with unit).
    pub label: String,
}

/// A shared context cell.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedCell {
    /// Text ("`<left>` ↔ `<right>`" when the sides differ).
    pub text: String,
    /// The sides differ.
    pub mismatch: bool,
    /// Node role for coloring.
    pub role: Option<NodeRole>,
}

/// A row of the unified table.
#[derive(Clone, Debug, PartialEq)]
pub struct UnifiedRow {
    /// Identity.
    pub identity: String,
    /// Role of the identity node.
    pub role: Option<NodeRole>,
    /// Status pill.
    pub status: MatchStatus,
    /// Shared context cells.
    pub shared: Vec<SharedCell>,
    /// Left value cells ("—" when missing).
    pub left: Vec<String>,
    /// Right value cells.
    pub right: Vec<String>,
    left_raw: Option<Json>,
    right_raw: Option<Json>,
}

/// The unified comparison table of one table kind.
#[derive(Clone, Debug, PartialEq)]
pub struct UnifiedTable {
    /// Which table.
    pub kind: TableKind,
    /// Headers: Match, shared context, left values, right values.
    pub headers: Vec<UnifiedHeader>,
    /// Rows in alignment order.
    pub rows: Vec<UnifiedRow>,
    /// "Matched `<n>`".
    pub matched: usize,
    /// "Left only `<n>`".
    pub left_only: usize,
    /// "Right only `<n>`".
    pub right_only: usize,
}

fn row_role(row: &Json, c: &ComparisonColumn) -> Option<NodeRole> {
    c.role.or_else(|| {
        c.role_key
            .and_then(|k| get_str(row, k))
            .and_then(|t| NodeRole::parse(&t))
    })
}

/// Build the unified table of two runs. Without a compatibility (not
/// loaded) every identity may match.
pub fn unified_table(
    left: &RunRecord,
    right: &RunRecord,
    kind: TableKind,
    compatibility: Option<&Compatibility>,
    unit: DisplayUnit,
) -> UnifiedTable {
    let (lrows, rrows) = (table_rows(left, kind), table_rows(right, kind));
    let cols = comparison_columns(kind);
    let ids = identity_columns(kind);
    let policy = compatibility.map(|c| match_policy(kind, c));
    let aligned = align_rows(&lrows, &rrows, &ids, policy.as_ref());
    let shared: Vec<&ComparisonColumn> = cols.iter().filter(|c| c.shared_context).collect();
    let values: Vec<&ComparisonColumn> = cols
        .iter()
        .filter(|c| !c.identity && !ids.contains(&c.key) && !c.shared_context)
        .collect();
    let id_col = &cols[0];
    let mut headers = vec![UnifiedHeader {
        target: SortTarget::Match,
        label: "Match".into(),
    }];
    headers.extend(shared.iter().map(|c| UnifiedHeader {
        target: SortTarget::Shared(c.key),
        label: column_label(c, unit),
    }));
    headers.extend(values.iter().map(|c| UnifiedHeader {
        target: SortTarget::Left(c.key),
        label: column_label(c, unit),
    }));
    headers.extend(values.iter().map(|c| UnifiedHeader {
        target: SortTarget::Right(c.key),
        label: column_label(c, unit),
    }));
    let rows: Vec<UnifiedRow> = aligned
        .iter()
        .map(|a| {
            let l = a.left.map(|k| &lrows[k]);
            let r = a.right.map(|k| &rrows[k]);
            let side = |row: Option<&Json>| -> Vec<String> {
                values
                    .iter()
                    .map(|c| {
                        row.map_or(MISSING.to_string(), |x| {
                            value_text(x.get(c.key).unwrap_or(&Json::Null), c, unit)
                        })
                    })
                    .collect()
            };
            let shared_cells = shared
                .iter()
                .map(|c| {
                    let lv = l
                        .and_then(|x| x.get(c.key))
                        .filter(|v| !matches!(v, Json::Null));
                    let rv = r
                        .and_then(|x| x.get(c.key))
                        .filter(|v| !matches!(v, Json::Null));
                    let role = l
                        .and_then(|x| row_role(x, c))
                        .or_else(|| r.and_then(|x| row_role(x, c)));
                    let (text, mismatch) = match (lv, rv) {
                        (None, None) => (MISSING.to_string(), false),
                        (Some(x), None) | (None, Some(x)) => (value_text(x, c, unit), false),
                        (Some(x), Some(y)) if scalar_text(x) == scalar_text(y) => {
                            (value_text(x, c, unit), false)
                        }
                        (Some(x), Some(y)) => (
                            format!("{} ↔ {}", value_text(x, c, unit), value_text(y, c, unit)),
                            true,
                        ),
                    };
                    SharedCell {
                        text,
                        mismatch,
                        role,
                    }
                })
                .collect();
            UnifiedRow {
                identity: a.identity.clone(),
                role: l.or(r).and_then(|x| row_role(x, id_col)),
                status: a.status,
                shared: shared_cells,
                left: side(l),
                right: side(r),
                left_raw: l.cloned(),
                right_raw: r.cloned(),
            }
        })
        .collect();
    let count = |s: MatchStatus| rows.iter().filter(|r| r.status == s).count();
    UnifiedTable {
        kind,
        headers,
        matched: count(MatchStatus::Matched),
        left_only: count(MatchStatus::LeftOnly),
        right_only: count(MatchStatus::RightOnly),
        rows,
    }
}

fn raw_sort_value(v: Option<&Json>) -> SortValue {
    match v {
        None | Some(Json::Null) => SortValue::Empty,
        Some(Json::Number(x)) => SortValue::Number(*x),
        Some(other) => SortValue::Text(scalar_text(other).unwrap_or_default()),
    }
}

impl UnifiedTable {
    /// Row order for a sort (F50): the Match column sorts by status
    /// (Matched, Left only, Right only) then identity in natural order;
    /// other columns by the raw value of that side (shared: left, else
    /// right), missing values last in both directions.
    pub fn sorted_rows(&self, sort: Option<(&SortTarget, SortDirection)>) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..self.rows.len()).collect();
        let Some((target, dir)) = sort else {
            return idx;
        };
        let flip = |o: Ordering| {
            if dir == SortDirection::Descending {
                o.reverse()
            } else {
                o
            }
        };
        if *target == SortTarget::Match {
            idx.sort_by(|&i, &j| {
                let (a, b) = (&self.rows[i], &self.rows[j]);
                flip(
                    a.status
                        .cmp(&b.status)
                        .then_with(|| natural_cmp(&a.identity, &b.identity)),
                )
            });
            return idx;
        }
        let value = |r: &UnifiedRow| -> SortValue {
            match target {
                SortTarget::Left(k) => raw_sort_value(r.left_raw.as_ref().and_then(|x| x.get(k))),
                SortTarget::Right(k) => raw_sort_value(r.right_raw.as_ref().and_then(|x| x.get(k))),
                SortTarget::Shared(k) => {
                    let l = r
                        .left_raw
                        .as_ref()
                        .and_then(|x| x.get(k))
                        .filter(|v| !matches!(v, Json::Null));
                    raw_sort_value(l.or_else(|| r.right_raw.as_ref().and_then(|x| x.get(k))))
                }
                SortTarget::Match => SortValue::Empty,
            }
        };
        let values: Vec<SortValue> = self.rows.iter().map(value).collect();
        let refs: Vec<&SortValue> = values.iter().collect();
        idx = crate::review::sort_order(&refs, dir);
        idx
    }
}

// ----- numeric comparison panel (F51) ------------------------------------

/// A way to match rows of the two tables.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchStrategy {
    /// `leftKeys::rightKeys`.
    pub id: String,
    /// Label of the selector.
    pub label: String,
    /// Keys on the left.
    pub left_keys: Vec<&'static str>,
    /// Keys on the right.
    pub right_keys: Vec<&'static str>,
}

const IDENTITY_ALIASES: &[(&str, &[&str], &[&str])] = &[
    ("Terminal ID", &["borne"], &["terminalId"]),
    ("Terminal ID", &["outlet"], &["terminalId"]),
    ("Node ID", &["noeud"], &["terminalId"]),
    ("Node ID", &["noeud"], &["transitNodeId"]),
    ("Source ID", &["noeud"], &["sourceId"]),
    (
        "Pipe endpoints",
        &["pred", "succ"],
        &["startNodeId", "endNodeId"],
    ),
    (
        "Pipe endpoints",
        &["sourceId", "targetId"],
        &["startNodeId", "endNodeId"],
    ),
];

const METRIC_ALIASES: &[(&str, &str)] = &[
    ("pressure", "meanPressureM"),
    ("required_pressure_m", "meanPressureM"),
    ("cible", "moySt"),
];

fn has_keys(kind: TableKind, keys: &[&str]) -> bool {
    let cols = comparison_columns(kind);
    keys.iter().all(|k| cols.iter().any(|c| c.key == *k))
}

/// Strategies to match rows (`comparisonEngine.ts`): every identity column
/// both tables have, then the identifier aliases (both directions). Empty
/// means "These tables do not expose a compatible entity identifier.
/// Side-by-side inspection remains available above."
pub fn match_strategies(left: TableKind, right: TableKind) -> Vec<MatchStrategy> {
    let mut out: Vec<MatchStrategy> = Vec::new();
    let mut add = |label: &str, l: Vec<&'static str>, r: Vec<&'static str>| {
        if !has_keys(left, &l) || !has_keys(right, &r) {
            return;
        }
        let id = format!("{}::{}", l.join("+"), r.join("+"));
        if out.iter().any(|s| s.id == id) {
            return;
        }
        out.push(MatchStrategy {
            id,
            label: label.to_string(),
            left_keys: l,
            right_keys: r,
        });
    };
    let right_ids = identity_columns(right);
    for k in identity_columns(left) {
        if right_ids.contains(&k) {
            add(k, vec![k], vec![k]);
        }
    }
    for (label, l, r) in IDENTITY_ALIASES {
        add(label, l.to_vec(), r.to_vec());
    }
    for (label, l, r) in IDENTITY_ALIASES {
        add(label, r.to_vec(), l.to_vec());
    }
    out
}

/// Numeric metric columns of a table.
pub fn comparable_columns(kind: TableKind) -> Vec<ComparisonColumn> {
    comparison_columns(kind)
        .into_iter()
        .filter(|c| c.comparable)
        .collect()
}

/// Units agree, or both have none and the same key.
pub fn units_compatible(a: &ComparisonColumn, b: &ComparisonColumn) -> bool {
    if a.unit.is_some() || b.unit.is_some() {
        a.unit == b.unit
    } else {
        a.key == b.key
    }
}

/// Recommended metric pair: the first left metric with the same key on
/// the right and compatible units, else a metric alias.
pub fn recommended_pair(left: TableKind, right: TableKind) -> Option<(&'static str, &'static str)> {
    let (lc, rc) = (comparable_columns(left), comparable_columns(right));
    for l in &lc {
        if let Some(r) = rc.iter().find(|r| r.key == l.key && units_compatible(l, r)) {
            return Some((l.key, r.key));
        }
    }
    for (a, b) in METRIC_ALIASES {
        for (x, y) in [(a, b), (b, a)] {
            if let (Some(l), Some(r)) = (
                lc.iter().find(|c| c.key == *x),
                rc.iter().find(|c| c.key == *y),
            ) && units_compatible(l, r)
            {
                return Some((l.key, r.key));
            }
        }
    }
    None
}

/// A matched row of the numeric panel.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricRow {
    /// Identity (key values joined with " → ").
    pub identity: String,
    /// Left value (canonical unit).
    pub left: f64,
    /// Right value.
    pub right: f64,
    /// right − left.
    pub difference: f64,
    /// right / left (`None` when left = 0).
    pub ratio: Option<f64>,
    /// (right − left) / left × 100 (`None` when left = 0).
    pub change_percent: Option<f64>,
}

/// Result of the numeric panel.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricComparison {
    /// Matched rows in left order.
    pub rows: Vec<MetricRow>,
    /// Left identities without a usable right row.
    pub unmatched_left: usize,
    /// Right identities never matched.
    pub unmatched_right: usize,
    /// Left rows skipped because their identity is not unique.
    pub duplicate_left: usize,
    /// Right rows skipped because their identity is not unique.
    pub duplicate_right: usize,
    /// Canonical unit of the left metric.
    pub left_unit: Option<&'static str>,
    /// Canonical unit of the right metric.
    pub right_unit: Option<&'static str>,
    /// Units agree.
    pub units_compatible: bool,
}

impl MetricComparison {
    /// Warning when the units differ.
    pub fn warning(&self) -> Option<&'static str> {
        (!self.units_compatible).then_some(
            "The selected metrics use different physical units. Difference, ratio and percentage change may not represent a meaningful hydraulic comparison.",
        )
    }

    /// Summary "Matched: n", "Left unmatched: n", "Right unmatched: n", and
    /// "Duplicate IDs skipped: n" when some were.
    pub fn summary(&self) -> Vec<String> {
        let mut out = vec![
            format!("Matched: {}", self.rows.len()),
            format!("Left unmatched: {}", self.unmatched_left),
            format!("Right unmatched: {}", self.unmatched_right),
        ];
        let dup = self.duplicate_left + self.duplicate_right;
        if dup > 0 {
            out.push(format!("Duplicate IDs skipped: {dup}"));
        }
        out
    }
}

fn unique_rows<'a>(rows: &'a [Json], keys: &[&str]) -> (Vec<(String, &'a Json)>, usize) {
    let mut order: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<&Json>> = BTreeMap::new();
    for row in rows {
        let parts: Option<Vec<String>> = keys
            .iter()
            .map(|k| {
                row.get(k)
                    .and_then(scalar_text)
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
            })
            .collect();
        let Some(parts) = parts else {
            continue;
        };
        let id = parts.join(" → ");
        groups
            .entry(id.clone())
            .or_insert_with(|| {
                order.push(id.clone());
                Vec::new()
            })
            .push(row);
    }
    let mut dup = 0;
    let mut out = Vec::new();
    for id in order {
        match groups.get(&id).map(Vec::as_slice) {
            Some([one]) => out.push((id, *one)),
            Some(many) => dup += many.len(),
            None => {}
        }
    }
    (out, dup)
}

fn number(row: &Json, key: &str) -> Option<f64> {
    match row.get(key) {
        Some(Json::Number(x)) if x.is_finite() => Some(*x),
        Some(Json::String(t)) => t.trim().parse::<f64>().ok().filter(|x| x.is_finite()),
        _ => None,
    }
}

/// Compare one metric of two tables (`compareMetrics`). Matching ignores
/// the compatibility policy; empty identity parts are skipped; duplicate
/// identities are skipped and counted; a non-numeric value counts as left
/// unmatched.
pub fn compare_metrics(
    left_rows: &[Json],
    left_column: &ComparisonColumn,
    right_rows: &[Json],
    right_column: &ComparisonColumn,
    strategy: &MatchStrategy,
) -> MetricComparison {
    let (lmap, duplicate_left) = unique_rows(left_rows, &strategy.left_keys);
    let (rmap, duplicate_right) = unique_rows(right_rows, &strategy.right_keys);
    let mut rows = Vec::new();
    let mut unmatched_left = 0;
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for (id, lrow) in &lmap {
        let Some((_, rrow)) = rmap.iter().find(|(k, _)| k == id) else {
            unmatched_left += 1;
            continue;
        };
        let (Some(l), Some(r)) = (
            number(lrow, left_column.key),
            number(rrow, right_column.key),
        ) else {
            unmatched_left += 1;
            continue;
        };
        used.insert(id.as_str());
        let difference = r - l;
        rows.push(MetricRow {
            identity: id.clone(),
            left: l,
            right: r,
            difference,
            ratio: (l != 0.0).then(|| r / l),
            change_percent: (l != 0.0).then(|| difference / l * 100.0),
        });
    }
    MetricComparison {
        unmatched_right: rmap
            .iter()
            .filter(|(k, _)| !used.contains(k.as_str()))
            .count(),
        rows,
        unmatched_left,
        duplicate_left,
        duplicate_right,
        left_unit: left_column.unit,
        right_unit: right_column.unit,
        units_compatible: units_compatible(left_column, right_column),
    }
}

/// Compare one metric of the same table of two runs with a strategy.
pub fn compare_run_metrics(
    left: &RunRecord,
    right: &RunRecord,
    kind: TableKind,
    left_key: &str,
    right_key: &str,
    strategy: &MatchStrategy,
) -> Option<MetricComparison> {
    let cols = comparable_columns(kind);
    let lc = cols.iter().find(|c| c.key == left_key)?;
    let rc = cols.iter().find(|c| c.key == right_key)?;
    Some(compare_metrics(
        &table_rows(left, kind),
        lc,
        &table_rows(right, kind),
        rc,
        strategy,
    ))
}

/// Sign of a value for its color (positive green, negative red).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    /// > 0.
    Positive,
    /// < 0.
    Negative,
    /// 0 or missing.
    Neutral,
}

fn sign(x: Option<f64>) -> Sign {
    match x {
        Some(v) if v > 0.0 => Sign::Positive,
        Some(v) if v < 0.0 => Sign::Negative,
        _ => Sign::Neutral,
    }
}

/// The result table of the numeric panel as text.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricTable {
    /// ID, Left (`<u>`), Right (`<u>`), Difference Δ (`<u>`), Ratio, Change %.
    pub headers: Vec<String>,
    /// Cells per row, with the sign of the difference and of the change.
    pub rows: Vec<(Vec<String>, Sign, Sign)>,
    /// "No matching numeric rows were found." when empty.
    pub empty_text: Option<&'static str>,
}

fn unit_suffix(u: Option<&str>, unit: DisplayUnit) -> String {
    match u {
        None | Some("ratio") => String::new(),
        Some(u) => format!(" ({})", display_unit_label(u, unit)),
    }
}

/// Text of the numeric result table. Values are converted to the display
/// unit after the calculation; the change is shown once as a signed
/// percentage ("+10.00%", D4).
pub fn metric_table(cmp: &MetricComparison, unit: DisplayUnit) -> MetricTable {
    let headers = vec![
        "ID".to_string(),
        format!("Left{}", unit_suffix(cmp.left_unit, unit)),
        format!("Right{}", unit_suffix(cmp.right_unit, unit)),
        format!(
            "Difference Δ{}",
            if cmp.units_compatible {
                unit_suffix(cmp.left_unit, unit)
            } else {
                String::new()
            }
        ),
        "Ratio".to_string(),
        "Change %".to_string(),
    ];
    let rows = cmp
        .rows
        .iter()
        .map(|r| {
            let diff = if cmp.units_compatible {
                to_display(r.difference, cmp.left_unit, unit)
            } else {
                r.difference
            };
            (
                vec![
                    r.identity.clone(),
                    adaptive(to_display(r.left, cmp.left_unit, unit)),
                    adaptive(to_display(r.right, cmp.right_unit, unit)),
                    signed(diff),
                    r.ratio.map_or(MISSING.to_string(), adaptive),
                    r.change_percent.map_or(MISSING.to_string(), signed_percent),
                ],
                sign(Some(r.difference)),
                sign(r.change_percent),
            )
        })
        .collect::<Vec<_>>();
    MetricTable {
        headers,
        empty_text: rows
            .is_empty()
            .then_some("No matching numeric rows were found."),
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revision::SourceFormat;
    use crate::run::{ConfigurationMode, RequestedOptions, RunStatus};
    use neatwork_nw5::{Arc, ArcState, Client, Node, NodeStatus, Terminal};

    fn node(id: &str, status: NodeStatus) -> Node {
        Node {
            id: id.into(),
            elevation: 0.0,
            x: 1.0,
            y: 2.0,
            taps: None,
            orifice_ideal: 0.0,
            orifice_commercial: 0.0,
            status,
        }
    }

    fn arc(k: usize, from: &str, to: &str) -> Arc {
        Arc {
            id: format!("A{k}"),
            from: from.into(),
            to: to.into(),
            length: 10.0,
            length1: 10.0,
            pipe1: Some("D1".into()),
            length2: 0.0,
            pipe2: None,
            state: ArcState::New,
            open: true,
        }
    }

    fn net() -> Network {
        Network {
            name: "n".into(),
            nodes: vec![
                node("S", NodeStatus::Source),
                node("J", NodeStatus::Junction),
                node("T1", NodeStatus::Terminal),
                node("T2", NodeStatus::Terminal),
            ],
            arcs: vec![arc(1, "S", "J"), arc(2, "J", "T1"), arc(3, "J", "T2")],
            clients: vec![
                Client {
                    id: "C1".into(),
                    terminal: "T1".into(),
                    probability: 0.3,
                    target_flow: 0.1,
                },
                Client {
                    id: "C2".into(),
                    terminal: "T2".into(),
                    probability: 0.3,
                    target_flow: 0.1,
                },
            ],
            terminals: vec![
                Terminal {
                    id: "T1".into(),
                    pressure: 2.0,
                },
                Terminal {
                    id: "T2".into(),
                    pressure: 2.0,
                },
            ],
            ..Network::default()
        }
    }

    #[test]
    fn compatibility_levels() {
        let a = net();
        assert_eq!(
            compatibility(&a, "x", &a, "x").level,
            CompatibilityLevel::SameSnapshot
        );
        let mut b = a.clone();
        b.nodes[1].x = 99.0;
        let c = compatibility(&a, "x", &b, "y");
        assert_eq!(c.level, CompatibilityLevel::EquivalentInput);
        assert!(c.warnings.is_empty());
        assert_eq!(c.pipes.matched, vec!["A1", "A2", "A3"]);
        b.params.set("pressure", "3");
        let c = compatibility(&a, "x", &b, "y");
        assert_eq!(c.level, CompatibilityLevel::SameTopology);
        assert_eq!(c.warnings.len(), 1);
        b.clients[1].terminal = "T1".into();
        assert_eq!(compatibility(&a, "x", &b, "y").warnings.len(), 2);
        // a new terminal T3 on J: J's context changes, T3 is right-only
        let mut d = a.clone();
        d.nodes.push(node("T3", NodeStatus::Terminal));
        d.arcs.push(arc(4, "J", "T3"));
        let c = compatibility(&a, "x", &d, "z");
        assert_eq!(c.level, CompatibilityLevel::TopologyChanged);
        assert!(!c.direct_comparison_allowed);
        assert_eq!(c.transit_nodes.context_changed, vec!["J"]);
        assert_eq!(c.terminals.matched, vec!["T1", "T2"]);
        assert_eq!(c.terminals.right_only, vec!["T3"]);
        assert_eq!(c.pipes.right_only, vec!["A4"]);
        assert_eq!(c.changes.lines()[0], "Nodes 4 → 5, Matched 4 · +1 · -0");
        assert_eq!(c.changes.lines()[1], "Pipes 3 → 4, Matched 3 · +1 · -0");
        assert_eq!(c.changes.added_terminals, vec!["T3"]);
    }

    fn rows(v: &str) -> Vec<Json> {
        match Json::parse(v) {
            Ok(Json::Array(r)) => r,
            _ => Vec::new(),
        }
    }

    #[test]
    fn alignment() {
        let l = rows(r#"[{"id": "a", "v": 1}, {"id": "b", "v": 2}, {"id": "b", "v": 3}]"#);
        let r = rows(r#"[{"id": "b", "v": 4}, {"id": "c", "v": 5}]"#);
        let a = align_rows(&l, &r, &["id"], None);
        let st: Vec<(String, MatchStatus)> =
            a.iter().map(|x| (x.identity.clone(), x.status)).collect();
        assert_eq!(
            st,
            vec![
                ("a".into(), MatchStatus::LeftOnly),
                ("b".into(), MatchStatus::Matched),
                ("b".into(), MatchStatus::LeftOnly),
                ("c".into(), MatchStatus::RightOnly),
            ]
        );
        let policy = MatchPolicy {
            allowed: ["a".to_string()].into_iter().collect(),
            forced_unmatched: BTreeSet::new(),
        };
        let a = align_rows(&l, &r, &["id"], Some(&policy));
        assert!(a.iter().all(|x| x.status != MatchStatus::Matched));
        let none = align_rows(&l, &r, &[], None);
        assert_eq!(none.len(), 5);
        assert_eq!(none[3].identity, "Right row 1");
    }

    #[test]
    fn metrics_and_change_percent_once() {
        let l = rows(
            r#"[{"id": "a", "v": 1}, {"id": "b", "v": 2}, {"id": "d", "v": 0}, {"id": "e", "v": "x"}, {"id": "", "v": 1}, {"id": "f", "v": 1}, {"id": "f", "v": 1}]"#,
        );
        let r = rows(
            r#"[{"id": "a", "v": 1.1}, {"id": "b", "v": 1}, {"id": "d", "v": 5}, {"id": "e", "v": 1}, {"id": "z", "v": 1}]"#,
        );
        let c = metric("v", "V", "m");
        let s = MatchStrategy {
            id: "id::id".into(),
            label: "id".into(),
            left_keys: vec!["id"],
            right_keys: vec!["id"],
        };
        let m = compare_metrics(&l, &c, &r, &c, &s);
        assert_eq!(m.rows.len(), 3);
        assert_eq!(m.unmatched_left, 1);
        assert_eq!(m.unmatched_right, 2);
        assert_eq!(m.duplicate_left, 2);
        assert_eq!(m.rows[2].ratio, None);
        assert_eq!(m.summary()[3], "Duplicate IDs skipped: 2");
        let t = metric_table(&m, DisplayUnit::Meter);
        assert_eq!(t.headers[1], "Left (m)");
        assert_eq!(t.headers[3], "Difference Δ (m)");
        assert_eq!(t.rows[0].0[5], "+10.00%");
        assert_eq!(t.rows[0].0[3], "+0.10");
        assert_eq!(t.rows[1].0[5], "-50.00%");
        assert_eq!(t.rows[1].1, Sign::Negative);
        assert_eq!(t.rows[2].0[4], "—");
        let pct = metric("v", "V", "ratio");
        let other = metric("v", "V", "L/s");
        let m2 = compare_metrics(&l, &pct, &r, &other, &s);
        assert!(m2.warning().is_some());
        assert_eq!(
            metric_table(&m2, DisplayUnit::Meter).headers[3],
            "Difference Δ"
        );
    }

    #[test]
    fn strategies_and_pairs() {
        let s = match_strategies(TableKind::PipeVelocity, TableKind::PipeVelocity);
        assert_eq!(
            s.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
            vec![
                "edgeId::edgeId",
                "startNodeId::startNodeId",
                "endNodeId::endNodeId"
            ]
        );
        assert_eq!(
            match_strategies(TableKind::Pressure, TableKind::Pressure).len(),
            1
        );
        assert_eq!(
            match_strategies(TableKind::Pressure, TableKind::SourceFlow).len(),
            0
        );
        assert_eq!(
            recommended_pair(TableKind::Pressure, TableKind::Pressure),
            Some(("minPressureM", "minPressureM"))
        );
        assert_eq!(
            recommended_pair(TableKind::Pressure, TableKind::SourceFlow),
            None
        );
    }

    fn record(n: u32, revision: &str, stats: &str) -> RunRecord {
        RunRecord {
            run_number: n,
            status: RunStatus::Completed,
            configuration_mode: ConfigurationMode::Default,
            revision: revision.into(),
            revision_version: 1,
            source_format: SourceFormat::Dsg,
            network_name: String::new(),
            created_at: String::new(),
            started_at: None,
            completed_at: None,
            requested: RequestedOptions::default(),
            resolved: Vec::new(),
            edge_states: Vec::new(),
            demand: Vec::new(),
            statistics: Json::parse(stats).ok(),
            failures: Vec::new(),
            error: None,
        }
    }

    #[test]
    fn sources_pairs_and_unified_table() {
        let st_l = r#"{"statistics": {"terminalPressure": [{"terminalId": "T1", "minPressureM": 1, "meanPressureM": 2, "maxPressureM": 3}],
            "transitPressure": [{"transitNodeId": "J", "minPressureM": 5, "meanPressureM": 6, "maxPressureM": 7}],
            "pipeVelocity": [{"edgeId": "A1", "startNodeId": "S", "endNodeId": "J", "minVelocityMps": 1, "meanVelocityMps": 1, "maxVelocityMps": 1}]}}"#;
        let st_r = r#"{"statistics": {"terminalPressure": [{"terminalId": "T1", "minPressureM": 1.5, "meanPressureM": 2, "maxPressureM": 3},
                                                          {"terminalId": "T3", "minPressureM": 1, "meanPressureM": 1, "maxPressureM": 1}],
            "transitPressure": [], "pipeVelocity": []}}"#;
        let runs = vec![
            record(1, "a", st_l),
            record(2, "b", st_r),
            record(3, "a", st_l),
        ];
        let src: Vec<u32> = comparison_sources(&runs, Some("b"))
            .iter()
            .map(|r| r.run_number)
            .collect();
        assert_eq!(src, vec![2, 3, 1]);
        assert_eq!(default_pair(&runs, None), Some((2, 3)));
        assert_eq!(default_pair(&runs, Some((1, 3))), Some((1, 3)));
        assert_eq!(default_pair(&runs, Some((1, 9))), Some((2, 3)));
        assert_eq!(default_pair(&runs[..1], None), None);
        let t = unified_table(
            &runs[0],
            &runs[1],
            TableKind::Pressure,
            None,
            DisplayUnit::Feet,
        );
        assert_eq!(t.headers[0].label, "Match");
        assert_eq!(t.headers[1].label, "Type");
        assert_eq!(t.headers[2].label, "Minimum (ft)");
        assert_eq!(t.headers.len(), 1 + 1 + 3 + 3);
        assert_eq!((t.matched, t.left_only, t.right_only), (1, 1, 1));
        assert_eq!(t.rows[0].identity, "T1");
        assert_eq!(t.rows[0].role, Some(NodeRole::Terminal));
        assert_eq!(t.rows[0].shared[0].text, "Terminal");
        assert_eq!(t.rows[0].left[0], "3.28");
        assert_eq!(t.rows[1].right[0], "—");
        // sort by right minimum: missing last
        let order = t.sorted_rows(Some((
            &SortTarget::Right("minPressureM"),
            SortDirection::Ascending,
        )));
        assert_eq!(order, vec![2, 0, 1]);
        let order = t.sorted_rows(Some((&SortTarget::Match, SortDirection::Ascending)));
        assert_eq!(order, vec![0, 1, 2]);
        let order = t.sorted_rows(Some((&SortTarget::Match, SortDirection::Descending)));
        assert_eq!(order, vec![2, 1, 0]);
        let pv = unified_table(
            &runs[0],
            &runs[1],
            TableKind::PipeVelocity,
            None,
            DisplayUnit::Meter,
        );
        assert_eq!(pv.rows[0].identity, "A1::S::J");
        assert_eq!(pv.rows[0].shared[0].text, "S");
        assert_eq!(pv.headers[1].label, "From");
    }
}
