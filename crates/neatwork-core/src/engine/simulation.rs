//! `MakeSimulation` and `RunSimulation`: run the flow computation for a set
//! of open taps and collect statistics per tap, pipe and node.

use std::collections::HashMap;

use super::defs::{CPipe, CoreNetwork};
use super::simul_flows::{self, Dat, Flows};
use crate::java::{JProperties, decimal_format, eq_ignore_case, parse_double, parse_int};
use crate::messages::Locale;
use crate::{Error, Result};

/// Kind of simulation (`simu.typesimu.value`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationType {
    /// `nbsim` runs; each tap opens with probability `simopentaps`.
    Random,
    /// One run per tap, with only that tap open.
    TapByTap,
    /// One run with the taps the user selected.
    Handmade,
}

impl SimulationType {
    pub fn parse(s: &str) -> Option<SimulationType> {
        match s {
            "random" => Some(SimulationType::Random),
            "tapbytap" => Some(SimulationType::TapByTap),
            "handmade" => Some(SimulationType::Handmade),
            _ => None,
        }
    }
}

/// Source of uniform numbers in [0, 1) for random simulations (the Java
/// program used `Math.random()`).
pub trait Uniform {
    fn next_f64(&mut self) -> f64;
}

/// SplitMix64 generator: small, seedable, reproducible.
#[derive(Clone, Debug)]
pub struct SplitMix64(pub u64);

impl Uniform for SplitMix64 {
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// Result of a simulation.
pub struct MakeSimulation {
    pub net: CoreNetwork,
    pub properties: JProperties,
    pub locale: Locale,
}

struct Params {
    outflow: f64,
    rate: f64,
    seuil: f64,
    seuil2: f64,
    coef_orif: f64,
    lbd: f64,
    alpha: f64,
    maxiter: usize,
    tolr: f64,
    tolx: f64,
}

fn num(p: &JProperties, key: &str, default: &str) -> Result<f64> {
    let v = p.get_property_or(key, default);
    parse_double(v).ok_or_else(|| Error::Parameter(format!("{key}={v}")))
}

impl MakeSimulation {
    /// `new MakeSimulation(design, dvector, ovector, properties, faucetRef,
    /// solver)`. `faucet_ref` lists the open taps of a handmade simulation.
    pub fn run(
        mut net: CoreNetwork,
        p: &JProperties,
        faucet_ref: &HashMap<String, bool>,
        rng: &mut dyn Uniform,
        locale: Locale,
        progress: &mut dyn FnMut(i32),
    ) -> Result<MakeSimulation> {
        let type_orifice = p.get_property_or("simu.typeorifice.value", "ideal");
        let type_s = p.get_property_or("simu.typesimu.value", "random");
        let nbsim_s = p.get_property_or("simu.nbsim.value", "10");
        let nb_sim = parse_int(nbsim_s)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| Error::Parameter(format!("simu.nbsim.value={nbsim_s}")))?;
        let outflow = num(p, "topo.targetflow.value", "0.12")? / 1000.0;
        let alpha = num(p, "topo.faucetcoef.value", "0.00000002")?;
        let rate = parse_double(p.get_property_or("simu.simopentaps.value", "0.4")).unwrap_or(1.0);
        let seuil = num(p, "simu.mincriticalflow.value", "0.1")?;
        let seuil2 = num(p, "simu.maxcriticalflow.value", "0.3")?;
        let coef_orif = num(p, "topo.orifcoef.value", "0.59")?;
        let lbd = net
            .pipes
            .0
            .first()
            .map(|pp| pp.p1)
            .ok_or_else(|| Error::Network("the design has no pipe".into()))?;
        num(p, "topo.limitbudget.value", "500")?;
        let params = Params {
            outflow,
            rate,
            seuil,
            seuil2,
            coef_orif,
            lbd,
            alpha,
            maxiter: 20,
            tolr: 1e-4,
            tolx: 1e-7,
        };

        if !is_tree(&net) {
            add_invers_branch(&mut net);
        }
        let commercial = type_orifice == "commercial";
        for t in &mut net.taps {
            t.orifice = if commercial { t.orif_com } else { t.orif_ideal };
            t.faucet_coef = alpha;
        }
        let mut big_f = vec![0.0; net.taps.len() + net.pipes.0.len() * 2 + 1];
        match SimulationType::parse(type_s) {
            Some(SimulationType::Random) => {
                net.pipes.initialize_simulation(nb_sim);
                net.nodes.initialize_simulation(nb_sim);
                for i in 0..nb_sim {
                    progress((i as f32 / nb_sim as f32 * 100.0).round() as i32);
                    run_simulation(
                        &mut big_f,
                        &mut net,
                        &params,
                        SimulationType::Random,
                        i,
                        rng,
                        progress,
                    )?;
                }
            }
            Some(SimulationType::TapByTap) => {
                net.pipes.initialize_simulation(net.taps.len());
                net.nodes.initialize_simulation(net.taps.len());
                run_simulation(
                    &mut big_f,
                    &mut net,
                    &params,
                    SimulationType::TapByTap,
                    0,
                    rng,
                    progress,
                )?;
            }
            Some(SimulationType::Handmade) => {
                for t in &mut net.taps {
                    t.open = faucet_ref.get(&t.name).copied().unwrap_or(false);
                }
                net.pipes.initialize_simulation(1);
                net.nodes.initialize_simulation(1);
                progress(50);
                run_simulation(
                    &mut big_f,
                    &mut net,
                    &params,
                    SimulationType::Handmade,
                    0,
                    rng,
                    progress,
                )?;
            }
            None => {}
        }
        net.pipes.calcul_speed();
        calcul_quartile(&mut net);
        calcul_stat_pressure(&mut net)?;
        Ok(MakeSimulation {
            net,
            properties: p.clone(),
            locale,
        })
    }

    fn tap_rows(&self) -> std::ops::Range<usize> {
        let np = self.net.pipes.0.len();
        np.saturating_sub(self.net.taps.len())..np
    }

    /// `getSimpleResultsSimu`: flow statistics per tap, global row first.
    pub fn simple_results(&self) -> Vec<Vec<String>> {
        let mut v = Vec::new();
        let (mut totsim, mut tots1, mut tots2, mut totfail) = (0i32, 0.0f64, 0.0f64, 0i32);
        let mut totmoy = 0.0;
        for i in self.tap_rows() {
            let p = &self.net.pipes.0[i];
            let name = display_name(&p.nodes_end, true);
            totsim += p.nbsim;
            totmoy += p.moyenne * f64::from(p.nbsim);
            let variability = (p.moyennec - p.moyenne * p.moyenne).sqrt() / p.moyenne * 100.0;
            tots1 += p.seuil;
            tots2 += p.seuil2;
            totfail += p.failure;
            v.push(vec![
                name,
                p.nbsim.to_string(),
                decimal_format("0.####", p.min),
                decimal_format("0.####", p.moyenne),
                decimal_format("0.####", p.max),
                decimal_format("0.##", variability),
                decimal_format("0.##", p.seuil / f64::from(p.nbsim) * 100.0),
                decimal_format("0.##", p.seuil2 / f64::from(p.nbsim) * 100.0),
                p.failure.to_string(),
                "-1".to_string(),
            ]);
        }
        let t = f64::from(totsim);
        let mut out = vec![vec![
            self.locale.global_average().to_string(),
            "-".to_string(),
            "-".to_string(),
            decimal_format("0.####", totmoy / t),
            "-".to_string(),
            "?".to_string(),
            decimal_format("0.##", tots1 / t * 100.0),
            decimal_format("0.##", tots2 / t * 100.0),
            totfail.to_string(),
            "-1".to_string(),
        ]];
        out.append(&mut v);
        out
    }

    /// `getQuartileSimu`.
    pub fn quartiles(&self) -> Vec<Vec<String>> {
        self.tap_rows()
            .map(|i| {
                let p = &self.net.pipes.0[i];
                vec![
                    display_name(&p.nodes_end, true),
                    p.nbsim.to_string(),
                    decimal_format("0.####", p.min),
                    decimal_format("0.####", p.quart10),
                    decimal_format("0.####", p.quart25),
                    decimal_format("0.####", p.quart50),
                    decimal_format("0.####", p.quart75),
                    decimal_format("0.####", p.quart90),
                    decimal_format("0.####", p.max),
                    "N".to_string(),
                    "N".to_string(),
                ]
            })
            .collect()
    }

    /// `getSpeedSimu`.
    pub fn speeds(&self) -> Vec<Vec<String>> {
        self.net
            .pipes
            .0
            .iter()
            .map(|p| {
                vec![
                    format!("{} -> {}", p.nodes_beg, display_name(&p.nodes_end, false)),
                    p.nbsim.to_string(),
                    decimal_format("0.##", p.speed),
                    decimal_format("0.##", p.speedmax),
                    "-1".to_string(),
                ]
            })
            .collect()
    }

    /// `getPressureSimu`.
    pub fn pressures(&self) -> Vec<Vec<String>> {
        self.net
            .nodes
            .0
            .iter()
            .map(|n| {
                vec![
                    n.name.clone(),
                    decimal_format("0.##", n.minpress),
                    decimal_format("0.##", n.averpress),
                    decimal_format("0.##", n.maxpress),
                ]
            })
            .collect()
    }

    /// `getPropertiesContent`: the `simu.*` parameters.
    pub fn properties_content(&self) -> String {
        let mut c = self.locale.simulation_properties_header().to_string();
        for name in self.properties.property_names() {
            if name.starts_with("simu.") && name.ends_with(".value") {
                let value = self.properties.get_property(&name).unwrap_or("");
                c.push_str(&format!("{},{value},N\n", &name[5..name.len() - 6]));
            }
        }
        c
    }
}

/// Tap names in the result tables: drop the part before `*` and turn the
/// `_a`, `_b` suffixes of a multi-tap node into `_1`, `_2`.
fn display_name(name: &str, strip_star: bool) -> String {
    let mut name = name.to_string();
    if strip_star && let Some(k) = name.rfind('*').filter(|k| *k > 0) {
        name = name[k + 1..].to_string();
    }
    if let Some(k) = name.rfind('_').filter(|k| *k > 0) {
        let after = &name[k + 1..];
        // charAt(0) - 'a' + 1 on a UTF-16 unit; an empty suffix throws in
        // Java, here it is kept as is.
        if let Some(c) = after.encode_utf16().next() {
            let n = i32::from(c) - i32::from(b'a') + 1;
            name = format!("{}{n}", &name[..=k]);
        }
    }
    name
}

/// `isTree`: no node has two incoming pipes.
fn is_tree(net: &CoreNetwork) -> bool {
    net.nodes.0.iter().all(|n| {
        net.pipes
            .0
            .iter()
            .filter(|p| eq_ignore_case(&n.name, &p.nodes_end))
            .count()
            <= 1
    })
}

/// `addInversBranch`: add a reverse pipe for every inner branch.
fn add_invers_branch(net: &mut CoreNetwork) {
    let source = net.nodes.0[0].name.clone();
    let mut i = 0;
    while i < net.pipes.0.len() {
        let p = net.pipes.0[i].clone();
        let nb_taps = net
            .nodes
            .0
            .iter()
            .find(|n| eq_ignore_case(&n.name, &p.nodes_end))
            .map_or(0, |n| n.nb_taps);
        if nb_taps == 0 && !net.is_tap(&p.nodes_end) && !eq_ignore_case(&p.nodes_beg, &source) {
            let mut inv = CPipe::new(&p.nodes_end, &p.nodes_beg, &p.nodes_end, 0.0);
            inv.l1 = p.l1;
            inv.l2 = p.l2;
            inv.d1 = p.d1;
            inv.d2 = p.d2;
            inv.beta1 = p.beta1;
            inv.beta2 = p.beta2;
            inv.p1 = p.p1;
            inv.p2 = p.p2;
            inv.q1 = p.q1;
            inv.q2 = p.q2;
            net.pipes.0.insert(1, inv);
            i += 1;
        }
        i += 1;
    }
}

/// Indexes of a sub-network (the Java sub-vectors hold the same objects).
#[derive(Default)]
struct Sub {
    nodes: Vec<usize>,
    pipes: Vec<usize>,
    taps: Vec<usize>,
}

/// `RunSimulation`.
fn run_simulation(
    big_f: &mut [f64],
    net: &mut CoreNetwork,
    prm: &Params,
    operation: SimulationType,
    index: usize,
    rng: &mut dyn Uniform,
    progress: &mut dyn FnMut(i32),
) -> Result<()> {
    let to_init = sort_node_names(net)?;
    let result = (|| -> Result<()> {
        match operation {
            SimulationType::Random => {
                for t in &mut net.taps {
                    t.open = false;
                }
                for t in &mut net.taps {
                    if rng.next_f64() < prm.rate {
                        t.open = true;
                    }
                }
                let taps: Vec<usize> = (0..net.taps.len()).filter(|&k| net.taps[k].open).collect();
                let sub = do_sub_reseau(net, taps);
                if !sub.taps.is_empty() {
                    let y = solve_sub(big_f, net, &sub, prm)?;
                    taps_statistic(big_f, net, &sub, prm, index, &y)?;
                }
            }
            SimulationType::TapByTap => {
                for i in 0..net.taps.len() {
                    progress((i as f32 / net.taps.len() as f32 * 100.0).round() as i32);
                    net.taps[i].open = true;
                    let sub = do_sub_reseau(net, vec![i]);
                    let y = solve_sub(big_f, net, &sub, prm)?;
                    taps_statistic(big_f, net, &sub, prm, i, &y)?;
                }
            }
            SimulationType::Handmade => {
                let taps: Vec<usize> = (0..net.taps.len()).filter(|&k| net.taps[k].open).collect();
                let sub = do_sub_reseau(net, taps);
                if !sub.taps.is_empty() {
                    let y = solve_sub(big_f, net, &sub, prm)?;
                    taps_statistic(big_f, net, &sub, prm, index, &y)?;
                }
            }
        }
        Ok(())
    })();
    revert_node_names(net, &to_init);
    result
}

/// `SortNodeNames`: rename nodes `1..n` in vector order; taps take the
/// numbers of the last nodes. Returns number -> old name.
fn sort_node_names(net: &mut CoreNetwork) -> Result<HashMap<String, String>> {
    let mut to_init = HashMap::new();
    let mut to_new = HashMap::new();
    for (i, n) in net.nodes.0.iter_mut().enumerate() {
        let num = (i + 1).to_string();
        to_new.insert(n.name.clone(), num.clone());
        to_init.insert(num.clone(), n.name.clone());
        n.name = num;
    }
    let nn = net.nodes.0.len();
    let nt = net.taps.len();
    for (i, t) in net.taps.iter_mut().enumerate() {
        t.name = (i + nn + 1 - nt).to_string();
    }
    for p in &mut net.pipes.0 {
        let get = |s: &str| {
            to_new
                .get(s)
                .cloned()
                .ok_or_else(|| Error::Network(format!("pipe refers to unknown node {s}")))
        };
        p.nodes_beg = get(&p.nodes_beg)?;
        p.nodes_end = get(&p.nodes_end)?;
        p.nodes_des = get(&p.nodes_des)?;
    }
    Ok(to_init)
}

fn revert_node_names(net: &mut CoreNetwork, to_init: &HashMap<String, String>) {
    let back = |s: &str| to_init.get(s).cloned().unwrap_or_else(|| s.to_string());
    for n in &mut net.nodes.0 {
        n.name = back(&n.name);
    }
    for t in &mut net.taps {
        t.name = back(&t.name);
    }
    for p in &mut net.pipes.0 {
        p.nodes_beg = back(&p.nodes_beg);
        p.nodes_end = back(&p.nodes_end);
        p.nodes_des = back(&p.nodes_des);
    }
}

/// `DoSubReseau`: all inner nodes and pipes, plus the open taps.
fn do_sub_reseau(net: &CoreNetwork, taps: Vec<usize>) -> Sub {
    let nn = net.nodes.0.len();
    let np = net.pipes.0.len();
    let nt = net.taps.len();
    let mut sub = Sub {
        taps,
        ..Sub::default()
    };
    sub.nodes.extend(0..nn.saturating_sub(nt));
    for i in nn.saturating_sub(nt)..nn {
        if sub.taps.contains(&(i - (nn - nt))) {
            sub.nodes.push(i);
        }
    }
    sub.pipes.extend(0..np.saturating_sub(nt));
    for i in np.saturating_sub(nt)..np {
        if sub.taps.contains(&(i - (np - nt))) {
            sub.pipes.push(i);
        }
    }
    sub
}

/// Terminal (open tap) and intermediate node names of a sub-network.
fn node_classes(net: &CoreNetwork, sub: &Sub) -> (Vec<String>, Vec<String>) {
    let terminal: Vec<String> = sub.taps.iter().map(|&k| net.taps[k].name.clone()).collect();
    let interm: Vec<String> = sub
        .nodes
        .iter()
        .map(|&k| &net.nodes.0[k])
        .filter(|n| !terminal.contains(&n.name) && n.height != 0.0)
        .map(|n| n.name.clone())
        .collect();
    (terminal, interm)
}

fn set(m: &mut [Vec<i32>], r: usize, c: usize) -> Result<()> {
    match m.get_mut(r).and_then(|row| row.get_mut(c)) {
        Some(v) => {
            *v = 1;
            Ok(())
        }
        None => Err(Error::Network(format!(
            "path matrix index ({r}, {c}) out of range: check node heights"
        ))),
    }
}

/// `getPathMatrix`: S[intermediate node][open tap] = 1 when the node is on
/// the tap's path to the source.
fn path_matrix(net: &CoreNetwork, sub: &Sub) -> Result<Vec<Vec<i32>>> {
    let rows = sub.nodes.len() as isize - sub.taps.len() as isize - 1;
    let rows = usize::try_from(rows).unwrap_or(0);
    let mut s = vec![vec![0; sub.taps.len()]; rows];
    let (terminal, interm) = node_classes(net, sub);
    let pipes: Vec<&CPipe> = sub.pipes.iter().map(|&k| &net.pipes.0[k]).collect();
    for i in (0..pipes.len()).rev() {
        let Some(t_idx) = terminal.iter().position(|t| *t == pipes[i].nodes_end) else {
            continue;
        };
        let mut prev = pipes[i].nodes_beg.clone();
        while !prev.is_empty() {
            if let Some(r) = interm.iter().position(|n| *n == prev) {
                set(&mut s, r, t_idx)?;
            }
            match (0..i).rev().find(|&j| pipes[j].nodes_end == prev) {
                Some(j) => prev = pipes[j].nodes_beg.clone(),
                None => break,
            }
        }
    }
    Ok(s)
}

/// `getPathMatrixIntermNode`: Sb[a][b] = 1 when intermediate node a is on
/// the path of intermediate node b (or a == b).
fn path_matrix_interm(net: &CoreNetwork, sub: &Sub) -> Result<Vec<Vec<i32>>> {
    let k = usize::try_from(sub.nodes.len() as isize - sub.taps.len() as isize - 1).unwrap_or(0);
    let mut sb = vec![vec![0; k]; k];
    let (_, interm) = node_classes(net, sub);
    let pipes: Vec<&CPipe> = sub.pipes.iter().map(|&k| &net.pipes.0[k]).collect();
    for i in (0..pipes.len()).rev() {
        let Some(b) = interm.iter().position(|n| *n == pipes[i].nodes_end) else {
            continue;
        };
        let mut prev = pipes[i].nodes_beg.clone();
        set(&mut sb, b, b)?;
        if let Some(a) = interm.iter().position(|n| *n == prev) {
            set(&mut sb, a, b)?;
        }
        for j in (0..i).rev() {
            if pipes[j].nodes_end == prev {
                prev = pipes[j].nodes_beg.clone();
                if let Some(a) = interm.iter().position(|n| *n == prev) {
                    set(&mut sb, a, b)?;
                }
            }
        }
    }
    Ok(sb)
}

/// `getBeta`: head-loss coefficient of every sub-network pipe, indexed by
/// its end node number.
fn get_beta(net: &CoreNetwork, sub: &Sub) -> Result<Vec<f64>> {
    let mut beta = vec![0.0; sub.pipes.len()];
    let (terminal, interm) = node_classes(net, sub);
    for &k in &sub.pipes {
        let p = &net.pipes.0[k];
        let n_end = parse_int(&p.nodes_end)
            .ok_or_else(|| Error::Network(format!("node {} is not numbered", p.nodes_end)))?;
        let mut adjusted = n_end - 2;
        for j in 2..n_end {
            let s = j.to_string();
            if !interm.contains(&s) && !terminal.contains(&s) {
                adjusted -= 1;
            }
        }
        let a = usize::try_from(adjusted)
            .ok()
            .filter(|a| *a < beta.len())
            .ok_or_else(|| Error::Network(format!("Adjusted index out of range: {adjusted}")))?;
        beta[a] = p.beta1 * p.length * (p.l1 / (p.length * p.d1.powf(p.q1)));
        if p.d2 != 0.0 {
            beta[a] += p.beta1 * p.length * (1.0 - p.l1 / p.length) / p.d2.powf(p.q1);
        }
    }
    Ok(beta)
}

/// Build the data of one flow computation and run it. Returns the node
/// heads (`y` in Java).
fn solve_sub(big_f: &mut [f64], net: &CoreNetwork, sub: &Sub, prm: &Params) -> Result<Vec<f64>> {
    let mut y = vec![0.0; sub.nodes.len()];
    let s = path_matrix(net, sub)?;
    let sb = path_matrix_interm(net, sub)?;
    if s.is_empty() {
        // S[0].length throws in Java when no intermediate node remains.
        return Err(Error::Network(
            "the network has no intermediate node".into(),
        ));
    }
    let h: Vec<f64> = sub.nodes[1..]
        .iter()
        .map(|&k| net.nodes.0[k].height)
        .collect();
    let alpha: Vec<f64> = sub
        .taps
        .iter()
        .map(|&k| 1.0 / prm.alpha + (prm.coef_orif / net.taps[k].orifice).powf(4.0))
        .collect();
    let beta = get_beta(net, sub)?;
    let pipe_end: Vec<i32> = sub
        .pipes
        .iter()
        .map(|&k| {
            parse_int(&net.pipes.0[k].nodes_end)
                .ok_or_else(|| Error::Network("node is not numbered".into()))
        })
        .collect::<Result<_>>()?;
    let dat = Dat {
        nb: s.len(),
        nt: s[0].len(),
        h,
        cible: prm.outflow,
        lbd: prm.lbd,
        s,
        nbouvert: Vec::new(),
        alpha,
        beta,
        maxiter: prm.maxiter,
        tolr: prm.tolr,
        tolx: prm.tolx,
    };
    let flows = Flows {
        pipe_end: &pipe_end,
        sb: &sb,
    };
    simul_flows::run(&mut y, big_f, sub.taps.len(), &dat, &flows, prm.rate)?;
    Ok(y)
}

/// `TapsStatistic`: store the run's node heads and pipe flows.
fn taps_statistic(
    big_f: &mut [f64],
    net: &mut CoreNetwork,
    sub: &Sub,
    prm: &Params,
    index: usize,
    y: &[f64],
) -> Result<()> {
    let mut i = 0;
    while i < sub.nodes.len() {
        let mut node = sub.nodes[i];
        let group = net.nodes.0[node].indexgrouptaps;
        if group != 0 {
            let number = sub
                .nodes
                .iter()
                .filter(|&&k| net.nodes.0[k].indexgrouptaps == group)
                .count();
            node = net.nodes.group_node(group, number)?;
            i = i + number - 1;
        }
        let yi = *y
            .get(i)
            .ok_or_else(|| Error::Network("node head index out of range".into()))?;
        let n = &mut net.nodes.0[node];
        // The Java code looks the tap up with `==` on two different String
        // objects, so it never finds it and never subtracts the tap loss.
        n.pressim[index] = -n.height - yi;
        i += 1;
    }
    let mut i = 0;
    while i < sub.pipes.len() {
        let mut pipe = sub.pipes[i];
        let group = net.pipes.0[pipe].indexgrouptaps;
        if group != 0 {
            let number = sub
                .pipes
                .iter()
                .filter(|&&k| net.pipes.0[k].indexgrouptaps == group)
                .count();
            pipe = net.pipes.group_pipe(group, number)?;
            i = i + number - 1;
        }
        let fi = 1 + i;
        if fi >= big_f.len() {
            return Err(Error::Network("flow index out of range".into()));
        }
        let p = &mut net.pipes.0[pipe];
        p.nbsim += 1;
        if big_f[fi] < 0.005 {
            big_f[fi] = 0.0;
            p.failure += 1;
        }
        let f = big_f[fi];
        p.simulation[index] = f;
        let nbsim = f64::from(p.nbsim);
        p.moyenne = (p.moyenne * (nbsim - 1.0) + f) / nbsim;
        p.moyennec = (p.moyennec * (nbsim - 1.0) + f.powf(2.0)) / nbsim;
        if f < prm.seuil {
            p.seuil += 1.0;
        }
        if f > prm.seuil2 {
            p.seuil2 += 1.0;
        }
        if f > p.max {
            p.max = f;
        }
        if p.nbsim == 1 || f < p.min {
            p.min = f;
        }
        i += 1;
    }
    Ok(())
}

/// `calculQuartile`.
fn calcul_quartile(net: &mut CoreNetwork) {
    let np = net.pipes.0.len();
    let nt = net.taps.len();
    for k in np.saturating_sub(nt)..np {
        let p = &mut net.pipes.0[k];
        let mut simu = p.simulation.clone();
        if simu.iter().any(|v| v.is_nan()) {
            bubble_sort_zeros_last(&mut simu);
        } else {
            // Same result as the Java bubble sort, in O(n log n): the bubble
            // sort ends with the non-zero values ascending and the zeros
            // last, in their first order (it never swaps two zeros).
            simu.sort_by(|a, b| {
                (*a == 0.0)
                    .cmp(&(*b == 0.0))
                    .then(a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            });
        }
        let failure = usize::try_from(p.failure).unwrap_or(0);
        let mut i = simu.len() as isize - 1;
        while i > failure as isize - 1 {
            let iu = i as usize;
            simu[iu] = simu[iu - failure];
            i -= 1;
        }
        for v in simu.iter_mut().take(failure.saturating_sub(1)) {
            *v = 0.0;
        }
        let q = |f: f64| {
            let k = (f64::from(p.nbsim) * f).floor() as usize;
            simu.get(k).copied().unwrap_or(0.0)
        };
        p.quart10 = q(0.1);
        p.quart25 = q(0.25);
        p.quart50 = q(0.5);
        p.quart75 = q(0.75);
        p.quart90 = q(0.9);
    }
}

/// The bubble sort of `calculQuartile`, kept for runs with NaN values.
fn bubble_sort_zeros_last(simu: &mut [f64]) {
    let mut sorted = false;
    while !sorted {
        sorted = true;
        for i in 0..simu.len().saturating_sub(1) {
            if (simu[i] > simu[i + 1] && simu[i + 1] != 0.0)
                || (simu[i] < simu[i + 1] && simu[i] == 0.0)
            {
                sorted = false;
                simu.swap(i, i + 1);
            }
        }
    }
}

/// `calculStatPressure`: min, mean and max head of every node but the
/// source; a run where the node was closed counts as head = -height.
fn calcul_stat_pressure(net: &mut CoreNetwork) -> Result<()> {
    for n in net.nodes.0.iter_mut().skip(1) {
        n.minpress = 10000.0;
        n.maxpress = *n
            .pressim
            .first()
            .ok_or_else(|| Error::Parameter("simu.nbsim.value must be at least 1".into()))?;
        for j in 0..n.pressim.len() {
            if n.pressim[j] == 0.0 {
                n.pressim[j] = -n.height;
            }
            n.averpress += n.pressim[j];
            if n.pressim[j] < n.minpress {
                n.minpress = n.pressim[j];
            }
            if n.pressim[j] > n.maxpress {
                n.maxpress = n.pressim[j];
            }
        }
        n.averpress /= n.pressim.len() as f64;
    }
    Ok(())
}

#[cfg(test)]
mod sort_tests {
    use super::*;

    #[test]
    fn fast_sort_equals_bubble_sort() {
        let mut rng = SplitMix64(9);
        for n in [0, 1, 2, 5, 50, 400] {
            let v: Vec<f64> = (0..n)
                .map(|_| {
                    let x = rng.next_f64();
                    if x < 0.3 {
                        0.0
                    } else if x < 0.4 {
                        -0.0
                    } else {
                        (x * 10.0).floor() / 10.0
                    }
                })
                .collect();
            let mut a = v.clone();
            bubble_sort_zeros_last(&mut a);
            let mut b = v.clone();
            b.sort_by(|x, y| {
                (*x == 0.0)
                    .cmp(&(*y == 0.0))
                    .then(x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
            });
            assert_eq!(
                a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
        }
    }
}
