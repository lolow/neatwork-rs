//! State of the application and the refresh of the window (Java
//! `FrameNeatwork`, `ProjectManager` and the panes).

use std::collections::HashMap;
use std::path::PathBuf;

use neatwork_core::java::{
    JHashtable, JProperties, decimal_format, double_to_string, parse_double,
};
use neatwork_core::project::network::TYPE_RESERVOIR;
use neatwork_core::project::{Cell, Design, SimulationFile, Topographie};
use neatwork_core::{Constraint, Database, Locale, application_properties};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::drawing::{self, Coloring, Layout, NetNode, NetPipe};
use crate::files::{FileKind, Files};
use crate::i18n::{self, Messages};
use crate::support;
use crate::table::{Col, Kind, Table};
use crate::{AppState, DrawEdge, DrawNode, LegendItem, MainWindow, ParamRow, ProjectTab, T};

/// An open project and the state of its panes.
pub struct OpenProject {
    pub proj: Proj,
    pub nodes: Table,
    pub pipes: Table,
    /// Simulation numbers of a design (`<design>.<n>.sim`).
    pub sims: Vec<u64>,
    pub sim_current: Option<usize>,
    pub sim: Option<SimulationFile>,
    /// Node highlighted in the tree view and the drawing.
    pub selected_node: Option<String>,
}

pub enum Proj {
    Topo(Topographie),
    Design(Design),
}

impl OpenProject {
    pub fn name(&self) -> &str {
        match &self.proj {
            Proj::Topo(t) => &t.name,
            Proj::Design(d) => &d.name,
        }
    }

    pub fn set_name(&mut self, name: &str) {
        match &mut self.proj {
            Proj::Topo(t) => t.name = name.to_string(),
            Proj::Design(d) => d.name = name.to_string(),
        }
    }

    pub fn kind(&self) -> FileKind {
        match &self.proj {
            Proj::Topo(_) => FileKind::Topo,
            Proj::Design(_) => FileKind::Design,
        }
    }

    pub fn content(&self) -> String {
        match &self.proj {
            Proj::Topo(t) => t.content(),
            Proj::Design(d) => d.content(),
        }
    }

    pub fn network(&self) -> &neatwork_core::project::network::Network {
        match &self.proj {
            Proj::Topo(t) => &t.network,
            Proj::Design(d) => &d.network,
        }
    }
}

/// Action waiting for the answer of the message box.
pub enum Pending {
    None,
    CloseCurrent,
    CloseAll,
    Exit,
    /// New topography over an existing file.
    NewOverwrite(String),
    /// Save as over an existing file.
    SaveOverwrite(String),
    /// Archive a NW5 revision (F11).
    Nw5ArchiveRevision(String),
    /// Delete a NW5 run (F39).
    Nw5DeleteRun(u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FdMode {
    Open,
    Save,
    Delete,
    New,
}

pub struct FileDialogState {
    pub mode: FdMode,
    pub kind: FileKind,
    pub files: Vec<String>,
}

/// Make-design dialog state (`MakeDesignDialog` and its panes).
pub struct MakeDesignState {
    pub project: usize,
    pub hsource: f64,
    pub diameters: Table,
    pub orifices: Table,
    pub load_factors: Table,
    /// Suggested load factors and the parameters they come from.
    pub suggested: JHashtable<String, f64>,
    pub modified: JHashtable<String, f64>,
    pub lf_params: (String, String, String),
    pub params: Vec<(Option<&'static str>, String)>,
    pub pipes: Vec<(String, String)>,
    pub constraints: Vec<(Constraint, String)>,
}

/// New simulation dialog state.
pub struct NewSimuState {
    pub project: usize,
}

/// Result of a background run, shown when the progress dialog closes.
pub enum Finished {
    Design {
        project_name: String,
        result: Result<(Box<neatwork_core::MadeDesign>, bool), String>,
    },
    Simulation {
        project: usize,
        result: Result<String, String>,
    },
    /// NW5 design of the NW5 tab `tab`.
    Nw5Design {
        tab: u64,
        result: Result<Box<crate::nw5::DesignOutcome>, String>,
    },
}

pub struct App {
    pub ui: slint::Weak<MainWindow>,
    pub msgs: Messages,
    pub lang: i32,
    pub files: Files,
    pub projects: Vec<OpenProject>,
    pub current: Option<usize>,
    pub db: Database,
    pub db_diameters: Table,
    pub db_orifices: Table,
    pub pending: Pending,
    pub fd: Option<FileDialogState>,
    pub md: Option<MakeDesignState>,
    pub ns: Option<NewSimuState>,
    pub faucets: Table,
    pub quick_check: Table,
    pub dialog_table: Table,
    pub finished: Option<Finished>,
    /// After the pressure table of a design: the design to add.
    pub design_to_add: Option<Design>,
    /// NeatWork 5 project tabs and dialogs.
    pub nw5: crate::nw5::Nw5State,
}

/// Parameters of the make-design dialog (`ParametersPane`): header keys and
/// property keys.
pub const PARAMS: [(Option<&str>, &str); 11] = [
    (
        Some("ParametersPane.<html><b>-_Make_Design_Parameters</b>"),
        "",
    ),
    (None, "topo.opentaps.value"),
    (None, "topo.servicequal.value"),
    (None, "topo.targetflow.value"),
    (None, "topo.limitbudget.value"),
    (
        Some("ParametersPane.<html><b>-_Physical_Parameters</b>"),
        "",
    ),
    (None, "topo.watertemp.value"),
    (None, "topo.pipelength.value"),
    (
        Some("ParametersPane.<html><b>-_Advanced_Parameters</b>"),
        "",
    ),
    (None, "topo.orifcoef.value"),
    (None, "topo.faucetcoef.value"),
];

fn sstr(s: &str) -> SharedString {
    SharedString::from(s)
}

fn strings(v: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    let v: Vec<SharedString> = v.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(v))
}

impl App {
    pub fn new(ui: &MainWindow, base: PathBuf) -> Self {
        let lang = crate::files::read_locale().map_or(i18n::EN, |c| i18n::lang_from_code(&c));
        let files = Files::new(&base);
        let nw5 = crate::nw5::Nw5State::new(base.join("projects"));
        let db = Database::new(
            &files.read_db("diameters.db"),
            &files.read_db("orifices.db"),
        );
        let mut app = App {
            ui: ui.as_weak(),
            msgs: Messages::new(),
            lang,
            files,
            projects: Vec::new(),
            current: None,
            db,
            db_diameters: Table::default(),
            db_orifices: Table::default(),
            pending: Pending::None,
            fd: None,
            md: None,
            ns: None,
            faucets: Table::default(),
            quick_check: Table::default(),
            dialog_table: Table::default(),
            finished: None,
            design_to_add: None,
            nw5,
        };
        app.db_diameters = app.db_diameter_table();
        app.db_orifices = app.db_orifice_table();
        app
    }

    /// Text of a message key in the current language.
    pub fn tr(&self, key: &str) -> String {
        self.msgs.get(self.lang, key)
    }

    pub fn locale(&self) -> Locale {
        match self.lang {
            i18n::FR => Locale::Fr,
            i18n::SP => Locale::Sp,
            _ => Locale::En,
        }
    }

    pub fn app_properties(&self) -> JProperties {
        application_properties(self.locale())
    }

    pub fn ui(&self) -> Option<MainWindow> {
        self.ui.upgrade()
    }

    pub fn cur(&self) -> Option<&OpenProject> {
        self.current.and_then(|i| self.projects.get(i))
    }

    pub fn cur_mut(&mut self) -> Option<&mut OpenProject> {
        self.current.and_then(|i| self.projects.get_mut(i))
    }

    // ------------------------------------------------------------ messages

    pub fn message(&mut self, title: &str, text: &str, buttons: i32, pending: Pending) {
        self.pending = pending;
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_msg_title(sstr(title));
            s.set_msg_text(sstr(text));
            s.set_msg_buttons(buttons);
            s.set_msg_visible(true);
        }
    }

    pub fn info(&mut self, title_key: &str, text: &str) {
        let title = self.tr(title_key);
        self.message(&title, text, 0, Pending::None);
    }

    // ------------------------------------------------------------- tables

    pub fn node_type_name(&self, t: &str) -> String {
        match t {
            "0" => self.tr("Node.TANK"),
            "1" => self.tr("Node.BRANCHING_NODE"),
            "2" => self.tr("Node.FAUCET_NODE"),
            _ => String::new(),
        }
    }

    fn title(&self, key: &str) -> String {
        self.tr(key).trim().to_string()
    }

    pub fn topo_tables(&self, t: &Topographie) -> (Table, Table) {
        let mut nodes = Table::new(
            vec![
                Col::new(
                    self.title("TopoNodeTableModel._ID"),
                    100.0,
                    true,
                    Kind::Text,
                ),
                Col::new(
                    self.title("TopoNodeTableModel._Height"),
                    100.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.title("TopoNodeTableModel._X"),
                    60.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.title("TopoNodeTableModel._Y"),
                    60.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.title("TopoNodeTableModel.__#_of_faucets"),
                    90.0,
                    true,
                    Kind::Int,
                ),
                Col::new(
                    self.title("TopoNodeTableModel._Nature"),
                    150.0,
                    false,
                    Kind::NodeType,
                ),
            ],
            true,
        );
        nodes.set_rows(
            t.network
                .nodes
                .iter()
                .map(|n| {
                    vec![
                        n.name.clone(),
                        double_to_string(n.height),
                        double_to_string(n.coord_x),
                        double_to_string(n.coord_y),
                        n.nb_taps.to_string(),
                        n.node_type.to_string(),
                    ]
                })
                .collect(),
        );
        let mut pipes = Table::new(
            vec![
                Col::new(
                    self.title("TopoPipeTableModel._Begin"),
                    100.0,
                    true,
                    Kind::Text,
                ),
                Col::new(
                    self.title("TopoPipeTableModel._End"),
                    100.0,
                    true,
                    Kind::Text,
                ),
                Col::new(
                    self.title("TopoPipeTableModel._Length"),
                    100.0,
                    true,
                    Kind::Double,
                ),
            ],
            true,
        );
        pipes.set_rows(
            t.network
                .pipes
                .iter()
                .map(|p| vec![p.begin.clone(), p.end.clone(), double_to_string(p.length)])
                .collect(),
        );
        (nodes, pipes)
    }

    pub fn design_tables(&self, d: &Design) -> (Table, Table) {
        let mut nodes = Table::new(
            vec![
                Col::new(self.tr("DesignNodeModel.ID"), 100.0, true, Kind::Text),
                Col::new(self.tr("DesignNodeModel.Height"), 100.0, true, Kind::Double),
                Col::new(self.tr("DesignNodeModel.X"), 60.0, true, Kind::Double),
                Col::new(self.tr("DesignNodeModel.Y"), 60.0, true, Kind::Double),
                Col::new(
                    self.tr("DesignNodeModel.Ideal_Orifice"),
                    120.0,
                    true,
                    Kind::Orifice,
                ),
                Col::new(
                    self.tr("DesignNodeModel.Commercial_Orifice"),
                    130.0,
                    true,
                    Kind::Orifice,
                ),
                Col::new(
                    self.tr("DesignNodeModel.Nature"),
                    130.0,
                    false,
                    Kind::NodeType,
                ),
            ],
            true,
        );
        nodes.set_rows(
            d.network
                .nodes
                .iter()
                .map(|n| {
                    vec![
                        n.name.clone(),
                        double_to_string(n.height),
                        double_to_string(n.coord_x),
                        double_to_string(n.coord_y),
                        double_to_string(n.orifice),
                        double_to_string(n.orifice_commercial),
                        n.node_type.to_string(),
                    ]
                })
                .collect(),
        );
        let mut pipes = Table::new(
            vec![
                Col::new(self.tr("DesignPipeModel.Begin"), 90.0, true, Kind::Text),
                Col::new(self.tr("DesignPipeModel.End"), 90.0, true, Kind::Text),
                Col::new(self.tr("DesignPipeModel.Length"), 80.0, true, Kind::Double),
                Col::new(
                    self.tr("DesignPipeModel.Length_1"),
                    80.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.tr("DesignPipeModel.diam_1"),
                    80.0,
                    true,
                    Kind::Diameter,
                ),
                Col::new(
                    self.tr("DesignPipeModel.Length_2"),
                    80.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.tr("DesignPipeModel.diam_2"),
                    80.0,
                    true,
                    Kind::Diameter,
                ),
            ],
            true,
        );
        pipes.set_rows(
            d.network
                .pipes
                .iter()
                .map(|p| {
                    vec![
                        p.begin.clone(),
                        p.end.clone(),
                        double_to_string(p.length),
                        double_to_string(p.length1),
                        p.ref_diam1.clone(),
                        double_to_string(p.length2),
                        p.ref_diam2.clone(),
                        "N".to_string(),
                    ]
                })
                .collect(),
        );
        (nodes, pipes)
    }

    pub fn db_diameter_table(&self) -> Table {
        let mut t = Table::new(
            vec![
                Col::new(
                    self.tr("DBDiameterTableModel.Nominal"),
                    90.0,
                    true,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("DBDiameterTableModel.SDR"),
                    75.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.tr("DBDiameterTableModel.Diameter"),
                    85.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.tr("DBDiameterTableModel.Cost/m"),
                    75.0,
                    true,
                    Kind::Double,
                ),
                Col::new(
                    self.tr("DBDiameterTableModel.Max_Pressure"),
                    100.0,
                    true,
                    Kind::Double,
                ),
                Col::new(self.tr("DBDiameterTableModel.Type"), 60.0, true, Kind::Int),
                Col::new(
                    self.tr("DBDiameterTableModel.Roughness").trim().to_string(),
                    85.0,
                    true,
                    Kind::Double,
                ),
            ],
            true,
        );
        t.set_rows(
            self.db
                .diameters()
                .iter()
                .map(|d| d.selection_line().split(',').map(str::to_string).collect())
                .collect(),
        );
        t
    }

    pub fn db_orifice_table(&self) -> Table {
        let mut t = Table::new(
            vec![Col::new(
                self.tr("DBOrificeTableModel.Diameter"),
                120.0,
                true,
                Kind::Double,
            )],
            true,
        );
        t.set_rows(
            self.db
                .orifices()
                .iter()
                .map(|o| vec![double_to_string(*o)])
                .collect(),
        );
        t
    }

    /// Label of a catalog diameter in a list (`ComboDatabase.getEnonce`).
    pub fn diameter_choices(&self) -> Vec<String> {
        self.db
            .diameters()
            .iter()
            .map(|d| {
                let t = if d.pipe_type == 2 {
                    self.tr("Diameter.IRON")
                } else {
                    self.tr("Diameter.PVC")
                };
                format!(
                    "diam {}:sdr {}:({t})",
                    double_to_string(d.diameter),
                    double_to_string(d.sdr)
                )
            })
            .collect()
    }

    /// Show a cell of a project table.
    fn show_cell(&self, p: &OpenProject, table: usize, col: usize, raw: &str) -> String {
        let t = if table == 0 { &p.nodes } else { &p.pipes };
        match t.cols.get(col).map(|c| c.kind) {
            Some(Kind::NodeType) => self.node_type_name(raw),
            Some(Kind::Orifice) => match parse_double(raw) {
                Some(v) => {
                    if v == 10000.0 {
                        "-".to_string()
                    } else {
                        decimal_format("0.##########", v)
                    }
                }
                None => raw.to_string(),
            },
            Some(Kind::Diameter) => match &p.proj {
                Proj::Design(d) => d
                    .diam_table
                    .get(&raw.to_string())
                    .map_or_else(|| "-".to_string(), |x| double_to_string(x.diameter)),
                Proj::Topo(_) => raw.to_string(),
            },
            _ => raw.to_string(),
        }
    }

    // ------------------------------------------------------------ refresh

    /// Update the whole window from the state.
    pub fn refresh(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<AppState>();
        ui.global::<T>().set_lang(self.lang);
        let tab = |name: &str, kind: i32| {
            let short = if name.chars().count() > 15 {
                name.chars().take(13).collect::<String>() + "..."
            } else {
                name.to_string()
            };
            ProjectTab {
                title: sstr(&short),
                tooltip: sstr(name),
                kind,
            }
        };
        let mut tabs: Vec<ProjectTab> = self
            .projects
            .iter()
            .map(|p| tab(p.name(), i32::from(p.kind() == FileKind::Design)))
            .collect();
        // NW5 project tabs follow the NW4 projects (kind 2)
        tabs.extend(self.nw5.tabs.iter().map(|t| tab(t.project.name(), 2)));
        s.set_project_count(tabs.len() as i32);
        s.set_projects(ModelRc::new(VecModel::from(tabs)));
        let nw5 = self.nw5_cur().and(self.nw5.current);
        s.set_current(match (self.current, nw5) {
            (Some(c), _) => c as i32,
            (None, Some(k)) => (self.projects.len() + k) as i32,
            _ => -1,
        });
        s.set_current_kind(match (self.cur(), nw5) {
            (Some(p), _) => i32::from(p.kind() == FileKind::Design),
            (None, Some(_)) => 2,
            _ => -1,
        });
        s.set_diameter_choices(strings(self.diameter_choices()));
        self.refresh_tables();
        self.refresh_text();
        self.refresh_tree();
        self.refresh_network();
        self.refresh_simulations();
        self.refresh_nw5();
    }

    pub fn refresh_tables(&self) {
        let (Some(ui), Some(p)) = (self.ui(), self.cur()) else {
            return;
        };
        let s = ui.global::<AppState>();
        let lock_pipes = |r: usize, c: usize| -> bool {
            // Diam 2 is read-only while Length 2 is 0
            c == 6
                && p.kind() == FileKind::Design
                && p.pipes.rows.get(r).and_then(|row| parse_double(&row[5])) == Some(0.0)
        };
        s.set_nodes_table(
            p.nodes
                .to_slint(&|_, c, raw| self.show_cell(p, 0, c, raw), &|_, _| false),
        );
        s.set_pipes_table(p.pipes.to_slint(
            &|r, c, raw| {
                if lock_pipes(r, c) {
                    "-".to_string()
                } else {
                    self.show_cell(p, 1, c, raw)
                }
            },
            &lock_pipes,
        ));
        if let Proj::Design(d) = &p.proj {
            s.set_cost_text(sstr(&support::cost_label(self, d)));
        }
    }

    pub fn refresh_text(&self) {
        if let (Some(ui), Some(p)) = (self.ui(), self.cur()) {
            ui.global::<AppState>().set_text_content(sstr(&p.content()));
        }
    }

    pub fn refresh_tree(&self) {
        let (Some(ui), Some(p)) = (self.ui(), self.cur()) else {
            return;
        };
        let s = ui.global::<AppState>();
        let view = support::tree_view(self, p.name(), p.network(), p.selected_node.as_deref());
        s.set_tree_message(sstr(&view.message));
        s.set_tree_rows(view.rows);
        s.set_tree_cols(view.cols);
        s.set_tree_cells(ModelRc::new(VecModel::from(view.cells)));
    }

    /// Coloring choices of the current project.
    fn colorings(&self) -> Vec<&'static str> {
        match self.cur().map(OpenProject::kind) {
            Some(FileKind::Design) => vec![
                "Gui.Color_type",
                "Gui.Color_height",
                "Gui.Color_diameter",
                "Gui.Color_flow",
                "Gui.Color_pressure",
            ],
            _ => vec!["Gui.Color_type", "Gui.Color_height"],
        }
    }

    pub fn refresh_network(&self) {
        let (Some(ui), Some(p)) = (self.ui(), self.cur()) else {
            return;
        };
        let s = ui.global::<AppState>();
        let colorings = self.colorings();
        s.set_net_colorings(strings(colorings.iter().map(|k| self.tr(k))));
        let ci = usize::try_from(s.get_net_coloring())
            .unwrap_or(0)
            .min(colorings.len() - 1);
        let layout = match s.get_net_layout() {
            1 => Layout::Profile,
            2 => Layout::Coordinates,
            _ => Layout::Tree,
        };
        let net = p.network();
        let sim = p.sim.as_ref();
        // Values of the simulation by node name
        let sim_name = |name: &str| support::sim_tap_name(name);
        let flows: HashMap<String, f64> = sim
            .map(|f| {
                f.flow_taps()
                    .iter()
                    .filter_map(|r| Some((r[0].display(), r.get(3)?.number()?)))
                    .collect()
            })
            .unwrap_or_default();
        let pressures: HashMap<String, f64> = sim
            .map(|f| {
                f.node_pressures()
                    .iter()
                    .filter_map(|r| Some((r[0].display(), r.get(2)?.number()?)))
                    .collect()
            })
            .unwrap_or_default();
        let key = colorings[ci];
        let coloring = match key {
            "Gui.Color_height" => Coloring::Height,
            "Gui.Color_diameter" => Coloring::Diameter,
            "Gui.Color_flow" => {
                let prop = |k: &str, d: f64| {
                    sim.and_then(|f| f.property(k))
                        .and_then(parse_double)
                        .unwrap_or(d)
                };
                Coloring::Flow {
                    low: prop("mincriticalflow", 0.1),
                    high: prop("maxcriticalflow", 0.3),
                }
            }
            "Gui.Color_pressure" => Coloring::Pressure,
            _ => Coloring::NodeType,
        };
        let nodes: Vec<NetNode> = net
            .nodes
            .iter()
            .map(|n| NetNode {
                name: n.name.clone(),
                node_type: n.node_type,
                height: n.height,
                x: n.coord_x,
                y: n.coord_y,
                value: match coloring {
                    Coloring::Flow { .. } => flows.get(&sim_name(&n.name)).copied(),
                    Coloring::Pressure => pressures.get(&n.name).copied(),
                    _ => None,
                },
            })
            .collect();
        let pipes: Vec<NetPipe> = net
            .pipes
            .iter()
            .map(|pp| {
                let diameter = match &p.proj {
                    Proj::Design(d) => {
                        let r = if pp.length1 >= pp.length2 {
                            &pp.ref_diam1
                        } else {
                            &pp.ref_diam2
                        };
                        d.diam_table.get(r).map(|x| x.diameter)
                    }
                    Proj::Topo(_) => None,
                };
                NetPipe {
                    begin: pp.begin.clone(),
                    end: pp.end.clone(),
                    length: pp.length,
                    diameter,
                }
            })
            .collect();
        let pic = drawing::draw(&nodes, &pipes, layout, coloring);
        let path = p
            .selected_node
            .as_deref()
            .map(|n| drawing::path_to_source(&pipes, n))
            .unwrap_or_default();
        let dn: Vec<DrawNode> = nodes
            .iter()
            .zip(&pic.nodes)
            .map(|(n, (x, y, c, r))| DrawNode {
                x: *x as f32,
                y: *y as f32,
                label: sstr(&n.name),
                color: *c,
                radius: *r as f32,
                highlight: path.contains(&n.name),
            })
            .collect();
        let de: Vec<DrawEdge> = pipes
            .iter()
            .zip(&pic.edges)
            .map(|(pp, (x1, y1, x2, y2, c, w))| DrawEdge {
                x1: *x1 as f32,
                y1: *y1 as f32,
                x2: *x2 as f32,
                y2: *y2 as f32,
                color: *c,
                width: *w as f32,
                highlight: path.contains(&pp.begin) && path.contains(&pp.end),
            })
            .collect();
        let legend: Vec<LegendItem> = pic
            .legend
            .iter()
            .map(|(c, l)| LegendItem {
                color: *c,
                label: sstr(&if l.contains('.') && !l.contains(' ') {
                    self.tr(l)
                } else {
                    l.clone()
                }),
            })
            .collect();
        s.set_net_width(pic.width as f32);
        s.set_net_height(pic.height as f32);
        s.set_net_nodes(ModelRc::new(VecModel::from(dn)));
        s.set_net_edges(ModelRc::new(VecModel::from(de)));
        s.set_net_legend(ModelRc::new(VecModel::from(legend)));
        let mut info = String::new();
        if pic.no_coordinates {
            info.push_str(&self.tr("Gui.No_coordinates"));
            info.push('\n');
        }
        if matches!(coloring, Coloring::Flow { .. } | Coloring::Pressure) && sim.is_none() {
            info.push_str(&self.tr("Gui.No_simulation"));
            info.push('\n');
        }
        if let Some(name) = &p.selected_node {
            info.push_str(&support::node_info(self, p, name, &flows, &pressures));
        }
        s.set_net_info(sstr(&info));
    }

    pub fn refresh_simulations(&self) {
        let (Some(ui), Some(p)) = (self.ui(), self.cur()) else {
            return;
        };
        let s = ui.global::<AppState>();
        s.set_sim_list(strings(p.sims.iter().map(|n| format!("simu {n}"))));
        s.set_sim_current(p.sim_current.map_or(-1, |i| i as i32));
        let Some(f) = &p.sim else {
            s.set_sim_params_text(sstr(""));
            return;
        };
        s.set_sim_params_text(sstr(&support::sim_params_text(self, f)));
        let table = support::sim_table(self, f, s.get_sim_table_kind());
        s.set_sim_table(table.plain());
    }

    // ------------------------------------------------------------ projects

    /// Add a project and select it (`ProjectManager.addProject`).
    pub fn add_project(&mut self, proj: Proj) {
        let (nodes, pipes) = match &proj {
            Proj::Topo(t) => self.topo_tables(t),
            Proj::Design(d) => self.design_tables(d),
        };
        let mut p = OpenProject {
            proj,
            nodes,
            pipes,
            sims: Vec::new(),
            sim_current: None,
            sim: None,
            selected_node: None,
        };
        if p.kind() == FileKind::Design {
            p.sims = self.files.simulations(p.name());
            if !p.sims.is_empty() {
                p.sim_current = Some(0);
                let name = format!("{}.{}", p.name(), p.sims[0]);
                p.sim = Some(SimulationFile::new(
                    &name,
                    &self.files.read(&name, FileKind::Simulation),
                ));
            }
        }
        self.projects.push(p);
        self.current = Some(self.projects.len() - 1);
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_pane_tab(0);
        }
        self.refresh();
    }

    /// Names of the open projects of a kind (for `isAlreadyLoaded`).
    pub fn open_names(&self, kind: FileKind) -> Vec<String> {
        self.projects
            .iter()
            .filter(|p| p.kind() == kind)
            .map(|p| p.name().to_string())
            .collect()
    }

    pub fn remove_current(&mut self) {
        if let Some(i) = self.current {
            if i < self.projects.len() {
                self.projects.remove(i);
            }
            self.current = if self.projects.is_empty() {
                None
            } else {
                Some(i.min(self.projects.len() - 1))
            };
        }
        self.refresh();
    }

    /// Rebuild the tables of the current project from its model.
    pub fn reload_tables(&mut self) {
        let Some(i) = self.current else {
            return;
        };
        let (n, p) = match &self.projects[i].proj {
            Proj::Topo(t) => self.topo_tables(t),
            Proj::Design(d) => self.design_tables(d),
        };
        self.projects[i].nodes = n;
        self.projects[i].pipes = p;
    }

    /// Load a simulation of the current design.
    pub fn select_simulation(&mut self, index: usize) {
        let Some(i) = self.current else {
            return;
        };
        let p = &self.projects[i];
        let Some(n) = p.sims.get(index) else {
            return;
        };
        let name = format!("{}.{n}", p.name());
        let content = self.files.read(&name, FileKind::Simulation);
        let p = &mut self.projects[i];
        p.sim_current = Some(index);
        p.sim = Some(SimulationFile::new(&name, &content));
    }

    // ------------------------------------------------------- make design

    pub fn md_params(&self, topo: &Topographie) -> Vec<(Option<&'static str>, String)> {
        PARAMS
            .iter()
            .map(|(h, k)| match h {
                Some(key) => (Some(*key), String::new()),
                None => (
                    None,
                    topo.properties.get_property(k).unwrap_or("").to_string(),
                ),
            })
            .collect()
    }

    pub fn md_param_rows(&self, md: &MakeDesignState) -> Vec<ParamRow> {
        PARAMS
            .iter()
            .zip(&md.params)
            .map(|((h, k), (_, v))| match h {
                Some(key) => ParamRow {
                    label: sstr(&self.tr(key)),
                    value: sstr(""),
                    header: true,
                },
                None => ParamRow {
                    label: sstr(&self.tr(&k.replace(".value", ".name"))),
                    value: sstr(v),
                    header: false,
                },
            })
            .collect()
    }

    /// Load factor rows (`LoadFactorTableModel`).
    pub fn load_factor_table(&self, topo: &Topographie, md: &MakeDesignState) -> Table {
        let load_taps = topo.load_taps().unwrap_or_default();
        let mut t = Table::new(
            vec![
                Col::new(
                    self.tr("LoadFactorTableModel.Begin"),
                    70.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("LoadFactorTableModel.End_2"),
                    70.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("LoadFactorTableModel.__#_of_faucets")
                        .trim()
                        .to_string(),
                    100.0,
                    false,
                    Kind::Int,
                ),
                Col::new(
                    self.tr("LoadFactorTableModel.Theoric_load_factor"),
                    160.0,
                    false,
                    Kind::Double,
                ),
                Col::new(
                    self.tr("LoadFactorTableModel.Modified_load_factor"),
                    160.0,
                    true,
                    Kind::Double,
                ),
            ],
            false,
        );
        t.set_rows(
            topo.expanded_nodes
                .iter()
                .filter(|n| n.node_type != TYPE_RESERVOIR)
                .map(|n| {
                    let begin = topo
                        .expanded_pred_pipe(&n.name)
                        .map(|p| p.begin.clone())
                        .unwrap_or_default();
                    let get = |h: &JHashtable<String, f64>| {
                        h.get(&n.name)
                            .map_or(String::new(), |v| double_to_string(*v))
                    };
                    vec![
                        begin,
                        n.name.clone(),
                        load_taps.get(&n.name).cloned().unwrap_or_default(),
                        get(&md.suggested),
                        get(&md.modified),
                    ]
                })
                .collect(),
        );
        t
    }

    pub fn push_make_design(&self) {
        let (Some(ui), Some(md)) = (self.ui(), &self.md) else {
            return;
        };
        let s = ui.global::<AppState>();
        s.set_md_diameters(md.diameters.plain());
        s.set_md_orifices(md.orifices.plain());
        s.set_md_load_factors(md.load_factors.plain());
        s.set_md_params(ModelRc::new(VecModel::from(self.md_param_rows(md))));
        s.set_md_pipes(strings(
            md.pipes.iter().map(|(b, e)| format!("Arc : {b} -> {e}")),
        ));
        let items: Vec<slint::StandardListViewItem> = md
            .constraints
            .iter()
            .map(|(_, t)| slint::StandardListViewItem::from(t.as_str()))
            .collect();
        s.set_md_constraint_items(ModelRc::new(VecModel::from(items)));
    }

    /// Simulation table cell text.
    pub fn cell(c: &Cell) -> String {
        c.display()
    }
}
