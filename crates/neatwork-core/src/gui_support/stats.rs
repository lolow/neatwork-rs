//! Network summary (`TopoStatDialog` and the counters of
//! `project.Network`) and the quick faucet check (`QuickCheckTableModel`).

use crate::engine::simul_flows::{jmax, jmin};
use crate::java::{decimal_format, double_to_string, parse_double};
use crate::project::Topographie;
use crate::project::network::{Network, TYPE_DISPATCH, TYPE_FAUCET, TYPE_RESERVOIR};
use crate::{Error, Result};

/// Counters of `project.Network`.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkStats {
    /// `getNbNodes()`.
    pub nb_nodes: usize,
    /// `getNbNodes(TYPE_RESERVOIR)`.
    pub nb_reservoirs: usize,
    /// `getNbNodes(TYPE_DISPATCH)`.
    pub nb_branching: usize,
    /// `getNbNodes(TYPE_FAUCET)`.
    pub nb_faucets: usize,
    /// `getNbTotalTaps`: sum of the tap counts of all nodes.
    pub nb_taps: i64,
    /// `getTotalHeightChange`: highest minus lowest height, with 0 in both.
    pub total_height_change: f64,
    /// `getNbPipes`.
    pub nb_pipes: usize,
    /// `getTotalLength`.
    pub total_length: f64,
    /// `getMinLength`: starts at 150.
    pub min_length: f64,
    /// `getMaxHeight`: starts at -150.
    pub max_height: f64,
}

/// Compute the counters of a network.
pub fn network_stats(network: &Network) -> NetworkStats {
    let count = |t: i32| network.nodes.iter().filter(|n| n.node_type == t).count();
    let (mut low, mut high) = (0.0, 0.0);
    for n in &network.nodes {
        if n.height > high {
            high = n.height;
        }
        if n.height < low {
            low = n.height;
        }
    }
    NetworkStats {
        nb_nodes: network.nodes.len(),
        nb_reservoirs: count(TYPE_RESERVOIR),
        nb_branching: count(TYPE_DISPATCH),
        nb_faucets: count(TYPE_FAUCET),
        nb_taps: network.nodes.iter().map(|n| i64::from(n.nb_taps)).sum(),
        total_height_change: high - low,
        nb_pipes: network.pipes.len(),
        total_length: network.pipes.iter().fold(0.0, |s, p| s + p.length),
        min_length: network.pipes.iter().fold(150.0, |m, p| jmin(p.length, m)),
        max_height: network.nodes.iter().fold(-150.0, |m, n| jmax(n.height, m)),
    }
}

/// Text of the `TopoStatDialog` label (title "Topography information").
pub fn topo_stat_html(name: &str, stats: &NetworkStats) -> String {
    format!(
        "<HTML> <blockquote><B>Topography name:</B> {name} <BR><BR>\
         <B>Number of Nodes:</B> {}<UL>\
         <LI> <i>Branching nodes:</i> {}\
         <LI> <i>Faucet nodes:</i>{} (with {} individual faucets)</UL>\
         <B>Total height change:</B> {} m<BR><BR>\
         <B>Number of Pipes:</B> {} <BR>\
         <B>Total length:</B> {} m <BR>",
        stats.nb_nodes,
        stats.nb_branching,
        stats.nb_faucets,
        stats.nb_taps,
        decimal_format("#", stats.total_height_change),
        stats.nb_pipes,
        decimal_format("#", stats.total_length),
    )
}

/// Content of the quick check table.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickCheck {
    /// Column titles: faucet ID, minimum flow, target flow.
    pub headers: [String; 3],
    /// One row per faucet: name, then `Ok` or `No` for each flow.
    pub rows: Vec<[String; 3]>,
}

/// `QuickCheckTableModel.updateData`.
///
/// A faucet passes a flow `q` (l/s) when its drop below the source is above
/// `(q / 1000)^2 / faucetcoef`. The two flows are `topo.seuil.value`
/// (default 0.1) and `topo.targetflow.value`.
pub fn quick_check(topo: &Topographie) -> Result<QuickCheck> {
    let param = |key: &str, default: Option<&str>| -> Result<f64> {
        topo.properties
            .get_property(key)
            .or(default)
            .and_then(parse_double)
            .ok_or_else(|| Error::Parameter(key.to_string()))
    };
    let targetflow = param("topo.targetflow.value", None)?;
    let alpha = param("topo.faucetcoef.value", None)?;
    let seuil = param("topo.seuil.value", Some("0.1"))?;
    let check = |height: f64, flow: f64| {
        // HotSpot computes Math.pow(x, 2) as x * x.
        let q = flow / 1000.0;
        if -height > q * q / alpha { "Ok" } else { "No" }.to_string()
    };
    Ok(QuickCheck {
        headers: [
            "Faucet ID".to_string(),
            format!("flow min ({} l/s)", double_to_string(seuil)),
            format!("target flow ({} l/s)", double_to_string(targetflow)),
        ],
        rows: topo
            .network
            .nodes
            .iter()
            .filter(|n| n.node_type == TYPE_FAUCET)
            .map(|n| {
                [
                    n.name.clone(),
                    check(n.height, seuil),
                    check(n.height, targetflow),
                ]
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_support::fixtures::*;

    #[test]
    fn stats_dialog_matches_java() {
        let t = topo("El_Bihajua", EL_BIHAJUA_TPO);
        assert_eq!(
            topo_stat_html(&t.name, &network_stats(&t.network)),
            include_str!("testdata/stats_El_Bihajua_tpo.html")
        );
        let t = topo("SampleNetwork", SAMPLE_NETWORK_TPO);
        assert_eq!(
            topo_stat_html(&t.name, &network_stats(&t.network)),
            include_str!("testdata/stats_SampleNetwork_tpo.html")
        );
    }

    #[test]
    fn counters() {
        let s = network_stats(&design("toy", TOY_DSG).network);
        assert_eq!(s.nb_nodes, 8);
        assert_eq!(s.nb_reservoirs, 1);
        assert_eq!(s.nb_branching, 3);
        assert_eq!(s.nb_faucets, 4);
        assert_eq!(s.nb_taps, 0);
        assert_eq!(s.total_height_change, 50.0);
        assert_eq!(s.nb_pipes, 7);
        assert_eq!(s.total_length, 1270.0);
        assert_eq!(s.min_length, 70.0);
        assert_eq!(s.max_height, 0.0);
        let empty = network_stats(&Network::default());
        assert_eq!(empty.min_length, 150.0);
        assert_eq!(empty.max_height, -150.0);
        assert_eq!(empty.total_height_change, 0.0);
    }

    fn table(q: &QuickCheck) -> String {
        let mut s = q.headers.join("\t") + "\n";
        for r in &q.rows {
            s.push_str(&r.join("\t"));
            s.push('\n');
        }
        s
    }

    #[test]
    fn quick_check_matches_java() {
        let q = quick_check(&topo("El_Bihajua", EL_BIHAJUA_TPO)).ok();
        assert_eq!(
            q.as_ref().map(table).as_deref(),
            Some(include_str!("testdata/quickcheck_El_Bihajua_tpo.txt"))
        );
        let q = quick_check(&topo("SampleNetwork", SAMPLE_NETWORK_TPO)).ok();
        assert_eq!(
            q.as_ref().map(table).as_deref(),
            Some(include_str!("testdata/quickcheck_SampleNetwork_tpo.txt"))
        );
    }

    #[test]
    fn quick_check_limits() {
        // With alpha = 1e-8, flow 0.1 l/s needs more than 1 m of drop and
        // 0.2 l/s more than 4 m.
        let tpo = "S,0,0,0,0,0\nA,-0.5,0,0,1,2\nB,-2,0,0,1,2\nC,-5,0,0,1,2\n\
                   S,A,1\nS,B,1\nS,C,1\nfaucetcoef,1.0E-8\nseuil,0.1\ntargetflow,0.2\n";
        let q = quick_check(&topo("q", tpo)).ok();
        let rows = q.map(|q| q.rows);
        assert_eq!(
            rows,
            Some(vec![
                ["A".to_string(), "No".to_string(), "No".to_string()],
                ["B".to_string(), "Ok".to_string(), "No".to_string()],
                ["C".to_string(), "Ok".to_string(), "Ok".to_string()],
            ])
        );
        let bad = topo("q", "S,0,0,0,0,0\nfaucetcoef,x\n");
        assert!(quick_check(&bad).is_err());
    }
}
