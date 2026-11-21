//! Graph data of a revision (`specs/app.md` F19), the node positions and
//! arc open/closed states set by the user (F25, D11) and the node search
//! (F26).
//!
//! The user state is kept in `graph.json` per revision family, keyed by
//! node ID and arc ID, so it survives a new revision of the same family.

use std::collections::BTreeMap;

use neatwork_nw5::{Network, NodeStatus};

use crate::json::{Json, get_f64, get_str, n, obj, s};
use crate::revision::SourceFormat;
use crate::{Error, Result};

/// Node type of the graph (from the node status).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// Status 0.
    Reservoir,
    /// Status 1.
    Junction,
    /// Status 2.
    Faucet,
}

impl NodeKind {
    /// Kind of a node status.
    pub fn of(status: NodeStatus) -> NodeKind {
        match status {
            NodeStatus::Source => NodeKind::Reservoir,
            NodeStatus::Junction => NodeKind::Junction,
            NodeStatus::Terminal => NodeKind::Faucet,
        }
    }

    /// `RESERVOIR`, `JUNCTION`, `FAUCET`.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Reservoir => "RESERVOIR",
            NodeKind::Junction => "JUNCTION",
            NodeKind::Faucet => "FAUCET",
        }
    }
}

/// Edge type of the graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// A pipe.
    Pipe,
    /// A `.tpo` pipe of length ≤ 0.
    Valve,
}

impl EdgeKind {
    /// `PIPE`, `VALVE`.
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeKind::Pipe => "PIPE",
            EdgeKind::Valve => "VALVE",
        }
    }
}

/// A node of the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    /// Node ID.
    pub id: String,
    /// Label (the ID).
    pub label: String,
    /// Type.
    pub kind: NodeKind,
    /// Visual coordinates: the user position, else the file coordinates,
    /// else the generated layout.
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// Coordinates of the file.
    pub source_x: f64,
    /// See `source_x`.
    pub source_y: f64,
    /// Elevation, m.
    pub elevation_m: f64,
    /// Required pressure of a terminal, m.
    pub required_pressure_m: Option<f64>,
    /// Mean pressure of the applied run, m (`None`: not simulated).
    pub pressure_m: Option<f64>,
    /// The applied minimum pressure of a terminal is below its required
    /// pressure (python-core `simulation_service.py:218`).
    pub low_pressure: bool,
    /// Visual and file coordinates differ.
    pub layout_generated: bool,
}

/// An edge of the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    /// Arc ID (`A<n>` in pipe order).
    pub id: String,
    /// Upstream node.
    pub source_id: String,
    /// Downstream node.
    pub target_id: String,
    /// Type.
    pub kind: EdgeKind,
    /// Open state (user state, default open).
    pub open: bool,
    /// Flow of the applied run, L/s (`None`: not simulated).
    pub flow_lps: Option<f64>,
    /// Mean velocity of the applied run, m/s (`None`: not simulated).
    pub velocity_mps: Option<f64>,
    /// Length, m (`.tpo`: at least 0.01).
    pub length_m: f64,
    /// Catalogue ID of segment 1.
    pub diameter_id: Option<String>,
    /// Catalogue values of segment 1.
    pub nominal_size: Option<String>,
    /// SDR (text in the catalogue).
    pub sdr: Option<String>,
    /// Inner diameter, m.
    pub inner_diameter: Option<f64>,
    /// Cost per m.
    pub cost: Option<f64>,
    /// Maximum pressure, m.
    pub pmax: Option<f64>,
    /// Pipe type code.
    pub pipe_type_code: Option<String>,
    /// Roughness.
    pub roughness: Option<f64>,
}

/// User state of one revision family: node visual positions and arc
/// open/closed states.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphState {
    /// Visual position by node ID.
    pub positions: BTreeMap<String, (f64, f64)>,
    /// Open state by arc ID (absent: open).
    pub arc_open: BTreeMap<String, bool>,
}

impl GraphState {
    /// Open state of every arc of a network, in arc order: the stored state,
    /// else open (the `arc_open` option of the next run, F25).
    pub fn arc_states(&self, net: &Network) -> Vec<(String, bool)> {
        net.arcs
            .iter()
            .map(|a| {
                (
                    a.id.clone(),
                    self.arc_open.get(&a.id).copied().unwrap_or(true),
                )
            })
            .collect()
    }

    fn to_json(&self) -> Json {
        obj(vec![
            (
                "nodePositions",
                Json::Object(
                    self.positions
                        .iter()
                        .map(|(id, (x, y))| (id.clone(), obj(vec![("x", n(*x)), ("y", n(*y))])))
                        .collect(),
                ),
            ),
            (
                "arcStates",
                Json::Object(
                    self.arc_open
                        .iter()
                        .map(|(id, open)| (id.clone(), s(if *open { "OPEN" } else { "CLOSED" })))
                        .collect(),
                ),
            ),
        ])
    }

    fn from_json(v: &Json) -> GraphState {
        let mut st = GraphState::default();
        if let Some(Json::Object(m)) = v.get("nodePositions") {
            for (id, p) in m {
                if let (Some(x), Some(y)) = (get_f64(p, "x"), get_f64(p, "y")) {
                    st.positions.insert(id.clone(), (x, y));
                }
            }
        }
        if let Some(Json::Object(m)) = v.get("arcStates") {
            for (id, state) in m {
                if let Json::String(t) = state {
                    st.arc_open
                        .insert(id.clone(), !t.eq_ignore_ascii_case("CLOSED"));
                }
            }
        }
        st
    }
}

/// Text of `graph.json`: the state of each family.
pub fn graph_states_to_json(states: &BTreeMap<String, GraphState>) -> String {
    obj(vec![(
        "families",
        Json::Object(
            states
                .iter()
                .map(|(f, st)| (f.clone(), st.to_json()))
                .collect(),
        ),
    )])
    .pretty()
}

/// Read `graph.json`.
pub fn graph_states_from_json(text: &str) -> Result<BTreeMap<String, GraphState>> {
    let v = Json::parse(text).map_err(|e| Error::Format(format!("graph.json: {e}")))?;
    let mut out = BTreeMap::new();
    if let Some(Json::Object(m)) = v.get("families") {
        for (family, st) in m {
            out.insert(family.clone(), GraphState::from_json(st));
        }
    }
    Ok(out)
}

/// Graph of a revision.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphData {
    /// Network name.
    pub name: String,
    /// Nodes in file order.
    pub nodes: Vec<GraphNode>,
    /// Edges in pipe order.
    pub edges: Vec<GraphEdge>,
}

/// Tree layout used when every node has x = y = 0
/// (`PY neatwork/parsers/dsg_parser.py:238-306`): roots are the status-0
/// nodes, else the nodes without incoming pipe, else the first node;
/// x = depth × 160; a leaf takes y = 80 × (leaf counter); a parent takes
/// the mean y of its children.
pub fn generated_layout(net: &Network) -> Option<Vec<(f64, f64)>> {
    if net.nodes.iter().any(|nd| nd.x != 0.0 || nd.y != 0.0) {
        return None;
    }
    let index: BTreeMap<&str, usize> = net
        .nodes
        .iter()
        .enumerate()
        .map(|(k, nd)| (nd.id.as_str(), k))
        .collect();
    let mut children = vec![Vec::new(); net.nodes.len()];
    let mut incoming = vec![false; net.nodes.len()];
    for a in &net.arcs {
        if let (Some(&p), Some(&c)) = (index.get(a.from.as_str()), index.get(a.to.as_str())) {
            children[p].push(c);
            incoming[c] = true;
        }
    }
    let mut roots: Vec<usize> = (0..net.nodes.len())
        .filter(|&k| net.nodes[k].status == NodeStatus::Source)
        .collect();
    if roots.is_empty() {
        roots = (0..net.nodes.len()).filter(|&k| !incoming[k]).collect();
    }
    if roots.is_empty() && !net.nodes.is_empty() {
        roots = vec![0];
    }
    struct Layout<'a> {
        children: &'a [Vec<usize>],
        visited: Vec<bool>,
        pos: Vec<(f64, f64)>,
        leaves: usize,
    }
    fn place(l: &mut Layout<'_>, k: usize, depth: usize) -> f64 {
        if l.visited[k] {
            return l.pos[k].1;
        }
        l.visited[k] = true;
        let kids = l.children[k].clone();
        let y = if kids.is_empty() {
            let y = l.leaves as f64 * 80.0;
            l.leaves += 1;
            y
        } else {
            let ys: Vec<f64> = kids.iter().map(|&c| place(l, c, depth + 1)).collect();
            ys.iter().sum::<f64>() / ys.len() as f64
        };
        l.pos[k] = (depth as f64 * 160.0, y);
        y
    }
    let mut l = Layout {
        children: &children,
        visited: vec![false; net.nodes.len()],
        pos: vec![(0.0, 0.0); net.nodes.len()],
        leaves: 0,
    };
    for r in roots {
        place(&mut l, r, 0);
    }
    for k in 0..net.nodes.len() {
        if !l.visited[k] {
            place(&mut l, k, 0);
        }
    }
    Some(l.pos)
}

impl GraphData {
    /// Graph of a network with the user state of its family.
    pub fn build(net: &Network, format: SourceFormat, state: &GraphState) -> GraphData {
        let layout = generated_layout(net);
        let nodes = net
            .nodes
            .iter()
            .enumerate()
            .map(|(k, nd)| {
                let (gx, gy) = layout.as_ref().map_or((nd.x, nd.y), |l| l[k]);
                let (x, y) = state.positions.get(&nd.id).copied().unwrap_or((gx, gy));
                GraphNode {
                    id: nd.id.clone(),
                    label: nd.id.clone(),
                    kind: NodeKind::of(nd.status),
                    x,
                    y,
                    source_x: nd.x,
                    source_y: nd.y,
                    elevation_m: nd.elevation,
                    required_pressure_m: net
                        .terminals
                        .iter()
                        .find(|t| t.id == nd.id)
                        .map(|t| t.pressure),
                    pressure_m: None,
                    low_pressure: false,
                    layout_generated: x != nd.x || y != nd.y,
                }
            })
            .collect();
        let edges = net
            .arcs
            .iter()
            .map(|a| {
                let pipe = a.pipe1.as_deref().and_then(|id| net.pipe(id));
                let (kind, length_m) = if format == SourceFormat::Tpo {
                    let kind = if a.length <= 0.0 {
                        EdgeKind::Valve
                    } else {
                        EdgeKind::Pipe
                    };
                    (kind, a.length.max(0.01))
                } else {
                    (EdgeKind::Pipe, a.length)
                };
                GraphEdge {
                    id: a.id.clone(),
                    source_id: a.from.clone(),
                    target_id: a.to.clone(),
                    kind,
                    open: state.arc_open.get(&a.id).copied().unwrap_or(true),
                    flow_lps: None,
                    velocity_mps: None,
                    length_m,
                    diameter_id: a.pipe1.clone(),
                    nominal_size: pipe.map(|p| p.nominal.clone()),
                    sdr: pipe.map(|p| p.sdr.clone()),
                    inner_diameter: pipe.map(|p| p.diameter),
                    cost: pipe.map(|p| p.cost),
                    pmax: pipe.map(|p| p.max_pressure),
                    pipe_type_code: pipe.map(|p| p.kind.clone()),
                    roughness: pipe.map(|p| p.roughness),
                }
            })
            .collect();
        GraphData {
            name: net.name.clone(),
            nodes,
            edges,
        }
    }

    /// Show the results of a run (its statistics payload): mean pressure per
    /// node and mean velocity per pipe.
    // PROPOSAL: NW5 does not say which per-run value the graph shows. The
    // port shows the mean pressure and mean velocity of the statistics;
    // per-pipe flows are not part of the statistics and stay `None`.
    // Low pressure follows python-core (`services/simulation_service.py:218`):
    // a terminal whose minimum pressure is below its required pressure;
    // transit nodes are never low. A payload without the minimum uses the
    // mean.
    pub fn apply_statistics(&mut self, statistics: &Json) {
        self.clear_results();
        let stats = statistics.get("statistics");
        let rows = |key: &str| -> Vec<Json> {
            match stats.and_then(|st| st.get(key)) {
                Some(Json::Array(v)) => v.clone(),
                _ => Vec::new(),
            }
        };
        for (key, id_key) in [
            ("terminalPressure", "terminalId"),
            ("transitPressure", "transitNodeId"),
        ] {
            for r in rows(key) {
                let (Some(id), Some(p)) = (get_str(&r, id_key), get_f64(&r, "meanPressureM"))
                else {
                    continue;
                };
                let min = get_f64(&r, "minPressureM").unwrap_or(p);
                let terminal = key == "terminalPressure";
                if let Some(nd) = self.nodes.iter_mut().find(|nd| nd.id == id) {
                    nd.pressure_m = Some(p);
                    nd.low_pressure =
                        terminal && nd.required_pressure_m.is_some_and(|req| min < req);
                }
            }
        }
        for r in rows("pipeVelocity") {
            let (Some(id), Some(v)) = (get_str(&r, "edgeId"), get_f64(&r, "meanVelocityMps"))
            else {
                continue;
            };
            if let Some(e) = self.edges.iter_mut().find(|e| e.id == id) {
                e.velocity_mps = Some(v);
            }
        }
    }

    /// Remove the results shown on the graph. Opening or closing a valve
    /// does this instead of showing placeholder values (D6).
    pub fn clear_results(&mut self) {
        for nd in &mut self.nodes {
            nd.pressure_m = None;
            nd.low_pressure = false;
        }
        for e in &mut self.edges {
            e.flow_lps = None;
            e.velocity_mps = None;
        }
    }

    /// Node search (F26), see [`find_node`].
    pub fn find_node(&self, query: &str) -> std::result::Result<&GraphNode, String> {
        find_node(&self.nodes, query)
    }
}

/// Node search (F26): an empty query is refused; an exact case-insensitive
/// match on ID or label wins; else a unique substring match.
pub fn find_node<'a>(
    nodes: &'a [GraphNode],
    query: &str,
) -> std::result::Result<&'a GraphNode, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Err("Enter a node ID.".into());
    }
    if let Some(nd) = nodes
        .iter()
        .find(|nd| nd.id.to_lowercase() == q || nd.label.to_lowercase() == q)
    {
        return Ok(nd);
    }
    let matches: Vec<&GraphNode> = nodes
        .iter()
        .filter(|nd| nd.id.to_lowercase().contains(&q) || nd.label.to_lowercase().contains(&q))
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err("Node not found.".into()),
        many => Err(format!(
            "{} nodes match. Enter a more specific ID.",
            many.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neatwork_nw5::{Arc, ArcState, Node};

    fn node(id: &str, status: NodeStatus) -> Node {
        Node {
            id: id.into(),
            elevation: 0.0,
            x: 0.0,
            y: 0.0,
            taps: None,
            orifice_ideal: 0.0,
            orifice_commercial: 0.0,
            status,
        }
    }

    fn arc(k: usize, from: &str, to: &str, length: f64) -> Arc {
        Arc {
            id: format!("A{k}"),
            from: from.into(),
            to: to.into(),
            length,
            length1: 0.0,
            pipe1: None,
            length2: 0.0,
            pipe2: None,
            state: ArcState::New,
            open: true,
        }
    }

    fn net() -> Network {
        Network {
            name: "t".into(),
            nodes: vec![
                node("S", NodeStatus::Source),
                node("J", NodeStatus::Junction),
                node("T1", NodeStatus::Terminal),
                node("T2", NodeStatus::Terminal),
            ],
            arcs: vec![
                arc(1, "S", "J", 10.0),
                arc(2, "J", "T1", 0.0),
                arc(3, "J", "T2", 5.0),
            ],
            ..Network::default()
        }
    }

    #[test]
    fn layout_types_and_valves() {
        let g = GraphData::build(&net(), SourceFormat::Tpo, &GraphState::default());
        let pos: Vec<(f64, f64)> = g.nodes.iter().map(|nd| (nd.x, nd.y)).collect();
        assert_eq!(
            pos,
            vec![(0.0, 40.0), (160.0, 40.0), (320.0, 0.0), (320.0, 80.0)]
        );
        assert!(g.nodes[1].layout_generated);
        assert!(g.nodes[2].layout_generated);
        assert_eq!(g.nodes[0].kind, NodeKind::Reservoir);
        assert_eq!(g.nodes[2].kind, NodeKind::Faucet);
        assert_eq!(g.edges[1].kind, EdgeKind::Valve);
        assert_eq!(g.edges[1].length_m, 0.01);
        let dsg = GraphData::build(&net(), SourceFormat::Dsg, &GraphState::default());
        assert_eq!(dsg.edges[1].kind, EdgeKind::Pipe);
    }

    #[test]
    fn user_state_and_round_trip() -> Result<()> {
        let mut st = GraphState::default();
        st.positions.insert("J".into(), (1.5, -2.0));
        st.arc_open.insert("A2".into(), false);
        let g = GraphData::build(&net(), SourceFormat::Tpo, &st);
        assert_eq!((g.nodes[1].x, g.nodes[1].y), (1.5, -2.0));
        assert!(!g.edges[1].open);
        assert_eq!(
            st.arc_states(&net()),
            vec![
                ("A1".into(), true),
                ("A2".into(), false),
                ("A3".into(), true)
            ]
        );
        let mut all = BTreeMap::new();
        all.insert("fam".to_string(), st);
        assert_eq!(graph_states_from_json(&graph_states_to_json(&all))?, all);
        Ok(())
    }

    #[test]
    fn search() {
        let g = GraphData::build(&net(), SourceFormat::Dsg, &GraphState::default());
        assert_eq!(g.find_node(" "), Err("Enter a node ID.".into()));
        assert_eq!(g.find_node("j").map(|nd| nd.id.as_str()), Ok("J"));
        assert_eq!(
            g.find_node("t"),
            Err("2 nodes match. Enter a more specific ID.".into())
        );
        assert_eq!(g.find_node("T2").map(|nd| nd.id.as_str()), Ok("T2"));
        assert_eq!(g.find_node("x"), Err("Node not found.".into()));
    }

    #[test]
    fn statistics_on_the_graph() -> Result<()> {
        let mut g = GraphData::build(&net(), SourceFormat::Dsg, &GraphState::default());
        g.nodes[2].required_pressure_m = Some(2.0);
        let stats = Json::parse(
            r#"{"statistics": {"terminalPressure": [{"terminalId": "T1", "meanPressureM": 1.5}],
                "transitPressure": [{"transitNodeId": "J", "meanPressureM": 9}],
                "pipeVelocity": [{"edgeId": "A1", "meanVelocityMps": 0.4}]}}"#,
        )
        .map_err(|e| Error::Format(e.to_string()))?;
        g.apply_statistics(&stats);
        assert_eq!(g.nodes[2].pressure_m, Some(1.5));
        assert!(g.nodes[2].low_pressure);
        assert_eq!(g.nodes[1].pressure_m, Some(9.0));
        assert_eq!(g.edges[0].velocity_mps, Some(0.4));
        g.clear_results();
        assert_eq!(g.edges[0].velocity_mps, None);
        assert!(!g.nodes[2].low_pressure);
        Ok(())
    }
}
