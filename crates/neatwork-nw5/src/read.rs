//! Readers of NW5 text files: `.tpo`, `.dsg` and `.db` catalogues
//! (`rust/nw5/specs/formats.md` sections 1, 3, 4, 11 and 12.3).
//!
//! The readers are lenient: they accept the variants of the three NW5 code
//! bases and of NW4.

use crate::model::{Arc, ArcState, Client, Network, Node, NodeStatus, Params, Pipe, Terminal};
use crate::{Error, Result};

/// A section of a text file: canonical name, raw header and data lines.
#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub header: String,
    pub lines: Vec<String>,
}

/// Canonical name of a header: text after `!`, trailing commas and spaces
/// removed, lower case, `_` and `-` as spaces, aliases mapped.
pub fn canonical_section(header: &str) -> String {
    let t = header.trim().trim_start_matches('!');
    let t = t.trim_end_matches([',', ' ', '\t']);
    let t: String = t
        .chars()
        .map(|c| if c == '_' || c == '-' { ' ' } else { c })
        .collect::<String>()
        .to_lowercase();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    match t.as_str() {
        "bornes" => "borne".to_string(),
        "client" => "clients".to_string(),
        _ => t,
    }
}

/// Split a file into sections. Consecutive identical headers count as one.
/// If a section appears twice with data, the first block wins (a warning is
/// returned).
pub fn sections(text: &str) -> (Vec<Section>, Vec<String>) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out: Vec<Section> = Vec::new();
    let mut warnings = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        let t = line.trim();
        if t.starts_with('!') {
            let name = canonical_section(t);
            // consecutive identical header: same section
            if let Some(last) = out.last()
                && last.name == name
                && last.lines.is_empty()
            {
                continue;
            }
            out.push(Section {
                name,
                header: t.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(s) = out.last_mut() {
            s.lines.push(line.to_string());
        }
    }
    // keep the first block of a repeated section
    let mut seen: Vec<String> = Vec::new();
    let mut kept = Vec::new();
    for s in out {
        if seen.contains(&s.name) {
            if !s.lines.is_empty() {
                warnings.push(format!("section !{} appears twice; the first block is used", s.name));
            }
            continue;
        }
        seen.push(s.name.clone());
        kept.push(s);
    }
    (kept, warnings)
}

/// Fields of a data line: split on `,`, each field trimmed. No quoting.
pub fn fields(line: &str) -> Vec<String> {
    line.split(',').map(|f| f.trim().to_string()).collect()
}

/// Number with the readers' syntax (`Inf`, `NaN` accepted).
pub fn num(s: &str) -> Option<f64> {
    let t = s.trim();
    match t.to_ascii_lowercase().as_str() {
        "inf" | "+inf" | "infinity" => return Some(f64::INFINITY),
        "-inf" | "-infinity" => return Some(f64::NEG_INFINITY),
        "nan" => return Some(f64::NAN),
        _ => {}
    }
    t.parse::<f64>().ok()
}

fn num_or(s: Option<&String>, line: &str, what: &str) -> Result<f64> {
    s.and_then(|v| num(v))
        .ok_or_else(|| Error::Format(format!("{what}: not a number in line `{line}`")))
}

/// "No pipe" markers of a segment.
fn pipe_ref(id: &str, length: f64) -> Option<String> {
    let t = id.trim();
    if matches!(t, "" | "0" | "N" | "-" | "fictif") && length == 0.0 {
        None
    } else if matches!(t, "" | "0" | "N" | "-" | "fictif") {
        // a length without a pipe: kept as an empty reference, validation
        // reports it
        Some(String::new())
    } else {
        Some(t.to_string())
    }
}

fn status(v: &str, line: &str) -> Result<NodeStatus> {
    num(v)
        .and_then(|x| NodeStatus::from_code(x as i32))
        .ok_or_else(|| Error::Format(format!("node status must be 0, 1 or 2 in line `{line}`")))
}

/// Read a `.db` diameter catalogue (rows `Nominal, SDR, diam, cost,
/// pressure, type, Roughness`, or with an ID first). IDs `D001`... are given
/// after a stable sort by diameter.
pub fn read_diameters_db(text: &str) -> Result<Vec<Pipe>> {
    let rows: Vec<Vec<String>> = text
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('!') && !l.trim().starts_with('#'))
        .map(fields)
        .collect();
    diameters_from_rows(&rows, false)
}

/// Catalogue rows of a `.tpo` (7 or 8 fields) or `.dsg` (8 or 9 fields
/// with an ID first).
fn diameters_from_rows(rows: &[Vec<String>], dsg: bool) -> Result<Vec<Pipe>> {
    let mut with_id = Vec::new();
    let mut without_id = Vec::new();
    for r in rows {
        let line = r.join(",");
        let has_id = if dsg { r.len() >= 8 } else { r.len() >= 8 };
        if r.len() < 7 {
            continue;
        }
        let o = usize::from(has_id);
        let pipe = Pipe {
            id: if has_id { r[0].clone() } else { String::new() },
            nominal: r[o].clone(),
            sdr: r[o + 1].clone(),
            diameter: num_or(r.get(o + 2), &line, "diameter")?,
            cost: num_or(r.get(o + 3), &line, "cost")?,
            max_pressure: num_or(r.get(o + 4), &line, "maximum pressure")?,
            kind: r.get(o + 5).cloned().unwrap_or_default(),
            roughness: r.get(o + 6).and_then(|v| num(v)).unwrap_or(0.0),
        };
        if pipe.id == "fictif" || pipe.nominal == "fictif" {
            continue;
        }
        if has_id {
            with_id.push(pipe);
        } else {
            without_id.push(pipe);
        }
    }
    if !without_id.is_empty() {
        without_id.sort_by(|a, b| a.diameter.partial_cmp(&b.diameter).unwrap_or(std::cmp::Ordering::Equal));
        for (k, p) in without_id.iter_mut().enumerate() {
            p.id = format!("D{:03}", k + 1);
        }
    }
    with_id.extend(without_id);
    Ok(with_id)
}

/// Read an orifice catalogue (`.db`: one diameter per line, extra fields
/// ignored).
pub fn read_orifices_db(text: &str) -> Vec<f64> {
    text.lines()
        .filter_map(|l| fields(l).first().and_then(|f| num(f)))
        .collect()
}

fn read_params(s: &Section, params: &mut Params) {
    for l in &s.lines {
        let f = fields(l);
        if f.len() >= 2 && !f[0].is_empty() {
            params.entries.push((f[0].clone(), f[1].clone()));
        }
    }
}

fn read_clients(s: &Section) -> Result<Vec<Client>> {
    let mut out = Vec::new();
    for l in &s.lines {
        let f = fields(l);
        if f.len() < 4 {
            continue;
        }
        out.push(Client {
            id: f[0].clone(),
            terminal: f[1].clone(),
            probability: num_or(f.get(2), l, "client probability")?,
            target_flow: num_or(f.get(3), l, "client target flow")?,
        });
    }
    Ok(out)
}

fn read_terminals(s: &Section) -> Result<Vec<Terminal>> {
    let mut out = Vec::new();
    for l in &s.lines {
        let f = fields(l);
        if f.len() < 2 {
            continue;
        }
        out.push(Terminal {
            id: f[0].clone(),
            pressure: num_or(f.get(1), l, "terminal pressure")?,
        });
    }
    Ok(out)
}

/// Clients `C<k><borne>` (k = 1..nbrob) and bornes for a file without
/// `!Clients` or `!Borne`.
fn complete_demand(net: &mut Network, has_clients: bool, has_terminals: bool) -> Vec<String> {
    let mut warnings = Vec::new();
    if !has_clients {
        let p = net.params.get("opentaps");
        let q = net.params.get("targetflow");
        let mut clients = Vec::new();
        for n in net.nodes.iter().filter(|n| n.status == NodeStatus::Terminal) {
            let k = n.taps.unwrap_or(0.0).max(0.0) as usize;
            for i in 1..=k {
                clients.push(Client {
                    id: format!("C{i}{}", n.id),
                    terminal: n.id.clone(),
                    probability: p,
                    target_flow: q,
                });
            }
        }
        // MATLAB order: (borne, client), plain string order
        clients.sort_by(|a, b| (&a.terminal, &a.id).cmp(&(&b.terminal, &b.id)));
        warnings.push(format!("no !Clients section: {} clients generated from the tap counts", clients.len()));
        net.clients = clients;
    }
    if !has_terminals {
        let p = net.params.get("pressure");
        net.terminals = net
            .nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Terminal)
            .map(|n| Terminal {
                id: n.id.clone(),
                pressure: p,
            })
            .collect();
        warnings.push(format!("no !Borne section: {} bornes generated", net.terminals.len()));
    }
    warnings
}

/// Result of a read: the network and the warnings.
pub struct Read {
    pub network: Network,
    pub warnings: Vec<String>,
}

/// Read a `.tpo` file (NW4 or NW5).
pub fn read_tpo(name: &str, text: &str) -> Result<Read> {
    let (secs, mut warnings) = sections(text);
    let mut net = Network {
        name: name.to_string(),
        ..Network::default()
    };
    let (mut has_clients, mut has_terminals) = (false, false);
    for s in &secs {
        match s.name.as_str() {
            "commentaires" => net.comments = s.lines.clone(),
            "nodes" => {
                for l in &s.lines {
                    let f = fields(l);
                    if f.len() < 6 {
                        continue;
                    }
                    net.nodes.push(Node {
                        id: f[0].clone(),
                        elevation: num_or(f.get(1), l, "elevation")?,
                        x: f.get(2).and_then(|v| num(v)).unwrap_or(0.0),
                        y: f.get(3).and_then(|v| num(v)).unwrap_or(0.0),
                        taps: f.get(4).and_then(|v| num(v)),
                        orifice_ideal: 0.0,
                        orifice_commercial: 0.0,
                        status: status(&f[5], l)?,
                    });
                }
            }
            "pipes" => {
                for l in &s.lines {
                    let f = fields(l);
                    if f.len() < 3 {
                        continue;
                    }
                    net.arcs.push(Arc {
                        id: format!("A{}", net.arcs.len() + 1),
                        from: f[0].clone(),
                        to: f[1].clone(),
                        length: num_or(f.get(2), l, "length")?,
                        length1: 0.0,
                        pipe1: None,
                        length2: 0.0,
                        pipe2: None,
                        state: ArcState::New,
                        open: true,
                    });
                }
            }
            "default properties" => read_params(s, &mut net.params),
            "diameters" => {
                let rows: Vec<Vec<String>> = s.lines.iter().map(|l| fields(l)).collect();
                net.pipes = diameters_from_rows(&rows, false)?;
            }
            "orifices" => net.orifices = s.lines.iter().filter_map(|l| fields(l).first().and_then(|f| num(f))).collect(),
            "clients" => {
                net.clients = read_clients(s)?;
                has_clients = true;
            }
            "borne" => {
                net.terminals = read_terminals(s)?;
                has_terminals = true;
            }
            "loadfactor" => net.loadfactor_raw = s.lines.iter().map(|l| fields(l)).collect(),
            _ => net.extra_sections.push((s.header.clone(), s.lines.clone())),
        }
    }
    warnings.extend(complete_demand(&mut net, has_clients, has_terminals));
    warnings.extend(net.apply_client_data());
    Ok(Read { network: net, warnings })
}

/// Read a `.dsg` file (NW4 or NW5).
pub fn read_dsg(name: &str, text: &str) -> Result<Read> {
    let (secs, mut warnings) = sections(text);
    let mut net = Network {
        name: name.to_string(),
        ..Network::default()
    };
    let (mut has_clients, mut has_terminals) = (false, false);
    for s in &secs {
        match s.name.as_str() {
            "commentaires" => net.comments = s.lines.clone(),
            "nodes" => {
                for l in &s.lines {
                    let f = fields(l);
                    if f.len() < 7 {
                        continue;
                    }
                    net.nodes.push(Node {
                        id: f[0].clone(),
                        elevation: num_or(f.get(1), l, "elevation")?,
                        x: f.get(2).and_then(|v| num(v)).unwrap_or(0.0),
                        y: f.get(3).and_then(|v| num(v)).unwrap_or(0.0),
                        taps: None,
                        orifice_ideal: f.get(4).and_then(|v| num(v)).unwrap_or(0.0),
                        orifice_commercial: f.get(5).and_then(|v| num(v)).unwrap_or(0.0),
                        status: status(&f[6], l)?,
                    });
                }
            }
            "pipes" => {
                for l in &s.lines {
                    let f = fields(l);
                    if f.len() < 7 {
                        continue;
                    }
                    let length1 = num_or(f.get(3), l, "length 1")?;
                    let length2 = num_or(f.get(5), l, "length 2")?;
                    net.arcs.push(Arc {
                        id: format!("A{}", net.arcs.len() + 1),
                        from: f[0].clone(),
                        to: f[1].clone(),
                        length: num_or(f.get(2), l, "length")?,
                        length1,
                        pipe1: pipe_ref(&f[4], length1),
                        length2,
                        pipe2: pipe_ref(&f[6], length2),
                        state: ArcState::from_code(f.get(7).map_or("N", String::as_str)),
                        open: true,
                    });
                }
            }
            "default properties" => read_params(s, &mut net.params),
            "diameters" => {
                let rows: Vec<Vec<String>> = s.lines.iter().map(|l| fields(l)).collect();
                net.pipes = diameters_from_rows(&rows, true)?;
            }
            "orifices" => net.orifices = s.lines.iter().filter_map(|l| fields(l).first().and_then(|f| num(f))).collect(),
            "clients" => {
                net.clients = read_clients(s)?;
                has_clients = true;
            }
            "borne" => {
                net.terminals = read_terminals(s)?;
                has_terminals = true;
            }
            "loadfactor" => net.loadfactor_raw = s.lines.iter().map(|l| fields(l)).collect(),
            "loadflow" => {
                for l in &s.lines {
                    let f = fields(l);
                    if f.len() >= 3
                        && let (Some(q), Some(m)) = (num(&f[1]), num(&f[2]))
                    {
                        net.loadflow.push((f[0].clone(), q, m));
                    }
                }
            }
            _ => net.extra_sections.push((s.header.clone(), s.lines.clone())),
        }
    }
    // NW4 files: one client per terminal with the uniform demand
    if !has_clients {
        for n in &mut net.nodes {
            if n.status == NodeStatus::Terminal {
                n.taps = Some(1.0);
            }
        }
    }
    warnings.extend(complete_demand(&mut net, has_clients, has_terminals));
    warnings.extend(net.apply_client_data());
    Ok(Read { network: net, warnings })
}

/// Read a file by its extension (`.tpo` or `.dsg`).
pub fn read_file(path: &std::path::Path) -> Result<Read> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    let text = String::from_utf8_lossy(&bytes);
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("tpo") => read_tpo(&name, &text),
        Some("dsg") => read_dsg(&name, &text),
        _ => Err(Error::Format(format!("{}: unknown file type (expected .tpo or .dsg)", path.display()))),
    }
}

/// Text of a number as MATLAB `num2str(x, '%.15g')` writes it.
pub fn g15(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Inf".into() } else { "-Inf".into() };
    }
    if x == 0.0 {
        return "0".into();
    }
    let exp = x.abs().log10().floor() as i32;
    let s = if !(-4..15).contains(&exp) {
        let m = format!("{x:.14e}");
        let (mant, e) = m.split_once('e').unwrap_or((&m, "0"));
        let mant = trim_zeros(mant);
        let e: i32 = e.parse().unwrap_or(0);
        format!("{mant}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
    } else {
        let decimals = (14 - exp).max(0) as usize;
        trim_zeros(&format!("{x:.decimals$}"))
    };
    s
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_names() {
        assert_eq!(canonical_section("!Nodes,,"), "nodes");
        assert_eq!(canonical_section("!Default Properties"), "default properties");
        assert_eq!(canonical_section("!Bornes"), "borne");
        assert_eq!(canonical_section("!load_factor"), "load factor");
    }

    #[test]
    fn g15_like_matlab() {
        assert_eq!(g15(200.0), "200");
        assert_eq!(g15(0.0182), "0.0182");
        assert_eq!(g15(1.87e-8), "1.87e-08");
        assert_eq!(g15(1e-6), "1e-06");
        assert_eq!(g15(5.5737646484375), "5.5737646484375");
        assert_eq!(g15(f64::INFINITY), "Inf");
        assert_eq!(g15(1e15), "1e+15");
    }

    #[test]
    fn tpo_without_clients() -> Result<()> {
        let t = "!Nodes,\nS,0,0,0,0,0\nA,-10,0,0,0,1\nP,-9,0,0,2,2\n!Pipes\nS,A,20\nA,P,8\n!Default properties\nopentaps,0.4\ntargetflow,0.2\n";
        let r = read_tpo("t", t)?;
        let n = r.network;
        assert_eq!(n.nodes.len(), 3);
        assert_eq!(n.clients.len(), 2);
        assert_eq!(n.clients[0].id, "C1P");
        assert_eq!(n.terminals.len(), 1);
        assert_eq!(n.params.get("uniform_dem"), 1.0);
        Ok(())
    }
}
