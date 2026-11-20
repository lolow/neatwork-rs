//! `neatwork.gui.tree`: the tree view of a network (`TreeNetwork`,
//! `TreeNetworkModel`, `TreeCellRenderer`, `utils.MapComparator`).

use std::collections::HashMap;

use crate::java::{JHashtable, JavaHash, eq_ignore_case};
use crate::project::network::{Network, Node, TYPE_FAUCET, TYPE_RESERVOIR};
use crate::{Error, Result};

const LINE_L: &str = "_*L";
const LINE_I: &str = "_*I";
const LINE_T: &str = "_*T";

/// What a cell of the tree view shows.
#[derive(Clone, Debug, PartialEq)]
pub enum TreeCellKind {
    /// A node box. `node_type` is `None` when the renderer draws no box
    /// (the name starts with `_`).
    Node {
        name: String,
        node_type: Option<i32>,
    },
    /// `_*L`: a corner from the cell above to the right edge.
    LineL,
    /// `_*I`: a vertical line.
    LineI,
    /// `_*T`: a vertical line with a branch to the right.
    LineT,
}

impl TreeCellKind {
    /// Text of the cell in the Java table model.
    pub fn text(&self) -> &str {
        match self {
            TreeCellKind::Node { name, .. } => name,
            TreeCellKind::LineL => LINE_L,
            TreeCellKind::LineI => LINE_I,
            TreeCellKind::LineT => LINE_T,
        }
    }

    /// `TreeCellRenderer`: the drawing follows the text of the cell.
    fn from_text(network: &Network, text: &str) -> Self {
        match text {
            LINE_L => TreeCellKind::LineL,
            LINE_I => TreeCellKind::LineI,
            LINE_T => TreeCellKind::LineT,
            _ => TreeCellKind::Node {
                name: text.to_string(),
                node_type: if text.is_empty() || text.starts_with('_') {
                    None
                } else {
                    node(network, text).map(|n| n.node_type)
                },
            },
        }
    }
}

/// One non-empty cell of the tree view.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeCell {
    pub row: usize,
    pub col: usize,
    pub kind: TreeCellKind,
}

/// Grid of the tree view. Depth goes from left to right.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeLayout {
    pub rows: usize,
    pub cols: usize,
    /// Non-empty cells, in the iteration order of the Java `Hashtable`.
    pub cells: Vec<TreeCell>,
}

impl TreeLayout {
    /// Cell at a position, if not empty.
    pub fn cell(&self, row: usize, col: usize) -> Option<&TreeCell> {
        self.cells.iter().find(|c| c.row == row && c.col == col)
    }
}

/// Content of the tree view.
#[derive(Clone, Debug, PartialEq)]
pub enum TreeView {
    Tree(TreeLayout),
    /// The network is not a tree; the view shows this single line.
    NotATree(String),
}

/// `java.awt.Point`, the key of the cell table. `x` is the column.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl JavaHash for Point {
    /// `Point2D.hashCode`.
    fn java_hash(&self) -> i32 {
        let mut bits = f64::from(self.x).to_bits() as i64;
        bits ^= (f64::from(self.y).to_bits() as i64).wrapping_mul(31);
        (bits as i32) ^ ((bits >> 32) as i32)
    }
}

/// `Network.getNode`: first node with this name.
fn node<'a>(network: &'a Network, name: &str) -> Option<&'a Node> {
    network.nodes.iter().find(|n| n.name == name)
}

/// `Network.getFirstPred`, when the node has a predecessor.
fn first_pred<'a>(network: &'a Network, name: &str) -> Option<&'a Node> {
    let pipe = network.pipes.iter().find(|p| p.end == name)?;
    node(network, &pipe.begin)
}

/// `Network.isATree`: the source has no predecessor, every other node
/// has exactly one.
pub fn is_a_tree(network: &Network) -> bool {
    network.nodes.iter().all(|n| {
        let preds = network.pipes.iter().filter(|p| p.end == n.name).count();
        let expected = if n.node_type == TYPE_RESERVOIR { 0 } else { 1 };
        preds == expected
    })
}

fn broken(what: &str) -> Error {
    Error::Network(format!("tree view: {what}"))
}

/// `TreeNetwork.setNetwork` and `buildTree`.
///
/// Nodes are placed by depth (column) and stacked so that each node has as
/// many rows as faucets below it. Siblings come in increasing number of
/// faucets, ties in `Hashtable` order. The Java code throws where this
/// returns an error: no source, a node that the source does not reach, a
/// cycle.
pub fn build_tree(name: &str, network: &Network) -> Result<TreeView> {
    if !is_a_tree(network) {
        return Ok(TreeView::NotATree(format!("_{name} is not a tree")));
    }
    let source = network.source().ok_or_else(|| broken("no source"))?;

    // Step 1: nodes by depth, each depth in Hashtable order.
    let mut map: JHashtable<String, i32> = JHashtable::new();
    for n in &network.nodes {
        map.put(n.name.clone(), 0);
    }
    depth(network, source, &mut map, 0)?;
    let mut nprof: Vec<Vec<String>> = Vec::new();
    let mut prof = 0;
    while !map.is_empty() {
        let line: Vec<String> = map
            .entries()
            .filter(|(_, p)| **p == prof)
            .map(|(n, _)| n.clone())
            .collect();
        for n in &line {
            map.remove(n);
        }
        nprof.push(line);
        prof += 1;
    }

    // Step 2: faucets below each node (a faucet counts itself), then a
    // stable sort of each depth (MapComparator).
    let mut faucets: HashMap<String, i32> =
        network.nodes.iter().map(|n| (n.name.clone(), 0)).collect();
    for n in &network.nodes {
        if n.node_type == TYPE_FAUCET {
            remonte_source(network, n, |name| {
                *faucets.entry(name.to_string()).or_insert(0) += 1;
            })?;
        }
    }
    let count = |n: &str| faucets.get(n).copied().unwrap_or(0);
    for line in &mut nprof {
        line.sort_by_key(|n| count(n));
    }

    // Step 3: column = depth; row = first free row of the parent.
    let mut map_x: JHashtable<String, i32> = JHashtable::new();
    for (i, line) in (0i32..).zip(&nprof) {
        for n in line {
            map_x.put(n.clone(), i);
        }
    }
    let mut map_y: HashMap<String, i32> = HashMap::new();
    map_y.insert(source.name.clone(), 0);
    for pair in nprof.windows(2) {
        for n in &pair[0] {
            let mut pos = *map_y
                .get(n)
                .ok_or_else(|| broken(&format!("node {n} is not reached from the source")))?;
            for succ in &pair[1] {
                if network
                    .pipes
                    .iter()
                    .any(|p| &p.begin == n && &p.end == succ)
                {
                    map_y.insert(succ.clone(), pos);
                    pos += count(succ);
                }
            }
        }
    }
    let position = |n: &str| -> Result<(i32, i32)> {
        match (map_x.get(&n.to_string()), map_y.get(n)) {
            (Some(x), Some(y)) => Ok((*x, *y)),
            _ => Err(broken(&format!("node {n} is not reached from the source"))),
        }
    };
    let mut coord: JHashtable<Point, String> = JHashtable::new();
    for n in map_x.keys() {
        let (x, y) = position(n)?;
        coord.put(Point { x, y }, n.clone());
    }

    // Step 4: lines from each node up to its parent, twice.
    for _ in 0..2 {
        for item in &network.nodes {
            let (mut x, mut y) = position(&item.name)?;
            let left = coord.get(&Point { x: x - 1, y }).cloned();
            if y > 0 && left.is_none() {
                coord.put(Point { x: x - 1, y }, LINE_L.to_string());
                x -= 1;
                y -= 1;
                let mut z = coord.get(&Point { x, y }).cloned();
                while y > 0 && z.is_none() {
                    coord.put(Point { x, y }, LINE_I.to_string());
                    y -= 1;
                    z = coord.get(&Point { x, y }).cloned();
                }
                let z = z.ok_or_else(|| broken("no parent above a node"))?;
                if z == LINE_I || z == LINE_L {
                    coord.put(Point { x, y }, LINE_T.to_string());
                }
            } else if y > 0 && left.as_deref() == Some(LINE_I) {
                coord.put(Point { x: x - 1, y }, LINE_T.to_string());
            }
        }
    }

    // TreeNetworkModel
    let mut layout = TreeLayout {
        rows: 0,
        cols: 0,
        cells: Vec::new(),
    };
    for (p, text) in coord.entries() {
        let (Ok(col), Ok(row)) = (usize::try_from(p.x), usize::try_from(p.y)) else {
            return Err(broken("negative position"));
        };
        layout.cols = layout.cols.max(col + 1);
        layout.rows = layout.rows.max(row + 1);
        layout.cells.push(TreeCell {
            row,
            col,
            kind: TreeCellKind::from_text(network, text),
        });
    }
    Ok(TreeView::Tree(layout))
}

/// `TreeNetwork.profondeur`: depth of every node below `n`.
fn depth(network: &Network, n: &Node, map: &mut JHashtable<String, i32>, prof: i32) -> Result<()> {
    if prof as usize > network.nodes.len() {
        return Err(broken("cycle below the source"));
    }
    map.put(n.name.clone(), prof);
    for p in network.pipes.iter().filter(|p| p.begin == n.name) {
        let succ =
            node(network, &p.end).ok_or_else(|| broken(&format!("unknown node {}", p.end)))?;
        depth(network, succ, map, prof + 1)?;
    }
    Ok(())
}

/// `TreeNetwork.remonteSource`: call `visit` on `n` and each predecessor up
/// to the source.
fn remonte_source(network: &Network, n: &Node, mut visit: impl FnMut(&str)) -> Result<()> {
    let mut cur = n;
    for _ in 0..=network.nodes.len() {
        visit(&cur.name);
        if cur.node_type == TYPE_RESERVOIR {
            return Ok(());
        }
        cur = first_pred(network, &cur.name)
            .ok_or_else(|| broken(&format!("node {} has no predecessor", cur.name)))?;
    }
    Err(broken("cycle above a faucet"))
}

/// `TreeNetwork.mouseClicked` and `remonteSource`: the clicked node and its
/// predecessors up to the source, which the view highlights. The window
/// reacts only to cells whose text is not empty and does not start with
/// `_`.
///
/// The Java code overflows the stack on a cycle and, for a node without
/// predecessor, goes on from the start of the last pipe. This stops at a
/// repeated node or at a node without predecessor.
pub fn path_to_source(network: &Network, node_name: &str) -> Vec<String> {
    let mut path: Vec<String> = Vec::new();
    let mut cur = node(network, node_name);
    while let Some(n) = cur {
        if path.contains(&n.name) {
            break;
        }
        path.push(n.name.clone());
        cur = if n.node_type == TYPE_RESERVOIR {
            None
        } else {
            first_pred(network, &n.name)
        };
    }
    path
}

/// `TreeNetwork.search`: position `(row, col)` of the first cell, in
/// `Hashtable` order, whose text equals `node_name` ignoring case. Line
/// cells (`_*L`, ...) match too, as in Java.
pub fn search(layout: &TreeLayout, node_name: &str) -> Option<(usize, usize)> {
    layout
        .cells
        .iter()
        .find(|c| eq_ignore_case(c.kind.text(), node_name))
        .map(|c| (c.row, c.col))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_support::fixtures::*;
    use crate::project::network::{Node, Pipe, TYPE_DISPATCH, TYPE_FAUCET, TYPE_RESERVOIR};

    /// Same text as the recording program: size, grid, Hashtable order.
    fn render(layout: &TreeLayout) -> String {
        let mut s = format!("rows {} cols {}\n", layout.rows, layout.cols);
        for r in 0..layout.rows {
            for c in 0..layout.cols {
                if c > 0 {
                    s.push('|');
                }
                if let Some(cell) = layout.cell(r, c) {
                    s.push_str(cell.kind.text());
                }
            }
            s.push('\n');
        }
        s.push_str("order\n");
        for cell in &layout.cells {
            s.push_str(&format!("{},{},{}\n", cell.row, cell.col, cell.kind.text()));
        }
        s
    }

    fn layout(name: &str, network: &Network) -> TreeLayout {
        match build_tree(name, network) {
            Ok(TreeView::Tree(l)) => l,
            other => panic!("not a tree layout: {other:?}"),
        }
    }

    #[test]
    fn layouts_match_java() {
        let cases = [
            (
                topo("El_Bihajua", EL_BIHAJUA_TPO).network,
                include_str!("testdata/tree_El_Bihajua_tpo.txt"),
            ),
            (
                topo("SampleNetwork", SAMPLE_NETWORK_TPO).network,
                include_str!("testdata/tree_SampleNetwork_tpo.txt"),
            ),
            (
                topo("baseexample", BASE_EXAMPLE_TPO).network,
                include_str!("testdata/tree_baseexample_tpo.txt"),
            ),
            (
                design("toy", TOY_DSG).network,
                include_str!("testdata/tree_toy_dsg.txt"),
            ),
            (
                design("El_Bihajua", EL_BIHAJUA_DSG).network,
                include_str!("testdata/tree_El_Bihajua_dsg.txt"),
            ),
        ];
        for (network, expected) in cases {
            assert_eq!(render(&layout("n", &network)), expected);
        }
    }

    #[test]
    fn toy_cells_and_types() {
        let l = layout("toy", &design("toy", TOY_DSG).network);
        assert_eq!(
            l.cell(0, 0).map(|c| &c.kind),
            Some(&TreeCellKind::Node {
                name: "Source".into(),
                node_type: Some(TYPE_RESERVOIR)
            })
        );
        assert_eq!(
            l.cell(2, 2).map(|c| &c.kind),
            Some(&TreeCellKind::Node {
                name: "B".into(),
                node_type: Some(TYPE_DISPATCH)
            })
        );
        assert_eq!(
            l.cell(3, 3).map(|c| &c.kind),
            Some(&TreeCellKind::Node {
                name: "P2".into(),
                node_type: Some(TYPE_FAUCET)
            })
        );
        assert_eq!(l.cell(1, 1).map(|c| &c.kind), Some(&TreeCellKind::LineI));
        assert_eq!(l.cell(1, 2).map(|c| &c.kind), Some(&TreeCellKind::LineL));
        assert_eq!(l.cell(1, 0), None);
    }

    #[test]
    fn search_ignores_case_and_follows_hashtable_order() {
        let l = layout("toy", &design("toy", TOY_DSG).network);
        assert_eq!(search(&l, "p3"), Some((1, 3)));
        assert_eq!(search(&l, "Source"), Some((0, 0)));
        assert_eq!(search(&l, "nothing"), None);
        // `_*L` cells are searched too; the first in Hashtable order wins.
        assert_eq!(search(&l, "_*l"), Some((1, 2)));
    }

    #[test]
    fn path_goes_up_to_the_source() {
        let net = design("toy", TOY_DSG).network;
        assert_eq!(path_to_source(&net, "P3"), vec!["P3", "C", "A", "Source"]);
        assert_eq!(path_to_source(&net, "Source"), vec!["Source"]);
        assert!(path_to_source(&net, "unknown").is_empty());
    }

    fn net(nodes: &[(&str, i32)], pipes: &[(&str, &str)]) -> Network {
        Network {
            nodes: nodes
                .iter()
                .map(|(n, t)| Node {
                    node_type: *t,
                    ..Node::named(n)
                })
                .collect(),
            pipes: pipes
                .iter()
                .map(|(b, e)| Pipe {
                    begin: b.to_string(),
                    end: e.to_string(),
                    ..Pipe::default()
                })
                .collect(),
            ..Network::default()
        }
    }

    #[test]
    fn not_a_tree() {
        // C has two predecessors.
        let n = net(
            &[("S", 0), ("A", 1), ("B", 1), ("C", 2)],
            &[("S", "A"), ("S", "B"), ("A", "C"), ("B", "C")],
        );
        assert!(!is_a_tree(&n));
        assert_eq!(
            build_tree("demo", &n).ok(),
            Some(TreeView::NotATree("_demo is not a tree".into()))
        );
        // The source has a predecessor.
        let n = net(&[("S", 0), ("A", 2)], &[("S", "A"), ("A", "S")]);
        assert!(!is_a_tree(&n));
        assert!(is_a_tree(&net(&[("S", 0), ("A", 2)], &[("S", "A")])));
    }

    #[test]
    fn java_exceptions_become_errors() {
        // Each node has one predecessor, but A and B form a cycle away from
        // the source: Java overflows the stack.
        let n = net(
            &[("S", 0), ("A", 1), ("B", 1), ("C", 2)],
            &[("A", "B"), ("B", "A"), ("B", "C")],
        );
        assert!(is_a_tree(&n));
        assert!(build_tree("x", &n).is_err());
        assert_eq!(path_to_source(&n, "C"), vec!["C", "B", "A"]);
    }
}
