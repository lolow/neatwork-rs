//! Cruscade Txt designs (section 9) and the NW4 to NW5 `.dsg` upgrade
//! (section 5).

#[expect(dead_code, reason = "this test file uses a subset of the shared helpers")]
mod common;

use common::{Skip, TestResult, compare, read, reference};
use neatwork_nw5::read::read_dsg;
use neatwork_nw5::{NodeStatus, Params};
use neatwork_nw5_io::txt::FlowUnit;
use neatwork_nw5_io::{read_txt_folder, upgrade_nw4_dsg, write_dsg};

fn tpo_examples() -> std::path::PathBuf {
    reference().join("neatwork5/examples/tpo_examples")
}

/// The Txt design of Cruscade has the topology and demand of `Cruscade.tpo`.
#[test]
fn cruscade_txt_matches_cruscade_tpo() -> TestResult {
    let r = read_txt_folder(&tpo_examples().join("CruscadeTxt"), "CruscadeDet2406", FlowUnit::CubicMetresPerSecond)?;
    let txt = r.network;
    let tpo = read(&tpo_examples().join("Cruscade/Cruscade.tpo"))?;
    assert_eq!(txt.nodes.len(), tpo.nodes.len());
    assert_eq!(txt.arcs.len(), tpo.arcs.len());
    assert_eq!(txt.clients.len(), tpo.clients.len());
    assert_eq!(txt.terminals.len(), tpo.terminals.len());
    let count = |n: &neatwork_nw5::Network, s: NodeStatus| n.nodes.iter().filter(|x| x.status == s).count();
    for s in [NodeStatus::Source, NodeStatus::Junction, NodeStatus::Terminal] {
        assert_eq!(count(&txt, s), count(&tpo, s), "{s:?}");
    }
    assert!(txt.is_design(), "every arc has a pipe from diam1/diam2");
    assert_eq!(txt.pipes.len(), 13);
    assert_eq!(txt.node("Station").map(|n| n.status), Some(NodeStatus::Source));
    // m3/s targets are converted to L/s
    assert!((txt.clients[0].target_flow - 1.045093).abs() < 1e-12, "{}", txt.clients[0].target_flow);
    assert_eq!(txt.clients[0].probability, 0.8);
    assert_eq!(txt.terminals[0].pressure, 37.0);
    assert_eq!(txt.params.get("pipelength"), 0.1);
    // a written .dsg reads back to the same tables
    let back = read_dsg(&txt.name, &write_dsg(&txt))?.network;
    assert!(compare(&txt, &back, Skip::default()).is_empty());
    // default unit is L/s
    let r = read_txt_folder(&tpo_examples().join("CruscadeTxt"), "CruscadeDet2406", FlowUnit::default())?;
    assert_eq!(r.network.clients[0].target_flow, 0.001045093);
    Ok(())
}

#[test]
fn toy_txt_designs() -> TestResult {
    let r = read_txt_folder(&tpo_examples().join("ToyTxt"), "Toy3b", FlowUnit::LitresPerSecond)?;
    let n = &r.network;
    assert_eq!(n.nodes.len(), 8);
    assert_eq!(n.arcs.len(), 7);
    assert_eq!(n.arcs[0].pipe1.as_deref(), Some("D007"));
    assert_eq!(n.arcs[2].pipe2, None);
    assert_eq!(n.node("P2").map(|x| (x.status, x.orifice_ideal)), Some((NodeStatus::Terminal, 10.0)));
    assert_eq!(n.node("A").map(|x| x.status), Some(NodeStatus::Junction));
    assert_eq!(n.params.get("uniform_dem"), 0.0);
    assert_eq!(n.params.get("bounded_flow"), 1.0, "non-uniform Txt demand is bounded");
    assert!(n.pipe("D022").is_some_and(|p| p.nominal == "\"3/4\"\"\"" && p.sdr == "17.0"));
    let r = read_txt_folder(&tpo_examples().join("ToyTxt"), "Toy2b", FlowUnit::LitresPerSecond)?;
    assert_eq!(r.network.params.get("uniform_dem"), 1.0);
    Ok(())
}

#[test]
fn txt_errors() {
    use neatwork_nw5_io::txt::{TxtFiles, read_txt_design};
    let files = TxtFiles {
        arcs: "pred\tsucc\tlon\tlon1\tlon2\nS\tA\t1\t1\t0\n",
        clients: "client,borne,proba,cible\nc,A,1,1\n",
        nodes: "noeud\thaut\txdata\tydata\torif_ideal\torif_com\tstatut\nS\t0\t0\t0\t0\t0\t9\nA\t0\t0\t0\t0\t0\t1\n",
        dbase: "ID\tdiam\tcost\nD1\t0.1\t1\n",
        param: "property,value\n",
    };
    let e = read_txt_design("t", &files, FlowUnit::default()).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("DsgArcs: il faut (diam1,diam2) ou (ID1,ID2)"), "{e}");
    let files = TxtFiles { arcs: "pred\tsucc\tlon\tlon1\tlon2\tdiam1\tdiam2\nS\tA\t1\t1\t0\t0.1\t0\n", ..files };
    let e = read_txt_design("t", &files, FlowUnit::default()).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("DsgNodes.statut doit contenir au moins 3 catégories"), "{e}");
    let files = TxtFiles { clients: "client,borne,cible\nc,A,1\n", ..files };
    let e = read_txt_design("t", &files, FlowUnit::default()).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("DsgClients: colonnes requises manquantes") && e.contains("proba"), "{e}");
}

/// `Toy2b_NW4.dsg`: fictitious faucets `P2_a..c`, `P3_a..b` are merged into
/// P2 and P3 with 3 and 2 clients.
#[test]
fn toy2b_nw4_upgrade() -> TestResult {
    let path = reference().join("neatwork5/examples/dsg_examples/Toy/Toy2b_NW4.dsg");
    let text = String::from_utf8_lossy(&std::fs::read(&path)?).to_string();
    let r = upgrade_nw4_dsg("Toy2b_NW4", &text)?;
    let n = &r.network;
    let ids: Vec<&str> = n.nodes.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["Source", "A", "B", "C", "P2", "P3", "P4", "P1"]);
    let p2 = n.node("P2").ok_or("P2")?;
    assert_eq!((p2.status, p2.elevation, p2.orifice_ideal, p2.orifice_commercial), (NodeStatus::Terminal, -110.0, 0.00270825, 0.0029));
    assert_eq!(n.arcs.len(), 7);
    assert!(n.arcs.iter().all(|a| !a.to.contains('_')));
    let at = |t: &str| n.clients.iter().filter(|c| c.terminal == t).count();
    assert_eq!((at("P1"), at("P2"), at("P3"), at("P4")), (1, 3, 2, 1));
    assert!(n.clients.iter().any(|c| c.id == "C3P2" && c.probability == 0.28 && c.target_flow == 0.12));
    assert_eq!(n.terminals.len(), 4);
    assert!(n.terminals.iter().all(|t| t.pressure == 2.0));
    assert_eq!(n.params.get("bounded_flow"), 0.0);
    assert_eq!(n.params.get("uniform_dem"), 1.0);
    assert_eq!(n.loadfactor_raw.len(), 8);
    assert!(n.loadfactor_raw.iter().all(|r| !r[0].contains('_')));
    // the written NW5 file has clients and bornes and reads back the same
    let written = write_dsg(n);
    assert!(written.contains("\n!Clients\nC1P1,P1,0.28,0.12\nC1P2,P2,0.28,0.12\n"));
    assert!(written.contains("\npressure,2\n"));
    let back = read_dsg("Toy2b_NW4", &written)?.network;
    assert!(compare(n, &back, Skip::default()).is_empty());
    // a NW5 file is refused
    let nw5 = std::fs::read_to_string(reference().join("neatwork5/examples/dsg_examples/Toy/Toy_Secu1.00_fusion0_split0.dsg"))?;
    let e = upgrade_nw4_dsg("t", &nw5).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("Ce n'est pas un fichier NW4"), "{e}");
    Ok(())
}

/// A real node with `_` in its name is not a faucet: only `P_<letters>`
/// terminals fed by a 1 m pipe from `P` are merged.
#[test]
fn upgrade_keeps_real_underscore_nodes() -> TestResult {
    let text = "!Nodes\nS,0,0,0,0,0,0\nN_1,-5,0,0,0,0,1\nT_a,-6,0,0,0.003,0.003,2\nP,-7,0,0,0,0,1\nP_a,-7,0,0,0.002,0.002,2\nP_b,-7,0,0,0.002,0.002,2\n!Pipes\nS,N_1,10,10,D001,0,0,N\nN_1,T_a,1,1,D001,0,0,N\nN_1,P,5,5,D001,0,0,N\nP,P_a,1,1,D001,0,0,N\nP,P_b,1,1,D001,0,0,N\n!Default properties\nopentaps,0.5\ntargetflow,0.1\n!Diameters\nD001,a,1,0.02,1,100,1,0,N\n";
    let n = upgrade_nw4_dsg("u", text)?.network;
    let ids: Vec<&str> = n.nodes.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["S", "N_1", "T_a", "P"]);
    assert_eq!(n.clients.iter().filter(|c| c.terminal == "P").count(), 2);
    assert_eq!(n.clients.iter().filter(|c| c.terminal == "T_a").count(), 1);
    assert_eq!(n.arcs.len(), 3);
    let p = Params::canonical("pressure");
    assert_eq!(n.params.get(p), 2.0);
    Ok(())
}
