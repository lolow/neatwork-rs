//! MATLAB design JSON `NW-json-v1` (`M:InOut/save_dsg_json.m`,
//! `M:InOut/read_Dsg_json.m`, `specs/formats.md` section 7.1).
//!
//! Writing follows `save_dsg_json` key order: `schema, TN, TA, TB, TC, TD,
//! TO, TFact, TAH, TCP, TS, TPar, pardsg, partec, reinfcost`, then `meta`
//! (a struct key that `read_Dsg_json` accepts). `meta` holds what the
//! MATLAB tables do not: the network name, the comments, the NW4
//! `!loadfactor` rows and the unknown sections. `TFact` holds the design
//! flows and margins (`!loadflow`, ADR-022). `TAH`, `TCP`, `TS` and
//! `reinfcost` are the `!Hydraulics`, `!ComplementTA`,
//! `!ConstraintSatisfaction` and `!Costs` sections of a design.
//!
//! Non-finite numbers are written as `null`, as `jsonencode` does. On
//! reading, a `null` parameter takes its default (`limitbudget` = `Inf`).

use crate::json::{Json, n, obj, s};
use crate::text::{TECHNICAL_PARAMS, canonical_params};
use crate::util::{complete_demand, lossless, param_text, pipe_ref};
use neatwork_nw5::model::DEFAULTS;
use neatwork_nw5::read::{Read, fields, g15, num};
use neatwork_nw5::{Arc, ArcState, Client, Error, Network, Node, NodeStatus, Pipe, Result, Terminal};

/// Version tag of the format.
pub const NW_JSON_SCHEMA: &str = "NW-json-v1";

/// Design result sections stored as MATLAB tables: `(header, key, columns)`.
/// `!Costs` (`name,value` rows) is the `reinfcost` struct.
const RESULT_TABLES: &[(&str, &str, &[&str])] = &[
    ("!Hydraulics", "TAH", &["pred", "succ", "equivResist", "designFlow", "pathLoss", "margin"]),
    ("!ComplementTA", "TCP", &["pred", "succ", "nature", "diam1", "diam2"]),
    (
        "!ConstraintSatisfaction",
        "TS",
        &["noeud", "constraint_type", "head_required", "head_available", "head_margin", "head_deficit"],
    ),
];

/// The rows of a section can be stored as a table of `columns` without
/// loss: canonical header, exact field count, no spaces around fields.
fn as_table(header: &str, lines: &[String], expected: &str, columns: usize) -> Option<Vec<Vec<String>>> {
    if header != expected {
        return None;
    }
    let rows: Vec<Vec<String>> = lines.iter().map(|l| fields(l)).collect();
    let exact = rows.iter().zip(lines).all(|(r, l)| r.len() == columns && r.join(",") == *l);
    exact.then_some(rows)
}

fn param_json(text: &str) -> Json {
    match num(text) {
        Some(x) => n(x),
        None => s(text),
    }
}

/// Write a network as a MATLAB `NW-json-v1` design JSON (pretty-printed).
pub fn write_nw_json(net: &Network) -> String {
    let tn = net
        .nodes
        .iter()
        .map(|x| {
            obj(vec![
                ("noeud", s(&x.id)),
                ("haut", n(x.elevation)),
                ("xdata", n(x.x)),
                ("ydata", n(x.y)),
                ("orif_ideal", n(x.orifice_ideal)),
                ("orif_com", n(x.orifice_commercial)),
                ("statut", Json::Number(f64::from(x.status.code()))),
            ])
        })
        .collect();
    let id = |p: &Option<String>| s(p.as_deref().filter(|t| !t.is_empty()).unwrap_or("fictif"));
    let ta = net
        .arcs
        .iter()
        .map(|a| {
            obj(vec![
                ("pred", s(&a.from)),
                ("succ", s(&a.to)),
                ("lon", n(a.length)),
                ("lon1", n(a.length1)),
                ("ID1", id(&a.pipe1)),
                ("lon2", n(a.length2)),
                ("ID2", id(&a.pipe2)),
                ("N", s(a.state.code())),
            ])
        })
        .collect();
    let tb = net
        .terminals
        .iter()
        .map(|t| obj(vec![("borne", s(&t.id)), ("pressure", n(t.pressure)), ("N", s("N"))]))
        .collect();
    let tc = net
        .clients
        .iter()
        .map(|c| {
            obj(vec![
                ("client", s(&c.id)),
                ("borne", s(&c.terminal)),
                ("proba", n(c.probability)),
                ("cible", n(c.target_flow)),
            ])
        })
        .collect();
    let td = net
        .pipes
        .iter()
        .map(|p| {
            obj(vec![
                ("ID", s(&p.id)),
                ("Nominal", s(&p.nominal)),
                ("SDR", lossless(&p.sdr)),
                ("diam", n(p.diameter)),
                ("cost", n(p.cost)),
                ("pressure", n(p.max_pressure)),
                ("type", lossless(&p.kind)),
                ("Roughness", n(p.roughness)),
            ])
        })
        .collect();
    let to = net.orifices.iter().map(|d| obj(vec![("in_diam", n(*d))])).collect();
    let tfact = net
        .loadflow
        .iter()
        .map(|(succ, q, m)| obj(vec![("succ", s(succ)), ("designFlow", n(*q)), ("margin", n(*m))]))
        .collect();

    // design result sections, others go to meta
    let mut results: Vec<(&str, Json)> = Vec::new();
    let mut reinfcost = None;
    let mut extra = Vec::new();
    for (header, lines) in &net.extra_sections {
        let table = RESULT_TABLES.iter().find_map(|(h, key, cols)| {
            as_table(header, lines, h, cols.len()).map(|rows| (*key, *cols, rows))
        });
        if let Some((key, cols, rows)) = table
            && !results.iter().any(|(k, _)| *k == key)
        {
            let rows = rows
                .iter()
                .map(|r| Json::Object(cols.iter().zip(r).map(|(c, v)| (c.to_string(), lossless(v))).collect()))
                .collect();
            results.push((key, Json::Array(rows)));
        } else if let Some(rows) = as_table(header, lines, "!Costs", 2)
            && reinfcost.is_none()
        {
            reinfcost = Some(Json::Object(rows.iter().map(|r| (r[0].clone(), lossless(&r[1]))).collect()));
        } else {
            extra.push(obj(vec![
                ("header", s(header)),
                ("lines", Json::Array(lines.iter().map(|l| s(l)).collect())),
            ]));
        }
    }
    let result = |key: &str| {
        results
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(Json::Array(Vec::new()), |(_, v)| v.clone())
    };

    let params = canonical_params(&net.params);
    let tpar = params
        .iter()
        .map(|(k, v)| obj(vec![("name", s(k)), ("value", param_json(v))]))
        .collect();
    let group = |technical: bool| {
        Json::Object(
            params
                .iter()
                .filter(|(k, _)| TECHNICAL_PARAMS.contains(&k.as_str()) == technical)
                .map(|(k, v)| (k.clone(), param_json(v)))
                .collect(),
        )
    };

    let mut top = vec![
        ("schema", s(NW_JSON_SCHEMA)),
        ("TN", Json::Array(tn)),
        ("TA", Json::Array(ta)),
        ("TB", Json::Array(tb)),
        ("TC", Json::Array(tc)),
        ("TD", Json::Array(td)),
        ("TO", Json::Array(to)),
        ("TFact", Json::Array(tfact)),
        ("TAH", result("TAH")),
        ("TCP", result("TCP")),
        ("TS", result("TS")),
        ("TPar", Json::Array(tpar)),
        ("pardsg", group(false)),
        ("partec", group(true)),
    ];
    if let Some(r) = reinfcost {
        top.push(("reinfcost", r));
    }
    top.push((
        "meta",
        obj(vec![
            ("name", s(&net.name)),
            ("comments", Json::Array(net.comments.iter().map(|c| s(c)).collect())),
            (
                "loadfactor",
                Json::Array(
                    net.loadfactor_raw
                        .iter()
                        .map(|r| Json::Array(r.iter().map(|f| s(f)).collect()))
                        .collect(),
                ),
            ),
            ("extraSections", Json::Array(extra)),
        ]),
    ));
    obj(top).pretty()
}

/// Text of a column (numbers as `%.15g`).
fn col(row: &Json, keys: &[&str]) -> Option<String> {
    row.first(keys).and_then(Json::text)
}

/// Number of a column. `null` is NaN, a missing column is `None`.
fn numcol(row: &Json, keys: &[&str]) -> Option<f64> {
    if keys.iter().any(|k| matches!(row.get(k), Some(Json::Null))) && row.first(keys).is_none() {
        return Some(f64::NAN);
    }
    row.first(keys).and_then(Json::as_f64)
}

fn need(row: &Json, keys: &[&str], table: &str) -> Result<f64> {
    numcol(row, keys).ok_or_else(|| Error::Format(format!("{table}: missing or invalid column {}", keys[0])))
}

fn needtext(row: &Json, keys: &[&str], table: &str) -> Result<String> {
    col(row, keys)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::Format(format!("{table}: missing column {}", keys[0])))
}

/// Read a MATLAB `NW-json-v1` design JSON. The table keys of
/// `read_Dsg_json` are accepted (`TN`/`TNodes`, `TA`/`TArcs`,
/// `TC`/`TClients`, `TD`/`TDn`, `TO`/`TOn`), a table can be an array or a
/// single object, and the `fictif` catalogue row is dropped. `name` is used
/// when not empty, else `meta.name`.
pub fn read_nw_json(name: &str, text: &str) -> Result<Read> {
    let j = Json::parse(text)?;
    if !matches!(j, Json::Object(_)) {
        return Err(Error::Format("NW-json: the top level must be an object".into()));
    }
    let mut warnings = Vec::new();
    match j.get("schema").and_then(Json::text) {
        Some(t) if t != NW_JSON_SCHEMA => warnings.push(format!("NW-json: unknown schema `{t}`, read as {NW_JSON_SCHEMA}")),
        _ => {}
    }
    let table = |keys: &[&str]| j.first(keys).map(Json::rows).unwrap_or_default();
    let meta = j.get("meta");
    let mut net = Network {
        name: if name.is_empty() {
            meta.and_then(|m| col(m, &["name"])).unwrap_or_default()
        } else {
            name.to_string()
        },
        ..Network::default()
    };

    // parameters: TPar, else pardsg then partec; null takes the default
    let mut push_param = |k: &str, v: &Json| {
        let value = param_text(v).or_else(|| {
            let canonical = neatwork_nw5::Params::canonical(k);
            DEFAULTS.iter().find(|(d, _)| *d == canonical).map(|(_, x)| g15(*x))
        });
        net.params.set(neatwork_nw5::Params::canonical(k), &value.unwrap_or_else(|| "NaN".into()));
    };
    let tpar = table(&["TPar"]);
    if tpar.is_empty() {
        for key in ["pardsg", "partec"] {
            if let Some(Json::Object(m)) = j.get(key) {
                for (k, v) in m {
                    push_param(k, v);
                }
            }
        }
    } else {
        for row in tpar {
            if let (Some(k), Some(v)) = (col(row, &["name", "property"]), row.get("value").or(row.get("valeur"))) {
                push_param(&k, v);
            }
        }
    }

    for row in table(&["TN", "TNodes"]) {
        let status = numcol(row, &["statut"]).and_then(|x| NodeStatus::from_code(x as i32));
        net.nodes.push(Node {
            id: needtext(row, &["noeud"], "TN")?,
            elevation: need(row, &["haut"], "TN")?,
            x: numcol(row, &["xdata"]).unwrap_or(0.0),
            y: numcol(row, &["ydata"]).unwrap_or(0.0),
            taps: numcol(row, &["nbrob"]),
            orifice_ideal: numcol(row, &["orif_ideal"]).unwrap_or(0.0),
            orifice_commercial: numcol(row, &["orif_com"]).unwrap_or(0.0),
            status: status.ok_or_else(|| Error::Format("TN: statut must be 0, 1 or 2".into()))?,
        });
    }
    for row in table(&["TA", "TArcs"]) {
        let length1 = numcol(row, &["lon1"]).unwrap_or(0.0);
        let length2 = numcol(row, &["lon2"]).unwrap_or(0.0);
        net.arcs.push(Arc {
            id: format!("A{}", net.arcs.len() + 1),
            from: needtext(row, &["pred"], "TA")?,
            to: needtext(row, &["succ"], "TA")?,
            length: need(row, &["lon"], "TA")?,
            length1,
            pipe1: pipe_ref(col(row, &["ID1"]), length1),
            length2,
            pipe2: pipe_ref(col(row, &["ID2"]), length2),
            state: ArcState::from_code(&col(row, &["N"]).unwrap_or_default()),
            open: true,
        });
    }
    for row in table(&["TB"]) {
        net.terminals.push(Terminal {
            id: needtext(row, &["borne"], "TB")?,
            pressure: need(row, &["pressure"], "TB")?,
        });
    }
    for row in table(&["TC", "TClients"]) {
        net.clients.push(Client {
            id: needtext(row, &["client"], "TC")?,
            terminal: needtext(row, &["borne"], "TC")?,
            probability: need(row, &["proba"], "TC")?,
            target_flow: need(row, &["cible"], "TC")?,
        });
    }
    for row in table(&["TD", "TDn"]) {
        let pipe = Pipe {
            id: needtext(row, &["ID"], "TD")?,
            nominal: col(row, &["Nominal"]).unwrap_or_default(),
            sdr: col(row, &["SDR"]).unwrap_or_default(),
            diameter: need(row, &["diam"], "TD")?,
            cost: numcol(row, &["cost"]).unwrap_or(0.0),
            max_pressure: numcol(row, &["pressure", "Pmax"]).unwrap_or(0.0),
            kind: col(row, &["type", "Type"]).unwrap_or_default(),
            roughness: numcol(row, &["Roughness", "rugo"]).unwrap_or(0.0),
        };
        if pipe.id != "fictif" && pipe.nominal != "fictif" {
            net.pipes.push(pipe);
        }
    }
    for row in table(&["TO", "TOn"]) {
        let d = match row {
            Json::Object(m) => numcol(row, &["in_diam", "Diameter"]).or_else(|| m.first().and_then(|(_, v)| v.as_f64())),
            other => other.as_f64(),
        };
        if let Some(d) = d {
            net.orifices.push(d);
        }
    }
    for row in table(&["TFact"]) {
        let succ = col(row, &["succ", "node"]).unwrap_or_default();
        if let (Some(q), Some(m)) = (numcol(row, &["designFlow"]), numcol(row, &["margin"])) {
            net.loadflow.push((succ, q, m));
        } else if let Some(lf) = numcol(row, &["loadFactor", "loadfactor"]) {
            let mut r: Vec<String> = col(row, &["pred"]).into_iter().collect();
            r.push(succ);
            r.push(g15(lf));
            net.loadfactor_raw.push(r);
        }
    }
    for (header, key, cols) in RESULT_TABLES {
        let rows = table(&[key]);
        if !rows.is_empty() {
            let lines = rows
                .iter()
                .map(|r| cols.iter().map(|c| col(r, &[c]).unwrap_or_default()).collect::<Vec<_>>().join(","))
                .collect();
            net.extra_sections.push((header.to_string(), lines));
        }
    }
    if let Some(Json::Object(m)) = j.get("reinfcost")
        && !m.is_empty()
    {
        let lines = m.iter().map(|(k, v)| format!("{k},{}", v.text().unwrap_or_else(|| "NaN".into()))).collect();
        net.extra_sections.push(("!Costs".into(), lines));
    }
    if let Some(meta) = meta {
        let strings = |v: &Json| v.rows().iter().filter_map(|x| x.text()).collect::<Vec<_>>();
        if let Some(Json::Array(c)) = meta.get("comments") {
            net.comments = c.iter().filter_map(|x| match x {
                Json::String(t) => Some(t.clone()),
                _ => None,
            }).collect();
        }
        if let Some(Json::Array(rows)) = meta.get("loadfactor") {
            net.loadfactor_raw.extend(rows.iter().map(strings));
        }
        if let Some(Json::Array(sections)) = meta.get("extraSections") {
            for sec in sections {
                let header = col(sec, &["header"]).unwrap_or_default();
                let lines = match sec.get("lines") {
                    Some(Json::Array(l)) => l.iter().filter_map(|x| match x {
                        Json::String(t) => Some(t.clone()),
                        _ => None,
                    }).collect(),
                    _ => Vec::new(),
                };
                net.extra_sections.push((header, lines));
            }
        }
    }
    warnings.extend(complete_demand(&mut net));
    warnings.extend(net.apply_client_data());
    Ok(Read { network: net, warnings })
}
