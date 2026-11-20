//! MATLAB `NW-json-v1` and application domain JSON (`specs/formats.md`
//! sections 7.1, 7.3 to 7.5).

mod common;

use common::{Skip, TestResult, compare, example_files, read, reference};
use neatwork_nw5::NodeStatus;
use neatwork_nw5_io::domain::behavior_id;
use neatwork_nw5_io::{read_domain_json, read_nw_json, write_domain_json, write_nw_json};

#[test]
fn every_example_round_trips_through_nw_json() -> TestResult {
    let mut failures = Vec::new();
    for p in example_files()? {
        let a = read(&p)?;
        let text = write_nw_json(&a);
        let b = read_nw_json(&a.name, &text)?.network;
        let d = compare(&a, &b, Skip { open: true, extra_order: true, name: false });
        if !d.is_empty() {
            failures.push(format!("{}:\n  {}", p.display(), d.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn every_example_round_trips_through_domain_json() -> TestResult {
    let mut failures = Vec::new();
    for p in example_files()? {
        let mut a = read(&p)?;
        if let Some(arc) = a.arcs.first_mut() {
            arc.open = false; // the domain JSON keeps the valve state
        }
        let text = write_domain_json(&a);
        let b = read_domain_json(&text)?.network;
        let d = compare(&a, &b, Skip { extra_order: true, ..Skip::default() });
        if !d.is_empty() {
            failures.push(format!("{}:\n  {}", p.display(), d.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn behaviour_ids_match_the_application() {
    assert_eq!(behavior_id(0.6, 0.12, None), "CB_BAE72D2A3E");
    assert_eq!(behavior_id(0.5, 2.0, None), "CB_89C66A034C");
    assert_eq!(behavior_id(0.28, 0.12, None), "CB_3090BA5D71");
    assert_eq!(behavior_id(1.0, 10.0, None), "CB_5DDB4706FD");
}

#[test]
fn toy_bbd_domain_json_snippets() -> TestResult {
    let net = read(&reference().join("neatwork5/examples/dsg_examples/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg"))?;
    let j = write_domain_json(&net);
    assert!(j.starts_with("{\n  \"name\": \"Toy_bbd_Secu1.00_fusion0_split0\",\n  \"sourceFormat\": \"DSG\",\n"), "{j}");
    assert!(j.contains("\"behaviorId\": \"CB_BAE72D2A3E\""));
    assert!(j.contains("\"name\": \"Imported behavior 1\""));
    assert!(j.contains("\"Nominal\": \"\\\"1/2\\\"\\\"\\\"\""));
    assert!(j.contains("\"limitbudget\": 1000000000"));
    assert!(j.contains("\"faucetcoef\": 1.87e-8"));
    assert!(j.contains("\"diameter_mm\": 1.7"));
    assert_eq!(j.matches("\"behaviorId\": \"CB_").count(), 16 + 3);
    let nw = write_nw_json(&net);
    assert!(nw.starts_with("{\n  \"schema\": \"NW-json-v1\",\n  \"TN\": [\n    {\n      \"noeud\": \"Source\",\n"), "{nw}");
    assert!(nw.contains("\"ID2\": \"fictif\""));
    Ok(())
}

/// A MATLAB `jsonencode` file: one-row tables as objects, `Inf` as `null`,
/// the `fictif` catalogue row, numeric IDs, TFact flows.
#[test]
fn matlab_style_json() -> TestResult {
    let text = r#"{
  "schema": "NW-json-v1",
  "TN": [
    {"noeud": "S", "haut": 0, "xdata": 0, "ydata": 0, "orif_ideal": 1, "orif_com": 1, "statut": 0},
    {"noeud": 150, "haut": -12.5, "xdata": 3, "ydata": 4, "orif_ideal": 0.0031, "orif_com": 0.0033, "statut": 2}
  ],
  "TA": {"pred": "S", "succ": 150, "lon": 100, "lon1": 100, "ID1": "D001", "lon2": 0, "ID2": "fictif", "N": "R", "equip": true, "diam1": 0.0182, "diam2": 0, "nature": "reinf"},
  "TB": {"borne": 150, "pressure": 4, "N": "N"},
  "TC": [{"client": "C1150", "borne": 150, "proba": 0.5, "cible": 0.2},
         {"client": "C2150", "borne": 150, "proba": 0.5, "cible": 0.2}],
  "TD": [{"ID": "fictif", "Nominal": "fictif", "SDR": "fictif", "diam": 0, "cost": 0, "pressure": 1000, "type": 1, "Roughness": 0},
         {"ID": "D001", "Nominal": "1/2\"", "SDR": "13.5", "diam": 0.0182, "cost": 0.28, "pressure": 200, "type": 1, "Roughness": 0.0015}],
  "TO": [{"in_diam": 0.0031}, {"in_diam": 0.0033}],
  "TFact": {"succ": 150, "designFlow": 0.4, "margin": -0.25},
  "TAH": [], "TCP": [], "TS": [],
  "TPar": [{"name": "limitbudget", "value": null}, {"name": "vmax", "value": 1.5}, {"name": "orifcoef", "value": 0.59}],
  "pardsg": {"limitbudget": null, "vmax": 1.5},
  "partec": {"orifcoef": 0.59},
  "reinfcost": {"costOptim": 28, "costInter": 28, "costFinal": 28, "costFixed": 0}
}"#;
    let r = read_nw_json("m", text)?;
    let n = &r.network;
    assert_eq!(n.nodes[1].id, "150");
    assert_eq!(n.nodes[1].status, NodeStatus::Terminal);
    assert_eq!(n.arcs.len(), 1);
    assert_eq!(n.arcs[0].pipe1.as_deref(), Some("D001"));
    assert_eq!(n.arcs[0].pipe2, None);
    assert_eq!(n.arcs[0].state, neatwork_nw5::ArcState::Reinforcement);
    assert_eq!(n.pipes.len(), 1, "the fictif row is dropped");
    assert_eq!(n.terminals[0].pressure, 4.0);
    assert_eq!(n.clients.len(), 2);
    assert_eq!(n.params.get("limitbudget"), f64::INFINITY);
    assert_eq!(n.params.get("uniform_dem"), 1.0);
    assert_eq!(n.loadflow, vec![("150".to_string(), 0.4, -0.25)]);
    assert_eq!(n.orifices, vec![0.0031, 0.0033]);
    let costs = n.extra_sections.iter().find(|(h, _)| h == "!Costs").map(|(_, l)| l.clone());
    assert_eq!(costs, Some(vec!["costOptim,28".into(), "costInter,28".into(), "costFinal,28".into(), "costFixed,0".into()]));
    Ok(())
}

/// Application JSON import: `{graph, domain}` with legacy client rows and
/// numeric node names (section 7.4), and TPO-style rows.
#[test]
fn application_import_json() -> TestResult {
    let text = r#"{"graph": {"nodes": [], "edges": []},
      "domain": {"params": {"opentaps": 0.4, "bounded-flow": true, "limitbudget": "Inf"},
        "tables": {
          "nodes": [{"noeud": "S", "haut": 5, "xdata": 0, "ydata": 0, "nbrob": 0, "statut": 0},
                    {"node_id": 7, "elevationM": 0, "x": 1, "y": 2, "nbrob": 2, "status": 2}],
          "pipes": [{"pred": "S", "succ": 7, "length": 50, "operational_status": "CLOSED"}],
          "clients": [{"client": "a", "borne": 7, "proba": 0.4, "cible": 0.1},
                      {"clientId": "b", "terminalId": "7", "openProbability": 0.3, "targetFlowLps": 0.1}],
          "bornes": [{"borne": 7, "pressure": 2, "N": ""}],
          "diameters": [{"size": "63", "SDR": 11, "diam": 0.0514, "cost": 2, "pressure": 100, "type": 1, "rugo": 0.0015, "ID": "D001"}],
          "orifices": [{"in_diam": 0.002, "N": "N"}]}}}"#;
    let n = read_domain_json(text)?.network;
    assert_eq!(n.nodes[1].id, "7");
    assert_eq!(n.nodes[1].taps, Some(2.0));
    assert_eq!(n.arcs[0].length, 50.0);
    assert_eq!(n.arcs[0].length1, 0.0);
    assert!(!n.arcs[0].open);
    assert_eq!(n.clients.len(), 2);
    assert_eq!(n.params.get("bounded_flow"), 1.0);
    assert_eq!(n.params.get("uniform_dem"), 0.0);
    assert_eq!(n.params.get("limitbudget"), f64::INFINITY);
    assert_eq!(n.pipes[0].nominal, "63");
    assert_eq!(n.pipes[0].max_pressure, 100.0);
    assert_eq!(n.orifices, vec![0.002]);
    let bad = r#"{"tables": {"nodes": [], "pipes": [], "clients": [{"client": "x", "borne": "P", "behaviorId": "CB_1"}]}}"#;
    let e = read_domain_json(bad).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("Unknown client behavior 'CB_1'."), "{e}");
    let bad = r#"{"tables": {"nodes": [], "pipes": [], "clients": [{"client": "x", "borne": "P"}]}}"#;
    let e = read_domain_json(bad).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("Client 'x' has neither behaviorId nor complete legacy demand values."), "{e}");
    Ok(())
}
