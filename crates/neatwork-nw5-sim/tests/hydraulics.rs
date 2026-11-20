//! Independent checks of the simulation that do not use the python
//! references: hand-solvable networks, flow conservation, head-loss law and
//! KKT conditions on every scenario, failure classification.

use std::path::PathBuf;

use neatwork_nw5::read::read_file;
use neatwork_nw5::{Arc, ArcState, Client, Network, Node, NodeStatus, Pipe, Terminal};
use neatwork_nw5_sim::{
    FailureStage, PipeKind, ScenarioStatus, SimError, SimulationOptions, SimulationResult,
    simulate, simulate_with_progress, statistics_json,
};

const BESCAL: f64 = 8.9396e-4;
const NU: f64 = 4.781;

fn beta(length: f64, d: f64) -> f64 {
    BESCAL * length / d.powf(NU)
}

/// Root of an increasing function on [a, b] by bisection.
fn bisect(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    for _ in 0..300 {
        let m = 0.5 * (a + b);
        if f(m) > 0.0 { b = m } else { a = m }
    }
    0.5 * (a + b)
}

/// Flow `q >= 0` with `k q^lambda + alpha q^2 = h` (0 for `h <= 0`).
fn flow_for_head(k: f64, alpha: f64, lambda: f64, h: f64) -> f64 {
    if h <= 0.0 {
        return 0.0;
    }
    bisect(|q| k * q.powf(lambda) + alpha * q * q - h, 0.0, 1.0)
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

fn options(n: usize) -> SimulationOptions {
    SimulationOptions::new(n, 7)
}

fn run(net: &Network, o: &SimulationOptions) -> Result<SimulationResult, String> {
    simulate(net, o).map_err(|e| e.to_string())
}

fn pipe(r: &SimulationResult, from: &str, to: &str) -> usize {
    r.pipes
        .iter()
        .position(|p| p.from == from && p.to == to)
        .unwrap_or(usize::MAX)
}

fn transit(r: &SimulationResult, id: &str) -> usize {
    r.transit_nodes
        .iter()
        .position(|n| n.id == id)
        .unwrap_or(usize::MAX)
}

/// NW4 terminal loss coefficient of one open client.
fn nw4_alpha(orifcoef: f64, orifice: f64, faucetcoef: f64) -> f64 {
    (orifcoef / orifice).powf(4.0) + 1.0 / faucetcoef
}

/// Builder of small networks for the hand-solvable tests.
struct NetBuilder {
    net: Network,
}

impl NetBuilder {
    fn new() -> NetBuilder {
        let mut net = Network {
            name: "test".into(),
            ..Network::default()
        };
        net.pipes.push(Pipe {
            id: "D1".into(),
            nominal: "x".into(),
            sdr: "17".into(),
            diameter: 0.05,
            cost: 1.0,
            max_pressure: 100.0,
            kind: "1".into(),
            roughness: 0.0,
        });
        NetBuilder { net }
    }

    fn param(mut self, k: &str, v: f64) -> Self {
        self.net.params.set(k, &format!("{v:e}"));
        self
    }

    fn diameter(mut self, id: &str, d: f64) -> Self {
        self.net.pipes.push(Pipe {
            id: id.into(),
            nominal: id.into(),
            sdr: "17".into(),
            diameter: d,
            cost: 1.0,
            max_pressure: 100.0,
            kind: "1".into(),
            roughness: 0.0,
        });
        self
    }

    fn node(mut self, id: &str, z: f64, status: NodeStatus, orifice: f64) -> Self {
        self.net.nodes.push(Node {
            id: id.into(),
            elevation: z,
            x: 0.0,
            y: 0.0,
            taps: None,
            orifice_ideal: orifice,
            orifice_commercial: orifice,
            status,
        });
        self
    }

    fn arc(mut self, from: &str, to: &str, length: f64, pipe: &str) -> Self {
        let id = format!("A{}", self.net.arcs.len() + 1);
        self.net.arcs.push(Arc {
            id,
            from: from.into(),
            to: to.into(),
            length,
            length1: length,
            pipe1: Some(pipe.into()),
            length2: 0.0,
            pipe2: None,
            state: ArcState::New,
            open: true,
        });
        self
    }

    fn client(mut self, id: &str, terminal: &str, p: f64, q: f64) -> Self {
        self.net.clients.push(Client {
            id: id.into(),
            terminal: terminal.into(),
            probability: p,
            target_flow: q,
        });
        self
    }

    fn borne(mut self, id: &str, pressure: f64) -> Self {
        self.net.terminals.push(Terminal {
            id: id.into(),
            pressure,
        });
        self
    }

    fn build(self) -> Network {
        self.net
    }
}

const D: f64 = 0.05;
const ORIF: f64 = 0.012;
const ORIFCOEF: f64 = 0.59;
const FAUCET: f64 = 1.87e-8;

/// NW4 parameters with one diameter `D1` of `D`.
fn nw4(lambda: f64) -> NetBuilder {
    NetBuilder::new()
        .param("lambda", lambda)
        .param("orifcoef", ORIFCOEF)
        .param("faucetcoef", FAUCET)
        .param("bounded_flow", 0.0)
}

/// Source S (z 20) -> junction J (z 12) -> terminal T (z 0), one client.
fn line(lambda: f64) -> Network {
    nw4(lambda)
        .node("S", 20.0, NodeStatus::Source, 0.0)
        .node("J", 12.0, NodeStatus::Junction, 0.0)
        .node("T", 0.0, NodeStatus::Terminal, ORIF)
        .arc("S", "J", 100.0, "D1")
        .arc("J", "T", 10.0, "D1")
        .client("C1", "T", 1.0, 0.3)
        .borne("T", 2.0)
        .build()
}

#[test]
fn one_line_follows_the_head_loss_law() -> Result<(), String> {
    for lambda in [2.0, 1.781] {
        let r = run(&line(lambda), &options(3))?;
        let (b1, b2) = (beta(100.0, D), beta(10.0, D));
        let alpha = nw4_alpha(ORIFCOEF, ORIF, FAUCET);
        // energy: z_S - z_T = (b1 + b2) q^lambda + alpha q^2
        let q = if lambda == 2.0 {
            (20.0 / (b1 + b2 + alpha)).sqrt()
        } else {
            flow_for_head(b1 + b2, alpha, lambda, 20.0)
        };
        for s in &r.scenarios {
            assert_eq!(s.status, ScenarioStatus::Success);
            for j in 0..2 {
                assert!(
                    rel(s.flow[j], q) < 1e-6,
                    "lambda {lambda}: flow {} vs hand {q}",
                    s.flow[j]
                );
            }
            let head_j = 20.0 - b1 * q.powf(lambda);
            assert!((s.head[transit(&r, "J")] - head_j).abs() < 1e-5);
            assert!(rel(s.satisfaction[0], q / 0.3e-3) < 1e-6);
        }
    }
    Ok(())
}

#[test]
fn bounded_line_full_service_and_shortfall() -> Result<(), String> {
    let make = |z_source: f64| {
        NetBuilder::new()
            .param("lambda", 1.781)
            .node("S", z_source, NodeStatus::Source, 0.0)
            .node("J", 5.0, NodeStatus::Junction, 0.0)
            .node("P1", 0.0, NodeStatus::Terminal, 1.0)
            .arc("S", "J", 400.0, "D1")
            .arc("J", "P1", 20.0, "D1")
            .client("C1", "P1", 0.7, 0.6)
            .client("C2", "P1", 0.4, 0.5)
            .borne("P1", 8.0)
            .build()
    };
    let bt = beta(420.0, D);
    // plenty of head: flow at the demand, residual pressure = excess over P_req
    let r = run(&make(40.0), &options(50))?;
    assert!(r.bounded_flow && !r.uniform_demand);
    for s in &r.scenarios {
        assert_eq!(s.status, ScenarioStatus::Success);
        let d = s.demand_lps[0] / 1000.0;
        assert!(rel(s.flow[1], d) < 1e-4);
        // the source pipe is at its bound too (G = D), so the head at J is not
        // unique: the excess over P_req splits between the two multipliers
        let q = s.flow[1];
        let excess = 40.0 - bt * q.powf(1.781) - 8.0 / (d * d) * q * q;
        let (source, terminal) = (-s.charge[0], -s.charge[1]);
        assert!(source >= -1e-6 && terminal >= -1e-6);
        assert!(
            (source + terminal - excess).abs() < 1e-4,
            "excess {source} + {terminal} vs {excess}"
        );
    }
    // too little head: hydraulic shortfall, not a failure
    let r = run(&make(7.0), &options(50))?;
    assert!(r.failures.is_empty());
    for s in &r.scenarios {
        assert_eq!(s.status, ScenarioStatus::Success);
        let d = s.demand_lps[0] / 1000.0;
        let q = flow_for_head(bt, 8.0 / (d * d), 1.781, 7.0);
        assert!(q < d);
        assert!(rel(s.flow[1], q) < 1e-6);
        assert!(rel(s.satisfaction[0], q / d) < 1e-6);
    }
    let st = r.statistics.as_ref().ok_or("no statistics")?;
    let ts = &st.terminal_satisfaction[0];
    assert_eq!(ts.water_shortfall_count, ts.evaluated_active_scenario_count);
    assert_eq!(st.summary.failed_scenarios, 0);
    Ok(())
}

#[test]
fn two_terminals_in_parallel() -> Result<(), String> {
    let net = nw4(1.781)
        .node("S", 30.0, NodeStatus::Source, 0.0)
        .node("J", 10.0, NodeStatus::Junction, 0.0)
        .node("T1", 0.0, NodeStatus::Terminal, ORIF)
        .node("T2", 6.0, NodeStatus::Terminal, 0.009)
        .arc("S", "J", 300.0, "D1")
        .arc("J", "T1", 15.0, "D1")
        .arc("J", "T2", 40.0, "D1")
        .client("C1", "T1", 1.0, 0.25)
        .client("C2", "T2", 1.0, 0.25)
        .borne("T1", 2.0)
        .borne("T2", 2.0)
        .build();
    let r = run(&net, &options(3))?;
    let (b0, b1, b2) = (beta(300.0, D), beta(15.0, D), beta(40.0, D));
    let (a1, a2) = (
        nw4_alpha(ORIFCOEF, ORIF, FAUCET),
        nw4_alpha(ORIFCOEF, 0.009, FAUCET),
    );
    let q = |h: f64| {
        (
            flow_for_head(b1, a1, 1.781, h),
            flow_for_head(b2, a2, 1.781, h - 6.0),
        )
    };
    // head at J: z_S - b0 (q1 + q2)^lambda = h
    let h = bisect(
        |h| {
            let (q1, q2) = q(h);
            h - (30.0 - b0 * (q1 + q2).powf(1.781))
        },
        6.0,
        30.0,
    );
    let (q1, q2) = q(h);
    for s in &r.scenarios {
        assert!(rel(s.flow[pipe(&r, "J", "T1")], q1) < 1e-6);
        assert!(rel(s.flow[pipe(&r, "J", "T2")], q2) < 1e-6);
        assert!(rel(s.flow[pipe(&r, "S", "J")], q1 + q2) < 1e-6);
        assert!((s.head[0] - h).abs() < 1e-5);
    }
    Ok(())
}

#[test]
fn meshed_loop_splits_by_resistance() -> Result<(), String> {
    let lambda = 1.781;
    let net = nw4(lambda)
        .diameter("D2", 0.04)
        .node("S", 30.0, NodeStatus::Source, 0.0)
        .node("J1", 20.0, NodeStatus::Junction, 0.0)
        .node("J2", 18.0, NodeStatus::Junction, 0.0)
        .node("J3", 10.0, NodeStatus::Junction, 0.0)
        .node("T", 0.0, NodeStatus::Terminal, ORIF)
        .arc("S", "J1", 100.0, "D1")
        .arc("J1", "J2", 200.0, "D2")
        .arc("J2", "J3", 150.0, "D2")
        .arc("J1", "J3", 300.0, "D2")
        .arc("J3", "T", 10.0, "D1")
        .client("C1", "T", 1.0, 0.4)
        .borne("T", 2.0)
        .build();
    let r = run(&net, &options(2))?;
    let (b0, bt) = (beta(100.0, D), beta(10.0, D));
    let ba = beta(300.0, 0.04);
    let bb = beta(200.0, 0.04) + beta(150.0, 0.04);
    let inv = 1.0 / lambda;
    let beq = 1.0 / (ba.powf(-inv) + bb.powf(-inv)).powf(lambda);
    let alpha = nw4_alpha(ORIFCOEF, ORIF, FAUCET);
    let q = flow_for_head(b0 + beq + bt, alpha, lambda, 30.0);
    let dh = beq * q.powf(lambda);
    let (qa, qb) = ((dh / ba).powf(inv), (dh / bb).powf(inv));
    for s in &r.scenarios {
        assert!(rel(s.flow[pipe(&r, "J1", "J3")], qa) < 1e-6);
        assert!(rel(s.flow[pipe(&r, "J1", "J2")], qb) < 1e-6);
        assert!(rel(s.flow[pipe(&r, "J2", "J3")], qb) < 1e-6);
        assert!(rel(s.flow[pipe(&r, "J3", "T")], q) < 1e-6);
        // loop: the two paths lose the same head
        let h = |id: &str| s.head[transit(&r, id)];
        assert!((h("J1") - h("J3") - dh).abs() < 1e-5);
    }
    Ok(())
}

#[test]
fn two_sources_share_the_supply() -> Result<(), String> {
    let lambda = 1.781;
    let make = |z2: f64| {
        nw4(lambda)
            .node("S1", 30.0, NodeStatus::Source, 0.0)
            .node("S2", z2, NodeStatus::Source, 0.0)
            .node("J", 10.0, NodeStatus::Junction, 0.0)
            .node("T", 0.0, NodeStatus::Terminal, ORIF)
            .arc("S1", "J", 400.0, "D1")
            .arc("S2", "J", 200.0, "D1")
            .arc("J", "T", 10.0, "D1")
            .client("C1", "T", 1.0, 0.5)
            .borne("T", 2.0)
            .build()
    };
    let (b1, b2, bt) = (beta(400.0, D), beta(200.0, D), beta(10.0, D));
    let alpha = nw4_alpha(ORIFCOEF, ORIF, FAUCET);
    for z2 in [26.0, 5.0] {
        let r = run(&make(z2), &options(2))?;
        let supply = |h: f64| {
            let q1 = if h < 30.0 {
                ((30.0 - h) / b1).powf(1.0 / lambda)
            } else {
                0.0
            };
            let q2 = if h < z2 {
                ((z2 - h) / b2).powf(1.0 / lambda)
            } else {
                0.0
            };
            (q1, q2)
        };
        // h: supply(h) = demand(h); the difference decreases with h
        let h = bisect(
            |h| {
                let (q1, q2) = supply(h);
                flow_for_head(bt, alpha, lambda, h) - (q1 + q2)
            },
            0.0,
            30.0,
        );
        let (q1, q2) = supply(h);
        let src = &r.statistics.as_ref().ok_or("no statistics")?.source_flow;
        for s in &r.scenarios {
            assert!(rel(s.flow[pipe(&r, "S1", "J")], q1) < 1e-6);
            if q2 > 0.0 {
                assert!(rel(s.flow[pipe(&r, "S2", "J")], q2) < 1e-6);
            } else {
                // the low source is shut by its bound (no back flow)
                assert!(s.flow[pipe(&r, "S2", "J")].abs() < 1e-8);
            }
        }
        assert_eq!(src[0].source_id, "S1");
        assert!(rel(src[0].moy_st, q1 * 1000.0) < 1e-6);
        assert!(src[1].moy_st.abs() < 1e-5 || rel(src[1].moy_st, q2 * 1000.0) < 1e-6);
    }
    Ok(())
}

/// Largest KKT violation of a run, from flows and heads only (the head-loss
/// law and the bounds of the spec, not the reported charges): returns
/// (conservation m3/s, interior head-loss residual m, bound sign violation m).
fn kkt(r: &SimulationResult) -> (f64, f64, f64) {
    let tolx = 1e-8;
    let lambda = r.lambda;
    let (mut cons, mut law, mut sign) = (0.0f64, 0.0f64, 0.0f64);
    for s in r
        .scenarios
        .iter()
        .filter(|s| s.status == ScenarioStatus::Success)
    {
        // conservation at every junction
        let mut balance = vec![0.0; r.transit_nodes.len()];
        let index = |id: &str| r.transit_nodes.iter().position(|n| n.id == id);
        for (j, p) in r.pipes.iter().enumerate() {
            if let Some(i) = index(&p.from) {
                balance[i] -= s.flow[j];
            }
            if let Some(i) = index(&p.to) {
                balance[i] += s.flow[j];
            }
        }
        cons = balance.iter().fold(cons, |m, b| m.max(b.abs()));
        let total: f64 = s.demand_lps.iter().sum::<f64>() / 1000.0;
        for (j, p) in r.pipes.iter().enumerate() {
            if !s.open_pipes[j] {
                continue;
            }
            let q = s.flow[j];
            let a = s.alpha[j];
            // head at both ends: sources at their elevation, terminals at elevation
            let head = |id: &str| -> f64 {
                index(id).map_or_else(
                    || {
                        r.sources
                            .iter()
                            .chain(&r.terminals)
                            .find(|n| n.id == id)
                            .map_or(f64::NAN, |n| n.elevation)
                    },
                    |i| s.head[i],
                )
            };
            let loss = if q.abs() > tolx {
                p.beta * q.abs().powf(lambda - 1.0) * q + a * q.abs() * q
            } else {
                tolx.powf(lambda - 1.0) * p.beta * q + tolx * a * q
            };
            // residual = loss - (head(from) - head(to)): the charge
            let residual = loss - (head(&p.from) - head(&p.to));
            let (lo, up) = match p.kind {
                PipeKind::Terminal => {
                    let t = r.terminals.iter().position(|n| n.id == p.to).unwrap_or(0);
                    let d = s.demand_lps[t] / 1000.0;
                    (0.0, if r.bounded_flow { d } else { 100.0 * d })
                }
                PipeKind::Source => (0.0, if r.bounded_flow { total } else { 100.0 * total }),
                PipeKind::Transit => {
                    let g = if r.bounded_flow { total } else { 100.0 * total };
                    (-g, g)
                }
            };
            let at_lower = q - lo < tolx;
            let at_upper = up - q < tolx;
            if !at_lower && !at_upper {
                law = law.max(residual.abs());
            }
            if at_upper {
                sign = sign.max(residual);
            }
            if at_lower {
                sign = sign.max(-residual);
            }
            assert!(
                (residual - s.charge[j]).abs() < 1e-7,
                "charge mismatch on {} ({residual} vs {})",
                p.id,
                s.charge[j]
            );
        }
        if let Some(d) = s.diagnostics {
            assert!(d.primal_residual_inf <= 1e-8);
        }
    }
    (cons, law, sign)
}

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/neatwork5/examples/dsg_examples")
}

fn example(path: &str) -> Result<Network, String> {
    read_file(&examples().join(path))
        .map(|r| r.network)
        .map_err(|e| e.to_string())
}

#[test]
fn conservation_head_loss_and_kkt_on_every_scenario() -> Result<(), String> {
    let cases = [
        "Toy/Toy_blu_Secu1.00_fusion0_split0.dsg",
        "Toy/Toy_bbu_Secu1.00_fusion0_split0.dsg",
        "Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg",
        "Toy/Toy_Secu1.00_fusion0_split0.dsg",
        "Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg",
        "Esperenza/Esperenza2S.dsg",
        "Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg",
    ];
    for case in cases {
        for seed in [1, 2024] {
            let r = run(&example(case)?, &SimulationOptions::new(60, seed))?;
            assert_eq!(r.successful_count(), 60, "{case}");
            let (cons, law, sign) = kkt(&r);
            eprintln!(
                "{case:45} seed {seed:4}: conservation {cons:.2e} m3/s, head-loss residual {law:.2e} m, bound sign {sign:.2e} m"
            );
            assert!(cons <= 1e-8, "{case}: conservation {cons}");
            assert!(law <= 1e-6, "{case}: head-loss residual {law}");
            assert!(sign <= 1e-6, "{case}: complementarity {sign}");
        }
    }
    Ok(())
}

#[test]
fn solver_failure_is_not_a_shortfall() -> Result<(), String> {
    let net = example("Toy/Toy_blu_Secu1.00_fusion0_split0.dsg")?;
    // iteration limit: some scenarios do not converge and leave the statistics
    let mut o = SimulationOptions::new(100, 42);
    o.solver.max_iterations = 9;
    let r = run(&net, &o)?;
    let failed: Vec<usize> = (0..100)
        .filter(|&k| r.scenarios[k].status != ScenarioStatus::Success)
        .collect();
    assert!(!failed.is_empty() && failed.len() < 100);
    for f in &r.failures {
        assert_eq!(f.code, "SOLVER_MAX_ITERATIONS");
        assert_eq!(f.stage, FailureStage::Solver);
        assert_eq!(
            r.scenarios[f.scenario_index].status,
            ScenarioStatus::NotConverged
        );
        assert!(
            r.scenarios[f.scenario_index]
                .flow
                .iter()
                .all(|v| v.is_nan())
        );
    }
    let st = r.statistics.as_ref().ok_or("no statistics")?;
    assert_eq!(st.summary.failed_scenarios, failed.len());
    assert_eq!(
        st.summary.failure_count_by_code,
        vec![("SOLVER_MAX_ITERATIONS".to_string(), failed.len())]
    );
    let evaluated: usize = (0..100)
        .filter(|k| !failed.contains(k) && r.scenarios[*k].open_clients[0] > 0)
        .count();
    assert_eq!(
        st.terminal_satisfaction[0].evaluated_active_scenario_count,
        evaluated
    );
    // no scenario converges: no statistics
    o.solver.max_iterations = 2;
    let r = run(&net, &o)?;
    assert!(r.statistics.is_none());
    match statistics_json(&r) {
        Err(e) => assert_eq!(e.code(), "STATISTICS_PROCESSING_FAILED"),
        Ok(_) => return Err("statistics without a successful scenario".into()),
    }
    // closing the only source pipe: preparation failure of every scenario
    let mut o = SimulationOptions::new(5, 1);
    o.arc_open = vec![("A1".into(), false)];
    let r = run(&net, &o)?;
    assert!(
        r.scenarios
            .iter()
            .all(|s| s.status == ScenarioStatus::PreparationFailed)
    );
    assert!(
        r.failures.iter().all(
            |f| f.code == "SCENARIO_PREPARATION_FAILED" && f.stage == FailureStage::Preparation
        )
    );
    Ok(())
}

#[test]
fn validation_codes() -> Result<(), String> {
    let mut net = line(1.781);
    net.arcs[0].pipe1 = Some("NOPE".into());
    net.arcs.push(Arc {
        id: "A9".into(),
        from: "S".into(),
        to: "T".into(),
        length: 1.0,
        length1: 1.0,
        pipe1: Some("D1".into()),
        length2: 0.0,
        pipe2: None,
        state: ArcState::New,
        open: true,
    });
    match simulate(&net, &options(2)) {
        Err(SimError::Validation(issues)) => {
            let codes: Vec<&str> = issues.iter().map(|i| i.code).collect();
            assert!(codes.contains(&"UNKNOWN_PIPE_DIAMETER"), "{codes:?}");
            assert!(
                codes.contains(&"DIRECT_SOURCE_TO_TERMINAL_PIPE"),
                "{codes:?}"
            );
            assert!(codes.contains(&"INVALID_TERMINAL_IN_DEGREE"), "{codes:?}");
        }
        other => return Err(format!("expected a validation error, got {other:?}")),
    }
    let mut o = options(2);
    o.arc_open = vec![("A7".into(), false)];
    match simulate(&line(1.781), &o) {
        Err(e) => assert_eq!(e.issues()[0].code, "UNKNOWN_EDGE_OVERRIDE"),
        Ok(_) => return Err("unknown edge accepted".into()),
    }
    let mut o = options(2);
    o.oversampling_factor = 1.0;
    match simulate(&line(1.781), &o) {
        Err(e) => assert_eq!(e.issues()[0].code, "INVALID_OVERSAMPLING_FACTOR"),
        Ok(_) => return Err("oversampling 1 accepted".into()),
    }
    Ok(())
}

#[test]
fn network_without_borne_section_runs() -> Result<(), String> {
    // python-core cannot simulate it (no !Borne); the reader gives `pressure`
    let r = run(&example("Ceiba/CeibaNW4.dsg")?, &options(20))?;
    assert_eq!(r.successful_count(), 20);
    Ok(())
}

#[test]
fn progress_and_cancellation() -> Result<(), String> {
    let net = line(1.781);
    let mut calls = Vec::new();
    simulate_with_progress(&net, &options(4), |d, t| {
        calls.push((d, t));
        true
    })
    .map_err(|e| e.to_string())?;
    assert_eq!(calls, vec![(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)]);
    match simulate_with_progress(&net, &options(4), |d, _| d < 2) {
        Err(e) => assert_eq!(e.code(), "SIMULATION_CANCELLED"),
        Ok(_) => return Err("not cancelled".into()),
    }
    Ok(())
}

#[test]
#[cfg_attr(debug_assertions, ignore = "heavy: run with --release")]
fn ceiba_1000_scenarios_is_fast() -> Result<(), String> {
    let net = example("Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg")?;
    let start = std::time::Instant::now();
    let r = run(&net, &SimulationOptions::new(1000, 42))?;
    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "Ceiba: {} nodes, 1000 scenarios in {elapsed:.3} s",
        net.nodes.len()
    );
    assert_eq!(r.successful_count(), 1000);
    assert!(elapsed < 5.0, "{elapsed} s");
    Ok(())
}
