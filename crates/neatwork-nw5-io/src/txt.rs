//! Cruscade Txt design (`M:InOut/LireTablesDsgTxt.m`, `specs/formats.md`
//! section 9): five text files `<prefix>DsgArcs.txt`, `DsgClients`,
//! `DsgNodes`, `DsgDbase`, `DsgParam`, each with a header row. Columns are
//! matched by name.
//!
//! Choices of the port:
//! - the client target unit is an explicit option (the Cruscade files hold
//!   m3/s, the Toy files L/s); the model stores L/s;
//! - arcs given by `diam1/diam2` are mapped to the cheapest catalogue pipe
//!   of that diameter (as `M:Simul/completeTables.m` costs them);
//! - text fields are kept raw, quotes included, as in `.dsg` files;
//! - there is no `!Borne`: bornes are built at the `pressure` parameter;
//! - a non-uniform demand sets `bounded_flow = 1`, as the MATLAB reader.

use crate::util::{complete_demand, pipe_ref};
use neatwork_nw5::read::{Read, num};
use neatwork_nw5::{Arc, ArcState, Client, Error, Network, Node, NodeStatus, Pipe, Result};
use std::path::Path;

/// Unit of the client target flows (`cible`) of a `DsgClients` file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlowUnit {
    /// L/s (Toy Txt files, every other NW5 table).
    #[default]
    LitresPerSecond,
    /// m3/s (Cruscade Txt files).
    CubicMetresPerSecond,
}

/// Contents of the five files of a Txt design.
#[derive(Clone, Copy, Debug)]
pub struct TxtFiles<'a> {
    /// `DsgArcs`.
    pub arcs: &'a str,
    /// `DsgClients`.
    pub clients: &'a str,
    /// `DsgNodes`.
    pub nodes: &'a str,
    /// `DsgDbase`.
    pub dbase: &'a str,
    /// `DsgParam`.
    pub param: &'a str,
}

/// A table read by column name.
struct Table {
    file: String,
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    /// First line = header. Delimiter: tab if the header has at least as
    /// many tabs as commas, or neither, else comma.
    fn parse(file: &str, text: &str) -> Table {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r')).filter(|l| !l.trim().is_empty());
        let first = lines.next().unwrap_or("");
        let (tabs, commas) = (first.matches('\t').count(), first.matches(',').count());
        let delim = if tabs >= commas { '\t' } else { ',' };
        let split = |l: &str| l.split(delim).map(|f| f.trim().to_string()).collect::<Vec<_>>();
        Table {
            file: file.to_string(),
            header: split(first),
            rows: lines.map(split).collect(),
        }
    }

    fn col(&self, name: &str) -> Option<usize> {
        self.header
            .iter()
            .position(|h| h == name)
            .or_else(|| self.header.iter().position(|h| h.eq_ignore_ascii_case(name)))
    }

    fn require(&self, base: &str, names: &[&str]) -> Result<()> {
        let missing: Vec<&str> = names.iter().copied().filter(|n| self.col(n).is_none()).collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(Error::Format(format!(
                "{base}: colonnes requises manquantes dans {}: {}",
                self.file,
                missing.join(", ")
            )))
        }
    }

    fn text<'r>(&self, row: &'r [String], name: &str) -> Option<&'r str> {
        self.col(name).and_then(|i| row.get(i)).map(String::as_str)
    }

    fn num(&self, row: &[String], name: &str) -> Result<f64> {
        let t = self.text(row, name).unwrap_or("");
        num(t).ok_or_else(|| Error::Format(format!("{}: column {name}: not a number `{t}`", self.file)))
    }
}

/// Map the Txt status codes to 0/1/2: at least three distinct codes; if they
/// are not exactly {0, 1, 2}, the minimum is a junction, the maximum the
/// source and the values in between terminals.
fn statuses(codes: &[f64]) -> Result<Vec<NodeStatus>> {
    let mut distinct: Vec<f64> = codes.to_vec();
    distinct.sort_by(f64::total_cmp);
    distinct.dedup();
    if distinct.len() < 3 {
        return Err(Error::Format(format!(
            "DsgNodes.statut doit contenir au moins 3 catégories : source, nœuds intermédiaires, nœuds terminaux. Trouvé: {distinct:?}"
        )));
    }
    let standard = distinct == [0.0, 1.0, 2.0];
    let (min, max) = (distinct[0], distinct[distinct.len() - 1]);
    codes
        .iter()
        .map(|&c| {
            if standard {
                NodeStatus::from_code(c as i32).ok_or_else(|| Error::Format("DsgNodes.statut".into()))
            } else if c == min {
                Ok(NodeStatus::Junction)
            } else if c == max {
                Ok(NodeStatus::Source)
            } else {
                Ok(NodeStatus::Terminal)
            }
        })
        .collect()
}

/// Read a Txt design from the contents of its five files.
pub fn read_txt_design(name: &str, files: &TxtFiles<'_>, client_flow: FlowUnit) -> Result<Read> {
    let file = |base: &str| format!("{name}{base}.txt");
    let arcs = Table::parse(&file("DsgArcs"), files.arcs);
    let by_diam = arcs.col("diam1").is_some() && arcs.col("diam2").is_some();
    let by_id = arcs.col("ID1").is_some() && arcs.col("ID2").is_some();
    if !(by_diam || by_id) {
        return Err(Error::Format(format!("DsgArcs: il faut (diam1,diam2) ou (ID1,ID2) dans {}.", arcs.file)));
    }
    arcs.require("DsgArcs", &["pred", "succ", "lon", "lon1", "lon2"])?;
    let clients = Table::parse(&file("DsgClients"), files.clients);
    clients.require("DsgClients", &["borne", "client", "proba", "cible"])?;
    let nodes = Table::parse(&file("DsgNodes"), files.nodes);
    nodes.require("DsgNodes", &["noeud", "haut", "xdata", "ydata", "orif_ideal", "orif_com", "statut"])?;
    let dbase = Table::parse(&file("DsgDbase"), files.dbase);
    dbase.require("DsgDbase", &["ID", "diam", "cost"])?;
    let param = Table::parse(&file("DsgParam"), files.param);
    param.require("DsgParam", &["property", "value"])?;

    let mut net = Network {
        name: name.to_string(),
        ..Network::default()
    };
    for r in &param.rows {
        if let (Some(k), Some(v)) = (param.text(r, "property"), param.text(r, "value"))
            && !k.is_empty()
        {
            net.params.entries.push((k.to_string(), v.to_string()));
        }
    }
    for r in &dbase.rows {
        let id = dbase.text(r, "ID").unwrap_or("").to_string();
        let text = |names: &[&str]| names.iter().find_map(|n| dbase.text(r, n)).unwrap_or("").to_string();
        let number = |names: &[&str]| names.iter().find_map(|n| dbase.text(r, n).and_then(num)).unwrap_or(0.0);
        let nominal = text(&["Nominal"]);
        net.pipes.push(Pipe {
            nominal: if nominal.is_empty() { id.clone() } else { nominal },
            sdr: text(&["SDR", "materiau"]),
            diameter: dbase.num(r, "diam")?,
            cost: dbase.num(r, "cost")?,
            max_pressure: number(&["Pmax", "pressure"]),
            kind: text(&["Type", "type"]),
            roughness: number(&["Roughness", "rugo"]),
            id,
        });
    }
    let codes = nodes.rows.iter().map(|r| nodes.num(r, "statut")).collect::<Result<Vec<_>>>()?;
    for (r, status) in nodes.rows.iter().zip(statuses(&codes)?) {
        net.nodes.push(Node {
            id: nodes.text(r, "noeud").unwrap_or("").to_string(),
            elevation: nodes.num(r, "haut")?,
            x: nodes.num(r, "xdata")?,
            y: nodes.num(r, "ydata")?,
            taps: None,
            orifice_ideal: nodes.num(r, "orif_ideal")?,
            orifice_commercial: nodes.num(r, "orif_com")?,
            status,
        });
    }
    // cheapest catalogue pipe of a diameter
    let pipe_of = |d: f64, from: &str, to: &str| -> Result<Option<String>> {
        if d == 0.0 {
            return Ok(None);
        }
        net.pipes
            .iter()
            .filter(|p| (p.diameter - d).abs() <= 1e-12 * d.abs().max(1.0))
            .min_by(|a, b| a.cost.total_cmp(&b.cost))
            .map(|p| Some(p.id.clone()))
            .ok_or_else(|| Error::Format(format!("DsgArcs: diameter {d} of arc {from} -> {to} is not in DsgDbase")))
    };
    let mut arc_rows = Vec::new();
    for r in &arcs.rows {
        let from = arcs.text(r, "pred").unwrap_or("").to_string();
        let to = arcs.text(r, "succ").unwrap_or("").to_string();
        let (length1, length2) = (arcs.num(r, "lon1")?, arcs.num(r, "lon2")?);
        let (pipe1, pipe2) = if by_id {
            (
                pipe_ref(arcs.text(r, "ID1").map(str::to_string), length1),
                pipe_ref(arcs.text(r, "ID2").map(str::to_string), length2),
            )
        } else {
            let d1 = if length1 > 0.0 { arcs.num(r, "diam1")? } else { 0.0 };
            let d2 = if length2 > 0.0 { arcs.num(r, "diam2")? } else { 0.0 };
            (pipe_of(d1, &from, &to)?, pipe_of(d2, &from, &to)?)
        };
        arc_rows.push(Arc {
            id: format!("A{}", arc_rows.len() + 1),
            length: arcs.num(r, "lon")?,
            length1,
            pipe1,
            length2,
            pipe2,
            state: ArcState::from_code(arcs.text(r, "N").unwrap_or("N")),
            open: true,
            from,
            to,
        });
    }
    net.arcs = arc_rows;
    let factor = match client_flow {
        FlowUnit::LitresPerSecond => 1.0,
        FlowUnit::CubicMetresPerSecond => 1000.0,
    };
    for r in &clients.rows {
        net.clients.push(Client {
            id: clients.text(r, "client").unwrap_or("").to_string(),
            terminal: clients.text(r, "borne").unwrap_or("").to_string(),
            probability: clients.num(r, "proba")?,
            target_flow: clients.num(r, "cible")? * factor,
        });
    }
    let mut warnings = complete_demand(&mut net);
    warnings.extend(net.apply_client_data());
    if net.params.get("uniform_dem") == 0.0 && net.params.get("bounded_flow") != 1.0 {
        warnings.push("non-uniform Txt demand: bounded_flow set to 1".into());
        net.params.set("bounded_flow", "1");
    }
    Ok(Read { network: net, warnings })
}

/// Read the Txt design `<dir>/<prefix>Dsg{Arcs,Clients,Nodes,Dbase,Param}.txt`.
pub fn read_txt_folder(dir: &Path, prefix: &str, client_flow: FlowUnit) -> Result<Read> {
    let load = |base: &str| {
        let p = dir.join(format!("{prefix}{base}.txt"));
        std::fs::read(&p)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .map_err(|e| Error::Io(format!("Fichier introuvable : {}: {e}", p.display())))
    };
    let (arcs, clients, nodes, dbase, param) =
        (load("DsgArcs")?, load("DsgClients")?, load("DsgNodes")?, load("DsgDbase")?, load("DsgParam")?);
    let files = TxtFiles {
        arcs: &arcs,
        clients: &clients,
        nodes: &nodes,
        dbase: &dbase,
        param: &param,
    };
    read_txt_design(prefix, &files, client_flow)
}
