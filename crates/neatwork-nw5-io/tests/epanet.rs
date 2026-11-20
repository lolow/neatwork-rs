//! EPANET `.inp` export (section 8.3 proposal) and import (section 8.2).

#[expect(dead_code, reason = "this test file uses a subset of the shared helpers")]
mod common;

use common::{TestResult, close, read, reference};
use neatwork_nw5::{Network, NodeStatus};
use neatwork_nw5_io::epanet::{DemandPolicy, InpImportOptions, Severity, diameter_catalog_id};
use neatwork_nw5_io::{read_inp, write_inp};

fn dsg(rel: &str) -> Result<Network, String> {
    read(&reference().join("neatwork5/examples/dsg_examples").join(rel))
}

fn section<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("[{name}]");
    text.lines()
        .skip_while(|l| *l != header)
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter(|l| !l.trim().is_empty() && !l.starts_with(';'))
        .collect()
}

fn deterministic() -> InpImportOptions {
    InpImportOptions {
        demand_policy: DemandPolicy::DeterministicAlwaysOpen,
        enable_hw_param_mapping: true,
        ..InpImportOptions::default()
    }
}

/// Export a design, import it again and check the topology, elevations,
/// lengths, diameters and demands.
fn export_reimport(net: &Network) -> TestResult {
    let out = write_inp(net)?;
    let text = &out.text;
    let order = ["[TITLE]", "[JUNCTIONS]", "[RESERVOIRS]", "[PIPES]", "[DEMANDS]", "[OPTIONS]", "[TIMES]", "[REPORT]", "[COORDINATES]", "[END]"];
    let pos: Vec<usize> = order.iter().map(|h| text.find(h).unwrap_or(usize::MAX)).collect();
    assert!(pos.windows(2).all(|w| w[0] < w[1]), "section order {pos:?}");
    let two = net.arcs.iter().filter(|a| a.length1 > 0.0 && a.length2 > 0.0).count();
    let segments: usize = net.arcs.iter().map(|a| usize::from(a.length1 > 0.0) + usize::from(a.length2 > 0.0)).sum();

    let back = read_inp(text, &deterministic())?;
    let b = &back.network;
    assert_eq!(b.nodes.len(), net.nodes.len() + two);
    assert_eq!(b.arcs.len(), segments);
    for x in &net.nodes {
        let y = b.node(&x.id).ok_or(format!("node {} lost", x.id))?;
        let head = if x.status == NodeStatus::Source { x.elevation + net.params.get("pump_pressure") } else { x.elevation };
        assert!(close(y.elevation, head), "{} elevation", x.id);
        assert!(close(y.x, x.x) && close(y.y, x.y));
        assert_eq!(y.status == NodeStatus::Source, x.status == NodeStatus::Source);
    }
    let length = |n: &Network| n.arcs.iter().map(|a| a.length1 + a.length2).sum::<f64>();
    assert!((length(net) - length(b)).abs() < 1e-9 * length(net));
    for a in &b.arcs {
        let d = b.pipe(a.pipe1.as_deref().unwrap_or("")).ok_or("pipe")?.diameter;
        assert!(net.pipes.iter().any(|p| close(p.diameter, d)), "diameter {d}");
    }
    if net.params.get("bounded_flow") == 1.0 {
        let total = |n: &Network| n.clients.iter().map(|c| c.target_flow).sum::<f64>();
        assert!((total(net) - total(b)).abs() < 1e-9, "{} / {}", total(net), total(b));
        assert!(b.clients.iter().all(|c| c.probability == 1.0));
        let p = net.terminals.iter().map(|t| t.pressure).fold(0.0, f64::max);
        assert!(b.terminals.iter().all(|t| t.pressure == p));
        assert!(back.issues.iter().all(|i| i.code != "DDA_NOT_SIMULATION_EQUIVALENT"));
    }
    // H-W mapping gives back the head-loss scale for one roughness value
    let c = net.params.get("bescal");
    assert!((b.params.get("bescal") - c).abs() < 1e-12 * c, "bescal {}", b.params.get("bescal"));
    Ok(())
}

#[test]
fn toy_designs_export_and_reimport() -> TestResult {
    for f in ["Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg", "Toy/Toy_bbu_Secu1.00_fusion0_split0.dsg", "Toy/Toy_blu_Secu1.00_fusion0_split0.dsg", "Esperenza/Esperenza2S.dsg"] {
        export_reimport(&dsg(f)?).map_err(|e| format!("{f}: {e}"))?;
    }
    Ok(())
}

#[test]
fn ceiba_design_exports_and_reimports() -> TestResult {
    export_reimport(&dsg("Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg")?)
}

/// Golden snippet of section 8.3: the two-diameter arc `A,B,500,38,D009,462,D004`.
#[test]
fn toy_bbd_two_diameter_arc() -> TestResult {
    let net = dsg("Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg")?;
    let out = write_inp(&net)?;
    let t = &out.text;
    assert!(t.starts_with("[TITLE]\nToy_bbd_Secu1.00_fusion0_split0\nbbd = mod"), "{t}");
    let junctions = section(t, "JUNCTIONS");
    assert!(junctions.contains(&"A_B_s\t-10.38\t0"), "{junctions:?}");
    let pipes = section(t, "PIPES");
    let ab1 = pipes.iter().find(|l| l.starts_with("A-B#1\t")).ok_or("A-B#1")?;
    let f: Vec<&str> = ab1.split('\t').collect();
    assert_eq!(f[..5], ["A-B#1", "A", "A_B_s", "38", "45.3"]);
    assert!((f[5].parse::<f64>()? - 158.95).abs() < 0.01, "{ab1}");
    assert_eq!(f[6..], ["0", "Open"]);
    assert!(pipes.iter().any(|l| l.starts_with("A-B#2\tA_B_s\tB\t462\t30.4\t")));
    assert!(pipes.iter().any(|l| l.starts_with("Source-A\tSource\tA\t200\t45.3\t")));
    assert!(section(t, "DEMANDS").contains(&"P1\t0.12\t\t;C1P1 p=0.6"));
    let options = section(t, "OPTIONS");
    for o in ["UNITS\tLPS", "HEADLOSS\tH-W", "DEMAND MODEL\tPDA", "MINIMUM PRESSURE\t0", "REQUIRED PRESSURE\t2", "PRESSURE EXPONENT\t0.5"] {
        assert!(options.contains(&o), "{o} in {options:?}");
    }
    assert!(section(t, "COORDINATES").contains(&"A_B_s\t0\t0"));
    assert!(t.ends_with("[END]\n"));
    assert!(out.warnings.iter().any(|w| w.contains("approximate")), "{:?}", out.warnings);
    Ok(())
}

/// Free flow: terminals are emitters `K = n 1000 / sqrt(alpha)` (rule 6).
#[test]
fn free_flow_terminals_are_emitters() -> TestResult {
    let mut net = dsg("Toy/Toy_blu_Secu1.00_fusion0_split0.dsg")?;
    let p1 = net.nodes.iter_mut().find(|n| n.id == "P1").ok_or("P1")?;
    p1.orifice_commercial = 0.0044;
    net.params.set("orifcoef", "0.59");
    net.params.set("faucetcoef", "1.87e-8");
    net.params.set("orif_com", "1");
    let t = write_inp(&net)?.text;
    let emitters = section(&t, "EMITTERS");
    let p1 = emitters.iter().find(|l| l.starts_with("P1\t")).ok_or("P1 emitter")?;
    let k: f64 = p1.split('\t').nth(1).ok_or("K")?.parse()?;
    let alpha = (0.59f64 / 0.0044).powi(4) + 1.0 / 1.87e-8;
    assert!((alpha - 3.7677e8).abs() < 1e4);
    let clients = net.clients.iter().filter(|c| c.terminal == "P1").count() as f64;
    assert!(close(k, clients * 1000.0 / alpha.sqrt()), "{k}");
    assert!((1000.0 / alpha.sqrt() - 0.0515).abs() < 1e-4);
    assert!(section(&t, "DEMANDS").iter().all(|l| !l.starts_with("P1\t")));
    assert!(section(&t, "OPTIONS").contains(&"EMITTER EXPONENT\t0.5"));
    assert!(section(&t, "OPTIONS").contains(&"DEMAND MODEL\tDDA"));
    Ok(())
}

/// The reference case of `APPT:reference_validation` (section 8.3 text).
const REFERENCE_INP: &str = "\
[TITLE]
reference_pda_hw

[JUNCTIONS]
;ID   Elev   Demand   Pattern
J1    0      0
P1    0      0

[RESERVOIRS]
;ID   Head
S1    12

[PIPES]
;ID   Node1   Node2   Length   Diameter   Roughness   MinorLoss   Status
E1    S1      J1      100      63         140         0           Open
E2    J1      P1      100      63         140         0           Open

[DEMANDS]
;Junction   Demand   Pattern   ;Category
P1          1                  ;C1 p=1

[OPTIONS]
UNITS               LPS
HEADLOSS            H-W
DEMAND MODEL        PDA
MINIMUM PRESSURE    0
REQUIRED PRESSURE   20
PRESSURE EXPONENT   0.5

[TIMES]
DURATION 0

[COORDINATES]
S1   0   0
J1   1   0
P1   2   0

[END]
";

#[test]
fn reference_case_import() -> TestResult {
    assert_eq!(diameter_catalog_id(0.063, 100.0, "H-W"), "EPD_34F2A471AF");
    let r = read_inp(REFERENCE_INP, &deterministic())?;
    let n = &r.network;
    assert!(r.simulation_ready, "{:?}", r.issues);
    assert_eq!(n.nodes.len(), 3);
    assert_eq!(n.node("S1").map(|x| (x.status, x.elevation)), Some((NodeStatus::Source, 12.0)));
    assert_eq!(n.node("P1").map(|x| x.status), Some(NodeStatus::Terminal));
    assert_eq!(n.node("J1").map(|x| (x.status, x.x)), Some((NodeStatus::Junction, 1.0)));
    assert_eq!(n.arcs.len(), 2);
    assert_eq!(n.arcs[0].id, "E1");
    assert_eq!(n.arcs[0].pipe1.as_deref(), Some(diameter_catalog_id(0.063, 140.0, "H-W").as_str()));
    assert_eq!(n.pipes.len(), 1);
    assert_eq!(n.pipes[0].nominal, "63 mm");
    assert_eq!(n.pipes[0].kind, "EPANET");
    assert_eq!(n.clients.len(), 1);
    assert_eq!(n.clients[0].id, "EP_P1_01");
    assert_eq!((n.clients[0].probability, n.clients[0].target_flow), (1.0, 1.0));
    assert_eq!(n.terminals.len(), 1);
    assert_eq!(n.terminals[0].pressure, 20.0);
    assert!((n.params.get("bescal") - 10.667 * 140f64.powf(-1.852)).abs() < 1e-15);
    assert_eq!(n.params.get("lambda"), 1.852);
    assert_eq!(n.comments, vec!["reference_pda_hw".to_string()]);

    // default policy: demands are kept unresolved and block the simulation
    let r = read_inp(REFERENCE_INP, &InpImportOptions::default())?;
    assert!(r.network.clients.is_empty());
    assert!(!r.simulation_ready);
    assert!(r.issues.iter().any(|i| i.code == "INTERMITTENT_PROBABILITY_UNRESOLVED" && i.severity == Severity::Blocking));
    Ok(())
}

#[test]
fn import_issues_and_units() -> TestResult {
    let text = "\u{feff}[TITLE]\nUS net\n[OPTIONS]\nUnits GPM\nHeadloss D-W\n[JUNCTIONS]\nJ1 100 10 pat ;x\nJ2\n[RESERVOIRS]\nR1 200\n[PIPES]\nP1 R1 J1 1000 12 0.5 0 CV\nP2 J1 R1 10 6\n[PUMPS]\nPU1 R1 J1 HEAD c1\n[EMITTERS]\nJ1 0.5\n[PATTERNS]\npat 1 2\npat 3\n[END]\n[JUNCTIONS]\nX 1\n";
    let r = read_inp(text, &deterministic())?;
    let codes: Vec<&str> = r.issues.iter().map(|i| i.code.as_str()).collect();
    for c in ["DW_HEADLOSS_MODEL_NOT_VALIDATED", "DDA_NOT_SIMULATION_EQUIVALENT", "INVALID_JUNCTION_ROW", "INVALID_PIPE_ROW", "CHECK_VALVE_NOT_SUPPORTED", "UNSUPPORTED_PUMPS", "EMITTER_ORIFICE_MAPPING_UNRESOLVED", "TIME_PATTERN_NOT_MAPPED", "TERMINAL_PRESSURE_UNRESOLVED"] {
        assert!(codes.contains(&c), "{c} not in {codes:?}");
    }
    let n = &r.network;
    assert_eq!(n.nodes.len(), 2, "X after [END] is ignored");
    assert!(close(n.node("J1").map_or(0.0, |x| x.elevation), 30.48));
    assert!(close(n.arcs[0].length, 304.8));
    assert!(close(n.pipes[0].diameter, 12.0 * 0.0254));
    assert_eq!(n.pipes[0].nominal, "12 in");
    assert!(close(n.clients[0].target_flow, 10.0 * 0.0630901964));
    assert_eq!(r.patterns, vec![("pat".to_string(), vec![1.0, 2.0, 3.0])]);
    assert_eq!(r.units, "GPM");
    assert!(r.preserved_sections.iter().any(|(s, _)| s == "PUMPS"));
    let bad = read_inp("[OPTIONS]\nUNITS XYZ\n", &InpImportOptions::default());
    assert!(bad.err().is_some_and(|e| e.to_string().contains("Unsupported EPANET flow units: XYZ")));
    Ok(())
}
