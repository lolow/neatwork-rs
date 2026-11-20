//! Drawing of a network (new in the Rust version): positions of the nodes
//! in three layouts, and colors.

use std::collections::{HashMap, HashSet};

use slint::Color;

/// A node to draw.
#[derive(Clone, Debug)]
pub struct NetNode {
    pub name: String,
    /// 0 tank, 1 branching, 2 faucet.
    pub node_type: i32,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    /// Value used by a numeric coloring (flow, pressure).
    pub value: Option<f64>,
}

/// A pipe to draw.
#[derive(Clone, Debug)]
pub struct NetPipe {
    pub begin: String,
    pub end: String,
    pub length: f64,
    /// Main diameter (design only).
    pub diameter: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Tree,
    Profile,
    Coordinates,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Coloring {
    NodeType,
    Height,
    Diameter,
    /// `value` of the faucets against two critical flows.
    Flow {
        low: f64,
        high: f64,
    },
    /// `value` of every node on a color scale.
    Pressure,
}

/// Positions in world units and the colors.
pub struct Picture {
    pub width: f64,
    pub height: f64,
    /// (x, y, color, radius) per node, in the input order.
    pub nodes: Vec<(f64, f64, Color, f64)>,
    /// (x1, y1, x2, y2, color, width) per pipe, in the input order.
    pub edges: Vec<(f64, f64, f64, f64, Color, f64)>,
    /// Legend: color and label key or text.
    pub legend: Vec<(Color, String)>,
    /// True when the coordinates layout fell back to the tree layout.
    pub no_coordinates: bool,
}

pub fn type_color(t: i32) -> Color {
    match t {
        0 => Color::from_rgb_u8(35, 209, 47),
        1 => Color::from_rgb_u8(200, 200, 200),
        _ => Color::from_rgb_u8(47, 177, 209),
    }
}

/// Color scale from blue (0) through green to red (1).
fn scale_color(t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let stops = [
        (0.0, (49, 99, 206)),
        (0.5, (64, 176, 64)),
        (1.0, (224, 64, 64)),
    ];
    let (a, b) = if t <= 0.5 {
        (stops[0], stops[1])
    } else {
        (stops[1], stops[2])
    };
    let u = (t - a.0) / (b.0 - a.0);
    let mix = |p: u8, q: u8| (f64::from(p) + (f64::from(q) - f64::from(p)) * u).round() as u8;
    Color::from_rgb_u8(mix(a.1.0, b.1.0), mix(a.1.1, b.1.1), mix(a.1.2, b.1.2))
}

/// Distinct colors for the diameters.
const PALETTE: [(u8, u8, u8); 12] = [
    (31, 119, 180),
    (255, 127, 14),
    (44, 160, 44),
    (214, 39, 40),
    (148, 103, 189),
    (140, 86, 75),
    (227, 119, 194),
    (127, 127, 127),
    (188, 189, 34),
    (23, 190, 207),
    (57, 59, 121),
    (173, 73, 74),
];

/// Children of each node, in pipe order, and the source.
fn tree<'a>(
    nodes: &[NetNode],
    pipes: &'a [NetPipe],
) -> (HashMap<&'a str, Vec<&'a str>>, Option<usize>) {
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for p in pipes {
        children
            .entry(p.begin.as_str())
            .or_default()
            .push(p.end.as_str());
    }
    let source = nodes.iter().position(|n| n.node_type == 0);
    (children, source)
}

/// Names of the nodes on the path from `name` to the source.
pub fn path_to_source(pipes: &[NetPipe], name: &str) -> HashSet<String> {
    let pred: HashMap<&str, &str> = pipes
        .iter()
        .map(|p| (p.end.as_str(), p.begin.as_str()))
        .collect();
    let mut out = HashSet::new();
    let mut cur = name;
    while out.insert(cur.to_string()) {
        match pred.get(cur) {
            Some(p) => cur = p,
            None => break,
        }
    }
    out
}

/// Compute the picture of a network.
pub fn draw(nodes: &[NetNode], pipes: &[NetPipe], layout: Layout, coloring: Coloring) -> Picture {
    let index: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();
    let (children, source) = tree(nodes, pipes);
    let mut pos: Vec<Option<(f64, f64)>> = vec![None; nodes.len()];
    let mut no_coordinates = false;

    let tree_layout = |pos: &mut Vec<Option<(f64, f64)>>| {
        // depth-first: leaves take the next row, a parent the middle of
        // its children
        let mut next_row = 0.0;
        fn walk(
            n: &str,
            depth: f64,
            children: &HashMap<&str, Vec<&str>>,
            index: &HashMap<&str, usize>,
            pos: &mut Vec<Option<(f64, f64)>>,
            next_row: &mut f64,
            seen: &mut HashSet<String>,
        ) -> f64 {
            if !seen.insert(n.to_string()) {
                return *next_row;
            }
            let kids = children.get(n).cloned().unwrap_or_default();
            let y = if kids.is_empty() {
                let y = *next_row;
                *next_row += 1.0;
                y
            } else {
                let ys: Vec<f64> = kids
                    .iter()
                    .map(|k| walk(k, depth + 1.0, children, index, pos, next_row, seen))
                    .collect();
                (ys[0] + ys[ys.len() - 1]) / 2.0
            };
            if let Some(&i) = index.get(n) {
                pos[i] = Some((depth * 130.0, y * 34.0));
            }
            y
        }
        let mut seen = HashSet::new();
        if let Some(s) = source {
            walk(
                &nodes[s].name,
                0.0,
                &children,
                &index,
                pos,
                &mut next_row,
                &mut seen,
            );
        }
        // nodes not reached from the source go below
        for p in pos.iter_mut() {
            if p.is_none() {
                *p = Some((0.0, next_row * 34.0));
                next_row += 1.0;
            }
        }
    };

    match layout {
        Layout::Tree => tree_layout(&mut pos),
        Layout::Profile => {
            // x: distance from the source along the pipes, y: height
            let mut dist: Vec<Option<f64>> = vec![None; nodes.len()];
            if let Some(s) = source {
                dist[s] = Some(0.0);
                let mut stack = vec![nodes[s].name.as_str()];
                while let Some(n) = stack.pop() {
                    let d = index.get(n).and_then(|&i| dist[i]).unwrap_or(0.0);
                    for p in pipes.iter().filter(|p| p.begin == n) {
                        if let Some(&j) = index.get(p.end.as_str())
                            && dist[j].is_none()
                        {
                            dist[j] = Some(d + p.length.max(0.0));
                            stack.push(p.end.as_str());
                        }
                    }
                }
            }
            let maxd = dist.iter().flatten().copied().fold(0.0, f64::max).max(1.0);
            let (minh, maxh) = nodes
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), n| {
                    (a.min(n.height), b.max(n.height))
                });
            let span = (maxh - minh).max(1.0);
            for (i, n) in nodes.iter().enumerate() {
                let x = dist[i].unwrap_or(0.0) / maxd * 1200.0;
                let y = (maxh - n.height) / span * 600.0;
                pos[i] = Some((x, y));
            }
        }
        Layout::Coordinates => {
            let has = nodes.iter().any(|n| n.x != 0.0 || n.y != 0.0);
            if has {
                let (minx, maxx, miny, maxy) = nodes.iter().fold(
                    (
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                    ),
                    |(a, b, c, d), n| (a.min(n.x), b.max(n.x), c.min(n.y), d.max(n.y)),
                );
                let span = (maxx - minx).max(maxy - miny).max(1e-9);
                for (i, n) in nodes.iter().enumerate() {
                    // y grows upward on a map
                    pos[i] = Some(((n.x - minx) / span * 1000.0, (maxy - n.y) / span * 1000.0));
                }
            } else {
                no_coordinates = true;
                tree_layout(&mut pos);
            }
        }
    }
    let pts: Vec<(f64, f64)> = pos.into_iter().map(|p| p.unwrap_or((0.0, 0.0))).collect();
    let width = pts.iter().map(|p| p.0).fold(0.0, f64::max).max(10.0);
    let height = pts.iter().map(|p| p.1).fold(0.0, f64::max).max(10.0);

    // colors
    let mut legend: Vec<(Color, String)> = Vec::new();
    let (minh, maxh) = nodes
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), n| {
            (a.min(n.height), b.max(n.height))
        });
    let mut diameters: Vec<f64> = pipes.iter().filter_map(|p| p.diameter).collect();
    diameters.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    diameters.dedup();
    let (minv, maxv) = nodes
        .iter()
        .filter_map(|n| n.value)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(v), b.max(v))
        });
    let node_color = |n: &NetNode| -> Color {
        match coloring {
            Coloring::Height => scale_color(if maxh > minh {
                (n.height - minh) / (maxh - minh)
            } else {
                0.5
            }),
            Coloring::Flow { low, high } => match (n.node_type, n.value) {
                (2, Some(v)) if v < low => Color::from_rgb_u8(224, 64, 64),
                (2, Some(v)) if v > high => Color::from_rgb_u8(49, 99, 206),
                (2, Some(_)) => Color::from_rgb_u8(64, 176, 64),
                (t, _) => type_color(t),
            },
            Coloring::Pressure => match n.value {
                Some(v) if maxv > minv => scale_color((v - minv) / (maxv - minv)),
                Some(_) => scale_color(0.5),
                None => type_color(n.node_type),
            },
            _ => type_color(n.node_type),
        }
    };
    match coloring {
        Coloring::NodeType | Coloring::Diameter => {
            legend.push((type_color(0), "Node.TANK".into()));
            legend.push((type_color(1), "Node.BRANCHING_NODE".into()));
            legend.push((type_color(2), "Node.FAUCET_NODE".into()));
        }
        Coloring::Height => {
            legend.push((scale_color(0.0), format!("{minh:.1} m")));
            legend.push((scale_color(1.0), format!("{maxh:.1} m")));
        }
        Coloring::Flow { .. } => {
            legend.push((Color::from_rgb_u8(224, 64, 64), "Gui.Low".into()));
            legend.push((Color::from_rgb_u8(64, 176, 64), "Gui.Ok_flow".into()));
            legend.push((Color::from_rgb_u8(49, 99, 206), "Gui.High".into()));
        }
        Coloring::Pressure => {
            if minv.is_finite() {
                legend.push((scale_color(0.0), format!("{minv:.2} m")));
                legend.push((scale_color(1.0), format!("{maxv:.2} m")));
            }
        }
    }
    if coloring == Coloring::Diameter {
        for (k, d) in diameters.iter().enumerate() {
            let c = PALETTE[k % PALETTE.len()];
            legend.push((
                Color::from_rgb_u8(c.0, c.1, c.2),
                format!("{:.1} mm", d * 1000.0),
            ));
        }
    }
    let node_out = nodes
        .iter()
        .zip(&pts)
        .map(|(n, p)| {
            (
                p.0,
                p.1,
                node_color(n),
                if n.node_type == 0 { 9.0 } else { 6.0 },
            )
        })
        .collect();
    let edges = pipes
        .iter()
        .map(|p| {
            let a = index.get(p.begin.as_str()).map_or((0.0, 0.0), |&i| pts[i]);
            let b = index.get(p.end.as_str()).map_or((0.0, 0.0), |&i| pts[i]);
            let (color, w) = match (coloring, p.diameter) {
                (Coloring::Diameter, Some(d)) => {
                    let k = diameters.iter().position(|x| *x == d).unwrap_or(0);
                    let c = PALETTE[k % PALETTE.len()];
                    let rank = (k + 1) as f64 / diameters.len().max(1) as f64;
                    (Color::from_rgb_u8(c.0, c.1, c.2), 1.5 + 4.0 * rank)
                }
                _ => (Color::from_rgb_u8(90, 90, 90), 2.0),
            };
            (a.0, a.1, b.0, b.1, color, w)
        })
        .collect();
    Picture {
        width,
        height,
        nodes: node_out,
        edges,
        legend,
        no_coordinates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net() -> (Vec<NetNode>, Vec<NetPipe>) {
        let n = |name: &str, t: i32, h: f64| NetNode {
            name: name.into(),
            node_type: t,
            height: h,
            x: 0.0,
            y: 0.0,
            value: None,
        };
        let p = |a: &str, b: &str, l: f64| NetPipe {
            begin: a.into(),
            end: b.into(),
            length: l,
            diameter: None,
        };
        (
            vec![
                n("S", 0, 0.0),
                n("A", 1, -10.0),
                n("P1", 2, -12.0),
                n("P2", 2, -20.0),
            ],
            vec![p("S", "A", 20.0), p("A", "P1", 50.0), p("A", "P2", 70.0)],
        )
    }

    #[test]
    fn tree_layout_puts_parent_between_children() {
        let (n, p) = net();
        let pic = draw(&n, &p, Layout::Tree, Coloring::NodeType);
        assert_eq!(pic.nodes[0].0, 0.0);
        assert_eq!(pic.nodes[1].1, (pic.nodes[2].1 + pic.nodes[3].1) / 2.0);
    }

    #[test]
    fn profile_uses_distance_and_height() {
        let (n, p) = net();
        let pic = draw(&n, &p, Layout::Profile, Coloring::Height);
        assert!(pic.nodes[3].0 > pic.nodes[2].0);
        assert!(pic.nodes[3].1 > pic.nodes[2].1);
        let pic = draw(&n, &p, Layout::Coordinates, Coloring::NodeType);
        assert!(pic.no_coordinates);
    }

    #[test]
    fn path() {
        let (_, p) = net();
        let s = path_to_source(&p, "P2");
        assert!(s.contains("A") && s.contains("S") && !s.contains("P1"));
    }
}
