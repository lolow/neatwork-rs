#![expect(
    clippy::unwrap_used,
    reason = "test code: a failure is the test result"
)]

//! Design runs on the NW5 examples: golden LP solutions of
//! `rust/nw5/specs/design.md` section 5.3, a reinforcement input, Monte Carlo
//! loads, and independent checks of the designed networks.

use std::collections::BTreeMap;
use std::path::PathBuf;

use neatwork_nw5::read::{read_dsg, read_orifices_db, read_tpo};
use neatwork_nw5::{ArcState, Client, Network, NodeStatus};
use neatwork_nw5_design::{DesignOptions, DesignResult, LoadMethod, RiskMeasure, design};

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/neatwork5/examples")
}

fn text(rel: &str) -> String {
    let bytes = std::fs::read(examples().join(rel)).unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn tpo(rel: &str) -> Network {
    read_tpo("t", &text(rel)).unwrap().network
}

fn dsg(rel: &str) -> Network {
    read_dsg("d", &text(rel)).unwrap().network
}

/// Load factors of an old example `.dsg`: `succ -> lf` (rows `pred,succ,lf`).
fn file_load_factors(rel: &str) -> BTreeMap<String, f64> {
    let t = text(rel);
    let mut on = false;
    let mut out = BTreeMap::new();
    for l in t.lines() {
        if l.starts_with('!') {
            on = l.starts_with("!loadfactor");
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if on && f.len() >= 3 {
            out.insert(f[1].to_string(), f[2].trim().parse::<f64>().unwrap());
        }
    }
    out
}

/// LP lengths per arc: `succ -> {pipe ID -> length}`.
fn lp_lengths(r: &DesignResult) -> BTreeMap<String, BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    for (a, arc) in r.arcs.iter().enumerate() {
        let m: BTreeMap<String, f64> = r.solution.x[a]
            .iter()
            .enumerate()
            .filter(|(_, v)| **v > 1e-9)
            .map(|(d, v)| (r.pipe_ids[d].clone(), v * arc.length))
            .collect();
        out.insert(arc.succ.clone(), m);
    }
    out
}

struct Golden<'a> {
    file: &'a str,
    lp_cost: f64,
    splits: &'a [(&'a str, &'a [(&'a str, f64)])],
    degenerate: &'a [&'a str],
}

fn check_golden(r: &DesignResult, g: &Golden<'_>) {
    // LP cost (5 decimals in the specification)
    assert!(
        (r.solution.fval - g.lp_cost).abs() < 1e-5,
        "LP cost {} != {}",
        r.solution.fval,
        g.lp_cost
    );
    // load factors equal to the file
    let lf = file_load_factors(g.file);
    for (a, arc) in r.arcs.iter().enumerate() {
        let got = r.loads.row_max(a) / r.params.targetflow;
        let want = lf[&arc.succ];
        assert!(
            (got - want).abs() <= 1e-13 * want,
            "load factor {}: {got} != {want}",
            arc.succ
        );
    }
    // pipe choice of the LP
    let file = dsg(g.file);
    let lengths = lp_lengths(r);
    for arc in &r.arcs {
        if g.degenerate.contains(&arc.succ.as_str()) {
            continue;
        }
        let got = &lengths[&arc.succ];
        if let Some((_, want)) = g.splits.iter().find(|(s, _)| *s == arc.succ) {
            assert_eq!(got.len(), want.len(), "pipes on {}: {got:?}", arc.succ);
            for (id, l) in *want {
                let v = got.get(*id).copied().unwrap_or(f64::NAN);
                assert!((v - l).abs() < 1e-3, "{} {id}: {v} != {l}", arc.succ);
            }
        } else {
            let f = file.arcs.iter().find(|x| x.to == arc.succ).unwrap();
            let id = f.pipe1.clone().unwrap();
            assert_eq!(got.len(), 1, "pipes on {}: {got:?}", arc.succ);
            assert!(
                (got[&id] - arc.length).abs() < 1e-6,
                "{} uses {id}: {got:?}",
                arc.succ
            );
        }
    }
}

/// Independent checks of a designed network: heads at every node for the
/// design flows (recomputed from the output pipes), velocity limits, and the
/// structure of the output.
fn check_design(input: &Network, r: &DesignResult) {
    let p = &r.params;
    let net = &r.network;
    let lambda = p.lambda;
    let resist = |id: &str| {
        let d = net.pipes.iter().find(|q| q.id == id).unwrap().diameter;
        p.bescal * d.powf(-p.nu)
    };
    let hsource = net
        .nodes
        .iter()
        .find(|n| n.status == NodeStatus::Source)
        .unwrap()
        .elevation
        + p.pump_pressure;
    let flows: BTreeMap<&str, f64> = net
        .loadflow
        .iter()
        .map(|(s, q, _)| (s.as_str(), *q))
        .collect();
    let qv: BTreeMap<&str, f64> = r
        .arcs
        .iter()
        .enumerate()
        .map(|(a, arc)| (arc.succ.as_str(), r.loads_velocity.row_max(a)))
        .collect();
    // conductance of each link from the output rows
    let mut cond: BTreeMap<(String, String), f64> = BTreeMap::new();
    for arc in &net.arcs {
        let mut rl = 0.0;
        for (l, id) in [(arc.length1, &arc.pipe1), (arc.length2, &arc.pipe2)] {
            if l > 0.0 {
                rl += l * resist(id.as_deref().unwrap());
            }
        }
        assert!(rl > 0.0, "arc {} -> {} has no pipe", arc.from, arc.to);
        *cond.entry((arc.from.clone(), arc.to.clone())).or_default() +=
            (rl / arc.length).powf(-1.0 / lambda);
        // velocity: new pipes and links that are not reinforced
        let reinforced_link = net.arcs.iter().filter(|b| b.to == arc.to).count() > 1;
        if arc.state == ArcState::Reinforcement || !reinforced_link {
            let dmin =
                2.0 * (qv[arc.to.as_str()] / 1000.0 / (std::f64::consts::PI * p.vmax)).sqrt();
            for (l, id) in [(arc.length1, &arc.pipe1), (arc.length2, &arc.pipe2)] {
                if l > 0.0 {
                    let d = net
                        .pipes
                        .iter()
                        .find(|q| Some(&q.id) == id.as_ref())
                        .unwrap()
                        .diameter;
                    assert!(
                        d >= dmin,
                        "velocity on {} -> {}: diameter {d} < {dmin}",
                        arc.from,
                        arc.to
                    );
                }
            }
        }
    }
    // head at every node, walking from the source
    let mut head: BTreeMap<String, f64> = BTreeMap::new();
    let src = net
        .nodes
        .iter()
        .find(|n| n.status == NodeStatus::Source)
        .unwrap();
    head.insert(src.id.clone(), 0.0);
    let mut changed = true;
    while changed {
        changed = false;
        for ((from, to), c) in &cond {
            if head.contains_key(to) {
                continue;
            }
            if let Some(&h) = head.get(from) {
                let len = net
                    .arcs
                    .iter()
                    .find(|a| &a.from == from && &a.to == to)
                    .unwrap()
                    .length;
                let q = flows[to.as_str()] / 1000.0;
                head.insert(to.clone(), h + len * c.powf(-lambda) * q.powf(lambda));
                changed = true;
            }
        }
    }
    for n in net.nodes.iter().filter(|n| n.status != NodeStatus::Source) {
        let loss = head[&n.id];
        let mut available = hsource - n.elevation;
        let mut required = 0.0;
        if n.status == NodeStatus::Terminal {
            available /= p.secu;
            if p.bounded_flow {
                required = net
                    .terminals
                    .iter()
                    .find(|t| t.id == n.id)
                    .unwrap()
                    .pressure;
            }
            if p.uniform_dem {
                required += (p.targetflow / 1000.0).powi(2) / p.faucetcoef;
            }
        }
        let margin = available - loss - required;
        assert!(margin >= -1e-9, "head margin at {} is {margin}", n.id);
        let m = net.loadflow.iter().find(|(s, _, _)| *s == n.id).unwrap().2;
        assert!(
            (m - margin).abs() <= 1e-9 * margin.abs().max(1.0),
            "margin {} {m} != {margin}",
            n.id
        );
    }
    assert!(r.ts.iter().all(|t| t.head_deficit == 0.0));
    // structure: existing pipes kept, every bare link reinforced
    let doubled = r
        .tcp
        .iter()
        .filter(|t| t.nature == neatwork_nw5_design::Nature::New)
        .count()
        + r.tcp
            .iter()
            .filter(|t| t.nature == neatwork_nw5_design::Nature::Old)
            .count()
        - r.arcs.len();
    assert_eq!(net.arcs.len(), r.arcs.len() + doubled);
    for a in input.arcs.iter().filter(|a| a.equipped()) {
        assert!(
            net.arcs
                .iter()
                .any(|b| b.id == a.id && b.length1 == a.length1 && b.pipe1 == a.pipe1)
        );
    }
    for t in &r.tcp {
        assert!(t.diam1 >= t.diam2 || t.nature == neatwork_nw5_design::Nature::Old);
    }
    assert!(r.costs.final_cost >= r.costs.inter - 1e-9 && r.costs.optim <= r.costs.inter + 1e-9);
}

#[test]
fn toy_blu() {
    let net = tpo("tpo_examples/Toy/Toy_blu.tpo");
    let r = design(&net, &DesignOptions::with_params(&[("vmax", 9.0)])).unwrap();
    check_golden(
        &r,
        &Golden {
            file: "dsg_examples/Toy/Toy_blu_Secu1.00_fusion0_split0.dsg",
            lp_cost: 602.56548,
            splits: &[
                ("C", &[("D004", 4.4555), ("D002", 65.5445)]),
                ("P2", &[("D004", 70.0956), ("D002", 109.9044)]),
                ("P3", &[("D002", 4.3538), ("D001", 85.6462)]),
            ],
            degenerate: &[],
        },
    );
    let p1 = r.network.node("P1").unwrap();
    assert!((p1.orifice_ideal - 0.004796589912316905).abs() <= 1e-12 * 0.0048);
    assert_eq!(p1.orifice_commercial, 0.0044);
    for id in ["P2", "P3", "P4", "A", "Source"] {
        let n = r.network.node(id).unwrap();
        assert_eq!((n.orifice_ideal, n.orifice_commercial), (1.0, 1.0), "{id}");
    }
    check_design(&net, &r);
}

#[test]
fn toy_bbu() {
    let net = tpo("tpo_examples/Toy/Toy_bbu.tpo");
    let r = design(
        &net,
        &DesignOptions::with_params(&[("bounded_flow", 1.0), ("vmax", 1.5)]),
    )
    .unwrap();
    check_golden(
        &r,
        &Golden {
            file: "dsg_examples/Toy/Toy_bbu_Secu1.00_fusion0_split0.dsg",
            lp_cost: 649.08633,
            splits: &[
                ("B", &[("D009", 52.8204), ("D004", 447.1796)]),
                ("P3", &[("D002", 46.3937), ("D001", 43.6063)]),
                ("P4", &[("D002", 78.1753), ("D001", 31.8247)]),
            ],
            degenerate: &[],
        },
    );
    // bounded flow: no orifice
    assert!(
        r.network
            .nodes
            .iter()
            .all(|n| n.orifice_ideal == 1.0 && n.orifice_commercial == 1.0)
    );
    // A -> B: the larger diameter is rounded up to a multiple of 6 m
    let ab: Vec<_> = r.network.arcs.iter().filter(|a| a.to == "B").collect();
    assert_eq!(ab.len(), 1);
    assert_eq!(
        (ab[0].length1, ab[0].pipe1.as_deref()),
        (54.0, Some("D009"))
    );
    assert_eq!(
        (ab[0].length2, ab[0].pipe2.as_deref()),
        (446.0, Some("D004"))
    );
    assert_eq!(ab[0].state, ArcState::Reinforcement);
    check_design(&net, &r);
}

#[test]
fn toy_secu() {
    let mut net = tpo("tpo_examples/Toy/Toy_blu.tpo");
    net.orifices = read_orifices_db(&text("tpo_examples/Toy/orifices.db"));
    let r = design(
        &net,
        &DesignOptions::with_params(&[("opentaps", 0.28), ("vmax", 1.5), ("bounded_flow", 0.0)]),
    )
    .unwrap();
    check_golden(
        &r,
        &Golden {
            file: "dsg_examples/Toy/Toy_Secu1.00_fusion0_split0.dsg",
            lp_cost: 514.79229,
            splits: &[
                ("B", &[("D004", 219.6846), ("D002", 280.3154)]),
                ("P4", &[("D002", 27.2241), ("D001", 82.7759)]),
            ],
            degenerate: &[],
        },
    );
    let p3 = r.network.node("P3").unwrap();
    assert!((p3.orifice_ideal - 0.006438404306132354).abs() <= 1e-12 * 0.0065);
    assert_eq!(p3.orifice_commercial, 0.0056);
    check_design(&net, &r);
}

#[test]
fn toy_borne() {
    let mut net = tpo("tpo_examples/Toy/Toy_blu.tpo");
    net.clients = ["P1", "P2", "P3", "P4"]
        .iter()
        .map(|t| Client {
            id: format!("C1{t}"),
            terminal: t.to_string(),
            probability: 0.28,
            target_flow: 0.12,
        })
        .collect();
    net.pipes = dsg("dsg_examples/Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg").pipes;
    let r = design(
        &net,
        &DesignOptions::with_params(&[("opentaps", 0.28), ("vmax", 1.5)]),
    )
    .unwrap();
    check_golden(
        &r,
        &Golden {
            file: "dsg_examples/Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg",
            lp_cost: 450.40528,
            splits: &[("B", &[("D002", 240.0399), ("D001", 259.9601)])],
            // tied optima (spec 4.6): compare the cost only
            degenerate: &["C"],
        },
    );
    check_design(&net, &r);
}

#[test]
fn reinforcement_of_an_existing_network() {
    // ToyDevReinf.dsg: bare link C -> P3, blocked link C -> P4 (cannot run in
    // MATLAB because of B1)
    let net = dsg("dsg_examples/Toy/ToyDevReinf.dsg");
    let r = design(&net, &DesignOptions::default()).unwrap();
    assert!(
        r.params.bounded_flow,
        "a .dsg input defaults to bounded flow"
    );
    assert!(r.solution.milp, "some y are free: MILP");
    let ix = |s: &str| r.arcs.iter().position(|a| a.succ == s).unwrap();
    assert_eq!(r.solution.y[ix("P3")], 1.0);
    assert_eq!(r.solution.y[ix("P4")], 0.0);
    let p4: Vec<_> = r.network.arcs.iter().filter(|a| a.to == "P4").collect();
    assert_eq!(p4.len(), 1);
    assert_eq!(p4[0].state, ArcState::Blocked);
    let p3: Vec<_> = r.network.arcs.iter().filter(|a| a.to == "P3").collect();
    assert_eq!(p3.len(), 1);
    assert_eq!(p3[0].state, ArcState::Reinforcement);
    // a doubled link keeps its existing row and gets a parallel R row
    for (a, arc) in r.arcs.iter().enumerate() {
        let rows = r.network.arcs.iter().filter(|b| b.to == arc.succ).count();
        let expected = if arc.equipped() && r.solution.y[a] == 1.0 {
            2
        } else {
            1
        };
        assert_eq!(rows, expected, "{}", arc.succ);
    }
    check_design(&net, &r);
}

#[test]
fn monte_carlo_toy_bbd() {
    let net = tpo("tpo_examples/Toy/Toy_bbd.tpo");
    let opts = DesignOptions {
        seed: 1,
        ..DesignOptions::with_params(&[("vmax", 19.0), ("bounded_flow", 1.0)])
    };
    let r = design(&net, &opts).unwrap();
    assert!(!r.params.uniform_dem);
    let lf = file_load_factors("dsg_examples/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg");
    for (a, arc) in r.arcs.iter().enumerate() {
        let got = r.loads.row_max(a) / r.params.targetflow;
        let want = lf[&arc.succ];
        assert!(
            (got - want).abs() <= 0.01 * want,
            "{}: {got} vs {want}",
            arc.succ
        );
    }
    check_design(&net, &r);
    // same seed, same result; another seed, other loads
    let again = design(&net, &opts).unwrap();
    assert_eq!(again.loads, r.loads);
    assert_eq!(again.network, r.network);
    let other = design(
        &net,
        &DesignOptions {
            seed: 2,
            ..opts.clone()
        },
    )
    .unwrap();
    assert_ne!(other.loads, r.loads);
    // CVaR is at least VaR
    let cvar = design(
        &net,
        &DesignOptions {
            risk: Some(RiskMeasure::Cvar),
            ..opts.clone()
        },
    )
    .unwrap();
    for a in 0..r.arcs.len() {
        assert!(cvar.loads.row_max(a) >= r.loads.row_max(a) - 1e-12);
    }
}

#[test]
fn monte_carlo_on_uniform_demand_is_close_to_binomial() {
    // one client per terminal: "one client open" (binomial) and "terminal
    // demand > 0" (Monte Carlo) are the same condition (spec 3.2)
    let mut net = tpo("tpo_examples/Toy/Toy_blu.tpo");
    net.clients.retain(|c| c.id.starts_with("C1"));
    let exact = design(&net, &DesignOptions::with_params(&[("vmax", 9.0)])).unwrap();
    let mc = design(
        &net,
        &DesignOptions {
            load_method: LoadMethod::MonteCarlo,
            n_scen_load: Some(50_000),
            seed: 7,
            ..DesignOptions::with_params(&[("vmax", 9.0)])
        },
    )
    .unwrap();
    for a in 0..exact.arcs.len() {
        let (e, m) = (exact.loads.row_max(a), mc.loads.row_max(a));
        assert!(
            (e - m).abs() <= 0.1 * e,
            "{}: {e} vs {m}",
            exact.arcs[a].succ
        );
    }
}

#[test]
fn progress_and_parameters() {
    let net = tpo("tpo_examples/Toy/Toy_blu.tpo");
    let mut stages = Vec::new();
    let r = neatwork_nw5_design::design_with_progress(&net, &DesignOptions::default(), &mut |s| {
        stages.push(s)
    })
    .unwrap();
    assert_eq!(stages.len(), 5);
    // reviewed parameters in the output, fusion/split forced to 0
    let params = &r.network.params;
    assert_eq!(params.raw("fusion"), Some("0"));
    assert_eq!(params.raw("limitbudget"), Some("100000000"));
    assert_eq!(params.entries[0].0, "bounded_flow");
    assert!(r.network.loadflow.len() == r.arcs.len());
    // unknown parameter
    assert!(design(&net, &DesignOptions::with_params(&[("nope", 1.0)])).is_err());
    // secu < 1
    assert!(design(&net, &DesignOptions::with_params(&[("secu", 0.5)])).is_err());
}

#[test]
fn infeasible_budget_is_a_design_failure() {
    let mut net = tpo("tpo_examples/Toy/Toy_blu.tpo");
    for n in &mut net.nodes {
        if n.id == "P1" {
            n.elevation = 1.0;
        }
    }
    let e = design(&net, &DesignOptions::default())
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("NW5:DesignFailure") && e.contains("Solver diagnostic"),
        "{e}"
    );
}
