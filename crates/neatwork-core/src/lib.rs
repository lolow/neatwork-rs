//! NeatWork engine in Rust.
//!
//! NeatWork designs gravity-driven water distribution networks for rural
//! areas. This crate is a port of the engine of NeatWork 2024 (Java, APLV
//! and Logilab). It reads and writes the same project files:
//!
//! - `.tpo` topography: nodes, pipes and design parameters;
//! - `.dsg` design: the topography with pipe diameters and orifices;
//! - `.sim` simulation: a design with flow and pressure statistics.
//!
//! The two operations are [`design`] (topography to design, with a linear
//! program solved by HiGHS) and [`simulate`] (design to simulation).
//!
//! Output files are byte-identical to the Java program's on the same input
//! (see `rust/DECISIONS.md` for the known exceptions). To reach that, the
//! crate copies some Java runtime behavior, in [`java`].

pub mod engine;
pub mod java;
pub mod messages;
pub mod project;

use std::collections::HashMap;

pub use engine::lp::{Lp, LpSolution};
pub use engine::simulation::{SimulationType, SplitMix64, Uniform};
pub use messages::Locale;
pub use project::{Database, Design, Topographie};

use engine::defs::{core_design, core_topographie, diameters_vector, orifices_vector};
use engine::make_design::MakeDesign;
use engine::simulation::MakeSimulation;
use java::{JHashtable, JProperties, parse_double, project_file_text, read_csv};
use project::network::TYPE_FAUCET;

/// Errors of the engine.
#[derive(Debug)]
pub enum Error {
    /// The network data is inconsistent (the Java program throws).
    Network(String),
    /// A parameter is missing or not a number.
    Parameter(String),
    /// The LP solver failed or its solution is unusable.
    Solver(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Network(m) => write!(f, "network error: {m}"),
            Error::Parameter(m) => write!(f, "parameter error: {m}"),
            Error::Solver(m) => write!(f, "solver error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// `default.properties` of NeatWork 2024.
pub const DEFAULT_PROPERTIES: &str = include_str!("../resources/default.properties");

/// The application properties that `Loader.main` builds for the standalone
/// program. Every project starts from them.
pub fn application_properties(locale: Locale) -> JProperties {
    let mut p = JProperties::new();
    p.load(DEFAULT_PROPERTIES);
    p.set_property("appli.standalone", "true");
    p.set_property("solver.distant", "false");
    p.set_property("file.distant", "false");
    p.set_property("file.path", ".");
    let mut po = JProperties::new();
    let code = match locale {
        Locale::En => "en",
        Locale::Fr => "fr",
        Locale::Sp => "sp",
    };
    po.set_property("appli.locale", code);
    p.put_all(&po);
    p
}

/// Project name of a file: its name without the last extension.
pub fn project_name(file_name: &str) -> String {
    match file_name.rfind('.') {
        Some(k) if k > 0 => file_name[..k].to_string(),
        _ => file_name.to_string(),
    }
}

/// Fixed or bounded diameter of one pipe (the make-design constraints).
#[derive(Clone, Debug)]
pub enum Constraint {
    /// Use only diameters above this value.
    MinDiameter {
        begin: String,
        end: String,
        diameter: f64,
    },
    /// Use only diameters below this value.
    MaxDiameter {
        begin: String,
        end: String,
        diameter: f64,
    },
    /// The whole pipe has this diameter.
    Fixed {
        begin: String,
        end: String,
        diameter: f64,
    },
    /// `length1` of `diameter1`, the rest of `diameter2`.
    Split {
        begin: String,
        end: String,
        diameter1: f64,
        length1: f64,
        diameter2: f64,
    },
}

/// Inputs of [`design`] besides the topography.
#[derive(Clone, Debug, Default)]
pub struct DesignOptions {
    pub locale: Locale,
    /// Catalogs to choose from; the default is the shipped catalogs, all
    /// rows selected.
    pub database: Option<Database>,
    pub constraints: Vec<Constraint>,
}

/// Result of [`design`].
#[derive(Clone, Debug)]
pub struct DesignOutput {
    /// Text of the `.dsg` file.
    pub dsg: String,
    /// The LP that was solved.
    pub lp: Lp,
    /// Rows `node, height, pressure, suction` (the GUI's pressure dialog).
    pub pressures: Vec<Vec<String>>,
    /// Messages from loading the topography.
    pub info: Vec<String>,
}

/// Make a design from a `.tpo` text, as the NeatWork GUI does with the
/// default dialog choices (every catalog row selected, computed load
/// factors). `solve` returns the LP columns; use [`solve_with_highs`].
pub fn design(
    name: &str,
    tpo: &str,
    opts: &DesignOptions,
    solve: &mut dyn FnMut(&Lp) -> Result<Vec<f64>>,
) -> Result<DesignOutput> {
    let app = application_properties(opts.locale);
    let mut topo = Topographie::new(name, tpo, &app, opts.locale);
    let info = topo.network.info.clone();

    // MakeDesignDialog, then FrameNeatwork restores the heights.
    let hsource = topo.network.set_source_height(0.0);
    topo.make_expanded_topo();
    topo.network.set_source_height(hsource);
    let load_taps = topo.load_taps()?;
    let load_factors = topo.load_factor(&load_taps)?;
    let modif = JHashtable::from_hashtable(&load_factors);

    let db = opts.database.clone().unwrap_or_default();
    let mut net = core_topographie(&topo.core_vector()?)?;
    apply_constraints(&mut net, &opts.constraints);
    let ovector = orifices_vector(&read_csv(&db.orifices_selection()))?;
    let temp = parse_double(
        topo.properties
            .get_property_or("topo.watertemp.value", "70"),
    )
    .ok_or_else(|| Error::Parameter("topo.watertemp.value".into()))?;
    let dvector = diameters_vector(&read_csv(&db.diameters_selection()), temp)?;
    let md = MakeDesign::run(net, dvector, ovector, &topo.properties, &modif, solve)?;

    let content = design_content(&md, &topo, hsource)?;
    let mut design = Design::new(&topo.name, &content, &app, opts.locale);
    design.set_properties(&topo.properties);
    design.set_load_factor_table(&modif);
    Ok(DesignOutput {
        dsg: project_file_text(&design.content()),
        lp: md.lp.clone(),
        pressures: md.pressure_data(hsource),
        info,
    })
}

/// Solve the design LP with HiGHS. A non-optimal status gives all-zero
/// columns, as with MOSEK in the Java program.
pub fn solve_with_highs(lp: &Lp) -> Result<Vec<f64>> {
    let s = lp.solve()?;
    if !s.optimal {
        eprintln!("warning: LP status {}: no feasible design", s.status);
    }
    Ok(s.x)
}

fn apply_constraints(net: &mut engine::defs::CoreNetwork, constraints: &[Constraint]) {
    for c in constraints {
        let (b, e) = match c {
            Constraint::MinDiameter { begin, end, .. }
            | Constraint::MaxDiameter { begin, end, .. }
            | Constraint::Fixed { begin, end, .. }
            | Constraint::Split { begin, end, .. } => (begin, end),
        };
        let Some(p) = net
            .pipes
            .0
            .iter_mut()
            .find(|p| &p.nodes_beg == b && &p.nodes_end == e)
        else {
            continue;
        };
        match c {
            Constraint::MinDiameter { diameter, .. } => p.imposdiammin = *diameter,
            Constraint::MaxDiameter { diameter, .. } => p.imposdiammax = *diameter,
            Constraint::Fixed { diameter, .. } => {
                p.imposdiam1 = *diameter;
                p.imposlength1 = p.length;
            }
            Constraint::Split {
                diameter1,
                length1,
                diameter2,
                ..
            } => {
                p.imposdiam1 = *diameter1;
                p.imposlength1 = *length1;
                p.imposdiam2 = *diameter2;
            }
        }
    }
}

/// `SolverDisk.getDesignContentMakeDesign`.
fn design_content(md: &MakeDesign, topo: &Topographie, hsource: f64) -> Result<String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in md.design_data(hsource) {
        match line.len() {
            11 => {
                let n = topo
                    .expanded_nodes
                    .iter()
                    .find(|n| n.name == line[0])
                    .ok_or_else(|| Error::Network(format!("unknown node {}", line[0])))?;
                let x = java::double_to_string(n.coord_x);
                let y = java::double_to_string(n.coord_y);
                if n.node_type != project::network::TYPE_RESERVOIR {
                    rows.push(vec![
                        line[0].clone(),
                        line[2].clone(),
                        x,
                        y,
                        line[9].clone(),
                        line[10].clone(),
                        n.node_type.to_string(),
                    ]);
                    rows.push(vec![
                        line[1].clone(),
                        line[0].clone(),
                        line[3].clone(),
                        line[5].clone(),
                        line[7].clone(),
                        line[6].clone(),
                        line[8].clone(),
                        "N".to_string(),
                    ]);
                } else {
                    rows.push(vec![
                        line[0].clone(),
                        line[2].clone(),
                        x,
                        y,
                        "0".to_string(),
                        "0".to_string(),
                        n.node_type.to_string(),
                    ]);
                }
            }
            8 => {
                let mut d = line.clone();
                d.push("N".to_string());
                rows.push(d);
            }
            6 => rows.push(line.clone()),
            _ => {}
        }
    }
    let mut content = txt(&rows);
    content.push_str(&topo.properties_content());
    Ok(content)
}

/// `Tools.getTxt`.
fn txt(rows: &[Vec<String>]) -> String {
    rows.iter().map(|r| r.join(",") + "\n").collect()
}

/// Inputs of [`simulate`] besides the design.
#[derive(Clone, Debug)]
pub struct SimulationOptions {
    pub locale: Locale,
    pub kind: SimulationType,
    /// Number of runs of a random simulation (default `simu.nbsim.value`).
    pub nb_sim: Option<String>,
    /// Probability that a tap is open in a random run (default
    /// `simu.simopentaps.value`).
    pub open_taps: Option<String>,
    /// `commercial` or `ideal` orifices (default `simu.typeorifice.value`).
    pub orifice: Option<String>,
    /// Open taps of a handmade simulation.
    pub open: Vec<String>,
    /// Seed of the random generator.
    pub seed: u64,
    /// Catalog used to refresh the design's diameters, as the GUI does on
    /// load. The default is the shipped catalog.
    pub database: Option<Database>,
}

impl Default for SimulationOptions {
    fn default() -> Self {
        SimulationOptions {
            locale: Locale::En,
            kind: SimulationType::Random,
            nb_sim: None,
            open_taps: None,
            orifice: None,
            open: Vec::new(),
            seed: 0,
            database: None,
        }
    }
}

/// Run a simulation on a `.dsg` text and return the `.sim` text.
pub fn simulate(name: &str, dsg: &str, opts: &SimulationOptions) -> Result<String> {
    let app = application_properties(opts.locale);
    let mut design = Design::new(name, dsg, &app, opts.locale);
    let db = opts.database.clone().unwrap_or_default();
    design.refresh(&db);

    let get = |k: &str| app.get_property(k).unwrap_or("").to_string();
    let mut nbsim = opts
        .nb_sim
        .clone()
        .unwrap_or_else(|| get("simu.nbsim.value"));
    let mut opentaps = opts
        .open_taps
        .clone()
        .unwrap_or_else(|| get("simu.simopentaps.value"));
    let orifice = opts
        .orifice
        .clone()
        .unwrap_or_else(|| get("simu.typeorifice.value"));
    let mut faucet_ref: HashMap<String, bool> = HashMap::new();
    let type_name = match opts.kind {
        SimulationType::Random => "random",
        SimulationType::TapByTap => "tapbytap",
        SimulationType::Handmade => "handmade",
    };
    if opts.kind != SimulationType::Random {
        let faucets = design
            .network
            .nodes
            .iter()
            .filter(|n| n.node_type == TYPE_FAUCET)
            .count();
        opentaps = "-".to_string();
        nbsim = if opts.kind == SimulationType::TapByTap {
            faucets.to_string()
        } else {
            "1".to_string()
        };
    }
    if opts.kind == SimulationType::Handmade {
        for t in &opts.open {
            faucet_ref.insert(t.clone(), true);
        }
    }

    // NewSimuDialog(design.getProperties()) and getParameters()
    let mut prop = JProperties::with_defaults(&design.properties);
    prop.put_all(&design.properties);
    let mut p = JProperties::with_defaults(&prop);
    let from_prop = |k: &str| prop.get_property(k).unwrap_or("").to_string();
    p.set_property("simu.nbsim.value", &nbsim);
    p.set_property("simu.simopentaps.value", &opentaps);
    p.set_property("simu.targetflow.value", &from_prop("simu.targetflow.value"));
    p.set_property(
        "simu.mincriticalflow.value",
        &from_prop("simu.mincriticalflow.value"),
    );
    p.set_property(
        "simu.maxcriticalflow.value",
        &from_prop("simu.maxcriticalflow.value"),
    );
    p.set_property("simu.typeorifice.value", &orifice);
    p.set_property("simu.typesimu.value", type_name);

    // SolverDisk.simulation
    let ovector = orifices_vector(&design.core_orifices())?;
    let _ = ovector;
    let temp = parse_double(p.get_property_or("topo.watertemp.value", "70"))
        .ok_or_else(|| Error::Parameter("topo.watertemp.value".into()))?;
    let dvector = diameters_vector(&design.core_diameters(), temp)?;
    let net = core_design(&design.core_design()?, &dvector)?;
    let mut rng = SplitMix64(opts.seed);
    let ms = MakeSimulation::run(net, &p, &faucet_ref, &mut rng, opts.locale)?;

    let mut content = design.content();
    content.push_str(&txt(&ms.simple_results()));
    content.push_str(&txt(&ms.quartiles()));
    content.push_str(&txt(&ms.speeds()));
    content.push_str(&txt(&ms.pressures()));
    content.push_str(&ms.properties_content());
    Ok(project_file_text(&content))
}
