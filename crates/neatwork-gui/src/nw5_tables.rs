//! Read-only domain tables of a NW5 revision (`rust/nw5/specs/app.md`
//! F27–F30): the tables of the application domain JSON (`nw5-io`), with
//! the NW5 column labels, units, cell formats and node colors.

use neatwork_nw5::Network;
use neatwork_nw5_project::DisplayUnit;
use neatwork_nw5_project::format::{
    MISSING, adaptive, fixed, normalize_column_key, table_value_to_display,
};
use neatwork_nw5_project::json::{Json, scalar_text};
use neatwork_nw5_project::review::NodeRole;

/// A cell: text and the role that colors it (F28).
#[derive(Clone, Debug, PartialEq)]
pub struct DomainCell {
    pub text: String,
    pub role: Option<NodeRole>,
}

/// A table of the Tables view.
#[derive(Clone, Debug, PartialEq)]
pub struct DomainTable {
    pub title: String,
    pub description: String,
    /// Header badges (F27): row count and units.
    pub badges: String,
    /// The Source / Junction / Terminal legend is shown.
    pub legend: bool,
    /// Column labels, "#" first.
    pub columns: Vec<String>,
    pub rows: Vec<Vec<DomainCell>>,
}

/// Known tables, in display order: title, description, aliases (F27).
const GROUPS: &[(&str, &str, &[&str])] = &[
    (
        "Network nodes",
        "Network topology nodes with terminal pressure and NeatWork metadata.",
        &["nodes", "node_list", "nodelist"],
    ),
    (
        "Network pipes",
        "Pipe links imported from the network topology file.",
        &[
            "pipes",
            "pipe_list",
            "pipelist",
            "arcs",
            "arc_list",
            "arclist",
            "edges",
            "edge_list",
            "edgelist",
        ],
    ),
    (
        "Pipe diameter catalog",
        "Available pipe diameters imported from the DSG file or diameter database.",
        &["diameters", "diameter_list", "diameterlist"],
    ),
    (
        "Orifice diameter catalog",
        "Available orifice diameters imported from the DSG file or orifice database.",
        &["orifices", "orifice_list", "orificelist"],
    ),
    (
        "Client behaviors",
        "Reusable demand behaviour definitions shared by client instances.",
        &["clientbehaviors", "client_behaviors", "behaviors"],
    ),
    (
        "Client demand table",
        "Client instances attached to terminal outlets and linked to reusable demand behaviors.",
        &["clients", "client_list", "clientlist", "generated_clients"],
    ),
    (
        "Terminal outlets",
        "Faucet or terminal outlet nodes used as consumption points.",
        &["bornes", "taps", "terminal_outlets", "outlets"],
    ),
];

/// Column labels of F27.
const LABELS: &[(&str, &str)] = &[
    ("noeud", "Node ID"),
    ("haut", "Elevation"),
    ("xdata", "X coordinate"),
    ("ydata", "Y coordinate"),
    ("orif_ideal", "Ideal orifice"),
    ("orif_com", "Commercial orifice"),
    ("statut", "Status"),
    ("pred", "From"),
    ("succ", "To"),
    ("lon", "Total pipe length"),
    ("lon1", "Segment 1 length"),
    ("ID1", "Segment 1 diameter ID"),
    ("lon2", "Segment 2 length"),
    ("ID2", "Segment 2 diameter ID"),
    ("N", "NeatWork flag"),
    ("Nominal", "Nominal size"),
    ("diam", "Inner diameter"),
    ("cost", "Cost"),
    ("Pmax", "Maximum pressure"),
    ("Type", "Pipe type code"),
    ("client", "Client"),
    ("borne", "Outlet"),
    ("proba", "Open probability"),
    ("cible", "Target flow"),
    ("behaviorId", "Behavior ID"),
    ("name", "Name"),
    ("openProbability", "Open probability"),
    ("targetFlowLps", "Target flow"),
    ("profileId", "Demand profile"),
];

/// Identifier columns shown as is (F29 rule 9).
const IDENTIFIERS: &[&str] = &[
    "noeud",
    "node",
    "id",
    "pred",
    "succ",
    "id1",
    "id2",
    "client",
    "borne",
    "outlet",
    "statut",
    "status",
    "source_index",
    "n",
    "nominal",
    "external_edge_id",
];

fn normalized_table_key(key: &str) -> String {
    normalize_column_key(key)
}

/// "Network nodes" for an unknown key: underscores → spaces, words
/// capitalized.
fn prettify(key: &str) -> String {
    key.split(['_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Role of a node status code (F28).
pub fn role_of_status(text: &str) -> Option<NodeRole> {
    match text.trim().to_ascii_uppercase().as_str() {
        "0" | "SOURCE" => Some(NodeRole::Source),
        "1" | "JUNCTION" | "TRANSIT" => Some(NodeRole::Junction),
        "2" | "TERMINAL" => Some(NodeRole::Terminal),
        _ => None,
    }
}

fn label(key: &str, unit: DisplayUnit) -> String {
    let base = LABELS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or_else(|| prettify(key), |(_, l)| (*l).to_string());
    let nk = normalize_column_key(key);
    let suffix = if nk == "orif_ideal" || nk == "orif_com" || nk == "diameter_m" {
        format!(" ({})", unit.orifice_symbol())
    } else if table_value_to_display(key, 1.0, unit).is_some() {
        format!(" ({})", unit.symbol())
    } else if nk == "cible" || nk.contains("target_flow") || nk == "targetflowlps" {
        " (L/s)".to_string()
    } else if nk.contains("proba") || nk == "openprobability" {
        " (ratio)".to_string()
    } else {
        String::new()
    };
    base + &suffix
}

fn value_text(v: &Json) -> Option<String> {
    match v {
        Json::Null => None,
        Json::Bool(b) => Some(if *b { "Yes" } else { "No" }.to_string()),
        other => scalar_text(other).filter(|t| !t.is_empty()),
    }
}

fn number(v: &Json) -> Option<f64> {
    match v {
        Json::Number(x) => Some(*x),
        Json::String(t) => t.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Generic cell format (F29 rules 1, 9, 10, 11).
fn generic_cell(key: &str, v: &Json, unit: DisplayUnit) -> String {
    let Some(text) = value_text(v) else {
        return "-".to_string();
    };
    let nk = normalize_column_key(key);
    if IDENTIFIERS.contains(&nk.as_str()) || matches!(v, Json::Bool(_)) {
        return text;
    }
    match number(v) {
        Some(x) => adaptive(table_value_to_display(key, x, unit).unwrap_or(x)),
        None => text,
    }
}

fn rows_of<'a>(tables: &'a Json, aliases: &[&str]) -> Option<(String, &'a [Json])> {
    let Json::Object(members) = tables else {
        return None;
    };
    let mut best: Option<(String, &[Json])> = None;
    for (k, v) in members {
        if !aliases.contains(&normalized_table_key(k).as_str()) {
            continue;
        }
        if let Json::Array(rows) = v {
            let better = match &best {
                None => true,
                Some((_, b)) => b.is_empty() && !rows.is_empty(),
            };
            if better {
                best = Some((k.clone(), rows.as_slice()));
            }
        }
    }
    best
}

/// Union of the row keys in first-seen order.
fn columns_of(rows: &[Json]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in rows {
        if let Json::Object(m) = r {
            for (k, _) in m {
                if !out.contains(k) {
                    out.push(k.clone());
                }
            }
        }
    }
    out
}

fn get<'a>(row: &'a Json, key: &str) -> &'a Json {
    row.get(key).unwrap_or(&Json::Null)
}

/// Inner diameter of a catalog pipe in the display unit (F29 rule 5).
fn diameter_text(net: &Network, id: &str, unit: DisplayUnit) -> String {
    match net.pipe(id) {
        Some(p) => {
            let digits = if unit == DisplayUnit::Meter { 3 } else { 2 };
            format!(
                "{id} ({} {})",
                fixed(unit.from_meters(p.diameter), digits),
                unit.symbol()
            )
        }
        None => id.to_string(),
    }
}

fn cell(text: String) -> DomainCell {
    DomainCell { text, role: None }
}

/// The tables of a network in the display unit (F27–F29).
pub fn domain_tables(net: &Network, unit: DisplayUnit) -> Vec<DomainTable> {
    let Ok(domain) = Json::parse(&neatwork_nw5_io::write_domain_json(net)) else {
        return Vec::new();
    };
    let Some(tables) = domain.get("tables") else {
        return Vec::new();
    };
    let roles: Vec<(String, NodeRole)> = net
        .nodes
        .iter()
        .filter_map(|n| Some((n.id.clone(), role_of_status(&n.status.code().to_string())?)))
        .collect();
    let role_of = |id: &str| roles.iter().find(|(k, _)| k == id).map(|(_, r)| *r);
    let bornes = rows_of(tables, GROUPS[6].2).map_or(&[][..], |(_, r)| r);
    let behaviors = rows_of(tables, GROUPS[4].2).map_or(&[][..], |(_, r)| r);
    let mut used: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for (k, (title, description, aliases)) in GROUPS.iter().enumerate() {
        let Some((_, rows)) = rows_of(tables, aliases) else {
            continue;
        };
        for (name, _) in match tables {
            Json::Object(m) => m.as_slice(),
            _ => &[],
        } {
            if aliases.contains(&normalized_table_key(name).as_str()) {
                used.push(name.clone());
            }
        }
        // Terminal outlets merge into the nodes; empty client tables hide.
        if k == 6 || (k == 5 && rows.is_empty()) {
            continue;
        }
        let mut cols = columns_of(rows);
        if k == 0 {
            for extra in ["pressure", "N"] {
                cols.retain(|c| c != extra);
                if !bornes.is_empty() {
                    cols.push(extra.to_string());
                }
            }
            cols.retain(|c| c != "statut");
        }
        if k == 3 {
            cols.retain(|c| c != "diameter_mm");
        }
        let mut columns = vec!["#".to_string()];
        columns.extend(cols.iter().map(|c| label(c, unit)));
        let rows: Vec<Vec<DomainCell>> = rows
            .iter()
            .enumerate()
            .map(|(r, row)| {
                let node_id = value_text(get(row, "noeud")).unwrap_or_default();
                let borne = bornes
                    .iter()
                    .find(|b| value_text(get(b, "borne")).as_deref() == Some(node_id.as_str()));
                let mut cells = vec![cell((r + 1).to_string())];
                for c in &cols {
                    let v = if k == 0 && (c == "pressure" || c == "N") {
                        borne.map_or(&Json::Null, |b| get(b, c))
                    } else {
                        get(row, c)
                    };
                    cells.push(format_cell(k, c, v, row, net, unit, &role_of, behaviors));
                }
                cells
            })
            .collect();
        let badges = match k {
            0 => format!(
                "{} rows · Length: {} · Orifice: {}",
                rows.len(),
                unit.symbol(),
                unit.orifice_symbol()
            ),
            3 => format!("{} rows · Unit: {}", rows.len(), unit.orifice_symbol()),
            _ => format!("{} rows · Unit: {}", rows.len(), unit.symbol()),
        };
        out.push(DomainTable {
            title: (*title).to_string(),
            description: (*description).to_string(),
            badges,
            legend: k == 0,
            columns,
            rows,
        });
    }
    // unknown tables, then the parameters
    if let Json::Object(m) = tables {
        for (name, v) in m {
            if used.contains(name) {
                continue;
            }
            let Json::Array(rows) = v else { continue };
            let cols = columns_of(rows);
            let mut columns = vec!["#".to_string()];
            columns.extend(cols.iter().map(|c| label(c, unit)));
            out.push(DomainTable {
                title: prettify(name),
                description: format!("Imported table: {name}"),
                badges: format!("{} rows · Unit: {}", rows.len(), unit.symbol()),
                legend: false,
                columns,
                rows: rows
                    .iter()
                    .enumerate()
                    .map(|(r, row)| {
                        let mut cells = vec![cell((r + 1).to_string())];
                        let id = value_text(get(row, "succ"));
                        for c in &cols {
                            let mut dc = cell(generic_cell(c, get(row, c), unit));
                            if c == "succ" {
                                dc.role = id.as_deref().and_then(&role_of);
                            }
                            cells.push(dc);
                        }
                        cells
                    })
                    .collect(),
            });
        }
    }
    // PROPOSAL: NW5 shows the parameters only in the editor (F31); the
    // port adds them as a read-only table "Default properties".
    if let Some(Json::Object(params)) = domain.get("params") {
        out.push(DomainTable {
            title: "Default properties".to_string(),
            description: "Parameters of the input file (!Default properties).".to_string(),
            badges: format!("{} rows", params.len()),
            legend: false,
            columns: vec!["#".into(), "Parameter".into(), "Value".into()],
            rows: params
                .iter()
                .enumerate()
                .map(|(r, (k, v))| {
                    vec![
                        cell((r + 1).to_string()),
                        cell(k.clone()),
                        cell(value_text(v).unwrap_or_else(|| "-".into())),
                    ]
                })
                .collect(),
        });
    }
    out
}

/// Cell of a known table (F29 rules 2–8).
#[allow(clippy::too_many_arguments)]
fn format_cell(
    table: usize,
    key: &str,
    v: &Json,
    row: &Json,
    net: &Network,
    unit: DisplayUnit,
    role_of: &dyn Fn(&str) -> Option<NodeRole>,
    behaviors: &[Json],
) -> DomainCell {
    let text = value_text(v);
    let nk = normalize_column_key(key);
    let mut out = cell(match (table, nk.as_str()) {
        (_, "n") => match text.as_deref() {
            None | Some("N") => MISSING.to_string(),
            Some(t) => t.to_string(),
        },
        (0, "orif_ideal" | "orif_com") => {
            let terminal = role_of(&value_text(get(row, "noeud")).unwrap_or_default())
                == Some(NodeRole::Terminal);
            match number(v) {
                Some(x) if terminal => fixed(unit.orifice_from_meters(x), 2),
                _ => MISSING.to_string(),
            }
        }
        (0, "pressure") if text.is_none() => MISSING.to_string(),
        (1, "id1") => text.map_or("-".into(), |id| diameter_text(net, &id, unit)),
        (1, "lon2") if number(v).is_some_and(|x| x.abs() < 1e-12) => MISSING.to_string(),
        (1, "id2") => {
            if number(get(row, "lon2")).is_some_and(|x| x.abs() < 1e-12) {
                MISSING.to_string()
            } else {
                text.map_or("-".into(), |id| diameter_text(net, &id, unit))
            }
        }
        (3, "diameter_m") => match number(v) {
            Some(x) => match unit {
                DisplayUnit::Meter => fixed(x * 1000.0, 2),
                DisplayUnit::Feet => fixed(unit.from_meters(x), 4),
                DisplayUnit::Inches => fixed(unit.from_meters(x), 3),
            },
            None => "-".into(),
        },
        (5, "behaviorid") => text
            .as_deref()
            .and_then(|id| {
                behaviors
                    .iter()
                    .find(|b| value_text(get(b, "behaviorId")).as_deref() == Some(id))
                    .and_then(|b| value_text(get(b, "name")))
            })
            .or(text)
            .unwrap_or_else(|| "-".into()),
        _ => generic_cell(key, v, unit),
    });
    out.role = match (table, nk.as_str()) {
        (0, "noeud") | (1, "pred" | "succ") | (5 | 6, "borne") => {
            value_text(v).as_deref().and_then(role_of)
        }
        _ => None,
    };
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toy() -> Network {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../reference/neatwork5/examples/dsg_examples/Toy/Toy_Secu1.00_fusion0_split0.dsg",
        );
        neatwork_nw5::read::read_file(&path)
            .map(|r| r.network)
            .unwrap_or_default()
    }

    #[test]
    fn toy_tables() {
        let net = toy();
        assert!(!net.nodes.is_empty());
        let t = domain_tables(&net, DisplayUnit::Meter);
        let titles: Vec<&str> = t.iter().map(|x| x.title.as_str()).collect();
        assert_eq!(
            &titles[..6],
            [
                "Network nodes",
                "Network pipes",
                "Pipe diameter catalog",
                "Orifice diameter catalog",
                "Client behaviors",
                "Client demand table"
            ]
        );
        assert!(titles.contains(&"Default properties"));
        // nodes: status hidden, terminal pressure merged, colored IDs
        let nodes = &t[0];
        assert!(nodes.columns.contains(&"Elevation (m)".to_string()));
        assert!(nodes.columns.contains(&"Ideal orifice (mm)".to_string()));
        assert!(!nodes.columns.iter().any(|c| c.starts_with("Status")));
        assert_eq!(
            nodes.columns.last().map(String::as_str),
            Some("NeatWork flag")
        );
        assert_eq!(nodes.rows[0][1].role, Some(NodeRole::Source));
        assert!(nodes.badges.ends_with("Length: m · Orifice: mm"));
        // a source has no orifice
        let oi = nodes
            .columns
            .iter()
            .position(|c| c == "Ideal orifice (mm)")
            .unwrap_or(0);
        assert_eq!(nodes.rows[0][oi].text, MISSING);
        // pipes: diameter with the inner diameter, lon2 = 0 → "—"
        let pipes = &t[1];
        let id1 = pipes
            .columns
            .iter()
            .position(|c| c == "Segment 1 diameter ID")
            .unwrap_or(0);
        assert!(
            pipes.rows[0][id1].text.contains(" m)"),
            "{:?}",
            pipes.rows[0]
        );
        // feet
        let t = domain_tables(&net, DisplayUnit::Feet);
        assert!(t[0].columns.contains(&"Elevation (ft)".to_string()));
    }

    #[test]
    fn labels_and_roles() {
        assert_eq!(prettify("loadflow"), "Loadflow");
        assert_eq!(prettify("design_flow"), "Design Flow");
        assert_eq!(role_of_status("2"), Some(NodeRole::Terminal));
        assert_eq!(role_of_status("TRANSIT"), Some(NodeRole::Junction));
        assert_eq!(label("cible", DisplayUnit::Meter), "Target flow (L/s)");
        assert_eq!(
            label("proba", DisplayUnit::Meter),
            "Open probability (ratio)"
        );
        assert_eq!(label("lon", DisplayUnit::Inches), "Total pipe length (in)");
    }
}
