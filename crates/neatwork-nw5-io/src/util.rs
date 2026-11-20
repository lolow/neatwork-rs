//! Helpers shared by the readers of this crate.

use crate::json::Json;
use neatwork_nw5::read::{g15, num};
use neatwork_nw5::{Client, Network, NodeStatus, Terminal};

/// Pipe reference of a segment: `fictif`, `0`, `N`, `-` and empty mean "no
/// pipe" for a segment of length 0. For a segment with a length they give an
/// empty reference that validation reports (as `neatwork_nw5::read`).
pub fn pipe_ref(id: Option<String>, length: f64) -> Option<String> {
    let t = id.unwrap_or_default();
    let t = t.trim();
    let none = matches!(t, "" | "0" | "N" | "-" | "fictif");
    match (none, length == 0.0) {
        (true, true) => None,
        (true, false) => Some(String::new()),
        _ => Some(t.to_string()),
    }
}

/// A text field as a JSON number when the number writes back to the same
/// text (`%.15g`), else as a string. Reading [`Json::text`] gives the text
/// back unchanged.
pub fn lossless(text: &str) -> Json {
    match num(text) {
        Some(x) if x.is_finite() && g15(x) == text => Json::Number(x),
        _ => Json::String(text.to_string()),
    }
}

/// Parameter text from a JSON value. `null` gives `None`.
pub fn param_text(v: &Json) -> Option<String> {
    match v {
        Json::Bool(b) => Some(if *b { "1" } else { "0" }.into()),
        Json::Number(_) | Json::String(_) => v.text(),
        _ => None,
    }
}

/// Clients and bornes of a file that has none: one client `C<k><node>` per
/// tap (`taps`, else 1) of each terminal with the uniform demand, and one
/// borne per terminal at the `pressure` parameter. Returns warnings.
pub fn complete_demand(net: &mut Network) -> Vec<String> {
    let mut warnings = Vec::new();
    if net.clients.is_empty() {
        let (p, q) = (net.params.get("opentaps"), net.params.get("targetflow"));
        for node in net
            .nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Terminal)
        {
            let k = node.taps.unwrap_or(1.0).max(0.0) as usize;
            for i in 1..=k {
                net.clients.push(Client {
                    id: format!("C{i}{}", node.id),
                    terminal: node.id.clone(),
                    probability: p,
                    target_flow: q,
                });
            }
        }
        net.clients
            .sort_by(|a, b| (&a.terminal, &a.id).cmp(&(&b.terminal, &b.id)));
        if !net.clients.is_empty() {
            warnings.push(format!(
                "no client table: {} clients generated from the terminals",
                net.clients.len()
            ));
        }
    }
    if net.terminals.is_empty() {
        let pressure = net.params.get("pressure");
        net.terminals = net
            .nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Terminal)
            .map(|n| Terminal {
                id: n.id.clone(),
                pressure,
            })
            .collect();
        if !net.terminals.is_empty() {
            warnings.push(format!(
                "no borne table: {} bornes generated",
                net.terminals.len()
            ));
        }
    }
    warnings
}
