//! Application domain JSON (`specs/formats.md` sections 7.3 to 7.5): the
//! snapshot `domainJson` of the python-core parsers, the JSON import
//! `{graph, domain}` and the typed simulation domain with its field
//! aliases.
//!
//! Compatibility choices of the writer:
//! - identifiers stay strings (the application turns `150` into a number,
//!   section 1.3);
//! - node and pipe rows always have the DSG columns, plus `nbrob` for a
//!   topography; pipe rows add `external_edge_id` and `enabled` (valve
//!   state) from the typed domain;
//! - clients are `{client, borne, behaviorId}` with `clientBehaviors`, as
//!   after `normalize_client_behaviors`;
//! - non-finite parameters are the strings `Inf`, `-Inf`, `NaN` (the
//!   application writes `inf`, which is not JSON);
//! - extra keys `tables.loadflow` (design flows and margins) and
//!   `preservedSections` (`!loadfactor` and unknown sections) keep the rest
//!   of a `.dsg`. The application ignores unknown keys.

use crate::fmt::plain_decimal;
use crate::json::{Json, n, obj, s};
use crate::sha1::sha1_hex;
use crate::text::canonical_params;
use crate::util::{complete_demand, lossless, param_text, pipe_ref};
use neatwork_nw5::read::{Read, canonical_section, fields, g15, num};
use neatwork_nw5::{Arc, ArcState, Client, Error, Network, Node, NodeStatus, Params, Pipe, Result, Terminal};

/// Stable client behaviour identifier of the application:
/// `CB_` + the first 10 hexadecimal digits (upper case) of the SHA-1 of
/// `<p>|<t>|<profile>`, numbers as plain shortest decimals
/// (`APP:neatwork/domain/client_behavior.py:12`).
pub fn behavior_id(probability: f64, target_flow_lps: f64, profile: Option<&str>) -> String {
    let canonical = format!(
        "{}|{}|{}",
        plain_decimal(probability),
        plain_decimal(target_flow_lps),
        profile.unwrap_or("")
    );
    format!("CB_{}", sha1_hex(canonical.as_bytes())[..10].to_uppercase())
}

/// Pipe status values that mean "closed" (`APP:schemas/simulation_domain.py:185`).
const CLOSED_STATES: &[&str] = &["CLOSED", "DISABLED", "INACTIVE", "OFF", "OUT_OF_SERVICE"];

fn param_value(text: &str) -> Json {
    match num(text) {
        Some(x) if x.is_finite() => Json::Number(x),
        Some(x) => s(&g15(x)),
        None => s(text),
    }
}

/// Write a network as an application domain snapshot (pretty-printed).
/// `sourceFormat` is `DSG` for a design, else `TPO`.
pub fn write_domain_json(net: &Network) -> String {
    let design = net.is_design();
    let nodes = net
        .nodes
        .iter()
        .map(|x| {
            let mut row = vec![
                ("noeud", s(&x.id)),
                ("haut", n(x.elevation)),
                ("xdata", n(x.x)),
                ("ydata", n(x.y)),
                ("orif_ideal", n(x.orifice_ideal)),
                ("orif_com", n(x.orifice_commercial)),
                ("statut", Json::Number(f64::from(x.status.code()))),
            ];
            if !design && let Some(t) = x.taps {
                row.push(("nbrob", n(t)));
            }
            obj(row)
        })
        .collect::<Vec<_>>();
    let id = |p: &Option<String>| p.as_deref().map_or(Json::Null, s);
    let pipes = net
        .arcs
        .iter()
        .map(|a| {
            obj(vec![
                ("external_edge_id", s(&a.id)),
                ("pred", s(&a.from)),
                ("succ", s(&a.to)),
                ("lon", n(a.length)),
                ("lon1", n(a.length1)),
                ("ID1", id(&a.pipe1)),
                ("lon2", n(a.length2)),
                ("ID2", id(&a.pipe2)),
                ("N", s(a.state.code())),
                ("enabled", Json::Bool(a.open)),
            ])
        })
        .collect::<Vec<_>>();
    let mut behaviors: Vec<(String, f64, f64)> = Vec::new();
    let clients = net
        .clients
        .iter()
        .map(|c| {
            let b = behavior_id(c.probability, c.target_flow, None);
            if !behaviors.iter().any(|(id, _, _)| *id == b) {
                behaviors.push((b.clone(), c.probability, c.target_flow));
            }
            obj(vec![("client", s(&c.id)), ("borne", s(&c.terminal)), ("behaviorId", s(&b))])
        })
        .collect();
    let behaviors = behaviors
        .iter()
        .enumerate()
        .map(|(k, (id, p, t))| {
            obj(vec![
                ("behaviorId", s(id)),
                ("name", s(&format!("Imported behavior {}", k + 1))),
                ("openProbability", n(*p)),
                ("targetFlowLps", n(*t)),
                ("profileId", Json::Null),
            ])
        })
        .collect();
    let bornes = net
        .terminals
        .iter()
        .map(|t| obj(vec![("borne", s(&t.id)), ("pressure", n(t.pressure)), ("N", s("N"))]))
        .collect();
    let diameters = net
        .pipes
        .iter()
        .map(|p| {
            obj(vec![
                ("ID", s(&p.id)),
                ("Nominal", s(&p.nominal)),
                ("SDR", lossless(&p.sdr)),
                ("diam", n(p.diameter)),
                ("cost", n(p.cost)),
                ("Pmax", n(p.max_pressure)),
                ("Type", lossless(&p.kind)),
                ("Roughness", n(p.roughness)),
                ("N", s("N")),
            ])
        })
        .collect();
    let orifices = net
        .orifices
        .iter()
        .enumerate()
        .map(|(k, d)| {
            obj(vec![
                ("source_index", Json::Number((k + 1) as f64)),
                ("diameter_m", n(*d)),
                ("diameter_mm", n(num(&g15(d * 1000.0)).unwrap_or(f64::NAN))),
            ])
        })
        .collect();
    let loadflow = net
        .loadflow
        .iter()
        .map(|(succ, q, m)| obj(vec![("succ", s(succ)), ("designFlow", n(*q)), ("margin", n(*m))]))
        .collect();
    let mut preserved = Vec::new();
    if !net.loadfactor_raw.is_empty() {
        let lines = net.loadfactor_raw.iter().map(|r| s(&r.join(","))).collect();
        preserved.push(obj(vec![("header", s("!loadfactor")), ("lines", Json::Array(lines))]));
    }
    for (header, lines) in &net.extra_sections {
        preserved.push(obj(vec![
            ("header", s(header)),
            ("lines", Json::Array(lines.iter().map(|l| s(l)).collect())),
        ]));
    }
    let params = canonical_params(&net.params)
        .into_iter()
        .map(|(k, v)| (k, param_value(&v)))
        .collect();
    obj(vec![
        ("name", s(&net.name)),
        ("sourceFormat", s(if design { "DSG" } else { "TPO" })),
        ("comments", Json::Array(net.comments.iter().map(|c| s(c)).collect())),
        ("params", Json::Object(params)),
        ("technicalParams", Json::Object(Vec::new())),
        ("designParams", Json::Object(Vec::new())),
        (
            "tables",
            obj(vec![
                ("nodes", Json::Array(nodes)),
                ("pipes", Json::Array(pipes.clone())),
                ("edges", Json::Array(pipes)),
                ("clients", Json::Array(clients)),
                ("clientBehaviors", Json::Array(behaviors)),
                ("bornes", Json::Array(bornes)),
                ("diameters", Json::Array(diameters)),
                ("orifices", Json::Array(orifices)),
                ("loadflow", Json::Array(loadflow)),
            ]),
        ),
        ("preservedSections", Json::Array(preserved)),
        ("warnings", Json::Array(Vec::new())),
    ])
    .pretty()
}

fn text(row: &Json, keys: &[&str]) -> Option<String> {
    row.first(keys).and_then(Json::text).filter(|t| !t.is_empty())
}

fn number(row: &Json, keys: &[&str]) -> Option<f64> {
    row.first(keys).and_then(Json::as_f64)
}

fn required<T>(v: Option<T>, entity: &str, id: &str, field: &str) -> Result<T> {
    v.ok_or_else(|| Error::Format(format!("{entity} '{id}': missing or invalid {field}")))
}

fn read_nodes(rows: &[&Json], graph: bool, net: &mut Network) -> Result<()> {
    for row in rows {
        let id = required(text(row, &["node_id", "nodeId", "noeud", "id"]), "Node", "", "identifier")?;
        let status = match number(row, &["status", "statut"]) {
            Some(code) => NodeStatus::from_code(code as i32),
            None if graph => Some(match row.get("type").and_then(Json::text).as_deref() {
                Some("RESERVOIR") => NodeStatus::Source,
                Some("FAUCET") => NodeStatus::Terminal,
                _ => NodeStatus::Junction,
            }),
            None => None,
        };
        let elevation = number(row, &["elevation_m", "elevationM", "haut"]).or(graph.then_some(0.0));
        net.nodes.push(Node {
            elevation: required(elevation, "Node", &id, "elevation")?,
            x: number(row, &["x", "xdata", "sourceX"]).unwrap_or(0.0),
            y: number(row, &["y", "ydata", "sourceY"]).unwrap_or(0.0),
            taps: number(row, &["nbrob"]),
            orifice_ideal: number(row, &["ideal_orifice_m", "idealOrificeM", "orif_ideal"]).unwrap_or(0.0),
            orifice_commercial: number(row, &["commercial_orifice_m", "commercialOrificeM", "orif_com"]).unwrap_or(0.0),
            status: required(status, "Node", &id, "status (0, 1 or 2)")?,
            id,
        });
    }
    Ok(())
}

fn read_pipes(rows: &[&Json], net: &mut Network) -> Result<()> {
    for row in rows {
        let id = text(row, &["edge_id", "edgeId", "external_edge_id", "externalEdgeId", "id"])
            .unwrap_or_else(|| format!("A{}", net.arcs.len() + 1));
        let from = required(text(row, &["pred", "predecessorId", "predecessor_id", "sourceId"]), "Pipe", &id, "predecessor")?;
        let to = required(text(row, &["succ", "successorId", "successor_id", "targetId"]), "Pipe", &id, "successor")?;
        let id1 = text(row, &["first_diameter_id", "firstDiameterId", "ID1"]);
        let id2 = text(row, &["second_diameter_id", "secondDiameterId", "ID2"]);
        let length = number(row, &["length_m", "lengthM", "length", "lon"]);
        let length2 = number(row, &["second_length_m", "secondLengthM", "lon2"]).unwrap_or(0.0);
        let has_pipe1 = pipe_ref(id1.clone(), 1.0).is_some_and(|p| !p.is_empty());
        let length1 = number(row, &["first_length_m", "firstLengthM", "lon1"])
            .unwrap_or(if has_pipe1 { length.unwrap_or(0.0) } else { 0.0 });
        let closed = row
            .first(&["operational_status", "operationalStatus", "status", "edgeStatus"])
            .and_then(Json::text)
            .is_some_and(|t| CLOSED_STATES.contains(&t.to_uppercase().as_str()));
        let enabled = match row.first(&["enabled", "is_open", "isOpen"]) {
            Some(Json::Bool(b)) => *b,
            Some(v) => v.as_f64().is_none_or(|x| x != 0.0),
            None => true,
        };
        net.arcs.push(Arc {
            from,
            to,
            length: length.unwrap_or(length1 + length2),
            length1,
            pipe1: pipe_ref(id1, length1),
            length2,
            pipe2: pipe_ref(id2, length2),
            state: ArcState::from_code(&text(row, &["metadata_n", "metadataN", "N"]).unwrap_or_default()),
            open: enabled && !closed,
            id,
        });
    }
    Ok(())
}

fn read_clients(rows: &[&Json], behaviors: &[&Json], net: &mut Network) -> Result<()> {
    for row in rows {
        let id = text(row, &["client", "clientId", "client_id"]).unwrap_or_default();
        let terminal = required(text(row, &["borne", "terminalId", "terminal_id"]), "Client", &id, "terminal")?;
        let (p, t) = match text(row, &["behavior_id", "behaviorId"]) {
            Some(b) => {
                let behavior = behaviors
                    .iter()
                    .find(|x| text(x, &["behaviorId", "behavior_id"]).as_deref() == Some(b.as_str()))
                    .ok_or_else(|| Error::Format(format!("Unknown client behavior '{b}'.")))?;
                (
                    number(behavior, &["openProbability", "open_probability"]),
                    number(behavior, &["targetFlowLps", "target_flow_lps"]),
                )
            }
            None => (
                number(row, &["proba", "openProbability", "open_probability"]),
                number(row, &["cible", "targetFlowLps", "target_flow_lps"]),
            ),
        };
        let (Some(probability), Some(target_flow)) = (p, t) else {
            return Err(Error::Format(format!(
                "Client '{id}' has neither behaviorId nor complete legacy demand values."
            )));
        };
        net.clients.push(Client {
            id,
            terminal,
            probability,
            target_flow,
        });
    }
    Ok(())
}

fn read_catalogues(tables: &Json, net: &mut Network) {
    let rows = |k: &str| tables.get(k).map(Json::rows).unwrap_or_default();
    let mut without_id = Vec::new();
    for row in rows("diameters") {
        let Some(diameter) = number(row, &["diam", "innerDiameter", "inner_diameter"]) else {
            continue;
        };
        let pipe = Pipe {
            id: text(row, &["ID", "id", "diameterId", "diameter_id"]).unwrap_or_default(),
            nominal: text(row, &["Nominal", "size", "nominalSize", "nominal_size"]).unwrap_or_default(),
            sdr: text(row, &["SDR", "sdr"]).unwrap_or_default(),
            diameter,
            cost: number(row, &["cost"]).unwrap_or(0.0),
            max_pressure: number(row, &["Pmax", "pressure", "maximumPressure", "maximum_pressure"]).unwrap_or(0.0),
            kind: text(row, &["type", "Type", "typeCode", "type_code"]).unwrap_or_default(),
            roughness: number(row, &["Roughness", "rugo", "rugosité", "roughness"]).unwrap_or(0.0),
        };
        if pipe.id == "fictif" || pipe.nominal == "fictif" {
            continue;
        }
        if pipe.id.is_empty() {
            without_id.push(pipe);
        } else {
            net.pipes.push(pipe);
        }
    }
    without_id.sort_by(|a, b| a.diameter.total_cmp(&b.diameter));
    for (k, mut p) in without_id.into_iter().enumerate() {
        p.id = format!("D{:03}", k + 1);
        net.pipes.push(p);
    }
    for row in rows("orifices") {
        let d = match row {
            Json::Object(_) => number(row, &["diameter_m", "in_diam", "diameterM"]),
            other => other.as_f64(),
        };
        if let Some(d) = d {
            net.orifices.push(d);
        }
    }
}

/// Read an application domain JSON: a snapshot `{name, sourceFormat,
/// params, tables, ...}`, an import document `{graph, domain}`, or a bare
/// visual graph `{nodes, edges}`. Field aliases of section 7.5 are
/// accepted. Parameters are `technicalParams < designParams < params`.
pub fn read_domain_json(text_in: &str) -> Result<Read> {
    let root = Json::parse(text_in)?;
    let domain = root.get("domain").unwrap_or(&root);
    let graph = domain.get("tables").is_none() && domain.get("nodes").is_some();
    let empty = Json::Object(Vec::new());
    let tables = if graph { domain } else { domain.get("tables").unwrap_or(&empty) };
    let rows = |k: &[&str]| tables.first(k).map(Json::rows).unwrap_or_default();
    let mut net = Network {
        name: text(domain, &["name"]).or_else(|| text(&root, &["name"])).unwrap_or_default(),
        ..Network::default()
    };
    for key in ["technicalParams", "technical_params", "designParams", "design_params", "params"] {
        if let Some(Json::Object(m)) = domain.get(key) {
            for (k, v) in m {
                if let Some(value) = param_text(v) {
                    net.params.set(Params::canonical(k), &value);
                }
            }
        }
    }
    if let Some(Json::Array(c)) = domain.get("comments") {
        net.comments = c
            .iter()
            .filter_map(|x| match x {
                Json::String(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
    }
    read_nodes(&rows(&["nodes"]), graph, &mut net)?;
    read_pipes(&rows(if graph { &["edges"] } else { &["pipes", "edges"] }), &mut net)?;
    let behaviors = rows(&["clientBehaviors", "client_behaviors"]);
    read_clients(&rows(&["clients"]), &behaviors, &mut net)?;
    for row in rows(&["bornes"]) {
        if let (Some(id), Some(pressure)) = (
            text(row, &["borne", "terminalId", "terminal_id"]),
            number(row, &["pressure", "requiredPressureM", "required_pressure_m"]),
        ) {
            net.terminals.push(Terminal { id, pressure });
        }
    }
    read_catalogues(tables, &mut net);
    for row in rows(&["loadflow"]) {
        if let (Some(succ), Some(q), Some(m)) =
            (text(row, &["succ"]), number(row, &["designFlow"]), number(row, &["margin"]))
        {
            net.loadflow.push((succ, q, m));
        }
    }
    if let Some(Json::Array(sections)) = domain.get("preservedSections") {
        for sec in sections {
            let header = sec.get("header").and_then(Json::text).unwrap_or_default();
            let lines: Vec<String> = match sec.get("lines") {
                Some(Json::Array(l)) => l
                    .iter()
                    .filter_map(|x| match x {
                        Json::String(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            if canonical_section(&header) == "loadfactor" {
                net.loadfactor_raw = lines.iter().map(|l| fields(l)).collect();
            } else {
                net.extra_sections.push((header, lines));
            }
        }
    }
    let mut warnings: Vec<String> = match domain.get("warnings") {
        Some(Json::Array(w)) => w.iter().filter_map(Json::text).collect(),
        _ => Vec::new(),
    };
    warnings.extend(complete_demand(&mut net));
    warnings.extend(net.apply_client_data());
    Ok(Read { network: net, warnings })
}
