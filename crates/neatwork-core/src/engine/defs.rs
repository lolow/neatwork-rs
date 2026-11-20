//! `neatwork.core.defs`: the engine's own network model.
//!
//! The Java vectors share objects: a sub-network holds references to the
//! nodes, pipes and taps of the full network. Here the full network owns the
//! values and sub-networks hold indexes.

use crate::java::{eq_ignore_case, parse_double, parse_int};
use crate::{Error, Result};

pub const MAXDIAM: f64 = 10000.0;

/// `Nodes`.
#[derive(Clone, Debug, Default)]
pub struct CNode {
    pub name: String,
    pub height: f64,
    /// Node names from this node up to the source (`Path`).
    pub path: Vec<String>,
    pub nb_taps: i32,
    /// 1 when the node is a tap.
    pub taps: i32,
    pub pseudo_taps: i32,
    pub pressure: f64,
    pub suction: f64,
    pub pressim: Vec<f64>,
    pub averpress: f64,
    pub minpress: f64,
    pub maxpress: f64,
    pub indexgrouptaps: i32,
}

impl CNode {
    pub fn new(name: &str, height: f64, pseudo_taps: i32) -> Self {
        CNode {
            name: name.to_string(),
            height,
            pseudo_taps,
            ..CNode::default()
        }
    }
}

/// `Pipes`.
#[derive(Clone, Debug)]
pub struct CPipe {
    pub nodes_beg: String,
    pub nodes_end: String,
    pub nodes_des: String,
    pub length: f64,
    pub indexgrouptaps: i32,
    pub d1: f64,
    pub d2: f64,
    pub l1: f64,
    pub l2: f64,
    pub p1: f64,
    pub p2: f64,
    pub q1: f64,
    pub q2: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub imposdiammin: f64,
    pub imposdiammax: f64,
    pub imposdiam1: f64,
    pub imposdiam2: f64,
    pub imposlength1: f64,
    pub moyenne: f64,
    pub moyennec: f64,
    pub min: f64,
    pub max: f64,
    pub seuil: f64,
    pub seuil2: f64,
    pub nbsim: i32,
    pub failure: i32,
    pub speed: f64,
    pub speedmin: f64,
    pub speedmax: f64,
    pub simulation: Vec<f64>,
    pub quart10: f64,
    pub quart25: f64,
    pub quart50: f64,
    pub quart75: f64,
    pub quart90: f64,
    pub ref_diam1: String,
    pub ref_diam2: String,
}

impl CPipe {
    pub fn new(beg: &str, end: &str, des: &str, length: f64) -> Self {
        CPipe {
            nodes_beg: beg.to_string(),
            nodes_end: end.to_string(),
            nodes_des: des.to_string(),
            length,
            indexgrouptaps: 0,
            d1: 0.0,
            d2: 1.0,
            l1: 0.0,
            l2: 0.0,
            p1: 1.0,
            p2: 1.0,
            q1: 1.0,
            q2: 1.0,
            beta1: 1.0,
            beta2: 1.0,
            imposdiammin: 0.0,
            imposdiammax: 10000.0,
            imposdiam1: 0.0,
            imposdiam2: 0.0,
            imposlength1: 0.0,
            moyenne: 0.0,
            moyennec: 0.0,
            min: 0.0,
            max: 0.0,
            seuil: 0.0,
            seuil2: 0.0,
            nbsim: 0,
            failure: 0,
            speed: 0.0,
            speedmin: 0.0,
            speedmax: 0.0,
            simulation: Vec::new(),
            quart10: 0.0,
            quart25: 0.0,
            quart50: 0.0,
            quart75: 0.0,
            quart90: 0.0,
            ref_diam1: "0".to_string(),
            ref_diam2: "0".to_string(),
        }
    }
}

/// `Taps`.
#[derive(Clone, Debug)]
pub struct CTap {
    pub name: String,
    pub faucet_coef: f64,
    pub open: bool,
    pub orifice: f64,
    pub orif_com: f64,
    pub orif_ideal: f64,
}

impl CTap {
    pub fn new(name: &str) -> Self {
        CTap {
            name: name.to_string(),
            faucet_coef: 0.0,
            open: false,
            orifice: MAXDIAM,
            orif_com: MAXDIAM,
            orif_ideal: MAXDIAM,
        }
    }
}

/// `Diameters` (engine view of a commercial pipe).
#[derive(Clone, Debug)]
pub struct CDiameter {
    pub nominal: String,
    pub diam: f64,
    pub sdr: f64,
    pub pipe_type: i32,
    pub rugosite: f64,
    /// Maximum head the pipe stands (the catalog's "max length" column).
    pub pression: f64,
    pub cost: f64,
    pub p: f64,
    pub q: f64,
    pub beta: f64,
}

/// Node list with the Java lookup helpers (`NodesVector`).
#[derive(Clone, Debug, Default)]
pub struct Nodes(pub Vec<CNode>);

impl Nodes {
    /// `addNodes(ID, height, pseudoTaps)`: no-op when the name exists
    /// (ignoring case).
    pub fn add(&mut self, id: &str, height: f64, pseudo_taps: i32) {
        if !self.0.iter().any(|n| eq_ignore_case(&n.name, id)) {
            self.0.push(CNode::new(id, height, pseudo_taps));
        }
    }

    /// `getPosition`.
    pub fn position(&self, name: &str) -> Result<usize> {
        self.0
            .iter()
            .position(|n| eq_ignore_case(&n.name, name))
            .ok_or_else(|| Error::Network(format!("unknown node {name}")))
    }

    /// `GetNodes(indexgrouptaps, number)`.
    pub fn group_node(&self, group: i32, number: usize) -> Result<usize> {
        let start = self
            .0
            .iter()
            .position(|n| n.indexgrouptaps == group)
            .ok_or_else(|| Error::Network(format!("unknown tap group {group}")))?;
        let i = start + number - 1;
        if i < self.0.len() {
            Ok(i)
        } else {
            Err(Error::Network(format!("tap group {group} out of range")))
        }
    }

    pub fn initialize_simulation(&mut self, nb_sim: usize) {
        for n in &mut self.0 {
            n.pressim = vec![0.0; nb_sim];
        }
    }
}

/// Pipe list with the Java lookup helpers (`PipesVector`).
#[derive(Clone, Debug, Default)]
pub struct Pipes(pub Vec<CPipe>);

impl Pipes {
    pub fn add(&mut self, beg: &str, end: &str, length: f64) {
        self.0.push(CPipe::new(beg, end, beg, length));
    }

    /// `GetPred`: begin of the first pipe that ends at `n`.
    pub fn pred(&self, n: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|p| eq_ignore_case(&p.nodes_end, n))
            .map(|p| p.nodes_beg.as_str())
    }

    /// `getPosition(n_end)`.
    pub fn position(&self, n_end: &str) -> Result<usize> {
        self.0
            .iter()
            .position(|p| eq_ignore_case(&p.nodes_end, n_end))
            .ok_or_else(|| Error::Network(format!("no pipe ends at {n_end}")))
    }

    /// `GetPipes(indexgrouptaps, number)`.
    pub fn group_pipe(&self, group: i32, number: usize) -> Result<usize> {
        let start = self
            .0
            .iter()
            .position(|p| p.indexgrouptaps == group)
            .ok_or_else(|| Error::Network(format!("unknown tap group {group}")))?;
        let i = start + number - 1;
        if i < self.0.len() {
            Ok(i)
        } else {
            Err(Error::Network(format!("tap group {group} out of range")))
        }
    }

    pub fn initialize_simulation(&mut self, nb_sim: usize) {
        for p in &mut self.0 {
            p.simulation = vec![0.0; nb_sim];
            p.moyenne = 0.0;
            p.moyennec = 0.0;
            p.max = 0.0;
            p.min = 0.0;
            p.nbsim = 0;
            p.seuil = 0.0;
            p.speed = 0.0;
            p.speedmax = 0.0;
            p.speedmin = 0.0;
        }
    }

    /// `CalculSpeed`.
    #[expect(clippy::approx_constant, reason = "the Java code divides by 3.14")]
    pub fn calcul_speed(&mut self) {
        for p in &mut self.0 {
            let area = p.d1.powf(2.0);
            p.speed = p.moyenne / 1000.0 * 4.0 / 3.14 / area;
            p.speedmin = p.min / 1000.0 * 4.0 / 3.14 / area;
            p.speedmax = p.max / 1000.0 * 4.0 / 3.14 / area;
        }
    }
}

/// `getPath`: add `n` and its predecessors up to the source to the path of
/// node `owner`; when the owner is a tap, count it at every node passed.
pub fn compute_path(pipes: &Pipes, nodes: &mut Nodes, owner: usize) -> Result<()> {
    let mut n = Some(nodes.0[owner].name.clone());
    let taps = nodes.0[owner].taps;
    while let Some(name) = n {
        nodes.0[owner].path.push(name.clone());
        if taps == 1 {
            let i = nodes.position(&name)?;
            nodes.0[i].nb_taps += 1;
        }
        n = pipes.pred(&name).map(str::to_string);
    }
    Ok(())
}

/// The three vectors of a core network.
#[derive(Clone, Debug, Default)]
pub struct CoreNetwork {
    pub nodes: Nodes,
    pub pipes: Pipes,
    pub taps: Vec<CTap>,
}

impl CoreNetwork {
    /// `TapsVector.isTaps(name)`.
    pub fn is_tap(&self, name: &str) -> bool {
        self.taps.iter().any(|t| t.name == name)
    }
}

/// `CoreTopographie`: rows `node, pred, height, length, #taps`, source
/// first.
pub fn core_topographie(data: &[Vec<String>]) -> Result<CoreNetwork> {
    let mut net = CoreNetwork::default();
    let first = data
        .first()
        .ok_or_else(|| Error::Network("empty topography".into()))?;
    net.nodes.add(&first[0], 0.0, 0);
    let mut index = 0usize;
    for v in &data[1..] {
        let nod = v[0].as_str();
        let pred = v[1].as_str();
        let height = parse_double(&v[2]).unwrap_or(0.0);
        let length = parse_double(&v[3]).unwrap_or(0.0);
        let tap = parse_int(&v[4]).unwrap_or(0);
        if tap < 0 {
            return Err(Error::Network(
                "fichier topo non valide (number of taps <0)".into(),
            ));
        }
        if tap == 0 {
            net.nodes.add(nod, height, tap);
            net.pipes.add(pred, nod, length);
            index += 1;
        }
        if tap == 1 {
            net.nodes.add(nod, height, tap);
            net.pipes.add(pred, nod, length);
        }
        if tap > 1 {
            net.nodes.0.insert(index + 1, CNode::new(nod, height, tap));
            net.pipes
                .0
                .insert(index, CPipe::new(pred, nod, pred, length));
            index += 1;
        }
        if tap == 1 {
            let i = find_from_one(&net.nodes, nod)?;
            net.nodes.0[i].taps = 1;
            net.taps.push(CTap::new(nod));
        } else if tap > 1 {
            for i in 0..tap {
                let name = format!("{nod} - {i}");
                net.nodes.add(&name, height, 1);
                net.pipes.add(nod, &name, 1.0);
                let j = find_from_one(&net.nodes, &name)?;
                net.nodes.0[j].taps = 1;
                net.taps.push(CTap::new(&name));
            }
        }
    }
    Ok(net)
}

/// `do { i++; } while (!nodes[i].equalsIgnoreCase(name))`.
fn find_from_one(nodes: &Nodes, name: &str) -> Result<usize> {
    nodes
        .0
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, n)| eq_ignore_case(&n.name, name))
        .map(|(i, _)| i)
        .ok_or_else(|| Error::Network(format!("unknown node {name}")))
}

/// `DiametersVector.getDiameters(d)`: first diameter within 1e-4 of `d`.
pub fn find_diameter(dvector: &[CDiameter], d: f64) -> Option<&CDiameter> {
    let mut i = 0;
    let mut diam = dvector.first()?;
    while i < dvector.len() - 1 && (diam.diam - d).abs() > 0.0001 {
        i += 1;
        diam = &dvector[i];
    }
    if (diam.diam - d).abs() > 0.0001 {
        None
    } else {
        Some(diam)
    }
}

/// `CoreDesign`: rows `node, pred, height, length, #taps, l1, l2, d1, d2,
/// ideal orifice, commercial orifice`, source first.
pub fn core_design(data: &[Vec<String>], dvector: &[CDiameter]) -> Result<CoreNetwork> {
    let mut net = CoreNetwork::default();
    let first = data
        .first()
        .ok_or_else(|| Error::Network("empty design".into()))?;
    net.nodes.add(&first[0], 0.0, 0);
    let mut prefix_group = " ".to_string();
    let mut index_group = 1;
    for v in &data[1..] {
        let num = |k: usize| v.get(k).and_then(|s| parse_double(s)).unwrap_or(0.0);
        let nod = v[0].as_str();
        let pred = v[1].as_str();
        let height = num(2);
        let length = num(3);
        let tap = v.get(4).and_then(|s| parse_int(s)).unwrap_or(0);
        let (lp1, lp2, dp1, dp2, ori, ori2) = (num(5), num(6), num(7), num(8), num(9), num(10));
        net.nodes.add(nod, height, tap);
        net.pipes.add(pred, nod, length);
        let missing = |d: f64| Error::Network(format!("diameter {d} is not in the diameter list"));
        let diam1 = find_diameter(dvector, dp1)
            .ok_or_else(|| missing(dp1))?
            .clone();
        let diam2 = if lp2 != 0.0 {
            Some(
                find_diameter(dvector, dp2)
                    .ok_or_else(|| missing(dp2))?
                    .clone(),
            )
        } else {
            None
        };
        let last = net.pipes.0.len() - 1;
        {
            let p = &mut net.pipes.0[last];
            p.l1 = lp1;
            p.l2 = lp2;
            p.d1 = dp1;
            p.d2 = dp2;
            p.p1 = diam1.p;
            p.q1 = diam1.q;
            // The Java code fixes beta1 here instead of using diam1.beta.
            p.beta1 = 0.00089396;
            if let Some(d2) = &diam2 {
                p.p2 = d2.p;
                p.q2 = d2.q;
                p.beta2 = d2.beta;
            }
        }
        if tap == 1 {
            let end = net.pipes.0[last].nodes_end.clone();
            let tokens: Vec<&str> = end.split('_').filter(|t| !t.is_empty()).collect();
            if tokens.len() > 1 {
                let prefix = tokens[0].to_string();
                if prefix_group != prefix {
                    index_group += 1;
                }
                net.pipes.0[last].indexgrouptaps = index_group;
                let ln = net.nodes.0.len() - 1;
                net.nodes.0[ln].indexgrouptaps = index_group;
                prefix_group = prefix;
            }
            let i = find_from_one(&net.nodes, nod)?;
            net.nodes.0[i].taps = 1;
            let mut t = CTap::new(nod);
            t.orif_ideal = ori;
            t.orif_com = ori2;
            t.orifice = ori;
            net.taps.push(t);
        }
    }
    Ok(net)
}

/// `t-a.properties`: water viscosity coefficient by temperature (F).
const TA_TABLE: &str = include_str!("../../resources/t-a.properties");

/// `DiametersVector.calculA`.
fn calcul_a(temperature: f64) -> f64 {
    let (mut t1, mut t2, mut a1, mut a2) = (0.0, 0.0, 0.0, 0.0);
    for line in TA_TABLE.lines() {
        if t2 >= temperature {
            break;
        }
        let Some((ts, a_s)) = line.split_once(',') else {
            continue;
        };
        if let (Some(t), Some(a)) = (parse_double(ts), parse_double(a_s)) {
            t2 = t;
            a2 = a;
        }
        if t2 < temperature {
            t1 = t2;
            a1 = a2;
        }
    }
    if t2 > temperature {
        a1 + (a2 - a1) * (temperature - t1) / (t2 - t1)
    } else {
        a2
    }
}

/// `new DiametersVector(data, properties)`: rows `nominal, SDR, diameter,
/// cost, max length, type, roughness`.
///
/// For iron pipes (type 2) the Java code computes the friction coefficients
/// into local copies that it never reads back, so A = B = 0: p = 2, q = 5,
/// beta = 0. This port keeps that result.
pub fn diameters_vector(data: &[Vec<String>], water_temp: f64) -> Result<Vec<CDiameter>> {
    let temperature = 9.0 * water_temp / 5.0 + 32.0;
    let a = calcul_a(temperature);
    let mut out = Vec::with_capacity(data.len());
    for line in data {
        let bad = || Error::Network(format!("bad diameter line {}", line.join(",")));
        let f = |k: usize| line.get(k).and_then(|s| parse_double(s)).ok_or_else(bad);
        let sdr = f(1)?;
        let diameter = f(2)?;
        let cost = f(3)?;
        let length = f(4)?;
        let pipe_type = line.get(5).and_then(|s| parse_int(s)).ok_or_else(bad)?;
        let rugosite = f(6)?;
        let (p, q, beta) = if pipe_type == 1 {
            (
                2.0 - 0.219,
                5.0 - 0.219,
                0.0826 * 0.235 * (a * 1e6).powf(-0.219),
            )
        } else {
            (2.0 - 0.0, 5.0 - 0.0, 0.0826 * 0.0 * (a * 1e6).powf(-0.0))
        };
        out.push(CDiameter {
            nominal: line[0].clone(),
            diam: diameter,
            sdr,
            pipe_type,
            rugosite,
            pression: length,
            cost,
            p,
            q,
            beta,
        });
    }
    Ok(out)
}

/// `new OrificesVector(data)`.
pub fn orifices_vector(data: &[Vec<String>]) -> Result<Vec<f64>> {
    data.iter()
        .map(|l| {
            l.first()
                .and_then(|s| parse_double(s))
                .ok_or_else(|| Error::Network(format!("bad orifice line {}", l.join(","))))
        })
        .collect()
}
