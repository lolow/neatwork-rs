//! `neatwork.project.Topographie`: a `.tpo` file.

use super::network::{
    Network, Node, Pipe, TYPE_DISPATCH, TYPE_FAUCET, TYPE_RESERVOIR, topo_node_csv, topo_pipe_csv,
};
use crate::engine::loi_normale;
use crate::java::{
    JHashtable, JProperties, decimal_format, double_to_string, parse_double, parse_int, read_csv,
};
use crate::messages::Locale;
use crate::{Error, Result};

/// A topography: the network before design, plus its `topo.*` parameters.
#[derive(Clone, Debug)]
pub struct Topographie {
    pub name: String,
    pub network: Network,
    pub properties: JProperties,
    /// Expanded copy (`nodeList2`, `pipeList2`): a node with several taps
    /// becomes a dispatch node plus one faucet per tap.
    pub expanded_nodes: Vec<Node>,
    pub expanded_pipes: Vec<Pipe>,
}

impl Topographie {
    /// `new Topographie(name, content, defaultProperties)`.
    pub fn new(name: &str, content: &str, defaults: &JProperties, locale: Locale) -> Self {
        let mut t = Topographie {
            name: name.to_string(),
            network: Network::new(locale),
            properties: JProperties::new(),
            expanded_nodes: Vec::new(),
            expanded_pipes: Vec::new(),
        };
        t.properties.put_all(defaults);
        t.set_content(content);
        t
    }

    /// `setContent`: parse the project text again. Messages go to
    /// `network.info`; on an error the properties are not applied.
    pub fn set_content(&mut self, content: &str) {
        let v = read_csv(content);
        let mut properties = JProperties::with_defaults(&self.properties);
        self.network = Network::new(self.network.locale);
        for t in &v {
            if t.len() == 6 {
                self.add_node_line(t);
            }
            if t.len() == 2 {
                properties.set_property(&format!("topo.{}.value", t[0]), &t[1]);
            }
        }
        for t in &v {
            if t.len() == 3 {
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
        match parse_double(&v[1]) {
            Some(x) => n.height = x,
            None => ok = false,
        }
        match parse_double(&v[2]) {
            Some(x) => n.coord_x = x,
            None => ok = false,
        }
        match parse_double(&v[3]) {
            Some(x) => n.coord_y = x,
            None => ok = false,
        }
        match parse_int(&v[4]) {
            Some(x) => n.nb_taps = x,
            None => ok = false,
        }
        match parse_int(&v[5]) {
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
            ..Pipe::default()
        };
        if let Some(x) = parse_double(&v[2]) {
            p.length = x;
            self.network.add_pipe(p);
        }
    }

    /// `getPropertiesContent`.
    pub fn properties_content(&self) -> String {
        let mut content = String::from("!Default properties\n");
        for name in self.properties.property_names() {
            if name.starts_with("topo.") && name.ends_with(".value") {
                let short = &name[5..name.len() - 6];
                let value = self.properties.get_property(&name).unwrap_or("");
                content.push_str(&format!("{short},{value}\n"));
            }
        }
        content
    }

    /// `getContent`: the `.tpo` text.
    pub fn content(&self) -> String {
        let mut c = String::from("!Nodes\n");
        for n in &self.network.nodes {
            c.push_str(&topo_node_csv(n));
            c.push('\n');
        }
        c.push_str("!Pipes\n");
        for p in &self.network.pipes {
            c.push_str(&topo_pipe_csv(p));
            c.push('\n');
        }
        c.push_str(&self.properties_content());
        c
    }

    /// `makeExpandedTopo`.
    pub fn make_expanded_topo(&mut self) {
        self.expanded_nodes.clear();
        self.expanded_pipes.clear();
        for item in &mut self.network.nodes {
            if item.node_type == TYPE_FAUCET && item.nb_taps < 1 {
                item.nb_taps = 1;
            }
        }
        for p in &self.network.pipes {
            self.expanded_pipes.push(Pipe {
                begin: p.begin.clone(),
                end: p.end.clone(),
                length: p.length,
                ..Pipe::default()
            });
        }
        for item in &self.network.nodes {
            if item.node_type == TYPE_FAUCET && item.nb_taps > 1 {
                let mut old = item.clone();
                old.node_type = TYPE_DISPATCH;
                old.nb_taps = 0;
                self.expanded_nodes.push(old);
                for k in 0..item.nb_taps {
                    let letter = 'a' as u32 + k as u32;
                    let mut n = item.clone();
                    let c = char::from_u32(letter).unwrap_or('?');
                    n.name = format!("{}_{c}", item.name);
                    n.nb_taps = 1;
                    self.expanded_pipes.push(Pipe {
                        begin: item.name.clone(),
                        end: n.name.clone(),
                        length: 1.0,
                        ..Pipe::default()
                    });
                    self.expanded_nodes.push(n);
                }
            } else {
                self.expanded_nodes.push(item.clone());
            }
        }
        self.expanded_nodes.sort_by_key(|n| n.node_type);
    }

    fn expanded_node(&self, name: &str) -> Option<&Node> {
        self.expanded_nodes.iter().find(|n| n.name == name)
    }

    /// `getPredPipe(node, pipeList2)`: first pipe ending at the node.
    pub fn expanded_pred_pipe(&self, name: &str) -> Option<&Pipe> {
        self.expanded_pipes.iter().find(|p| p.end == name)
    }

    /// `getLoadTaps`: for each expanded node, the number of taps downstream.
    pub fn load_taps(&self) -> Result<JHashtable<String, String>> {
        let mut table: JHashtable<String, String> = JHashtable::new();
        for n in &self.expanded_nodes {
            table.put(n.name.clone(), "0".to_string());
        }
        for node in &self.expanded_nodes {
            if node.node_type == TYPE_FAUCET {
                let mut pion = node.clone();
                while pion.node_type != TYPE_RESERVOIR {
                    let pred = self
                        .expanded_pred_pipe(&pion.name)
                        .ok_or_else(|| {
                            Error::Network(format!("node {} has no incoming pipe", pion.name))
                        })?
                        .begin
                        .clone();
                    let passage = table
                        .get(&pred)
                        .and_then(|s| parse_int(s))
                        .ok_or_else(|| Error::Network(format!("unknown node {pred}")))?;
                    table.put(pred.clone(), (passage + 1).to_string());
                    pion = self
                        .expanded_node(&pred)
                        .ok_or_else(|| Error::Network(format!("unknown node {pred}")))?
                        .clone();
                }
                table.put(node.name.clone(), "1".to_string());
            }
        }
        Ok(table)
    }

    fn property_f64(&self, key: &str) -> Result<f64> {
        self.properties
            .get_property(key)
            .and_then(parse_double)
            .ok_or_else(|| Error::Parameter(key.to_string()))
    }

    /// `getLoadFactor`: load factor of each expanded node, rounded with
    /// `0.##` and stored as a `Double`.
    pub fn load_factor(
        &self,
        load_taps: &JHashtable<String, String>,
    ) -> Result<JHashtable<String, f64>> {
        let targetflow = self.property_f64("topo.targetflow.value")?;
        let opentaps = self.property_f64("topo.opentaps.value")?;
        let qualite = self.property_f64("topo.servicequal.value")?;
        let nfaucets = self
            .expanded_nodes
            .iter()
            .filter(|n| n.node_type == TYPE_FAUCET)
            .count();
        let proba = calcul_proba(targetflow, nfaucets, opentaps, qualite);
        let mut table = JHashtable::new();
        for item in load_taps.keys() {
            let nb = load_taps
                .get(item)
                .and_then(|s| parse_int(s))
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| Error::Network(format!("bad tap count for {item}")))?;
            let p = proba
                .get(nb)
                .copied()
                .ok_or_else(|| Error::Network(format!("bad tap count for {item}")))?;
            let rounded = decimal_format("0.##", p / targetflow);
            let value = parse_double(&rounded).unwrap_or(0.0);
            table.put(item.clone(), value);
        }
        Ok(table)
    }

    /// `getCoreVector`: rows `node, pred, height, length, #taps` with the
    /// source first, from the expanded network.
    pub fn core_vector(&mut self) -> Result<Vec<Vec<String>>> {
        self.expanded_nodes.sort_by_key(|n| n.node_type);
        let source = self
            .network
            .source()
            .ok_or_else(|| Error::Network("no source".to_string()))?;
        let mut data = vec![vec![
            source.name.clone(),
            source.name.clone(),
            double_to_string(source.height),
            "0".to_string(),
            "0".to_string(),
        ]];
        for node in &self.expanded_nodes {
            if node.node_type != TYPE_RESERVOIR {
                let pred = self.expanded_pred_pipe(&node.name).ok_or_else(|| {
                    Error::Network(format!("node {} has no incoming pipe", node.name))
                })?;
                data.push(vec![
                    node.name.clone(),
                    pred.begin.clone(),
                    double_to_string(node.height),
                    double_to_string(pred.length),
                    node.nb_taps.to_string(),
                ]);
            }
        }
        Ok(data)
    }
}

/// `Topographie.calculProba`.
pub(crate) fn calcul_proba(outflow: f64, n: usize, p: f64, qualite: f64) -> Vec<f64> {
    let mut proba = vec![0.0; n + 1];
    if n >= 1 {
        proba[1] = outflow;
    }
    for (i, slot) in proba.iter_mut().enumerate().skip(2) {
        let v = (outflow * calcul_proba2(i as i32, p, qualite)).min(outflow * i as f64);
        *slot = if v < outflow { outflow } else { v };
    }
    proba
}

fn calcul_proba2(n: i32, pfrac: f64, proba: f64) -> f64 {
    let nf = f64::from(n);
    let proba1 = proba * (1.0 - (1.0 - pfrac).powf(nf)) + (1.0 - pfrac).powf(nf);
    let t = loi_normale::param(proba1);
    t * (nf * pfrac * (1.0 - pfrac)).powf(0.5) + nf * pfrac
}
