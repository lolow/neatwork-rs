//! NeatWork 5 network model: the tables of the MATLAB code (TN, TA, TC,
//! TB, TD, TO, TP), see `rust/nw5/specs/formats.md` section 2.
//!
//! Units: lengths and elevations in m, diameters in m, target flows in L/s,
//! pressures in m of water. Identifiers are strings.

/// Node status (`statut`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeStatus {
    /// 0: source (reservoir).
    Source,
    /// 1: intermediate node (junction, transit).
    Junction,
    /// 2: terminal (borne) that serves clients.
    Terminal,
}

impl NodeStatus {
    pub fn code(self) -> i32 {
        match self {
            NodeStatus::Source => 0,
            NodeStatus::Junction => 1,
            NodeStatus::Terminal => 2,
        }
    }

    pub fn from_code(c: i32) -> Option<NodeStatus> {
        match c {
            0 => Some(NodeStatus::Source),
            1 => Some(NodeStatus::Junction),
            2 => Some(NodeStatus::Terminal),
            _ => None,
        }
    }
}

/// TN row.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// `noeud`
    pub id: String,
    /// `haut`: physical elevation, m.
    pub elevation: f64,
    /// `xdata`, `ydata`: drawing coordinates.
    pub x: f64,
    pub y: f64,
    /// `nbrob` of a `.tpo`: tap count in the file. Recomputed from the
    /// clients after reading.
    pub taps: Option<f64>,
    /// `orif_ideal`, `orif_com` of a `.dsg`, m. 0 = unset, 1 (NW5) or 10000
    /// (NW4) = no orifice.
    pub orifice_ideal: f64,
    pub orifice_commercial: f64,
    /// `statut`
    pub status: NodeStatus,
}

/// Arc status (`N` column of `!Pipes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcState {
    /// `N`: new or normal.
    New,
    /// `B`: existing pipe that must not be reinforced.
    Blocked,
    /// `R`: reinforcement pipe added by the design.
    Reinforcement,
}

impl ArcState {
    pub fn code(self) -> &'static str {
        match self {
            ArcState::New => "N",
            ArcState::Blocked => "B",
            ArcState::Reinforcement => "R",
        }
    }

    pub fn from_code(s: &str) -> ArcState {
        match s.trim() {
            "B" => ArcState::Blocked,
            "R" => ArcState::Reinforcement,
            _ => ArcState::New,
        }
    }
}

/// TA row: a link between two nodes, with at most two pipe segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Arc {
    /// Stable identifier (`A1`, `A2`, ... in file order when the file has
    /// none).
    pub id: String,
    /// `pred`, `succ`
    pub from: String,
    pub to: String,
    /// `lon`: total length, m.
    pub length: f64,
    /// `lon1`, `ID1`: first segment (larger diameter after design).
    pub length1: f64,
    pub pipe1: Option<String>,
    /// `lon2`, `ID2`: second segment. `None` and 0 when absent.
    pub length2: f64,
    pub pipe2: Option<String>,
    pub state: ArcState,
    /// Hydraulic open state (valves, application). Files without this
    /// information give `true`.
    pub open: bool,
}

impl Arc {
    /// The arc has at least one pipe segment.
    pub fn equipped(&self) -> bool {
        self.length1 > 0.0 || self.length2 > 0.0
    }
}

/// TC row: a client (demand point) attached to a terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Client {
    /// `client`
    pub id: String,
    /// `borne`
    pub terminal: String,
    /// `proba`: opening probability, 0 to 1.
    pub probability: f64,
    /// `cible`: target flow, L/s.
    pub target_flow: f64,
}

/// TB row: required pressure at a terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Terminal {
    /// `borne`
    pub id: String,
    /// `pressure`, m of water.
    pub pressure: f64,
}

/// TD row: a commercial pipe.
#[derive(Clone, Debug, PartialEq)]
pub struct Pipe {
    /// `ID` (`D001`, ...).
    pub id: String,
    /// `Nominal` (raw text).
    pub nominal: String,
    /// `SDR`: kept as text (some catalogues hold `PEHD`, `Fonte`).
    pub sdr: String,
    /// `diam`: inner diameter, m.
    pub diameter: f64,
    /// `cost` per m.
    pub cost: f64,
    /// `pressure` / `Pmax`: maximum allowed pressure, m.
    pub max_pressure: f64,
    /// `type` / `Type`: material code as text (`1`, `2`, `EPANET`).
    pub kind: String,
    /// `Roughness`
    pub roughness: f64,
}

/// TP: parameters `name, value` in file order. Values are kept as text so a
/// file writes back as it was read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    pub entries: Vec<(String, String)>,
}

/// Canonical defaults (MATLAB `extract_parameters.m`, with `lambda` 1.781,
/// ADR-021). See `specs/formats.md` section 2.7.
pub const DEFAULTS: &[(&str, f64)] = &[
    ("bescal", 8.9396e-4),
    ("lambda", 1.781),
    ("nu", 4.781),
    ("orifcoef", 1.0),
    ("faucetcoef", 1.0),
    ("targetflow", 0.0),
    ("opentaps", 1.0),
    ("pressure", 2.0),
    ("pump_pressure", 0.0),
    ("secu", 1.0),
    ("vmax", 1.5),
    ("vmax_secu", 0.95),
    ("servicequal", 0.65),
    ("fusion", 0.0),
    ("split", 0.0),
    ("bounded_flow", 0.0),
    ("uniform_dem", 1.0),
    ("pipelength", 6.0),
    ("seuil", 0.0),
    ("limitbudget", f64::INFINITY),
    ("watertemp", 20.0),
    ("use_efficient_pipes", 0.0),
    ("nScenLoad", 200000.0),
    ("use_cvar", 0.0),
    ("nbsim", 100.0),
    ("orif_com", 1.0),
];

/// Misspelled keys of some NW5 files and their canonical names.
pub const ALIASES: &[(&str, &str)] = &[
    ("bounded-flow", "bounded_flow"),
    ("serviceequal", "servicequal"),
    ("uniform-dem", "uniform_dem"),
    ("pump-pressure", "pump_pressure"),
];

impl Params {
    /// Canonical name of a key.
    pub fn canonical(name: &str) -> &str {
        let n = name.trim();
        ALIASES.iter().find(|(a, _)| *a == n).map_or(n, |(_, c)| c)
    }

    /// Raw text of a parameter (last occurrence wins).
    pub fn raw(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| Params::canonical(k) == name)
            .map(|(_, v)| v.as_str())
    }

    /// Number value: file value, else the canonical default.
    pub fn get(&self, name: &str) -> f64 {
        self.raw(name)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .or_else(|| DEFAULTS.iter().find(|(k, _)| *k == name).map(|(_, v)| *v))
            .unwrap_or(f64::NAN)
    }

    /// Set a value (replaces the existing entry or appends).
    pub fn set(&mut self, name: &str, value: &str) {
        match self
            .entries
            .iter_mut()
            .find(|(k, _)| Params::canonical(k) == name)
        {
            Some(e) => {
                e.0 = name.to_string();
                e.1 = value.to_string();
            }
            None => self.entries.push((name.to_string(), value.to_string())),
        }
    }
}

/// A NW5 network: a topography (`.tpo`, no pipe chosen) or a design
/// (`.dsg`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Network {
    pub name: String,
    pub nodes: Vec<Node>,
    pub arcs: Vec<Arc>,
    pub clients: Vec<Client>,
    pub terminals: Vec<Terminal>,
    /// TD: pipe catalogue.
    pub pipes: Vec<Pipe>,
    /// TO: orifice inner diameters, m.
    pub orifices: Vec<f64>,
    pub params: Params,
    /// Design flows and head margins per arc (`!loadflow`, ADR-022):
    /// `(succ, design flow L/s, margin m)`.
    pub loadflow: Vec<(String, f64, f64)>,
    /// Rows of `!loadfactor` as read (three variants, kept for round-trip).
    pub loadfactor_raw: Vec<Vec<String>>,
    /// `!Commentaires` lines.
    pub comments: Vec<String>,
    /// Sections the reader does not use, kept verbatim: `(header, lines)`.
    pub extra_sections: Vec<(String, Vec<String>)>,
}

impl Network {
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn pipe(&self, id: &str) -> Option<&Pipe> {
        self.pipes.iter().find(|p| p.id == id)
    }

    /// The network has pipes chosen on every arc (a design).
    pub fn is_design(&self) -> bool {
        !self.arcs.is_empty() && self.arcs.iter().all(Arc::equipped)
    }

    /// `apply_client_data`: tap counts come from the clients; uniform demand
    /// fixes `opentaps` and `targetflow`; diverse demand at a shared terminal
    /// needs bounded flow. Returns warnings.
    pub fn apply_client_data(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        for n in &mut self.nodes {
            let count = self.clients.iter().filter(|c| c.terminal == n.id).count();
            n.taps = Some(count as f64);
        }
        let Some(first) = self.clients.first() else {
            return warnings;
        };
        let uniform = self
            .clients
            .iter()
            .all(|c| c.probability == first.probability && c.target_flow == first.target_flow);
        if uniform {
            let (p, q) = (first.probability, first.target_flow);
            self.params.set("uniform_dem", "1");
            self.params.set("opentaps", &fmt_num(p));
            self.params.set("targetflow", &fmt_num(q));
        } else {
            self.params.set("uniform_dem", "0");
            let shared = self
                .terminals
                .iter()
                .any(|t| self.clients.iter().filter(|c| c.terminal == t.id).count() > 1);
            if shared && self.params.get("bounded_flow") == 0.0 {
                warnings.push(
                    "NW5:BoundedFlowRequired: diverse demand at a shared terminal needs bounded flow; bounded_flow set to 1".into(),
                );
                self.params.set("bounded_flow", "1");
            }
        }
        warnings
    }
}

/// Shortest text of a number (as MATLAB `num2str` with enough digits).
pub fn fmt_num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}
