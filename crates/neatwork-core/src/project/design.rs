//! `neatwork.project.Design` (a `.dsg` file) and `Database` (the diameter
//! and orifice catalogs).

use super::network::{Network, Node, Pipe, TYPE_FAUCET, sort_rows_by};
use crate::java::{JHashtable, JProperties, double_to_string, parse_double, parse_int, read_csv};
use crate::messages::Locale;
use crate::{Error, Result};

/// `neatwork.project.Diameter`: one commercial pipe.
#[derive(Clone, Debug, PartialEq)]
pub struct Diameter {
    pub nominal: String,
    pub sdr: f64,
    pub diameter: f64,
    pub cost: f64,
    pub max_length: f64,
    pub pipe_type: i32,
    pub roughness: f64,
}

impl Diameter {
    /// `Diameter.isSame`: same diameter, SDR and type.
    fn is_same(&self, o: &Diameter) -> bool {
        o.diameter == self.diameter && o.sdr == self.sdr && o.pipe_type == self.pipe_type
    }

    /// Row of the material selection, as the make-design dialog passes it
    /// to the solver: nominal, sdr, diameter, cost, max length, type,
    /// roughness.
    pub fn selection_line(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            self.nominal,
            double_to_string(self.sdr),
            double_to_string(self.diameter),
            double_to_string(self.cost),
            double_to_string(self.max_length),
            self.pipe_type,
            double_to_string(self.roughness)
        )
    }
}

/// A design: the network with pipe diameters, orifices and load factors.
#[derive(Clone, Debug)]
pub struct Design {
    pub name: String,
    pub network: Network,
    pub properties: JProperties,
    pub diam_table: JHashtable<String, Diameter>,
    pub load_factor: JHashtable<String, String>,
    pub orifices: Vec<String>,
}

impl Design {
    /// `new Design(name, content, defaultProperties)`.
    pub fn new(name: &str, content: &str, defaults: &JProperties, locale: Locale) -> Self {
        let mut d = Design {
            name: name.to_string(),
            network: Network::new(locale),
            properties: JProperties::new(),
            diam_table: JHashtable::new(),
            load_factor: JHashtable::new(),
            orifices: Vec::new(),
        };
        d.properties.put_all(defaults);
        d.set_content(content);
        d
    }

    /// `setContent`: parse the project text again. Messages go to
    /// `network.info`; on an error the properties are not applied.
    pub fn set_content(&mut self, content: &str) {
        let v = read_csv(content);
        let mut properties = JProperties::with_defaults(&self.properties);
        self.diam_table = JHashtable::new();
        self.load_factor = JHashtable::new();
        self.orifices = Vec::new();
        self.network = Network::new(self.network.locale);
        for t in &v {
            if t.len() == 9 {
                self.add_diameter_line(t);
            }
            if t.len() == 7 {
                self.add_node_line(t);
            }
            if t.len() == 2 {
                properties.set_property(&format!("topo.{}.value", t[0]), &t[1]);
            }
            if t.len() == 3 {
                self.load_factor.put(t[0].clone(), t[1].clone());
            }
            if t.len() == 6 {
                self.orifices.push(t[0].clone());
            }
        }
        for t in &v {
            if t.len() == 8 {
                self.add_pipe_line(t);
            }
        }
        if self.network.source().is_none() {
            self.network.add_new_source();
        }
        self.network.sort_nodes();
        if !self.network.has_error {
            self.properties.put_all(&properties);
        }
    }

    fn add_node_line(&mut self, v: &[String]) {
        let mut ok = true;
        let mut n = Node::named(&v[0]);
        let mut num = |s: &str, slot: &mut f64| match parse_double(s) {
            Some(x) => *slot = x,
            None => ok = false,
        };
        num(&v[1], &mut n.height);
        num(&v[2], &mut n.coord_x);
        num(&v[3], &mut n.coord_y);
        num(&v[4], &mut n.orifice);
        num(&v[5], &mut n.orifice_commercial);
        match parse_int(&v[6]) {
            Some(x) => {
                n.node_type = x;
                if !(0..=2).contains(&x) {
                    ok = false;
                }
            }
            None => ok = false,
        }
        if ok {
            self.network.add_node(n);
        }
    }

    fn add_pipe_line(&mut self, v: &[String]) {
        let mut p = Pipe {
            begin: v[0].clone(),
            end: v[1].clone(),
            ref_diam1: v[4].clone(),
            ref_diam2: v[6].clone(),
            ..Pipe::default()
        };
        let (a, b, c) = (
            parse_double(&v[2]),
            parse_double(&v[3]),
            parse_double(&v[5]),
        );
        if let (Some(l), Some(l1), Some(l2)) = (a, b, c) {
            p.length = l;
            p.length1 = l1;
            p.length2 = l2;
            self.network.add_pipe(p);
        }
    }

    fn add_diameter_line(&mut self, v: &[String]) {
        let fields = (
            parse_double(&v[2]),
            parse_double(&v[3]),
            parse_double(&v[4]),
            parse_double(&v[5]),
            parse_int(&v[6]),
            parse_double(&v[7]),
        );
        if let (Some(sdr), Some(diameter), Some(cost), Some(max_length), Some(t), Some(r)) = fields
        {
            self.diam_table.put(
                v[0].clone(),
                Diameter {
                    nominal: v[1].clone(),
                    sdr,
                    diameter,
                    cost,
                    max_length,
                    pipe_type: t,
                    roughness: r,
                },
            );
        }
    }

    /// `Project.setProperties`: add the own entries of `p`.
    pub fn set_properties(&mut self, p: &JProperties) {
        self.properties.put_all(p);
    }

    /// `setLoadFactorTable`: clear, then copy `table` (values are `Double`).
    pub fn set_load_factor_table(&mut self, table: &JHashtable<String, f64>) {
        self.load_factor.clear();
        for (k, v) in table.entries() {
            self.load_factor.put(k.clone(), double_to_string(*v));
        }
    }

    /// `refreshDesign`: replace each diameter by the equal catalog entry.
    pub fn refresh(&mut self, db: &Database) {
        let keys: Vec<String> = self.diam_table.keys().cloned().collect();
        for k in keys {
            let found = self
                .diam_table
                .get(&k)
                .and_then(|item| db.find_diameter(item))
                .cloned();
            if let Some(f) = found {
                self.diam_table.put(k, f);
            }
        }
    }

    /// `getContent`: the `.dsg` text.
    pub fn content(&self) -> String {
        let mut c = self.nodes_content();
        c.push_str(&self.pipes_content());
        c.push_str(&self.properties_content());
        c.push_str(&self.diameters_content());
        c.push_str(&self.orifices_content());
        c
    }

    /// `getNodesContent`.
    pub fn nodes_content(&self) -> String {
        let mut c = String::from("!Nodes\n");
        for n in &self.network.nodes {
            c.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                n.name,
                double_to_string(n.height),
                double_to_string(n.coord_x),
                double_to_string(n.coord_y),
                double_to_string(n.orifice),
                double_to_string(n.orifice_commercial),
                n.node_type
            ));
        }
        c
    }

    /// `getPipesContent`.
    pub fn pipes_content(&self) -> String {
        let mut c = String::from("!Pipes\n");
        for p in &self.network.pipes {
            c.push_str(&format!(
                "{},{},{},{},{},{},{},N\n",
                p.begin,
                p.end,
                double_to_string(p.length),
                double_to_string(p.length1),
                p.ref_diam1,
                double_to_string(p.length2),
                p.ref_diam2
            ));
        }
        c
    }

    /// `getPropertiesContent`: the `topo.*` parameters and the load factors.
    pub fn properties_content(&self) -> String {
        let mut c = String::from("!Default properties\n");
        for name in self.properties.property_names() {
            if name.starts_with("topo.") && name.ends_with(".value") {
                let value = self.properties.get_property(&name).unwrap_or("");
                c.push_str(&format!("{},{value}\n", &name[5..name.len() - 6]));
            }
        }
        c.push_str("!loadfactor\n");
        for (k, v) in self.load_factor.entries() {
            c.push_str(&format!("{k},{v},0\n"));
        }
        c
    }

    /// `getDiametersContent`.
    pub fn diameters_content(&self) -> String {
        let mut c = String::from("!Diameters\n");
        for (k, d) in self.diam_table.entries() {
            c.push_str(&format!(
                "{k},{},{},{},{},{},{},{},N\n",
                d.nominal,
                double_to_string(d.sdr),
                double_to_string(d.diameter),
                double_to_string(d.cost),
                double_to_string(d.max_length),
                d.pipe_type,
                double_to_string(d.roughness)
            ));
        }
        c
    }

    /// `getOrificesContent`.
    pub fn orifices_content(&self) -> String {
        let mut c = String::from("!Orifices\n");
        for o in &self.orifices {
            c.push_str(&format!("{o},N,N,N,N,N\n"));
        }
        c
    }

    fn diameter(&self, reference: &str) -> Result<&Diameter> {
        self.diam_table
            .get(&reference.to_string())
            .ok_or_else(|| Error::Network(format!("unknown diameter reference {reference}")))
    }

    /// `getCoreDesign`: rows `node, pred, height, length, #taps, l1, l2,
    /// d1, d2, ideal orifice, commercial orifice`; source first, then the
    /// branching nodes and the taps, each group sorted by name.
    pub fn core_design(&mut self) -> Result<Vec<Vec<String>>> {
        let source = self
            .network
            .source()
            .ok_or_else(|| Error::Network("no source".to_string()))?
            .clone();
        let hsource = source.height;
        self.network.sort_nodes();
        let mut data: Vec<Vec<String>> = Vec::new();
        for pipe in &self.network.pipes {
            let node = self
                .network
                .node(&pipe.end)
                .ok_or_else(|| Error::Network(format!("unknown node {}", pipe.end)))?;
            let d1 = self.diameter(&pipe.ref_diam1)?;
            let d2 = if pipe.length2 > 0.0 {
                double_to_string(self.diameter(&pipe.ref_diam2)?.diameter)
            } else {
                "0".to_string()
            };
            data.push(vec![
                pipe.end.clone(),
                pipe.begin.clone(),
                double_to_string(node.height - hsource),
                double_to_string(pipe.length),
                if node.node_type == TYPE_FAUCET {
                    "1"
                } else {
                    "0"
                }
                .to_string(),
                double_to_string(pipe.length1),
                double_to_string(pipe.length2),
                double_to_string(d1.diameter),
                d2,
                double_to_string(node.orifice),
                double_to_string(node.orifice_commercial),
            ]);
        }
        sort_rows_by(&mut data, 0);
        sort_rows_by(&mut data, 4);
        let mut first = vec![source.name.clone(), source.name.clone()];
        first.extend(std::iter::repeat_n("0".to_string(), 9));
        data.insert(0, first);
        Ok(data)
    }

    /// `getCoreOrifice`: distinct commercial orifices, sorted as strings.
    pub fn core_orifices(&self) -> Vec<Vec<String>> {
        let mut set: Vec<String> = self
            .network
            .nodes
            .iter()
            .map(|n| double_to_string(n.orifice_commercial))
            .collect();
        set.sort_by(|a, b| crate::java::compare_utf16(a, b));
        set.dedup();
        set.into_iter().map(|s| vec![s]).collect()
    }

    /// `getCoreDiameter`: the diameter table in `elements()` order.
    pub fn core_diameters(&self) -> Vec<Vec<String>> {
        self.diam_table
            .values()
            .map(|d| {
                vec![
                    d.nominal.clone(),
                    double_to_string(d.sdr),
                    double_to_string(d.diameter),
                    double_to_string(d.cost),
                    double_to_string(d.max_length),
                    d.pipe_type.to_string(),
                    double_to_string(d.roughness),
                ]
            })
            .collect()
    }
}

/// `neatwork.project.Database`: diameter and orifice catalogs.
#[derive(Clone, Debug)]
pub struct Database {
    diameters: JHashtable<String, Diameter>,
    orifices: Vec<f64>,
}

/// Catalogs shipped with NeatWork 2024.
pub const DEFAULT_DIAMETERS: &str = include_str!("../../resources/diameters.db");
pub const DEFAULT_ORIFICES: &str = include_str!("../../resources/orifices.db");

impl Default for Database {
    fn default() -> Self {
        Database::new(DEFAULT_DIAMETERS, DEFAULT_ORIFICES)
    }
}

impl Database {
    /// Build from catalog texts. An empty text means the shipped catalog,
    /// as in the Java program.
    pub fn new(diameters: &str, orifices: &str) -> Self {
        let diameters = if diameters.is_empty() {
            DEFAULT_DIAMETERS
        } else {
            diameters
        };
        let orifices = if orifices.is_empty() {
            DEFAULT_ORIFICES
        } else {
            orifices
        };
        let mut db = Database {
            diameters: JHashtable::new(),
            orifices: Vec::new(),
        };
        for t in read_csv(orifices) {
            if t.len() == 1
                && let Some(d) = parse_double(&t[0])
                && !db.orifices.contains(&d)
            {
                db.orifices.push(d);
            }
        }
        db.orifices
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for t in read_csv(diameters) {
            if t.len() == 7 {
                db.add_diameter(&t);
            }
        }
        db
    }

    fn add_diameter(&mut self, v: &[String]) {
        let fields = (
            parse_double(&v[1]),
            parse_double(&v[2]),
            parse_double(&v[3]),
            parse_double(&v[4]),
            parse_int(&v[5]),
            parse_double(&v[6]),
        );
        if let (Some(sdr), Some(diameter), Some(cost), Some(max_length), Some(t), Some(r)) = fields
            && (1..=3).contains(&t)
        {
            let mut cpt = 1;
            while self.diameters.contains_key(&format!("D{cpt}")) {
                cpt += 1;
            }
            self.diameters.put(
                format!("D{cpt}"),
                Diameter {
                    nominal: v[0].clone(),
                    sdr,
                    diameter,
                    cost,
                    max_length,
                    pipe_type: t,
                    roughness: r,
                },
            );
        }
    }

    /// `getDiameters`: catalog sorted by diameter (stable).
    pub fn diameters(&self) -> Vec<Diameter> {
        let mut v: Vec<Diameter> = self.diameters.values().cloned().collect();
        v.sort_by(|a, b| {
            a.diameter
                .partial_cmp(&b.diameter)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    }

    /// Sorted orifice diameters.
    pub fn orifices(&self) -> &[f64] {
        &self.orifices
    }

    fn find_diameter(&self, d: &Diameter) -> Option<&Diameter> {
        let sorted = self.diameters();
        let found = sorted.iter().find(|item| item.is_same(d))?;
        self.diameters.values().find(|x| *x == found)
    }

    /// Material selection with every catalog row, as the solver receives it.
    pub fn diameters_selection(&self) -> String {
        self.diameters()
            .iter()
            .map(|d| d.selection_line() + "\n")
            .collect()
    }

    /// Orifice selection with every catalog row.
    pub fn orifices_selection(&self) -> String {
        self.orifices
            .iter()
            .map(|o| double_to_string(*o) + "\n")
            .collect()
    }
}
