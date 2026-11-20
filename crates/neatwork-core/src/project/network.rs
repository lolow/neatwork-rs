//! `neatwork.project.Network`, `Node` and `Pipe`: the editable network that
//! topography and design files describe.

use crate::java::{compare_utf16, double_to_string};
use crate::messages::Locale;

/// Node types (`Node.TYPE_*`).
pub const TYPE_RESERVOIR: i32 = 0;
pub const TYPE_DISPATCH: i32 = 1;
pub const TYPE_FAUCET: i32 = 2;

/// `neatwork.project.Node`.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub coord_x: f64,
    pub coord_y: f64,
    pub height: f64,
    pub node_type: i32,
    pub nb_taps: i32,
    pub orifice: f64,
    pub orifice_commercial: f64,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            name: "node".to_string(),
            coord_x: 0.0,
            coord_y: 0.0,
            height: 0.0,
            node_type: TYPE_DISPATCH,
            nb_taps: 0,
            orifice: 0.0,
            orifice_commercial: 0.0,
        }
    }
}

impl Node {
    pub fn named(name: &str) -> Self {
        Node {
            name: name.to_string(),
            ..Node::default()
        }
    }
}

/// `neatwork.project.Pipe`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pipe {
    pub begin: String,
    pub end: String,
    pub length: f64,
    pub length1: f64,
    pub length2: f64,
    pub ref_diam1: String,
    pub ref_diam2: String,
}

impl Pipe {
    /// `Pipe.isSame`: same arc in either direction.
    fn is_same(&self, other: &Pipe) -> bool {
        (self.begin == other.begin && self.end == other.end)
            || (self.begin == other.end && self.end == other.begin)
    }
}

/// `neatwork.project.Network`: node and pipe lists plus the load messages.
#[derive(Clone, Debug, Default)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub pipes: Vec<Pipe>,
    /// Messages collected while loading (`Project.infoModif`).
    pub info: Vec<String>,
    /// True when a message is an error (`indexOf(Project.ERROR) != -1`).
    pub has_error: bool,
    pub locale: Locale,
}

impl Network {
    pub fn new(locale: Locale) -> Self {
        Network {
            locale,
            ..Network::default()
        }
    }

    fn error(&mut self, msg: String) {
        self.info
            .push(format!("{} {msg}", self.locale.error_prefix()));
        self.has_error = true;
    }

    pub fn source(&self) -> Option<&Node> {
        // getSource returns the last reservoir found.
        self.nodes
            .iter()
            .rev()
            .find(|n| n.node_type == TYPE_RESERVOIR)
    }

    pub fn node(&self, name: &str) -> Option<&Node> {
        // getNode returns the last match.
        self.nodes.iter().rev().find(|n| n.name == name)
    }

    pub fn add_node(&mut self, node: Node) {
        if node.node_type == TYPE_RESERVOIR && self.source().is_some() {
            self.error(format!(
                "can't add node {}: there is already a source",
                node.name
            ));
        } else if !self.nodes.iter().any(|n| n.name == node.name) {
            self.nodes.push(node);
            self.set_nodes_type();
        } else {
            self.error(format!("can't add node {}: existing name", node.name));
        }
    }

    pub fn add_pipe(&mut self, pipe: Pipe) {
        let arc = format!("{} -> {}", pipe.begin, pipe.end);
        if pipe.begin == pipe.end {
            self.error(format!("can't add arc {arc}: begin and end are same"));
        } else if !self.nodes.iter().any(|n| n.name == pipe.begin) {
            self.error(format!("can't add arc {arc}: begin doesn't exist"));
        } else if !self.nodes.iter().any(|n| n.name == pipe.end) {
            self.error(format!("can't add arc {arc}: end doesn't exist"));
        } else if !self.pipes.iter().any(|p| p.is_same(&pipe)) {
            self.pipes.push(pipe);
            self.set_nodes_type();
        } else {
            self.error(format!("can't add arc {arc}: existing name"));
        }
    }

    /// Every non-reservoir node is a dispatch node when a pipe starts from
    /// it, otherwise a faucet.
    fn set_nodes_type(&mut self) {
        for i in 0..self.nodes.len() {
            if self.nodes[i].node_type != TYPE_RESERVOIR {
                let name = &self.nodes[i].name;
                let has_succ = self.pipes.iter().any(|p| &p.begin == name);
                self.nodes[i].node_type = if has_succ { TYPE_DISPATCH } else { TYPE_FAUCET };
            }
        }
    }

    /// `addNewSource`: only on an empty network.
    pub fn add_new_source(&mut self) {
        if self.nodes.is_empty() {
            let mut node = Node::named(self.locale.source_name());
            node.node_type = TYPE_RESERVOIR;
            let name = node.name.clone();
            self.add_node(node);
            self.info.push(format!(
                "{} a source ({name}) has been added",
                self.locale.warning_prefix()
            ));
        }
    }

    /// `Collections.sort(nodeList, new Node())`: stable, by type.
    pub fn sort_nodes(&mut self) {
        self.nodes.sort_by_key(|n| n.node_type);
    }

    /// `setHauteurSource`: shift all heights so the source is at `height`;
    /// returns the old source height.
    pub fn set_source_height(&mut self, height: f64) -> f64 {
        let hsource = self.source().map_or(0.0, |s| s.height);
        for n in &mut self.nodes {
            n.height = n.height + height - hsource;
        }
        hsource
    }
}

/// `Topographie.getNodeCSV`.
pub fn topo_node_csv(n: &Node) -> String {
    format!(
        "{},{},{},{},{},{}",
        n.name,
        double_to_string(n.height),
        double_to_string(n.coord_x),
        double_to_string(n.coord_y),
        n.nb_taps,
        n.node_type
    )
}

/// `Topographie.getPipeCSV`.
pub fn topo_pipe_csv(p: &Pipe) -> String {
    format!("{},{},{}", p.begin, p.end, double_to_string(p.length))
}

/// Stable sort of string rows on one column, like `VectorComparator`.
pub fn sort_rows_by(rows: &mut [Vec<String>], col: usize) {
    rows.sort_by(|a, b| compare_utf16(&a[col], &b[col]));
}
