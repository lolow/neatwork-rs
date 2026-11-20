#![expect(
    clippy::unwrap_used,
    reason = "test code: a failure is the test result"
)]

//! The MATLAB unit tests of `reference/neatwork5/tests/matlab`, with the
//! expected values of `rust/nw5/specs/design.md` section 8. Fixtures are
//! rebuilt in memory. Checks that only concern MATLAB tables (column types,
//! missing columns) have no Rust equivalent and are left out.

use neatwork_nw5::read::{read_dsg, read_tpo};
use neatwork_nw5::{Arc, ArcState, Client, Network, Node, NodeStatus, Params, Pipe, Terminal};
use neatwork_nw5_design::loads::{LoadTable, incidence_loads};
use neatwork_nw5_design::model::{LpModel, Solution, build_model, solve_model};
use neatwork_nw5_design::params::{DesignParams, review};
use neatwork_nw5_design::post::{PostInput, process_solution, reduce_pipe_mixture};
use neatwork_nw5_design::prepare::{
    Catalog, DesignArc, PathMatrix, Segment, Tr, equiv_resist, prepare, validate_existing_pressure,
};
use neatwork_nw5_design::{ConstraintType, DesignOptions, LoadMethod, Nature, Twister, design};

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}

fn assert_close(a: &[f64], b: &[f64], tol: f64) {
    assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
    for (x, y) in a.iter().zip(b) {
        assert!(close(*x, *y, tol), "{a:?} vs {b:?}");
    }
}

fn err_text<T: std::fmt::Debug>(r: neatwork_nw5::Result<T>) -> String {
    r.unwrap_err().to_string()
}

fn seg(length: f64, pipe: Option<usize>) -> Segment {
    Segment { length, pipe }
}

fn arc(pred: &str, succ: &str, length: f64) -> DesignArc {
    DesignArc {
        source: None,
        id: succ.into(),
        pred: pred.into(),
        succ: succ.into(),
        length,
        segments: [Segment::default(); 2],
        state: ArcState::New,
        open: true,
        pmax: 0.0,
        haut: 0.0,
        status: NodeStatus::Junction,
        nbrob: 0.0,
        pressure: 0.0,
        fixed_cost: 0.0,
    }
}

fn table(arcs: &[&str], terminals: &[&str], values: &[&[f64]]) -> LoadTable {
    LoadTable {
        arcs: arcs.iter().map(|s| s.to_string()).collect(),
        terminals: terminals.iter().map(|s| s.to_string()).collect(),
        values: values.iter().map(|r| r.to_vec()).collect(),
    }
}

fn tree3() -> PathMatrix {
    PathMatrix::from_dense(&[
        vec![1.0, 1.0, 1.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ])
}

// ---------------------------------------------------------------- 8.1

fn integration_network(h: f64, permuted: bool) -> Network {
    let node = |id: &str, z: f64, x: f64, y: f64, s: NodeStatus| Node {
        id: id.into(),
        elevation: z + h,
        x,
        y,
        taps: None,
        orifice_ideal: 0.0,
        orifice_commercial: 0.0,
        status: s,
    };
    let mut nodes = vec![
        node("S", 0.0, 0.0, 0.0, NodeStatus::Source),
        node("J", -50.0, 1.0, 0.0, NodeStatus::Junction),
        node("B1", -200.0, 2.0, 1.0, NodeStatus::Terminal),
        node("B2", -15.0, 2.0, -1.0, NodeStatus::Terminal),
    ];
    let a = |id: &str, f: &str, t: &str, l: f64, l1: f64, p1: Option<&str>| Arc {
        id: id.into(),
        from: f.into(),
        to: t.into(),
        length: l,
        length1: l1,
        pipe1: p1.map(String::from),
        length2: 0.0,
        pipe2: None,
        state: ArcState::New,
        open: true,
    };
    let mut arcs = vec![
        a("A1", "S", "J", 10.0, 0.0, None),
        a("A2", "J", "B1", 20.0, 20.0, Some("D1")),
        a("A3", "J", "B2", 20.0, 20.0, Some("D1")),
    ];
    let pipe = |id: &str, nominal: &str, d: f64, c: f64| Pipe {
        id: id.into(),
        nominal: nominal.into(),
        sdr: "1".into(),
        diameter: d,
        cost: c,
        max_pressure: 1000.0,
        kind: "1".into(),
        roughness: 0.0,
    };
    let mut pipes = vec![pipe("D1", "petit", 0.1, 2.0), pipe("D2", "grand", 0.2, 5.0)];
    let mut terminals = vec![
        Terminal {
            id: "B1".into(),
            pressure: 9.0,
        },
        Terminal {
            id: "B2".into(),
            pressure: 9.0,
        },
    ];
    let client = |id: &str, t: &str| Client {
        id: id.into(),
        terminal: t.into(),
        probability: 1.0,
        target_flow: 1.0,
    };
    let mut clients = vec![client("C1", "B1"), client("C2", "B2")];
    if permuted {
        arcs = vec![arcs[2].clone(), arcs[0].clone(), arcs[1].clone()];
        nodes = vec![
            nodes[3].clone(),
            nodes[1].clone(),
            nodes[0].clone(),
            nodes[2].clone(),
        ];
        pipes.reverse();
        terminals.reverse();
        clients.reverse();
    }
    let mut params = Params::default();
    for (k, v) in [
        ("uniform_dem", "1"),
        ("bounded_flow", "1"),
        ("pressure", "1"),
        ("targetflow", "1"),
        ("opentaps", "1"),
        ("servicequal", "0.95"),
        ("vmax_secu", "0.99"),
        ("use_cvar", "1"),
        ("vmax", "0.2"),
        ("secu", "1"),
        ("pipelength", "6"),
        ("pump_pressure", "0"),
        ("lambda", "2"),
        ("nu", "4"),
        ("bescal", "100"),
        ("faucetcoef", "1e-4"),
        ("orifcoef", "0.001"),
    ] {
        params.set(k, v);
    }
    Network {
        name: "integration".into(),
        nodes,
        arcs,
        clients,
        terminals,
        pipes,
        params,
        ..Network::default()
    }
}

#[test]
fn test_reinforcement_integration() {
    for h in [0.0, 100.0] {
        for permuted in [false, true] {
            let net = integration_network(h, permuted);
            // tables
            let rev = review(&net, &[], None, None).unwrap();
            let mut w = Vec::new();
            let prep = prepare(&net, &rev.clients, &rev.params, &mut w).unwrap();
            let succ: Vec<&str> = prep.arcs.iter().map(|a| a.succ.as_str()).collect();
            assert_eq!(succ, ["J", "B1", "B2"]);
            assert_close(
                &prep.arcs.iter().map(|a| a.haut).collect::<Vec<_>>(),
                &[-50.0, -200.0, -15.0],
                1e-8,
            );
            assert_close(
                &prep.arcs.iter().map(|a| a.pmax).collect::<Vec<_>>(),
                &[50.0, 200.0, 50.0],
                1e-8,
            );
            assert_close(
                &prep.arcs.iter().map(|a| a.pressure).collect::<Vec<_>>(),
                &[0.0, 1.0, 1.0],
                1e-8,
            );
            let degree: Vec<(String, usize)> = net
                .nodes
                .iter()
                .map(|n| n.id.clone())
                .zip(prep.degree.iter().copied())
                .collect();
            for (id, d) in [("S", 1), ("J", 3), ("B1", 1), ("B2", 1)] {
                assert!(degree.contains(&(id.to_string(), d)));
            }
            assert_eq!(prep.paths.dense(), tree3().dense());
            // full chain
            let r = design(&net, &DesignOptions::default()).unwrap();
            assert_eq!(r.loads.terminals, ["B1", "B2"]);
            assert_eq!(r.loads.arcs, ["J", "B1", "B2"]);
            assert_eq!(
                r.loads.values,
                vec![vec![2.0, 2.0], vec![1.0, 0.0], vec![0.0, 1.0]]
            );
            assert!(r.network.terminals.iter().all(|t| t.pressure == 1.0));
            assert_eq!(r.solution.y, [1.0, 0.0, 1.0]);
            assert_close(
                &r.solution.x.concat(),
                &[0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
                1e-8,
            );
            assert!(close(r.solution.fval, 90.0, 1e-8));
            assert!(r.solution.slack.iter().all(|s| *s >= -1e-6));
            let margin: Vec<f64> = r.tah.iter().map(|t| t.margin).collect();
            assert_close(&r.solution.slack, &margin, 1e-8);
            assert_eq!(r.network.arcs.len(), 4);
            assert_eq!(r.network.nodes.len(), 4);
            for n in &r.network.nodes {
                let i = net.nodes.iter().find(|m| m.id == n.id).unwrap();
                assert_eq!((n.elevation, n.x, n.y), (i.elevation, i.x, i.y));
            }
            assert_eq!(r.tcp.iter().filter(|t| t.nature == Nature::Old).count(), 2);
            assert_eq!(r.tcp.iter().filter(|t| t.nature == Nature::New).count(), 2);
            let tah_succ: Vec<&str> = r.tah.iter().map(|t| t.succ.as_str()).collect();
            assert_eq!(tah_succ, ["J", "B1", "B2"]);
            assert_close(
                &r.tah.iter().map(|t| t.equiv_resist).collect::<Vec<_>>(),
                &[62500.0, 1e6, 250000.0],
                1e-8,
            );
            assert_close(
                &r.tah.iter().map(|t| t.path_loss).collect::<Vec<_>>(),
                &[2.5, 22.5, 7.5],
                1e-8,
            );
            assert_close(&margin, &[47.5, 176.49, 6.49], 1e-8);
            assert_close(
                &r.ts.iter().map(|t| t.head_margin).collect::<Vec<_>>(),
                &margin,
                0.0,
            );
            assert!(r.ts.iter().all(|t| t.head_deficit == 0.0));
            assert!(close(r.costs.optim, 90.0, 1e-8) && close(r.costs.final_cost, 90.0, 1e-8));
            assert_eq!(r.costs.fixed, 0.0);
            for (s, n) in [("J", 1), ("B1", 1), ("B2", 2)] {
                assert_eq!(
                    r.network.arcs.iter().filter(|a| a.to == s).count(),
                    n,
                    "{s}"
                );
            }
            let new_cost: f64 = r
                .network
                .arcs
                .iter()
                .filter(|a| a.state == ArcState::Reinforcement)
                .map(|a| a.length1 * r.network.pipe(a.pipe1.as_deref().unwrap()).unwrap().cost)
                .sum();
            assert!(close(new_cost, 90.0, 1e-8));
        }
    }
}

// ---------------------------------------------------------------- 8.2

struct ModelFixture {
    arcs: Vec<DesignArc>,
    catalog: Catalog,
    tl: LoadTable,
    tlv: LoadTable,
    tr: Tr,
    p: DesignParams,
}

fn tr_for(rexist: &[f64], catalog: &Catalog, lambda: f64) -> Tr {
    Tr {
        rexist: rexist.to_vec(),
        r: rexist
            .iter()
            .map(|&re| {
                catalog
                    .pipes
                    .iter()
                    .map(|q| {
                        if re.is_finite() {
                            (re.powf(-1.0 / lambda) + q.cond).powf(-lambda)
                        } else {
                            q.resist
                        }
                    })
                    .collect()
            })
            .collect(),
    }
}

fn model_fixture() -> ModelFixture {
    let lambda = 2.0;
    let catalog = Catalog::with_resistances(
        &[("D1", 0.05, 2.0, 10.0, 1e6), ("D2", 0.2, 5.0, 30.0, 1e4)],
        lambda,
    );
    let mut arcs = vec![
        arc("S", "J", 10.0),
        arc("J", "B1", 20.0),
        arc("J", "B2", 30.0),
    ];
    for (a, (haut, pmax, term, pressure)) in arcs.iter_mut().zip([
        (-1000.0, 1.0, false, 0.0),
        (-1500.0, 20.0, true, 2.0),
        (-2000.0, 1.0, true, 3.0),
    ]) {
        a.haut = haut;
        a.pmax = pmax;
        a.status = if term {
            NodeStatus::Terminal
        } else {
            NodeStatus::Junction
        };
        a.pressure = pressure;
    }
    arcs[1].segments[0] = seg(20.0, Some(0));
    arcs[2].segments[1] = seg(30.0, Some(0));
    arcs[2].state = ArcState::Blocked;
    let p = DesignParams {
        lambda,
        faucetcoef: 1e-6,
        vmax: 1.0,
        targetflow: 0.4,
        secu: 1.25,
        bounded_flow: true,
        uniform_dem: true,
        ..DesignParams::default()
    };
    let tr = tr_for(&[f64::INFINITY, 1e6, 1e6], &catalog, lambda);
    ModelFixture {
        arcs,
        tl: table(
            &["J", "B1", "B2"],
            &["B1", "B2"],
            &[&[2.0, 1.0], &[0.5, 1.0], &[1.0, 0.0]],
        ),
        tlv: table(
            &["J", "B1", "B2"],
            &["B1", "B2"],
            &[&[10.0, 3.0], &[1.0, 0.5], &[0.5, 1.0]],
        ),
        catalog,
        tr,
        p,
    }
}

fn build(f: &ModelFixture) -> neatwork_nw5::Result<LpModel> {
    build_model(&f.arcs, &tree3(), &f.catalog, &f.tr, &f.tl, &f.tlv, &f.p)
}

#[test]
fn test_build_reinf_model() {
    let f = model_fixture();
    let m = build(&f).unwrap();
    assert_eq!((m.nb_a, m.nb_d, m.n_var()), (3, 2, 9));
    assert_eq!(m.f, [20.0, 50.0, 40.0, 100.0, 60.0, 150.0, 0.0, 0.0, 0.0]);
    assert_eq!(m.lb, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_eq!(m.ub, [0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]);
    assert_eq!(
        m.activ_variables(),
        m.ub.iter().map(|&u| u == 1.0).collect::<Vec<_>>()
    );
    assert_eq!(m.b_eq, [0.0; 3]);
    assert_eq!(
        m.a_eq_dense(),
        vec![
            vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0],
        ]
    );
    // hydraulic oracle
    let paths = tree3();
    for (bounded, uniform) in [(false, true), (true, false), (true, true)] {
        for lambda in [1.2, 1.781, 2.0] {
            let mut g = model_fixture();
            g.p.bounded_flow = bounded;
            g.p.uniform_dem = uniform;
            g.p.lambda = lambda;
            g.catalog = Catalog::with_resistances(
                &[("D1", 0.05, 2.0, 10.0, 1e6), ("D2", 0.2, 5.0, 30.0, 1e4)],
                lambda,
            );
            g.tr = tr_for(&[f64::INFINITY, 1e6, 1e6], &g.catalog, lambda);
            let m = build(&g).unwrap();
            for x in [
                [[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
                [[0.5, 0.5], [0.5, 0.5], [0.5, 0.5]],
                [[0.2, 0.8], [0.7, 0.0], [0.0, 0.3]],
            ] {
                let y: Vec<f64> = x.iter().map(|r| r[0] + r[1]).collect();
                let z: Vec<f64> = x
                    .iter()
                    .flatten()
                    .copied()
                    .chain(y.iter().copied())
                    .collect();
                let loss: Vec<f64> = (0..3)
                    .map(|a| {
                        let q = g.tl.row_max(a) / 1000.0;
                        let base = if g.tr.rexist[a].is_finite() {
                            g.tr.rexist[a]
                        } else {
                            0.0
                        };
                        let r =
                            (1.0 - y[a]) * base + x[a][0] * g.tr.r[a][0] + x[a][1] * g.tr.r[a][1];
                        g.arcs[a].length * r * q.powf(lambda)
                    })
                    .collect();
                let act = m.ineq_activity(&z);
                for (j, arc) in g.arcs.iter().enumerate() {
                    let mut available = -arc.haut;
                    if arc.terminal() {
                        available /= g.p.secu;
                        if bounded {
                            available -= arc.pressure;
                        }
                        if uniform {
                            available -= (g.p.targetflow / 1000.0).powi(2) / g.p.faucetcoef;
                        }
                    }
                    let margin: f64 =
                        paths.paths[j].iter().map(|&a| loss[a]).sum::<f64>() - available;
                    assert!(
                        close(act[j] - m.b_ineq[j], margin, 1e-9),
                        "{bounded} {uniform} {lambda} {j}"
                    );
                }
                assert!(m.eq_activity(&z).iter().all(|v| v.abs() <= 1e-9));
            }
        }
    }
    // every candidate carries the whole flow
    let mut slow = model_fixture();
    slow.p.vmax = 0.18;
    slow.tlv.values = vec![vec![0.5; 2]; 3];
    slow.arcs[2].state = ArcState::New;
    let m = build(&slow).unwrap();
    assert_eq!(m.ub[..6], [0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    assert_eq!(m.lb[6..], [1.0, 1.0, 1.0]);
    check_mandatory_speed_reinforcement();
    // fixed costs
    let mut fixed = model_fixture();
    for (a, c) in fixed.arcs.iter_mut().zip([7.0, 11.0, 13.0]) {
        a.fixed_cost = c;
    }
    let mf = build(&fixed).unwrap();
    assert_eq!(mf.f[..6], m.f[..6]);
    assert_eq!(mf.f[6..], [7.0, 11.0, 13.0]);
    // errors
    let mut bad = model_fixture();
    bad.tr.rexist.pop();
    bad.tr.r.pop();
    assert!(err_text(build(&bad)).contains("NW5:TRSize"));
    let mut blocked = model_fixture();
    blocked.arcs[0].state = ArcState::Blocked;
    let e = err_text(build(&blocked));
    assert!(
        e.contains("NW5:ReinforcementBlocked") && e.contains('J'),
        "{e}"
    );
    let mut unsupported = model_fixture();
    unsupported.p.bounded_flow = false;
    unsupported.p.uniform_dem = false;
    assert!(build(&unsupported).is_err());
    // known MILP optimum
    let model = build(&f).unwrap();
    assert!(model.is_milp());
    let sol = solve_model(&model).unwrap();
    assert!(close(sol.fval, 50.0, 1e-9));
    assert_close(&sol.z, &[0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], 1e-9);
    let act = model.ineq_activity(&sol.z);
    assert!(act.iter().zip(&model.b_ineq).all(|(a, b)| *a <= b + 1e-8));
}

fn check_mandatory_speed_reinforcement() {
    let mut f = model_fixture();
    for a in &mut f.arcs {
        a.state = ArcState::New;
    }
    // D1 carries about 1.9635 L/s at 1 m/s
    f.tlv.values[1] = vec![2.0, 2.0];
    let m = build(&f).unwrap();
    assert_eq!(m.lb[6..], [1.0, 1.0, 0.0]);
    let previous = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    assert!(
        m.ineq_activity(&previous)
            .iter()
            .zip(&m.b_ineq)
            .all(|(a, b)| *a <= b + 1e-8)
    );
    assert!(m.eq_activity(&previous).iter().all(|v| v.abs() <= 1e-9));

    let mut blocked = model_fixture();
    blocked.arcs[0].state = ArcState::New;
    blocked.arcs[2].state = ArcState::New;
    blocked.arcs[1].state = ArcState::Blocked;
    blocked.tlv.values[1] = vec![2.0, 2.0];
    let e = err_text(build(&blocked));
    assert!(
        e.contains("NW5:ReinforcementBlocked") && e.contains("B1"),
        "{e}"
    );

    // a short small second section controls the existing capacity
    let mut split = model_fixture();
    for a in &mut split.arcs {
        a.state = ArcState::New;
    }
    split.tlv.values[1] = vec![2.0, 2.0];
    split.arcs[1].segments = [seg(19.9, Some(1)), seg(0.1, Some(0))];
    let rex = (19.9 * 1e4 + 0.1 * 1e6) / 20.0;
    split.tr = tr_for(&[f64::INFINITY, rex, 1e6], &split.catalog, 2.0);
    assert_eq!(build(&split).unwrap().lb[7], 1.0);
    // a fictitious zero-length section does not limit an existing D2
    split.arcs[1].segments = [seg(20.0, Some(1)), seg(0.0, None)];
    split.tr = tr_for(&[f64::INFINITY, 1e4, 1e6], &split.catalog, 2.0);
    assert_eq!(build(&split).unwrap().lb[7], 0.0);
    split.arcs[1].state = ArcState::Blocked;
    let m = build(&split).unwrap();
    assert_eq!((m.lb[7], m.ub[7]), (0.0, 0.0));
    // equipment carried by lon2 only
    let mut lon2 = model_fixture();
    for a in &mut lon2.arcs {
        a.state = ArcState::New;
    }
    lon2.tlv.values[1] = vec![2.0, 2.0];
    lon2.tlv.values[2] = vec![2.0, 2.0];
    assert_eq!(build(&lon2).unwrap().lb[8], 1.0);
}

// ---------------------------------------------------------------- 8.3

fn sparse(rows: &[&[f64]]) -> Vec<Vec<(usize, f64)>> {
    rows.iter()
        .map(|r| {
            r.iter()
                .enumerate()
                .filter(|(_, v)| **v != 0.0)
                .map(|(j, v)| (j, *v))
                .collect()
        })
        .collect()
}

fn solve_fixture() -> LpModel {
    LpModel {
        nb_a: 3,
        nb_d: 2,
        f: vec![2.0, 6.0, 3.0, 9.0, 4.0, 8.0, 5.0, 7.0, 11.0],
        a_eq: sparse(&[
            &[1.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0],
            &[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0],
        ]),
        b_eq: vec![0.0; 3],
        a_ineq: sparse(&[
            &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
        ]),
        b_ineq: vec![0.25, -2.0, 0.0],
        lb: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        ub: vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
    }
}

fn check_solution(m: &LpModel, sol: &Solution, expected: &[f64], cost: f64) {
    assert_eq!(sol.status, "Optimal");
    assert_eq!(sol.x.len(), 3);
    assert!(sol.x.iter().all(|r| r.len() == 2));
    let revised: Vec<f64> = sol
        .x
        .concat()
        .into_iter()
        .chain(sol.y.iter().copied())
        .collect();
    for (a, b) in revised.iter().zip(expected) {
        assert!((a - b).abs() < 1e-7, "{revised:?} vs {expected:?}");
    }
    assert!(sol.y.iter().all(|&v| v == 0.0 || v == 1.0));
    assert_eq!(
        sol.reinforced,
        expected[6..].iter().map(|&v| v == 1.0).collect::<Vec<_>>()
    );
    for a in 0..3 {
        assert!((sol.x[a].iter().sum::<f64>() - sol.y[a]).abs() < 1e-7);
    }
    assert!(m.eq_activity(&revised).iter().all(|v| v.abs() < 1e-7));
    assert!((0..9).all(|j| revised[j] >= m.lb[j] - 1e-7 && revised[j] <= m.ub[j] + 1e-7));
    let slack: Vec<f64> = m
        .b_ineq
        .iter()
        .zip(m.ineq_activity(&revised))
        .map(|(b, a)| b - a)
        .collect();
    assert_close(&sol.slack, &slack, 1e-7);
    assert!(sol.slack.iter().all(|&s| s >= -1e-6));
    assert!((sol.new_fval - m.objective(&revised)).abs() < 1e-7);
    assert!((sol.new_fval - cost).abs() < 1e-7);
    assert!((sol.fval - m.objective(&sol.z)).abs() < 1e-7);
    assert!((sol.fval - cost).abs() < 1e-6);
}

#[test]
fn test_solve_model() {
    let expected = [0.25, 0.75, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    let model = solve_fixture();
    let sol = solve_model(&model).unwrap();
    assert!(sol.milp);
    check_solution(&model, &sol, &expected, 20.0);

    let mut lp = solve_fixture();
    lp.lb[6..].copy_from_slice(&[1.0, 1.0, 0.0]);
    let sol = solve_model(&lp).unwrap();
    assert!(!sol.milp);
    check_solution(&lp, &sol, &expected, 20.0);

    let mut optional = solve_fixture();
    optional.b_ineq[1] = 0.0;
    let sol = solve_model(&optional).unwrap();
    check_solution(
        &optional,
        &sol,
        &[0.25, 0.75, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        10.0,
    );

    let mut none = optional.clone();
    none.lb[6..].fill(0.0);
    none.ub[6..].fill(0.0);
    let sol = solve_model(&none).unwrap();
    check_solution(&none, &sol, &[0.0; 9], 0.0);

    let mut tiny = lp.clone();
    tiny.a_ineq[0] = vec![(1, 1.0)];
    tiny.b_ineq[0] = 1e-9;
    tiny.f[0] = 6.0;
    tiny.f[1] = 2.0;
    let sol = solve_model(&tiny).unwrap();
    check_solution(
        &tiny,
        &sol,
        &[1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
        21.0,
    );

    for fixed in [false, true] {
        let mut bad = solve_fixture();
        if fixed {
            bad.lb[6..].copy_from_slice(&[1.0, 1.0, 0.0]);
        }
        bad.a_ineq[0] = vec![(0, 1.0), (1, 1.0)];
        bad.b_ineq[0] = 0.5;
        let e = err_text(solve_model(&bad));
        assert!(
            e.contains("NW5:DesignFailure") && e.contains("Solver diagnostic"),
            "{e}"
        );
    }
}

// ---------------------------------------------------------------- 8.4

fn check_mixture(x: &[f64], rho: &[f64], cost: &[f64]) {
    let result = reduce_pipe_mixture(x, rho, cost).unwrap();
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(u, v)| u * v).sum::<f64>();
    let target = dot(rho, x);
    let mut best = f64::INFINITY;
    for a in 0..x.len() {
        if (rho[a] - target).abs() < 1e-12 {
            best = best.min(cost[a]);
        }
        for b in a + 1..x.len() {
            if rho[a] == rho[b] {
                continue;
            }
            let w = (target - rho[b]) / (rho[a] - rho[b]);
            if (0.0..=1.0).contains(&w) {
                best = best.min(w * cost[a] + (1.0 - w) * cost[b]);
            }
        }
    }
    assert!(result.iter().filter(|v| **v != 0.0).count() <= 2 && result.iter().all(|v| *v >= 0.0));
    assert!((result.iter().sum::<f64>() - x.iter().sum::<f64>()).abs() < 1e-12);
    assert!((dot(rho, &result) - target).abs() < 1e-10 * target.abs().max(1.0));
    assert!(
        (dot(cost, &result) - best).abs() < 1e-10 * best.abs().max(1.0),
        "{result:?} {best}"
    );
}

#[test]
fn test_reduce_pipe_mixture() {
    check_mixture(&[0.2, 0.5, 0.3], &[10.0, 5.0, 1.0], &[1.0, 9.0, 10.0]);
    check_mixture(&[0.2, 0.5, 0.3], &[10.0, 5.0, 1.0], &[1.0, 3.0, 10.0]);
    check_mixture(&[0.2, 0.5, 0.3], &[5.0, 5.0, 5.0], &[7.0, 2.0, 8.0]);
    check_mixture(&[0.2, 0.5, 0.3], &[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]);
    let mut rng = Twister::new(42);
    for _ in 0..100 {
        let n = 3 + (rng.next_f64() * 13.0) as usize;
        let mut x: Vec<f64> = (0..n).map(|_| rng.next_f64()).collect();
        let s: f64 = x.iter().sum();
        x.iter_mut().for_each(|v| *v /= s);
        let rho: Vec<f64> = (0..n)
            .map(|_| 1.0 + (rng.next_f64() * 20.0).floor())
            .collect();
        let cost: Vec<f64> = (0..n).map(|_| 20.0 * rng.next_f64()).collect();
        check_mixture(&x, &rho, &cost);
        let lambda: f64 = 1.781;
        let eq: Vec<f64> = rho
            .iter()
            .map(|r| (r.powf(-1.0 / lambda) + 7f64.powf(-1.0 / lambda)).powf(-lambda))
            .collect();
        check_mixture(&x, &eq, &cost);
    }
}

// ---------------------------------------------------------------- 8.5

fn one_arc() -> (Vec<DesignArc>, PathMatrix) {
    let mut a = arc("S", "B1", 1.0);
    a.status = NodeStatus::Terminal;
    (
        vec![a],
        PathMatrix {
            paths: vec![vec![0]],
        },
    )
}

fn clients(rows: &[(&str, f64, f64)]) -> Vec<Client> {
    rows.iter()
        .enumerate()
        .map(|(i, (t, p, q))| Client {
            id: format!("C{}", i + 1),
            terminal: t.to_string(),
            probability: *p,
            target_flow: *q,
        })
        .collect()
}

fn loads_of(
    arcs: &[DesignArc],
    paths: &PathMatrix,
    c: &[Client],
    p: &DesignParams,
) -> neatwork_nw5::Result<LoadTable> {
    incidence_loads(
        arcs,
        paths,
        c,
        p,
        p.servicequal,
        LoadMethod::Auto,
        &mut Twister::new(42),
    )
}

#[test]
fn test_build_incidence_loads_cvar() {
    let (arcs, paths) = one_arc();
    let tc = clients(&[("B1", 1.0, 1.0), ("B1", 0.3, 2.0), ("B1", 0.7, 4.0)]);
    let mut p = DesignParams {
        uniform_dem: false,
        n_scen_load: 101,
        servicequal: 0.65,
        ..DesignParams::default()
    };
    let var = loads_of(&arcs, &paths, &tc, &p).unwrap();
    assert_eq!(var.arcs, ["B1"]);
    // the same draw as MATLAB rng(42); rand(3,101) (column-major)
    let mut rng = Twister::new(42);
    let mut totals: Vec<f64> = (0..101)
        .map(|_| {
            tc.iter()
                .map(|c| {
                    if rng.next_f64() < c.probability {
                        c.target_flow
                    } else {
                        0.0
                    }
                })
                .sum()
        })
        .collect();
    totals.sort_by(|a, b| b.total_cmp(a));
    p.use_cvar = true;
    for alpha in [0.5, 0.65, 0.95, 1.0] {
        p.servicequal = alpha;
        let tl = loads_of(&arcs, &paths, &tc, &p).unwrap();
        let expected = if alpha == 1.0 {
            totals[0]
        } else {
            let mass = (1.0 - alpha) * 101.0;
            let whole = mass.floor() as usize;
            (totals[..whole].iter().sum::<f64>() + (mass - whole as f64) * totals[whole]) / mass
        };
        assert!(
            (tl.values[0][0] - expected).abs() < 1e-12,
            "{alpha}: {} vs {expected}",
            tl.values[0][0]
        );
        if alpha == 0.65 {
            assert!(tl.values[0][0] >= var.values[0][0]);
        }
    }
    let all = clients(&[("B1", 1.0, 1.0), ("B1", 1.0, 2.0), ("B1", 1.0, 4.0)]);
    p.servicequal = 0.95;
    assert!((loads_of(&arcs, &paths, &all, &p).unwrap().values[0][0] - 7.0).abs() < 1e-12);

    // exact binomial branch against the discrete upper tail
    let one = clients(&[("B1", 1.0, 1.0)]);
    p.uniform_dem = true;
    p.targetflow = 0.4;
    for n in [0usize, 1, 4, 20] {
        let mut arcs = arcs.clone();
        arcs[0].nbrob = (n + 1) as f64;
        for prob in [0.01, 0.3, 0.95, 1.0] {
            p.opentaps = prob;
            let pmf: Vec<f64> = (0..=n)
                .map(|k| {
                    let mut c = 1.0;
                    for i in 0..k {
                        c = c * (n - i) as f64 / (i + 1) as f64;
                    }
                    c * prob.powi(k as i32) * (1.0 - prob).powi((n - k) as i32)
                })
                .collect();
            for alpha in [0.5, 0.65, 0.95, 0.999, 1.0] {
                p.servicequal = alpha;
                let tl = loads_of(&arcs, &paths, &one, &p).unwrap();
                let expected = if alpha == 1.0 {
                    1.0 + n as f64
                } else {
                    let mut remaining = 1.0 - alpha;
                    let mut tail = 0.0;
                    for k in (0..=n).rev() {
                        let take = remaining.min(pmf[k]);
                        tail += take * (1.0 + k as f64);
                        remaining = (remaining - take).max(0.0);
                    }
                    tail / (1.0 - alpha)
                };
                let got = tl.values[0][0] / 0.4;
                assert!(
                    (got - expected).abs() < 1e-9,
                    "n {n} p {prob} a {alpha}: {got} vs {expected}"
                );
            }
        }
    }
    check_uniform_probability_bounds();
}

fn check_uniform_probability_bounds() {
    let mut arcs = vec![arc("S", "J", 1.0), arc("J", "B1", 1.0), arc("J", "B2", 1.0)];
    for (a, n) in arcs.iter_mut().zip([0.0, 2.0, 1.0]) {
        a.nbrob = n;
    }
    arcs[1].status = NodeStatus::Terminal;
    arcs[2].status = NodeStatus::Terminal;
    let tc = clients(&[("B1", 1.0, 0.4), ("B1", 1.0, 0.4), ("B2", 1.0, 0.4)]);
    let mut p = DesignParams {
        uniform_dem: true,
        targetflow: 0.4,
        opentaps: 1.0,
        ..DesignParams::default()
    };
    for cvar in [false, true] {
        p.use_cvar = cvar;
        p.opentaps = 1.0;
        for alpha in [0.5, 0.95, 1.0] {
            p.servicequal = alpha;
            let tl = loads_of(&arcs, &tree3(), &tc, &p).unwrap();
            assert_eq!(tl.arcs, ["J", "B1", "B2"]);
            assert_eq!(tl.terminals, ["B1", "B2"]);
            assert_close(&tl.values.concat(), &[1.2, 1.2, 0.8, 0.0, 0.0, 0.4], 1e-12);
        }
        p.opentaps = 0.0;
        assert!(err_text(loads_of(&arcs, &tree3(), &tc, &p)).contains("NW5:LoadProbability"));
        let single = vec![arcs[2].clone()];
        let e = loads_of(
            &single,
            &PathMatrix {
                paths: vec![vec![0]],
            },
            &tc[2..],
            &p,
        );
        assert!(err_text(e).contains("NW5:LoadProbability"));
    }
}

// ---------------------------------------------------------------- 8.6

#[test]
fn test_equiv_resist() {
    for lambda in [1.2, 1.781, 2.0] {
        let catalog = Catalog::with_resistances(
            &[("D1", 0.1, 0.0, 0.0, 16.0), ("D2", 0.2, 0.0, 0.0, 4.0)],
            lambda,
        );
        let mk = |s: &str, s1: Segment, s2: Segment| {
            let mut a = arc("S", s, 100.0);
            a.segments = [s1, s2];
            a
        };
        let none = Segment::default();
        let arcs = vec![
            mk("Z", none, none),
            mk("A", seg(100.0, None), none),
            mk("Y", seg(100.0, Some(0)), none),
            mk("B", seg(25.0, Some(0)), seg(75.0, Some(1))),
            mk("X", seg(25.0, Some(0)), seg(75.0, None)),
            mk("C", none, seg(100.0, Some(1))),
        ];
        let tr = equiv_resist(&arcs, &catalog, lambda).unwrap();
        assert_eq!(tr.r.len(), 6);
        for a in 0..2 {
            assert_eq!(tr.rexist[a], f64::INFINITY);
            assert_eq!(tr.r[a], [16.0, 4.0]);
        }
        let existing = [16.0, 7.0, 4.0, 4.0];
        for (k, a) in (2..6).enumerate() {
            assert!((tr.rexist[a] - existing[k]).abs() < 1e-12);
            for (d, rd) in [16.0f64, 4.0].iter().enumerate() {
                let ratio = (rd / existing[k]).powf(1.0 / lambda);
                let expected = existing[k] * (ratio / (1.0 + ratio)).powf(lambda);
                let candidate = rd * (1.0 / (1.0 + ratio)).powf(lambda);
                assert!((expected - candidate).abs() < 1e-12);
                assert!((tr.r[a][d] - expected).abs() < 1e-12 * expected.max(1.0));
                assert!(tr.r[a][d] < existing[k].min(*rd));
            }
        }
        let mut bad = arcs.clone();
        bad[2].segments[0].length = 99.0;
        assert!(err_text(equiv_resist(&bad, &catalog, lambda)).contains("NW5:EquipmentLength"));
    }
}

// ---------------------------------------------------------------- 8.7

fn pressure_fixture() -> (Vec<DesignArc>, Catalog) {
    let catalog = Catalog::with_resistances(
        &[("D20", 0.02, 1.0, 20.0, 1.0), ("D30", 0.03, 1.0, 30.0, 1.0)],
        2.0,
    );
    let mut a = vec![
        arc("S", "J", 10.0),
        arc("J", "B1", 10.0),
        arc("J", "B2", 10.0),
    ];
    a[0].pmax = 100.0;
    a[1].pmax = 30.0;
    a[1].segments[0] = seg(10.0, Some(1));
    a[2].pmax = 20.0;
    a[2].segments[1] = seg(10.0, Some(0));
    a[2].state = ArcState::Blocked;
    (a, catalog)
}

fn must_reject(arcs: &[DesignArc], catalog: &Catalog, texts: &[&str]) {
    let e = err_text(validate_existing_pressure(arcs, catalog));
    assert!(e.contains("NW5:ExistingPressure"), "{e}");
    for t in texts {
        assert!(e.contains(t), "missing {t} in {e}");
    }
}

fn pump_network(pump: f64, class: f64) -> neatwork_nw5::Result<()> {
    let mut net = read_tpo(
        "pump",
        "!Nodes\nS,100,0,0,0,0\nJ,110,1,0,0,1\nB,90,2,0,1,2\n!Pipes\nS,J,20\nJ,B,20\n!Default properties\ntargetflow,0.1\n",
    )
    .unwrap()
    .network;
    let pipe = |id: &str, d: f64, c: f64| Pipe {
        id: id.into(),
        nominal: id.into(),
        sdr: "1".into(),
        diameter: d,
        cost: 1.0,
        max_pressure: c,
        kind: "1".into(),
        roughness: 0.0,
    };
    net.pipes = vec![pipe("P15", 0.05, class), pipe("P100", 0.06, 100.0)];
    net.arcs[0].length1 = 20.0;
    net.arcs[0].pipe1 = Some("P15".into());
    net.arcs[1].length1 = 20.0;
    net.arcs[1].pipe1 = Some("P100".into());
    let rev = review(
        &net,
        &[("pump_pressure".into(), pump), ("pressure".into(), 0.0)],
        None,
        None,
    )?;
    let prep = prepare(&net, &rev.clients, &rev.params, &mut Vec::new())?;
    if pump == 20.0 {
        assert_eq!(prep.arcs.iter().find(|a| a.pred == "S").unwrap().pmax, 20.0);
    }
    Ok(())
}

#[test]
fn test_existing_pressure() {
    for permuted in [false, true] {
        let (mut arcs, catalog) = pressure_fixture();
        if permuted {
            arcs = vec![arcs[2].clone(), arcs[0].clone(), arcs[1].clone()];
        }
        validate_existing_pressure(&arcs, &catalog).unwrap();
        let mut first = arcs.clone();
        first.iter_mut().find(|a| a.succ == "B1").unwrap().pmax = 31.0;
        must_reject(
            &first,
            &catalog,
            &["J", "B1", "D30", "31 m", "30 m", "troncon 1"],
        );
        let mut second = arcs.clone();
        let b2 = second.iter_mut().find(|a| a.succ == "B2").unwrap();
        assert_eq!(b2.state, ArcState::Blocked);
        b2.pmax = 21.0;
        must_reject(
            &second,
            &catalog,
            &["J", "B2", "D20", "21 m", "20 m", "troncon 2"],
        );
    }
    let (mut arcs, catalog) = pressure_fixture();
    arcs[1].segments[1] = seg(0.0, Some(0));
    validate_existing_pressure(&arcs, &catalog).unwrap();
    let mut two = arcs.clone();
    two[1].segments = [seg(5.0, Some(1)), seg(5.0, Some(0))];
    two[1].pmax = 31.0;
    must_reject(
        &two,
        &catalog,
        &[
            "J",
            "B1",
            "D30",
            "D20",
            "troncon 1",
            "troncon 2",
            "31 m",
            "30 m",
            "20 m",
        ],
    );
    let mut negative = arcs.clone();
    negative.iter_mut().for_each(|a| a.pmax = -1.0);
    validate_existing_pressure(&negative, &catalog).unwrap();
    validate_existing_pressure(&arcs[..1], &Catalog::default()).unwrap();
    let mut unknown = catalog.clone();
    unknown.pipes[1].max_pressure = f64::NAN;
    assert!(
        err_text(validate_existing_pressure(&arcs, &unknown)).contains("NW5:ExistingPressureData")
    );
    let mut unlimited = catalog.clone();
    unlimited.pipes[1].max_pressure = f64::INFINITY;
    validate_existing_pressure(&arcs, &unlimited).unwrap();
    // source pump
    pump_network(0.0, 15.0).unwrap();
    let e = err_text(pump_network(20.0, 15.0));
    for t in [
        "NW5:ExistingPressure",
        "S",
        "J",
        "P15",
        "20 m",
        "15 m",
        "troncon 1",
    ] {
        assert!(e.contains(t), "{t} in {e}");
    }
    pump_network(20.0, 20.0).unwrap();
    // an unknown existing pipe
    let mut net = integration_network(0.0, false);
    net.arcs[1].pipe1 = Some("inconnu".into());
    let rev = review(&net, &[], None, None).unwrap();
    assert!(
        err_text(prepare(&net, &rev.clients, &rev.params, &mut Vec::new()))
            .contains("NW5:UnknownPipe")
    );
    // B1: segments that sum to the length are accepted, others rejected
    let mut net = integration_network(0.0, false);
    net.arcs[1].length1 = 19.0;
    let rev = review(&net, &[], None, None).unwrap();
    assert!(
        err_text(prepare(&net, &rev.clients, &rev.params, &mut Vec::new()))
            .contains("NW5:EquipmentLength")
    );
}

// ---------------------------------------------------------------- 8.8

#[test]
fn test_physical_elevations() {
    for h in [100.0, 350.0] {
        for pump in [0.0, 20.0] {
            for permuted in [false, true] {
                let mut text = format!(
                    "!Nodes\nS,{},0,0,0,0\nJ,{},500000.125,200000.25,0,1\nB,{},1,1,1,2\n!Pipes\nS,J,10\nJ,B,10\n!Default properties\ntargetflow,0.1\n!Diameters\nD1,P,1,0.05,1,1000,1,0\n",
                    h,
                    h + 10.0,
                    h - 10.0
                );
                if permuted {
                    text = text.replace("S,J,10\nJ,B,10", "J,B,10\nJ,S,10");
                }
                let net = read_tpo("e", &text).unwrap().network;
                let rev = review(&net, &[("pump_pressure".into(), pump)], None, None).unwrap();
                let prep = prepare(&net, &rev.clients, &rev.params, &mut Vec::new()).unwrap();
                let haut: Vec<f64> = prep.arcs.iter().map(|a| a.haut).collect();
                let pmax: Vec<f64> = prep.arcs.iter().map(|a| a.pmax).collect();
                assert_close(&haut, &[10.0 - pump, -10.0 - pump], 1e-10);
                assert_close(&pmax, &[pump, 10.0 + pump], 1e-10);
                assert_eq!(prep.arcs[0].pred, "S");
                assert_eq!(net.nodes[1].x, 500000.125);
            }
        }
    }
}

// ---------------------------------------------------------------- 8.9

fn smoke_inputs(bare: bool) -> (Vec<DesignArc>, Catalog, Tr, LoadTable, DesignParams) {
    let p = DesignParams {
        targetflow: 0.4,
        pipelength: 6.0,
        secu: 1.0,
        uniform_dem: true,
        bounded_flow: true,
        bescal: 9.42e-4 * 1.27f64.powf(-0.219),
        nu: 4.781,
        lambda: 1.781,
        faucetcoef: 1.83e-7,
        orifcoef: 0.001,
        ..DesignParams::default()
    };
    let catalog = Catalog::new(
        &[(0.05, 5.0, "D1"), (0.10, 8.0, "D2")].map(|(d, c, id)| Pipe {
            id: id.into(),
            nominal: id.into(),
            sdr: "1".into(),
            diameter: d,
            cost: c,
            max_pressure: 1000.0,
            kind: "1".into(),
            roughness: 0.0,
        }),
        &p,
    )
    .unwrap();
    let mut arcs = vec![
        arc("S", "J", 60.0),
        arc("J", "B1", 30.0),
        arc("J", "B2", 42.0),
    ];
    for (a, z) in arcs.iter_mut().zip([-2.0, -5.0, -6.0]) {
        a.haut = z;
    }
    arcs[1].status = NodeStatus::Terminal;
    arcs[2].status = NodeStatus::Terminal;
    if !bare {
        arcs[1].segments[0] = seg(30.0, Some(1));
        arcs[2].segments[0] = seg(42.0, Some(1));
    }
    let tr = equiv_resist(&arcs, &catalog, p.lambda).unwrap();
    let tl = table(
        &["J", "B1", "B2"],
        &["B1", "B2"],
        &[&[0.7, 0.5], &[0.7, 0.0], &[0.0, 0.5]],
    );
    (arcs, catalog, tr, tl, p)
}

fn smoke_solution(x: [[f64; 2]; 3], y: [f64; 3]) -> Solution {
    Solution {
        x: x.iter().map(|r| r.to_vec()).collect(),
        y: y.to_vec(),
        fval: 200.0,
        slack: vec![0.0; 3],
        ..Solution::default()
    }
}

#[test]
fn test_process_sol_smoke() {
    let (arcs, catalog, tr, tl, p) = smoke_inputs(false);
    let orifices = [0.004, 0.005];
    let paths = tree3();
    let input = PostInput {
        arcs: &arcs,
        paths: &paths,
        catalog: &catalog,
        tr: &tr,
        loads: &tl,
        params: &p,
        orifices: &orifices,
    };
    let sol = smoke_solution([[1.0, 0.0], [0.4, 0.6], [0.0, 0.0]], [1.0, 1.0, 0.0]);
    let out = process_solution(input, &sol).unwrap();
    assert_eq!(out.out_arcs.len(), 4);
    assert_eq!(
        out.tcp.iter().filter(|t| t.nature == Nature::Old).count(),
        2
    );
    assert_eq!(
        out.tcp.iter().filter(|t| t.nature == Nature::New).count(),
        2
    );
    assert_eq!(
        out.tcp
            .iter()
            .filter(|t| t.pred == "J" && t.succ == "B1")
            .count(),
        2
    );
    assert!(out.out_arcs.iter().all(|o| o.segments[0].pipe.is_some()));
    for o in &out.out_arcs {
        assert!((o.segments[0].length + o.segments[1].length - arcs[o.arc].length).abs() < 1e-9);
    }
    let req: Vec<f64> = out.tah.iter().map(|t| t.equiv_resist).collect();
    assert!(req[1] < tr.rexist[1]);
    assert!((req[2] - tr.rexist[2]).abs() < 1e-12 * tr.rexist[2]);
    assert!(
        out.tah
            .iter()
            .all(|t| t.equiv_resist.is_finite() && t.margin.is_finite())
    );
    let margins: Vec<f64> = out.tah.iter().map(|t| t.margin).collect();
    assert_eq!(
        out.ts.iter().map(|t| t.head_margin).collect::<Vec<_>>(),
        margins
    );
    // values derived for the specification (1e-9 relative)
    assert_close(
        &catalog.pipes.iter().map(|q| q.resist).collect::<Vec<_>>(),
        &[1484.370440077769, 53.99056694027162],
        1e-9,
    );
    assert_close(
        &req,
        &[1484.370440077769, 15.710263367299225, 53.99056694027162],
        1e-9,
    );
    assert_close(
        &out.tah.iter().map(|t| t.path_loss).collect::<Vec<_>>(),
        &[0.21419688800488, 0.21533039532629, 0.21719213579228],
        1e-9,
    );
    assert_close(
        &margins,
        &[1.78580311199512, 3.91035266478300, 4.90849092431701],
        1e-9,
    );
    assert!(close(out.costs.optim, 504.0, 1e-9));
    assert!(close(out.costs.inter, 531.8611816471366, 1e-9));
    assert!(close(out.costs.final_cost, 540.0, 1e-9));
    let b1_new = out
        .out_arcs
        .iter()
        .find(|o| o.arc == 1 && o.nature == Nature::New)
        .unwrap();
    assert_eq!(b1_new.segments, [seg(30.0, Some(1)), seg(0.0, None)]);
    assert_eq!(b1_new.state, ArcState::Reinforcement);

    // a bare arc that is not reinforced
    let bad = smoke_solution([[0.0, 0.0], [0.4, 0.6], [0.0, 0.0]], [0.0, 1.0, 0.0]);
    assert!(err_text(process_solution(input, &bad)).contains("NW5:DisconnectedArc"));

    // existing pipe on lon2 only
    let mut arcs2 = arcs.clone();
    arcs2[2].segments = [seg(0.0, None), seg(42.0, Some(1))];
    let out2 = process_solution(
        PostInput {
            arcs: &arcs2,
            ..input
        },
        &sol,
    )
    .unwrap();
    assert_eq!(out2.out_arcs.len(), 4);
    assert!(
        out2.out_arcs
            .iter()
            .any(|o| o.segments == [seg(0.0, None), seg(42.0, Some(1))])
    );

    // all arcs bare
    let (arcs0, _, tr0, _, _) = smoke_inputs(true);
    let sol0 = smoke_solution([[1.0, 0.0], [1.0, 0.0], [1.0, 0.0]], [1.0, 1.0, 1.0]);
    let out0 = process_solution(
        PostInput {
            arcs: &arcs0,
            tr: &tr0,
            ..input
        },
        &sol0,
    )
    .unwrap();
    assert_eq!(out0.out_arcs.len(), 3);
    assert!(out0.out_arcs.iter().all(|o| o.segments[0].pipe == Some(0)));
}

// ---------------------------------------------------------------- 8.10

#[test]
fn test_constraint_satisfaction() {
    let lambda = 2.0;
    let catalog = Catalog::with_resistances(&[("D1", 0.05, 5.0, 1000.0, 1e5)], lambda);
    let mut arcs = vec![arc("S", "J", 1.0), arc("J", "B1", 1.0), arc("J", "B2", 1.0)];
    for (a, (z, t, pr)) in
        arcs.iter_mut()
            .zip([(-2.0, false, 0.0), (-10.0, true, 1.0), (1.0, true, 1.0)])
    {
        a.haut = z;
        a.status = if t {
            NodeStatus::Terminal
        } else {
            NodeStatus::Junction
        };
        a.pressure = pr;
        a.segments[0] = seg(1.0, Some(0));
    }
    let tr = Tr {
        rexist: vec![1e5; 3],
        r: vec![vec![25000.0]; 3],
    };
    let tl = table(&["J", "B1", "B2"], &["B"], &[&[1.0], &[1.0], &[1.0]]);
    let p = DesignParams {
        targetflow: 0.4,
        pipelength: 1.0,
        secu: 1.2,
        uniform_dem: true,
        bounded_flow: true,
        lambda,
        faucetcoef: 1.83e-7,
        orifcoef: 0.001,
        ..DesignParams::default()
    };
    let paths = tree3();
    let sol = Solution {
        x: vec![vec![0.0]; 3],
        y: vec![0.0; 3],
        ..Solution::default()
    };
    let input = PostInput {
        arcs: &arcs,
        paths: &paths,
        catalog: &catalog,
        tr: &tr,
        loads: &tl,
        params: &p,
        orifices: &[0.005],
    };
    let out = process_solution(input, &sol).unwrap();
    let ts = &out.ts;
    assert_eq!(
        ts.iter().map(|t| t.node.as_str()).collect::<Vec<_>>(),
        ["J", "B1", "B2"]
    );
    assert_eq!(
        ts.iter().map(|t| t.constraint_type).collect::<Vec<_>>(),
        [
            ConstraintType::IntrusionPrevention,
            ConstraintType::DemandSatisfaction,
            ConstraintType::DemandSatisfaction
        ]
    );
    assert_eq!(ts[0].constraint_type.as_str(), "intrusion_prevention");
    let avail = [2.0 - 0.1, 10.0 / 1.2 - 0.2, -1.0 / 1.2 - 0.2];
    for (t, a) in ts.iter().zip(avail) {
        assert!((t.head_available - a).abs() < 1e-12);
    }
    let req = 1.0 + 0.0004f64.powi(2) / 1.83e-7;
    for (t, r) in ts.iter().zip([0.0, req, req]) {
        assert!((t.head_required - r).abs() < 1e-12);
        assert_eq!(t.head_deficit, (-t.head_margin).max(0.0));
    }
    assert_eq!(
        ts.iter().map(|t| t.head_margin).collect::<Vec<_>>(),
        out.tah.iter().map(|t| t.margin).collect::<Vec<_>>()
    );
    assert!(ts[1].head_margin > 0.0 && ts[2].head_margin < 0.0);
    assert!(
        out.warnings
            .iter()
            .any(|w| w.contains("NW5:PressureAfterRounding"))
    );
}

// ---------------------------------------------------------------- 8.11

fn precedence_file(dsg: bool, clients: &[&str], uniform_flag: u8) -> Network {
    let nodes = if dsg {
        "!Nodes\nB2,70,2,-1,0.006,0.007,2\nS,100,0,0,1,1,0\nB1,80,2,1,0.004,0.005,2\nJ,90,1,0,1,1,1\n"
    } else if clients.is_empty() {
        "!Nodes\nB2,70,2,-1,1,2\nS,100,0,0,0,0\nB1,80,2,1,2,2\nJ,90,1,0,0,1\n"
    } else {
        "!Nodes\nB2,70,2,-1,9,2\nS,100,0,0,0,0\nB1,80,2,1,17,2\nJ,90,1,0,4,1\n"
    };
    let pipes = if dsg {
        "!Pipes\nJ,B2,100,100,D1,0,fictif,N\nS,J,100,100,D1,0,fictif,N\nJ,B1,100,100,D1,0,fictif,N\n"
    } else {
        "!Pipes\nJ,B2,100\nS,J,100\nJ,B1,100\n"
    };
    let mut text = format!(
        "{nodes}{pipes}!Default properties\nopentaps,0.1\ntargetflow,9\nuniform_dem,{uniform_flag}\nbounded_flow,1\npressure,2\n!Diameters\nD1,P50,11,0.05,2,1000,1,0{}\n",
        if dsg { ",N" } else { "" }
    );
    if !clients.is_empty() {
        text.push_str("!Clients\n");
        text.push_str(&clients.join("\n"));
        text.push('\n');
    }
    if dsg {
        read_dsg("p", &text)
    } else {
        read_tpo("p", &text)
    }
    .unwrap()
    .network
}

#[test]
fn test_client_data_precedence() {
    let uniform = ["C3,B2,0.7,0.6", "C2,B1,0.7,0.6", "C1,B1,0.7,0.6"];
    let hetero = ["C3,B2,0.2,0.5", "C2,B1,0.8,0.4", "C1,B1,0.2,1"];
    let counts = |net: &Network| -> Vec<f64> {
        ["S", "J", "B1", "B2"]
            .iter()
            .map(|id| net.node(id).unwrap().taps.unwrap())
            .collect()
    };
    for dsg in [false, true] {
        let net = precedence_file(dsg, &uniform, 0);
        assert_eq!(counts(&net), [0.0, 0.0, 2.0, 1.0]);
        let rev = review(&net, &[], None, None).unwrap();
        assert!(rev.params.uniform_dem);
        assert_eq!((rev.params.opentaps, rev.params.targetflow), (0.7, 0.6));
        let net = precedence_file(dsg, &hetero, 1);
        assert_eq!(counts(&net), [0.0, 0.0, 2.0, 1.0]);
        let rev = review(&net, &[], None, None).unwrap();
        assert!(!rev.params.uniform_dem);
        assert_eq!((rev.params.opentaps, rev.params.targetflow), (0.1, 9.0));
        let probs: Vec<f64> = rev.clients.iter().map(|c| c.probability).collect();
        assert_eq!(probs, [0.2, 0.8, 0.2]);
        if dsg {
            let b1 = net.node("B1").unwrap();
            let b2 = net.node("B2").unwrap();
            assert_eq!((b1.orifice_ideal, b2.orifice_ideal), (0.004, 0.006));
            assert_eq!(
                (b1.orifice_commercial, b2.orifice_commercial),
                (0.005, 0.007)
            );
        }
    }
    let legacy = precedence_file(false, &[], 0);
    assert_eq!(counts(&legacy), [0.0, 0.0, 2.0, 1.0]);
    let rev = review(&legacy, &[], None, None).unwrap();
    assert_eq!(rev.clients.len(), 3);
    assert!(rev.params.uniform_dem);
    assert!(
        rev.clients
            .iter()
            .all(|c| c.probability == 0.1 && c.target_flow == 9.0)
    );
    assert_eq!((rev.params.opentaps, rev.params.targetflow), (0.1, 9.0));
    // A .dsg without !Clients is an error in MATLAB; the shared NW5 reader
    // builds the clients for NW4 compatibility (outside this crate).
}
