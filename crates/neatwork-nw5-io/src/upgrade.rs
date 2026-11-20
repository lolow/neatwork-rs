//! NW4 to NW5 upgrade of a `.dsg` (`M:InOut/maj_NW4_NW5.m`,
//! `specs/formats.md` section 5.2, Rust rule).
//!
//! NW4 represents a terminal with several taps as a parent node and
//! fictitious faucets `<P>_a`, `<P>_b`, ... joined by 1 m pipes. The
//! upgrade merges them back into one terminal with one client per faucet.
//! Unlike the MATLAB script, a faucet is recognised by its structure, not by
//! the `_` in its name: a terminal `<P>_<lower-case letters>` with exactly
//! one incoming pipe, of length 1 m, from an existing node `<P>`, and no
//! outgoing pipe.

use neatwork_nw5::read::{Read, read_dsg, sections};
use neatwork_nw5::{Client, Error, Network, NodeStatus, Result, Terminal};

/// The node is a fictitious NW4 faucet; returns its parent.
fn faucet_parent<'a>(net: &'a Network, id: &str) -> Option<&'a str> {
    let node = net.node(id)?;
    if node.status != NodeStatus::Terminal || net.arcs.iter().any(|a| a.from == id) {
        return None;
    }
    let incoming: Vec<_> = net.arcs.iter().filter(|a| a.to == id).collect();
    let [arc] = incoming.as_slice() else {
        return None;
    };
    let suffix = id.strip_prefix(arc.from.as_str())?.strip_prefix('_')?;
    let is_faucet = !suffix.is_empty()
        && suffix.chars().all(|c| c.is_ascii_lowercase())
        && (arc.length - 1.0).abs() < 1e-9
        && net.node(&arc.from).is_some();
    is_faucet.then_some(arc.from.as_str())
}

/// Upgrade a NW4 design read by `neatwork_nw5::read::read_dsg`: merge the
/// fictitious faucets into their parent (the parent takes the elevation,
/// coordinates, orifices and terminal status of its first faucet), drop
/// their pipes and `!loadfactor` rows, add the missing parameters
/// `pressure = 2`, `bounded_flow = 0`, `uniform_dem = 1`, and rebuild the
/// clients (`C<k><node>`, one per faucet, else one per terminal, with
/// `opentaps` and `targetflow`) and the bornes. Returns warnings.
pub fn upgrade_nw4_network(input: &Network) -> (Network, Vec<String>) {
    let mut net = input.clone();
    let mut warnings = Vec::new();
    let faucets: Vec<(String, String)> = net
        .nodes
        .iter()
        .filter_map(|n| faucet_parent(input, &n.id).map(|p| (n.id.clone(), p.to_string())))
        .collect();
    let mut taps: Vec<(String, usize)> = Vec::new();
    for (faucet, parent) in &faucets {
        match taps.iter_mut().find(|(p, _)| p == parent) {
            Some(e) => e.1 += 1,
            None => {
                taps.push((parent.clone(), 1));
                if let (Some(f), Some(p)) = (input.node(faucet).cloned(), net.nodes.iter_mut().find(|n| n.id == *parent)) {
                    *p = neatwork_nw5::Node { id: parent.clone(), taps: None, ..f };
                }
            }
        }
    }
    let is_faucet = |id: &str| faucets.iter().any(|(f, _)| f == id);
    net.nodes.retain(|n| !is_faucet(&n.id));
    net.arcs.retain(|a| !is_faucet(&a.to));
    for (k, a) in net.arcs.iter_mut().enumerate() {
        a.id = format!("A{}", k + 1);
    }
    net.loadfactor_raw.retain(|r| r.first().is_none_or(|n| !is_faucet(n)));
    if !faucets.is_empty() {
        warnings.push(format!("{} fictitious faucets merged into {} terminals", faucets.len(), taps.len()));
    }
    for (name, value) in [("pressure", "2"), ("bounded_flow", "0"), ("uniform_dem", "1")] {
        if net.params.raw(name).is_none() {
            net.params.set(name, value);
        }
    }
    let (p, q, pressure) = (net.params.get("opentaps"), net.params.get("targetflow"), net.params.get("pressure"));
    net.clients.clear();
    for n in net.nodes.iter().filter(|n| n.status == NodeStatus::Terminal) {
        let k = taps.iter().find(|(id, _)| *id == n.id).map_or(1, |(_, k)| *k);
        for i in 1..=k {
            net.clients.push(Client {
                id: format!("C{i}{}", n.id),
                terminal: n.id.clone(),
                probability: p,
                target_flow: q,
            });
        }
    }
    net.clients.sort_by(|a, b| (&a.terminal, &a.id).cmp(&(&b.terminal, &b.id)));
    net.terminals = net
        .nodes
        .iter()
        .filter(|n| n.status == NodeStatus::Terminal)
        .map(|n| Terminal { id: n.id.clone(), pressure })
        .collect();
    warnings.extend(net.apply_client_data());
    (net, warnings)
}

/// Upgrade the text of a NW4 `.dsg`. A file with a `!Clients` section is
/// not a NW4 file and is refused.
pub fn upgrade_nw4_dsg(name: &str, text: &str) -> Result<Read> {
    if sections(text).0.iter().any(|s| s.name == "clients") {
        return Err(Error::Format(
            "Ce fichier contient une section Clients. Ce n'est pas un fichier NW4.".into(),
        ));
    }
    let read = read_dsg(name, text)?;
    let (network, mut warnings) = upgrade_nw4_network(&read.network);
    warnings.splice(0..0, read.warnings);
    Ok(Read { network, warnings })
}
