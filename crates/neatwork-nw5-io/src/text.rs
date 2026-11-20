//! Writers of `.tpo`, `.dsg` and `.db` files in the NW5 layout of
//! `specs/formats.md` section 12.3.
//!
//! Numbers use `%.15g` (`neatwork_nw5::read::g15`). Text fields (IDs,
//! `Nominal`, `SDR`, `type`) are written as they were read. Lines end with
//! LF and the file ends with a newline.

use neatwork_nw5::read::{g15, num};
use neatwork_nw5::{Network, Params, Pipe};

/// Technical parameters (MATLAB `partec`). They are written after the design
/// parameters (`pardsg`), each group in ASCII order.
pub const TECHNICAL_PARAMS: &[&str] = &["bescal", "faucetcoef", "lambda", "nu", "orifcoef"];

/// Parameters with canonical names, one entry per name (the last value of
/// the file wins), design parameters in ASCII order then technical ones.
/// Values are the raw text.
pub fn canonical_params(params: &Params) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, v) in &params.entries {
        let name = Params::canonical(k);
        if name.is_empty() {
            continue;
        }
        match out.iter_mut().find(|(n, _)| n == name) {
            Some(e) => e.1 = v.trim().to_string(),
            None => out.push((name.to_string(), v.trim().to_string())),
        }
    }
    out.sort_by(|(a, _), (b, _)| {
        let ta = TECHNICAL_PARAMS.contains(&a.as_str());
        let tb = TECHNICAL_PARAMS.contains(&b.as_str());
        ta.cmp(&tb).then_with(|| a.as_bytes().cmp(b.as_bytes()))
    });
    out
}

/// Text of a parameter value: `%.15g` for a number, else the raw text.
pub fn param_text(value: &str) -> String {
    num(value).map_or_else(|| value.trim().to_string(), g15)
}

/// The catalogue IDs are those a 7-field catalogue gets on reading
/// (`D001`, `D002`, ... in order of increasing diameter).
pub fn generated_ids(pipes: &[Pipe]) -> bool {
    pipes.iter().enumerate().all(|(k, p)| p.id == format!("D{:03}", k + 1))
        && pipes.windows(2).all(|w| w[0].diameter <= w[1].diameter)
}

fn section(out: &mut String, header: &str, rows: impl IntoIterator<Item = String>) {
    out.push_str(header);
    out.push('\n');
    for r in rows {
        out.push_str(&r);
        out.push('\n');
    }
}

fn comments(out: &mut String, net: &Network) {
    if !net.comments.is_empty() {
        section(out, "!Commentaires", net.comments.iter().cloned());
    }
}

fn params(out: &mut String, net: &Network) {
    let rows = canonical_params(&net.params).into_iter().map(|(k, v)| format!("{k},{}", param_text(&v)));
    section(out, "!Default properties", rows);
}

fn loadfactor(out: &mut String, net: &Network) {
    if !net.loadfactor_raw.is_empty() {
        section(out, "!loadfactor", net.loadfactor_raw.iter().map(|r| r.join(",")));
    }
}

fn catalogue_row(p: &Pipe) -> String {
    format!(
        "{},{},{},{},{},{},{}",
        p.nominal,
        p.sdr,
        g15(p.diameter),
        g15(p.cost),
        g15(p.max_pressure),
        p.kind,
        g15(p.roughness)
    )
}

fn sorted_clients(net: &Network) -> Vec<String> {
    let mut clients: Vec<_> = net.clients.iter().collect();
    clients.sort_by(|a, b| {
        a.terminal
            .as_bytes()
            .cmp(b.terminal.as_bytes())
            .then_with(|| a.id.as_bytes().cmp(b.id.as_bytes()))
    });
    clients
        .iter()
        .map(|c| format!("{},{},{},{}", c.id, c.terminal, g15(c.probability), g15(c.target_flow)))
        .collect()
}

fn demand(out: &mut String, net: &Network) {
    if !net.clients.is_empty() {
        section(out, "!Clients", sorted_clients(net));
    }
    if !net.terminals.is_empty() {
        section(out, "!Borne", net.terminals.iter().map(|t| format!("{},{},N", t.id, g15(t.pressure))));
    }
}

fn extra(out: &mut String, net: &Network) {
    for (header, lines) in &net.extra_sections {
        section(out, header, lines.iter().cloned());
    }
}

/// Write a `.tpo` file: `!Commentaires` (if any), `!Nodes` (6 fields),
/// `!Pipes` (3 fields), `!Default properties`, `!loadfactor` (rows kept from
/// the file, if any), `!Diameters` (7 fields for generated IDs, else 8 with
/// the ID first), `!Orifices` (`d,N`), `!Clients`, `!Borne`, then the
/// unknown sections verbatim. Pipe choices and design flows are not part of
/// a topography and are not written.
pub fn write_tpo(net: &Network) -> String {
    let mut out = String::new();
    comments(&mut out, net);
    section(
        &mut out,
        "!Nodes",
        net.nodes.iter().map(|n| {
            let taps = n
                .taps
                .unwrap_or_else(|| net.clients.iter().filter(|c| c.terminal == n.id).count() as f64);
            format!("{},{},{},{},{},{}", n.id, g15(n.elevation), g15(n.x), g15(n.y), g15(taps), n.status.code())
        }),
    );
    section(&mut out, "!Pipes", net.arcs.iter().map(|a| format!("{},{},{}", a.from, a.to, g15(a.length))));
    params(&mut out, net);
    loadfactor(&mut out, net);
    if !net.pipes.is_empty() {
        let with_id = !generated_ids(&net.pipes);
        section(
            &mut out,
            "!Diameters",
            net.pipes.iter().map(|p| {
                if with_id {
                    format!("{},{}", p.id, catalogue_row(p))
                } else {
                    catalogue_row(p)
                }
            }),
        );
    }
    if !net.orifices.is_empty() {
        section(&mut out, "!Orifices", net.orifices.iter().map(|d| format!("{},N", g15(*d))));
    }
    demand(&mut out, net);
    extra(&mut out, net);
    out
}

/// Write a `.dsg` file in the current NW5 layout: `!Commentaires` (if any),
/// `!Nodes` (7 fields), `!Pipes` (8 fields, `fictif` for a segment without
/// pipe), `!Default properties`, `!loadfactor` (NW4 meaning, rows kept from
/// the file), `!Diameters` (9 fields), `!Orifices` (6 fields), `!Clients`
/// sorted by `(borne, client)`, `!Borne`, `!loadflow` (design flows and
/// margins, ADR-022), then the unknown sections verbatim (`!Hydraulics`,
/// `!ComplementTA`, ... of a design).
pub fn write_dsg(net: &Network) -> String {
    let mut out = String::new();
    comments(&mut out, net);
    section(
        &mut out,
        "!Nodes",
        net.nodes.iter().map(|n| {
            format!(
                "{},{},{},{},{},{},{}",
                n.id,
                g15(n.elevation),
                g15(n.x),
                g15(n.y),
                g15(n.orifice_ideal),
                g15(n.orifice_commercial),
                n.status.code()
            )
        }),
    );
    let id = |p: &Option<String>| match p.as_deref() {
        None | Some("") => "fictif".to_string(),
        Some(s) => s.to_string(),
    };
    section(
        &mut out,
        "!Pipes",
        net.arcs.iter().map(|a| {
            format!(
                "{},{},{},{},{},{},{},{}",
                a.from,
                a.to,
                g15(a.length),
                g15(a.length1),
                id(&a.pipe1),
                g15(a.length2),
                id(&a.pipe2),
                a.state.code()
            )
        }),
    );
    params(&mut out, net);
    loadfactor(&mut out, net);
    if !net.pipes.is_empty() {
        section(&mut out, "!Diameters", net.pipes.iter().map(|p| format!("{},{},N", p.id, catalogue_row(p))));
    }
    if !net.orifices.is_empty() {
        section(&mut out, "!Orifices", net.orifices.iter().map(|d| format!("{},N,N,N,N,N", g15(*d))));
    }
    demand(&mut out, net);
    if !net.loadflow.is_empty() {
        section(
            &mut out,
            "!loadflow",
            net.loadflow.iter().map(|(succ, q, m)| format!("{succ},{},{}", g15(*q), g15(*m))),
        );
    }
    extra(&mut out, net);
    out
}

/// Write a diameter catalogue `.db`: one pipe per line, 7 fields
/// `Nominal, SDR, diam, cost, pressure, type, Roughness`, no ID.
pub fn write_diameters_db(pipes: &[Pipe]) -> String {
    let mut out = String::new();
    for p in pipes {
        out.push_str(&catalogue_row(p));
        out.push('\n');
    }
    out
}

/// Write an orifice catalogue `.db`: one inner diameter (m) per line.
pub fn write_orifices_db(orifices: &[f64]) -> String {
    let mut out = String::new();
    for d in orifices {
        out.push_str(&g15(*d));
        out.push('\n');
    }
    out
}
