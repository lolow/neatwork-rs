//! NeatWork 5 workspace (`rust/nw5/specs/app.md`): NW5 project folders open
//! as project tabs next to the NW4 projects, and the refresh of the
//! workspace from `neatwork-nw5-project`. The actions are in
//! [`crate::nw5_actions`].

use std::path::PathBuf;

use neatwork_nw5::Network;
use neatwork_nw5_design::Costs;
use neatwork_nw5_project::compare::{
    Compatibility, CompatibilityLevel, MatchStatus, Sign, column_label, common_tables,
    comparable_columns, compare_run_metrics, comparison_sources, default_pair, match_strategies,
    metric_table, recommended_pair, unified_table,
};
use neatwork_nw5_project::format::{MISSING, fixed};
use neatwork_nw5_project::graph::{EdgeKind, NodeKind};
use neatwork_nw5_project::review::{
    NodeRole, SortDirection, SortState, TableKind, execution_details, result_table, run_label,
    run_summary, sort_indicator, status_label,
};
use neatwork_nw5_project::{
    CancelFlag, DisplayUnit, GraphData, LoadedRevision, Project, ProjectEntry, RequestedOptions,
    RunRecord,
};
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};

use crate::app::App;
use crate::drawing::{Nw5Layout, nw5_edge_style, nw5_node_color, nw5_node_radius, nw5_picture};
use crate::nw5_tables::domain_tables;
use crate::{
    Nw5, Nw5Cell, Nw5Column, Nw5Edge, Nw5Node, Nw5Pair, Nw5ProjectItem, Nw5Revision, Nw5Row,
    Nw5Run, Nw5Table,
};

/// Selected node or edge of the network view (F24).
#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    Node(String),
    Edge(String),
}

/// The run in progress of a project (one at a time, ADR-025 item 4).
pub struct ActiveRun {
    pub run_number: u32,
    pub cancel: CancelFlag,
    pub done: usize,
    pub total: usize,
}

/// Result of a NW5 design run, sent back from the design thread.
pub struct DesignOutcome {
    pub network: Network,
    pub costs: Costs,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

/// An open NW5 project and the state of its workspace.
pub struct Nw5Tab {
    /// Identifies the tab for the results of background threads.
    pub id: u64,
    pub project: Project,
    pub loaded: Option<LoadedRevision>,
    pub load_error: Option<String>,
    /// Graph of the active revision, without results.
    pub graph: Option<GraphData>,
    pub runs: Vec<RunRecord>,
    /// Warnings of the last import (F18).
    pub warnings: Vec<String>,
    /// Error of the last run (F38) or action.
    pub banner: String,
    pub notice: String,
    pub selection: Option<Selection>,
    pub flash: Option<String>,
    /// Run whose statistics color the graph.
    pub results_run: Option<u32>,
    /// Options of the next run (F36).
    pub requested: RequestedOptions,
    pub job: Option<ActiveRun>,
    pub last_summary: String,
    pub review_run: Option<u32>,
    pub review_sort: Option<SortState>,
    pub cmp_pair: Option<(u32, u32)>,
    pub cmp_table: Option<TableKind>,
    pub cmp_sort: Option<(usize, SortDirection)>,
    /// Strategy, left metric and right metric of the numeric panel.
    pub cmp_metric: Option<(usize, usize, usize)>,
    pub compat: Option<((u32, u32), Result<Compatibility, String>)>,
}

/// State of the NW5 dialogs.
#[derive(Default)]
pub struct Nw5Dialogs {
    /// `.tpo` waiting for its catalogues (F15).
    pub tpo_path: Option<PathBuf>,
    /// Design parameters: name, value shown at opening, current text.
    pub design_params: Vec<(String, String, String)>,
    pub design_result: Option<(u64, Box<DesignOutcome>)>,
    /// Project folder to delete (F6).
    pub delete_target: Option<PathBuf>,
    pub open_list: Vec<ProjectEntry>,
}

/// NW5 projects of the application.
pub struct Nw5State {
    /// Folder of the project folders (`projects/`).
    pub dir: PathBuf,
    pub tabs: Vec<Nw5Tab>,
    pub current: Option<usize>,
    pub next_id: u64,
    pub dialogs: Nw5Dialogs,
}

impl Nw5State {
    pub fn new(dir: PathBuf) -> Self {
        Nw5State {
            dir,
            tabs: Vec::new(),
            current: None,
            next_id: 1,
            dialogs: Nw5Dialogs::default(),
        }
    }
}

impl Nw5Tab {
    pub fn new(id: u64, project: Project) -> Self {
        let requested = project.meta().last_run_options.clone().unwrap_or_default();
        let mut tab = Nw5Tab {
            id,
            project,
            loaded: None,
            load_error: None,
            graph: None,
            runs: Vec::new(),
            warnings: Vec::new(),
            banner: String::new(),
            notice: String::new(),
            selection: None,
            flash: None,
            results_run: None,
            requested,
            job: None,
            last_summary: String::new(),
            review_run: None,
            review_sort: None,
            cmp_pair: None,
            cmp_table: None,
            cmp_sort: None,
            cmp_metric: None,
            compat: None,
        };
        tab.reload();
        tab
    }

    /// Read the active revision, its graph and the runs again.
    pub fn reload(&mut self) {
        self.load_error = None;
        self.loaded = None;
        if self.project.active_revision().is_some() {
            match self.project.load_active() {
                Ok(l) => self.loaded = Some(l),
                Err(e) => self.load_error = Some(e.to_string()),
            }
        }
        self.reload_graph();
        self.reload_runs();
        let active = self.active_file();
        self.results_run = self
            .runs
            .iter()
            .rev()
            .find(|r| {
                r.status.is_completed()
                    && r.statistics.is_some()
                    && Some(&r.revision) == active.as_ref()
            })
            .map(|r| r.run_number);
        self.compat = None;
    }

    /// Rebuild the graph with the stored node positions and arc states.
    pub fn reload_graph(&mut self) {
        self.graph = self.loaded.as_ref().map(|l| {
            GraphData::build(
                &l.network,
                l.revision.format,
                &self.project.graph_state(&l.revision.file),
            )
        });
        let keep = match (&self.selection, &self.graph) {
            (Some(Selection::Node(id)), Some(g)) => g.nodes.iter().any(|n| &n.id == id),
            (Some(Selection::Edge(id)), Some(g)) => g.edges.iter().any(|e| &e.id == id),
            _ => false,
        };
        if !keep {
            self.selection = None;
        }
    }

    pub fn reload_runs(&mut self) {
        match self.project.runs() {
            Ok(r) => self.runs = r,
            Err(e) => self.banner = e.to_string(),
        }
    }

    pub fn active_file(&self) -> Option<String> {
        self.project.active_revision().map(|r| r.file.clone())
    }

    pub fn run(&self, number: u32) -> Option<&RunRecord> {
        self.runs.iter().find(|r| r.run_number == number)
    }

    /// Completed runs with statistics, oldest first (F43).
    pub fn completed_runs(&self) -> Vec<&RunRecord> {
        self.runs
            .iter()
            .filter(|r| r.status.is_completed() && r.statistics.is_some())
            .collect()
    }

    /// Graph with the results of the shown run (F21, `apply_statistics`).
    pub fn graph_with_results(&self, show: bool) -> Option<GraphData> {
        let mut g = self.graph.clone()?;
        if show
            && let Some(st) = self
                .results_run
                .and_then(|n| self.run(n))
                .and_then(|r| r.statistics.as_ref())
        {
            g.apply_statistics(st);
        }
        Some(g)
    }
}

// ----------------------------------------------------------------- helpers

pub fn ss(s: &str) -> SharedString {
    SharedString::from(s)
}

pub fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

pub fn pair(k: &str, v: &str) -> Nw5Pair {
    Nw5Pair {
        key: ss(k),
        value: ss(v),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb_u8(r, g, b)
}

fn text_color() -> Color {
    rgb(0xe5, 0xf6, 0xff)
}

/// Text and background colors of a node role (F28).
pub fn role_colors(role: Option<NodeRole>) -> (Color, Color) {
    match role {
        Some(NodeRole::Source) => (rgb(0x7d, 0xd3, 0xfc), Color::from_argb_u8(18, 56, 189, 248)),
        Some(NodeRole::Junction) => (rgb(0xfb, 0xbf, 0x24), Color::from_argb_u8(18, 245, 158, 11)),
        Some(NodeRole::Terminal) => (rgb(0x86, 0xef, 0xac), Color::from_argb_u8(18, 34, 197, 94)),
        None => (text_color(), Color::from_argb_u8(0, 0, 0, 0)),
    }
}

pub fn plain_cell(text: &str) -> Nw5Cell {
    let (fg, bg) = role_colors(None);
    Nw5Cell {
        text: ss(text),
        fg,
        bg,
    }
}

pub fn role_cell(text: &str, role: Option<NodeRole>) -> Nw5Cell {
    let (fg, bg) = role_colors(role);
    Nw5Cell {
        text: ss(text),
        fg,
        bg,
    }
}

/// A table for `Nw5Grid`: column widths from the longest text.
pub fn grid(headers: &[String], rows: Vec<Vec<Nw5Cell>>, tints: Vec<Color>) -> Nw5Table {
    let mut widths: Vec<f32> = headers
        .iter()
        .map(|h| h.chars().count() as f32 * 7.5 + 16.0)
        .collect();
    for r in &rows {
        for (k, c) in r.iter().enumerate() {
            if let Some(w) = widths.get_mut(k) {
                *w = w.max(c.text.chars().count() as f32 * 7.0 + 14.0);
            }
        }
    }
    let mut x = 0.0f32;
    let columns: Vec<Nw5Column> = headers
        .iter()
        .zip(&widths)
        .map(|(h, w)| {
            let w = w.clamp(36.0, 360.0);
            let c = Nw5Column {
                title: ss(h),
                x,
                width: w,
            };
            x += w;
            c
        })
        .collect();
    let transparent = Color::from_argb_u8(0, 0, 0, 0);
    let rows: Vec<Nw5Row> = rows
        .into_iter()
        .enumerate()
        .map(|(k, cells)| Nw5Row {
            cells: model(cells),
            tint: tints.get(k).copied().unwrap_or(transparent),
        })
        .collect();
    Nw5Table {
        columns: model(columns),
        rows: model(rows),
        width: x,
    }
}

fn empty_grid() -> Nw5Table {
    grid(&[], Vec::new(), Vec::new())
}

/// "Simulation #n" or "Simulation #n · file v<k>".
fn source_label(r: &RunRecord) -> String {
    format!(
        "{} · {} v{}",
        run_label(r.run_number),
        r.revision,
        r.revision_version
    )
}

/// Date and time of an ISO text ("2026-11-20 23:34").
pub fn short_time(iso: &str) -> String {
    let t = iso.replace('T', " ");
    t.chars().take(16).collect()
}

/// Summary line of a finished run (F35).
pub fn run_line(r: &RunRecord, low_pressure: usize) -> String {
    let summary = r.statistics.as_ref().and_then(|s| s.get("summary"));
    let num = |k: &str| {
        summary
            .and_then(|s| s.get(k))
            .and_then(neatwork_nw5_project::json::Json::as_f64)
    };
    let mut parts = vec![run_label(r.run_number), status_label(r.status).to_string()];
    if let (Some(ok), Some(total)) = (num("successfulScenarios"), num("scenarioCount")) {
        parts.push(format!("Success {ok}/{total}"));
    }
    if let Some(f) = num("failedScenarios") {
        parts.push(format!("Execution failures {f}"));
    }
    if r.status.is_completed() {
        parts.push(format!("Low P {low_pressure}"));
    }
    if let Some(s) = r.effective_seed() {
        parts.push(format!("Seed {s}"));
    }
    if !r.failures.is_empty() {
        parts.push(format!("Failure records {}", r.failures.len()));
    }
    parts.join(" · ")
}

impl App {
    /// The NW5 tab shown, when no NW4 project is selected.
    pub fn nw5_cur(&self) -> Option<&Nw5Tab> {
        if self.current.is_some() {
            return None;
        }
        self.nw5.current.and_then(|k| self.nw5.tabs.get(k))
    }

    pub fn nw5_cur_mut(&mut self) -> Option<&mut Nw5Tab> {
        if self.current.is_some() {
            return None;
        }
        self.nw5.current.and_then(|k| self.nw5.tabs.get_mut(k))
    }

    pub fn nw5_tab_mut(&mut self, id: u64) -> Option<&mut Nw5Tab> {
        self.nw5.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Update the NW5 workspace from the current NW5 tab.
    pub fn refresh_nw5(&self) {
        let (Some(ui), Some(tab)) = (self.ui(), self.nw5_cur()) else {
            return;
        };
        let s = ui.global::<Nw5>();
        let unit = tab.project.display_unit();
        s.set_project_name(ss(tab.project.name()));
        let lock = match tab.project.allowed_import_format() {
            Some(f) => format!(
                "{} {} {}",
                self.tr("Nw5.Format_lock_1"),
                f.as_str(),
                self.tr("Nw5.Format_lock_2")
            ),
            None => self.tr("Nw5.Format_lock_none"),
        };
        let header = match tab.project.active_revision() {
            Some(r) => format!(
                "{} {} (v{}, {}) · {} {} · {}",
                self.tr("Nw5.Active_revision"),
                r.file,
                r.version,
                r.format.as_str(),
                self.tr("Nw5.Unit"),
                unit.symbol(),
                lock
            ),
            None => lock.clone(),
        };
        s.set_header(ss(&header));
        s.set_format_lock(ss(&lock));
        let banner = match &tab.load_error {
            Some(e) => format!("{e}\n{}", tab.banner).trim().to_string(),
            None => tab.banner.clone(),
        };
        s.set_banner(ss(&banner));
        s.set_notice(ss(&tab.notice));
        s.set_has_domain(tab.loaded.is_some());
        s.set_running(tab.job.is_some());
        s.set_can_run(tab.loaded.is_some());
        s.set_last_summary(ss(&tab.last_summary));
        self.nw5_refresh_progress(tab);
        self.nw5_refresh_revisions(tab);
        self.nw5_refresh_tables(tab);
        self.nw5_refresh_network();
        self.nw5_refresh_text(tab);
        self.nw5_refresh_runs(tab);
        self.nw5_refresh_review(tab);
        self.nw5_refresh_comparison(tab);
    }

    pub fn nw5_refresh_progress(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        if let Some(j) = &tab.job {
            s.set_running(true);
            s.set_progress(if j.total > 0 {
                j.done as f32 / j.total as f32
            } else {
                0.0
            });
            s.set_run_status(ss(&format!(
                "{} · {} {}/{}",
                run_label(j.run_number),
                self.tr("Nw5.Running"),
                j.done,
                j.total
            )));
        }
    }

    fn nw5_refresh_revisions(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let active = tab.active_file();
        let revs: Vec<Nw5Revision> = tab
            .project
            .revisions()
            .iter()
            .map(|r| Nw5Revision {
                file: ss(&r.file),
                version: ss(&format!("v{}", r.version)),
                format: ss(r.format.as_str()),
                parent: ss(&match (&r.parent, &r.imported_from) {
                    (Some(p), _) => format!("{} {p}", self.tr("Nw5.Revision_of")),
                    (None, Some(f)) => format!("{} {f}", self.tr("Nw5.Imported_from")),
                    _ => String::new(),
                }),
                created: ss(&short_time(&r.created_at)),
                active: Some(&r.file) == active.as_ref(),
                archived: r.archived,
            })
            .collect();
        ui.global::<Nw5>().set_revisions(model(revs));
    }

    fn nw5_refresh_tables(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let Some(l) = &tab.loaded else {
            s.set_table_names(model(Vec::new()));
            s.set_table(empty_grid());
            return;
        };
        let tables = domain_tables(&l.network, tab.project.display_unit());
        s.set_table_names(model(
            tables
                .iter()
                .map(|t| ss(&format!("{} ({})", t.title, t.rows.len())))
                .collect(),
        ));
        let k = usize::try_from(s.get_table_index())
            .unwrap_or(0)
            .min(tables.len().saturating_sub(1));
        s.set_table_index(k as i32);
        let mut warnings = tab.warnings.clone();
        for w in &l.warnings {
            if !warnings.contains(w) {
                warnings.push(w.clone());
            }
        }
        s.set_table_warnings(ss(&warnings
            .iter()
            .map(|w| format!("• {w}"))
            .collect::<Vec<_>>()
            .join("\n")));
        let Some(t) = tables.get(k) else {
            s.set_table(empty_grid());
            return;
        };
        s.set_table_title(ss(&t.title));
        s.set_table_description(ss(&t.description));
        s.set_table_badges(ss(&t.badges));
        s.set_table_legend(t.legend);
        let rows = t
            .rows
            .iter()
            .map(|r| r.iter().map(|c| role_cell(&c.text, c.role)).collect())
            .collect();
        s.set_table(grid(&t.columns, rows, Vec::new()));
    }

    /// The network view (F19–F26).
    pub fn nw5_refresh_network(&self) {
        let (Some(ui), Some(tab)) = (self.ui(), self.nw5_cur()) else {
            return;
        };
        let s = ui.global::<Nw5>();
        let layout = nw5_layout_of(s.get_layout());
        let unit = tab.project.display_unit();
        let Some(g) = tab.graph_with_results(s.get_show_results()) else {
            s.set_nodes(model(Vec::new()));
            s.set_edges(model(Vec::new()));
            s.set_inspector_kind(0);
            return;
        };
        let pic = nw5_picture(&g, layout);
        let sel_node = match &tab.selection {
            Some(Selection::Node(id)) => Some(id.as_str()),
            _ => None,
        };
        let sel_edge = match &tab.selection {
            Some(Selection::Edge(id)) => Some(id.as_str()),
            _ => None,
        };
        let len = |m: f64| format!("{} {}", fixed(unit.from_meters(m), 2), unit.symbol());
        let nodes: Vec<Nw5Node> = g
            .nodes
            .iter()
            .zip(&pic.points)
            .map(|(n, p)| {
                let selected = sel_node == Some(n.id.as_str());
                let details = format!(
                    "z={}{}",
                    len(n.elevation_m),
                    n.pressure_m
                        .map(|p| format!(" · p={}", len(p)))
                        .unwrap_or_default()
                );
                Nw5Node {
                    x: p.0 as f32,
                    y: p.1 as f32,
                    label: ss(&n.label),
                    details: ss(&details),
                    color: nw5_node_color(n),
                    radius: nw5_node_radius(n, selected) as f32,
                    selected,
                    flash: tab.flash.as_deref() == Some(n.id.as_str()),
                }
            })
            .collect();
        let index = |id: &str| g.nodes.iter().position(|n| n.id == id);
        let edges: Vec<Nw5Edge> = g
            .edges
            .iter()
            .filter_map(|e| {
                let a = pic.points[index(&e.source_id)?];
                let b = pic.points[index(&e.target_id)?];
                let selected = sel_edge == Some(e.id.as_str());
                let (color, width) = nw5_edge_style(e, selected);
                let label = match (e.flow_lps, e.velocity_mps) {
                    (Some(q), _) => format!("{} · {} L/s", e.id, fixed(q, 2)),
                    (None, Some(v)) => format!(
                        "{} · {} {}",
                        e.id,
                        fixed(unit.from_meters(v), 2),
                        unit.velocity_symbol()
                    ),
                    _ => e.id.clone(),
                };
                Some(Nw5Edge {
                    x1: a.0 as f32,
                    y1: a.1 as f32,
                    x2: b.0 as f32,
                    y2: b.1 as f32,
                    color,
                    width: width as f32,
                    // PROPOSAL: a closed pipe gets the red valve marker too,
                    // so the closed state shows at every zoom.
                    valve: e.kind == EdgeKind::Valve || !e.open,
                    open: e.open,
                    selected,
                    label: ss(&label),
                })
            })
            .collect();
        s.set_net_width(pic.width as f32);
        s.set_net_height(pic.height as f32);
        s.set_label_zoom(pic.label_zoom as f32);
        s.set_nodes(model(nodes));
        s.set_edges(model(edges));
        let count = |k: NodeKind| g.nodes.iter().filter(|n| n.kind == k).count();
        let counters = format!(
            "{} {} · {} {} · {} {}",
            self.tr("Nw5.Sources"),
            count(NodeKind::Reservoir),
            self.tr("Nw5.Nodes"),
            g.nodes.len(),
            self.tr("Nw5.Pipes"),
            g.edges.len()
        );
        let (header, note) = match layout {
            Nw5Layout::Visual => (g.name.clone(), self.tr("Nw5.Visual_note")),
            Nw5Layout::Raw => (
                self.tr("Nw5.Raw_header"),
                if pic.raw_identical {
                    self.tr("Nw5.Raw_identical")
                } else {
                    self.tr("Nw5.Raw_note")
                },
            ),
            Nw5Layout::Linear => (self.tr("Nw5.Linear_header"), self.tr("Nw5.Linear_note")),
        };
        s.set_net_header(ss(&header));
        s.set_net_note(ss(&format!("{counters} · {note}")));
        let results = match tab.results_run {
            Some(n) => format!("{} {}", self.tr("Nw5.Results_of"), run_label(n)),
            None => self.tr("Nw5.No_results"),
        };
        s.set_results_label(ss(&results));
        let legend = [
            ("Nw5.Legend_reservoir", rgb(0x38, 0xbd, 0xf8)),
            ("Nw5.Legend_faucet", rgb(0xfb, 0x92, 0x3c)),
            ("Nw5.Legend_junction", rgb(0x94, 0xa3, 0xb8)),
            ("Nw5.Legend_high", rgb(0x22, 0xc5, 0x5e)),
            ("Nw5.Legend_medium", rgb(0xfa, 0xcc, 0x15)),
            ("Nw5.Legend_low", rgb(0xef, 0x44, 0x44)),
            ("Nw5.Legend_valve_open", rgb(0x22, 0xc5, 0x5e)),
            ("Nw5.Legend_valve_closed", rgb(0xef, 0x44, 0x44)),
            ("Nw5.Legend_flash", rgb(0xf4, 0x72, 0xb6)),
        ];
        s.set_legend_labels(model(legend.iter().map(|(k, _)| ss(&self.tr(k))).collect()));
        s.set_legend_colors(model(legend.iter().map(|(_, c)| *c).collect()));
        // inspector (F24)
        let opt = |x: Option<f64>, f: &dyn Fn(f64) -> String| x.map_or("-".to_string(), f);
        match &tab.selection {
            Some(Selection::Node(id)) => {
                let Some(n) = g.nodes.iter().find(|n| &n.id == id) else {
                    s.set_inspector_kind(0);
                    return;
                };
                let mut rows = vec![
                    pair("Type", n.kind.as_str()),
                    pair("X", &len(n.source_x)),
                    pair("Y", &len(n.source_y)),
                ];
                if n.layout_generated {
                    rows.push(pair("Visual X", &fixed(n.x, 1)));
                    rows.push(pair("Visual Y", &fixed(n.y, 1)));
                }
                rows.push(pair("Elevation", &len(n.elevation_m)));
                rows.push(pair("Pressure", &opt(n.pressure_m, &len)));
                if let Some(r) = n.required_pressure_m {
                    rows.push(pair("Required", &len(r)));
                }
                rows.push(pair(
                    "Status",
                    if n.low_pressure {
                        "LOW PRESSURE"
                    } else {
                        "NORMAL"
                    },
                ));
                s.set_inspector_kind(1);
                s.set_inspector_title(ss(&n.label));
                s.set_inspector_rows(model(rows));
            }
            Some(Selection::Edge(id)) => {
                let Some(e) = g.edges.iter().find(|e| &e.id == id) else {
                    s.set_inspector_kind(0);
                    return;
                };
                let not_simulated =
                    e.flow_lps.unwrap_or(0.0) == 0.0 && e.velocity_mps.unwrap_or(0.0) == 0.0;
                let two = |x: f64| fixed(x, 2);
                let mut rows = vec![
                    pair("Type", e.kind.as_str()),
                    pair("Status", if e.open { "OPEN" } else { "CLOSED" }),
                ];
                if not_simulated {
                    rows.push(pair("Flow", "Not simulated"));
                    rows.push(pair("Velocity", "Not simulated"));
                } else {
                    rows.push(pair(
                        "Flow",
                        &opt(e.flow_lps, &|q| format!("{} L/s", fixed(q, 2))),
                    ));
                    rows.push(pair(
                        "Velocity",
                        &opt(e.velocity_mps, &|v| {
                            format!(
                                "{} {}",
                                fixed(unit.from_meters(v), 2),
                                unit.velocity_symbol()
                            )
                        }),
                    ));
                }
                rows.push(pair("From", &e.source_id));
                rows.push(pair("To", &e.target_id));
                rows.push(pair("Length", &len(e.length_m)));
                rows.push(pair("Diameter ID", e.diameter_id.as_deref().unwrap_or("-")));
                rows.push(pair("Nominal", e.nominal_size.as_deref().unwrap_or("-")));
                rows.push(pair(
                    "Pipe type",
                    e.pipe_type_code.as_deref().unwrap_or("-"),
                ));
                rows.push(pair(
                    "SDR",
                    &e.sdr
                        .as_deref()
                        .and_then(|t| t.trim().parse::<f64>().ok())
                        .map_or("-".to_string(), two),
                ));
                rows.push(pair("Roughness", &opt(e.roughness, &two)));
                rows.push(pair(
                    "Inner",
                    &opt(e.inner_diameter, &|d| {
                        format!("{} {}", unit.from_meters(d), unit.symbol())
                    }),
                ));
                rows.push(pair("Cost", &opt(e.cost, &|c| fixed(c, 3))));
                rows.push(pair("pmax", &opt(e.pmax, &|p| fixed(p, 1))));
                s.set_inspector_kind(2);
                s.set_inspector_title(ss(&e.id));
                s.set_inspector_rows(model(rows));
                s.set_inspector_valve(e.kind == EdgeKind::Valve);
                s.set_inspector_open(e.open);
            }
            None => s.set_inspector_kind(0),
        }
    }

    pub fn nw5_refresh_text(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let Some(l) = &tab.loaded else {
            s.set_text_content(ss(""));
            return;
        };
        let (meta, desc, text) = match s.get_text_mode() {
            1 => (
                self.tr("Nw5.Parsed_json"),
                self.tr("Nw5.Json_description"),
                neatwork_nw5_io::write_domain_json(&l.network),
            ),
            2 => (
                "NW-json-v1".to_string(),
                self.tr("Nw5.Nw_json_description"),
                neatwork_nw5_io::write_nw_json(&l.network),
            ),
            _ => (
                format!(
                    "{} · {} · v{}",
                    l.revision.file,
                    l.revision.format.as_str(),
                    l.revision.version
                ),
                self.tr("Nw5.Source_description"),
                tab.project
                    .revision_text(&l.revision.file)
                    .unwrap_or_else(|e| e.to_string()),
            ),
        };
        s.set_text_meta(ss(&meta));
        s.set_text_description(ss(&desc));
        s.set_text_content(ss(&text));
    }

    fn nw5_refresh_runs(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let runs: Vec<Nw5Run> = tab
            .runs
            .iter()
            .rev()
            .map(|r| Nw5Run {
                number: r.run_number as i32,
                title: ss(&run_label(r.run_number)),
                status: ss(status_label(r.status)),
                created: ss(&short_time(&r.created_at)),
                input: ss(&format!(
                    "{} · v{} · {}",
                    r.revision,
                    r.revision_version,
                    r.source_format.as_str()
                )),
                mode: ss(r.configuration_mode.label()),
                scenarios: ss(&r
                    .scenario_count()
                    .map_or(MISSING.to_string(), |c| c.to_string())),
                seed: ss(&r
                    .effective_seed()
                    .map_or(MISSING.to_string(), |c| c.to_string())),
                error: ss(&r
                    .error
                    .as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_default()),
                completed: r.status.is_completed() && r.statistics.is_some(),
                terminal: r.status.is_terminal()
                    && tab.job.as_ref().map(|j| j.run_number) != Some(r.run_number),
            })
            .collect();
        ui.global::<Nw5>().set_runs(model(runs));
    }

    /// Review of one run (F42–F47).
    fn nw5_refresh_review(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let unit = tab.project.display_unit();
        let runs = tab.completed_runs();
        s.set_review_runs(model(runs.iter().map(|r| ss(&source_label(r))).collect()));
        let Some(k) = tab
            .review_run
            .and_then(|n| runs.iter().position(|r| r.run_number == n))
            .or_else(|| runs.len().checked_sub(1))
        else {
            s.set_review_empty(ss(&self.tr("Nw5.No_completed_simulation")));
            return;
        };
        s.set_review_run(k as i32);
        let rec = runs[k];
        let kind = TableKind::ALL
            .get(usize::try_from(s.get_review_kind()).unwrap_or(0))
            .copied()
            .unwrap_or(TableKind::TerminalSatisfaction);
        let Some(table) = result_table(rec, kind, unit) else {
            s.set_review_empty(ss(&self.tr("Nw5.No_result_tables")));
            return;
        };
        s.set_review_empty(ss(""));
        let sum = run_summary(rec);
        s.set_review_title(ss(&sum.title));
        s.set_review_badge(ss(sum.configuration));
        s.set_review_params(model(vec![
            pair("Bounded flow", &sum.bounded_flow),
            pair("Orifice model", &sum.orifice_model),
            pair("Target flow", &sum.target_flow),
            pair("Open faucets", &sum.open_faucets),
        ]));
        s.set_review_trace(model(
            sum.traceability.iter().map(|(k, v)| pair(k, v)).collect(),
        ));
        s.set_review_strip(model(sum.strip.iter().map(|(k, v)| pair(k, v)).collect()));
        s.set_review_failure_count(ss(&sum.failure_count_text()));
        let fh: Vec<String> = ["Scenario", "Stage", "Code", "Message"]
            .iter()
            .map(|x| (*x).to_string())
            .collect();
        s.set_review_failures(grid(
            &fh,
            sum.failures
                .iter()
                .map(|f| f.iter().map(|t| plain_cell(t)).collect())
                .collect(),
            Vec::new(),
        ));
        let details = s.get_review_details();
        let visible = table.visible_columns(details);
        let mut headers = vec!["#".to_string()];
        headers.extend(visible.iter().map(|&c| {
            format!(
                "{} {}",
                table.columns[c].header(),
                sort_indicator(tab.review_sort, c)
            )
        }));
        let order = table.sorted_rows(tab.review_sort, details);
        let rows = order
            .iter()
            .enumerate()
            .map(|(k, &r)| {
                let mut cells = vec![plain_cell(&(k + 1).to_string())];
                cells.extend(visible.iter().map(|&c| {
                    let cell = &table.rows[r][c];
                    role_cell(&cell.text, cell.role)
                }));
                cells
            })
            .collect();
        s.set_review_table(grid(&headers, rows, Vec::new()));
        s.set_review_table_title(ss(table.title()));
        s.set_review_table_description(ss(table.description()));
        s.set_review_row_count(ss(&table.row_count_text()));
    }

    /// The comparison of two runs (F48–F51).
    fn nw5_refresh_comparison(&self, tab: &Nw5Tab) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let unit = tab.project.display_unit();
        let active = tab.active_file();
        let sources = comparison_sources(&tab.runs, active.as_deref());
        s.set_cmp_sources(model(
            sources.iter().map(|r| ss(&source_label(r))).collect(),
        ));
        let Some((ln, rn)) = self.nw5_cmp_pair(tab) else {
            s.set_cmp_empty(ss(&self.tr("Nw5.Two_runs_required")));
            return;
        };
        let pos = |n: u32| sources.iter().position(|r| r.run_number == n);
        let (Some(li), Some(ri)) = (pos(ln), pos(rn)) else {
            s.set_cmp_empty(ss(&self.tr("Nw5.Two_runs_required")));
            return;
        };
        s.set_cmp_left(li as i32);
        s.set_cmp_right(ri as i32);
        let (left, right) = (sources[li], sources[ri]);
        let tables = common_tables(left, right);
        if tables.is_empty() {
            s.set_cmp_empty(ss(&self.tr("Nw5.No_common_table")));
            return;
        }
        s.set_cmp_empty(ss(""));
        s.set_cmp_tables(model(tables.iter().map(|t| ss(t.title())).collect()));
        let kind = tab
            .cmp_table
            .filter(|t| tables.contains(t))
            .unwrap_or(tables[0]);
        s.set_cmp_table(tables.iter().position(|t| *t == kind).unwrap_or(0) as i32);
        // compatibility (F49)
        let compat = match &tab.compat {
            Some((key, c)) if *key == (ln, rn) => c.as_ref().ok(),
            _ => None,
        };
        match (&tab.compat, compat) {
            (_, Some(c)) => {
                let (title, text) = c.level.banner();
                s.set_cmp_banner_title(ss(title));
                s.set_cmp_banner_text(ss(text));
                s.set_cmp_banner_color(match c.level {
                    CompatibilityLevel::SameSnapshot | CompatibilityLevel::EquivalentInput => {
                        rgb(0xdc, 0xfc, 0xe7)
                    }
                    CompatibilityLevel::SameTopology => rgb(0xfe, 0xf9, 0xc3),
                    CompatibilityLevel::TopologyChanged => rgb(0xfe, 0xe2, 0xe2),
                });
                s.set_cmp_warnings(ss(&c.warnings.join("\n")));
                let changes = if c.level == CompatibilityLevel::TopologyChanged {
                    let ch = &c.changes;
                    let mut lines = ch.lines();
                    for (label, list) in [
                        ("Added nodes", &ch.added_nodes),
                        ("Removed nodes", &ch.removed_nodes),
                        ("Changed roles", &ch.changed_roles),
                        ("Added pipes", &ch.added_pipes),
                        ("Removed pipes", &ch.removed_pipes),
                        ("Changed endpoints", &ch.changed_endpoints),
                    ] {
                        if !list.is_empty() {
                            lines.push(format!("{label}: {}", list.join(", ")));
                        }
                    }
                    lines.join("\n")
                } else {
                    String::new()
                };
                s.set_cmp_changes(ss(&changes));
            }
            (Some((_, Err(e))), _) => {
                s.set_cmp_banner_title(ss(&self.tr("Nw5.Source_error")));
                s.set_cmp_banner_text(ss(e));
                s.set_cmp_banner_color(rgb(0xfe, 0xe2, 0xe2));
                s.set_cmp_warnings(ss(""));
                s.set_cmp_changes(ss(""));
            }
            _ => {
                s.set_cmp_banner_title(ss(&self.tr("Nw5.Checking_compatibility")));
                s.set_cmp_banner_text(ss(""));
                s.set_cmp_banner_color(rgb(0xee, 0xee, 0xee));
                s.set_cmp_warnings(ss(""));
                s.set_cmp_changes(ss(""));
            }
        }
        let card = |r: &RunRecord| {
            let sum = run_summary(r);
            model(vec![
                pair("Simulation", &sum.title),
                pair(
                    "Input",
                    &format!("{} · v{}", r.revision, r.revision_version),
                ),
                pair("Configuration", sum.configuration),
                pair("Bounded flow", &sum.bounded_flow),
                pair("Orifice model", &sum.orifice_model),
                pair("Target flow", &sum.target_flow),
                pair("Open faucets", &sum.open_faucets),
            ])
        };
        s.set_cmp_left_card(card(left));
        s.set_cmp_right_card(card(right));
        // unified table (F50)
        let t = unified_table(left, right, kind, compat, unit);
        s.set_cmp_counters(ss(&format!(
            "Matched {} · Left only {} · Right only {}",
            t.matched, t.left_only, t.right_only
        )));
        let sort = tab
            .cmp_sort
            .and_then(|(c, d)| t.headers.get(c).map(|h| (&h.target, d)));
        let headers: Vec<String> = t
            .headers
            .iter()
            .enumerate()
            .map(|(k, h)| {
                let ind = match tab.cmp_sort {
                    Some((c, d)) if c == k => {
                        if d == SortDirection::Ascending {
                            "↑"
                        } else {
                            "↓"
                        }
                    }
                    _ => "↕",
                };
                format!("{} {ind}", h.label)
            })
            .collect();
        let (mut rows, mut tints) = (Vec::new(), Vec::new());
        for &i in &t.sorted_rows(sort) {
            let r = &t.rows[i];
            let (pill_bg, tint) = match r.status {
                MatchStatus::Matched => (
                    Color::from_argb_u8(26, 34, 197, 94),
                    Color::from_argb_u8(0, 0, 0, 0),
                ),
                MatchStatus::LeftOnly => (
                    Color::from_argb_u8(26, 245, 158, 11),
                    Color::from_argb_u8(6, 245, 158, 11),
                ),
                MatchStatus::RightOnly => (
                    Color::from_argb_u8(26, 59, 130, 246),
                    Color::from_argb_u8(6, 59, 130, 246),
                ),
            };
            let mut cells = vec![Nw5Cell {
                text: ss(&format!("{}  [{}]", r.identity, r.status.label())),
                fg: role_colors(r.role).0,
                bg: pill_bg,
            }];
            for c in &r.shared {
                let mut cell = role_cell(&c.text, c.role);
                if c.mismatch {
                    cell.bg = Color::from_argb_u8(40, 245, 158, 11);
                }
                cells.push(cell);
            }
            cells.extend(r.left.iter().map(|v| plain_cell(v)));
            cells.extend(r.right.iter().map(|v| plain_cell(v)));
            rows.push(cells);
            tints.push(tint);
        }
        s.set_cmp_unified(grid(&headers, rows, tints));
        // numeric panel (F51)
        let strategies = match_strategies(kind, kind);
        let metrics = comparable_columns(kind);
        s.set_cmp_strategies(model(strategies.iter().map(|m| ss(&m.label)).collect()));
        s.set_cmp_metrics(model(
            metrics.iter().map(|c| ss(&column_label(c, unit))).collect(),
        ));
        if strategies.is_empty() || metrics.is_empty() {
            s.set_cmp_metric_summary(ss(&self.tr("Nw5.No_identifier")));
            s.set_cmp_metric_warning(ss(""));
            s.set_cmp_metric_table(empty_grid());
            return;
        }
        let (si, lm, rm) = tab.cmp_metric.unwrap_or_else(|| {
            let (lk, rk) = recommended_pair(kind, kind).unwrap_or((metrics[0].key, metrics[0].key));
            let at = |k: &str| metrics.iter().position(|c| c.key == k).unwrap_or(0);
            (0, at(lk), at(rk))
        });
        let si = si.min(strategies.len() - 1);
        let (lm, rm) = (lm.min(metrics.len() - 1), rm.min(metrics.len() - 1));
        s.set_cmp_strategy(si as i32);
        s.set_cmp_left_metric(lm as i32);
        s.set_cmp_right_metric(rm as i32);
        let Some(m) = compare_run_metrics(
            left,
            right,
            kind,
            metrics[lm].key,
            metrics[rm].key,
            &strategies[si],
        ) else {
            s.set_cmp_metric_table(empty_grid());
            return;
        };
        s.set_cmp_metric_summary(ss(&m.summary().join(" · ")));
        s.set_cmp_metric_warning(ss(m.warning().unwrap_or("")));
        let mt = metric_table(&m, unit);
        let sign_color = |x: Sign| match x {
            Sign::Positive => rgb(0x6e, 0xe7, 0xb7),
            Sign::Negative => rgb(0xfc, 0xa5, 0xa5),
            Sign::Neutral => text_color(),
        };
        let rows = if mt.rows.is_empty() {
            vec![vec![plain_cell(mt.empty_text.unwrap_or(""))]]
        } else {
            mt.rows
                .iter()
                .map(|(cells, d, c)| {
                    cells
                        .iter()
                        .enumerate()
                        .map(|(k, t)| {
                            let mut cell = plain_cell(t);
                            if k == 3 {
                                cell.fg = sign_color(*d);
                            }
                            if k == 5 {
                                cell.fg = sign_color(*c);
                            }
                            cell
                        })
                        .collect()
                })
                .collect()
        };
        s.set_cmp_metric_table(grid(&mt.headers, rows, Vec::new()));
    }

    /// Left and right runs of the comparison: the user choice, else the
    /// saved workspace or the two newest runs (D5).
    pub fn nw5_cmp_pair(&self, tab: &Nw5Tab) -> Option<(u32, u32)> {
        let completed = tab.completed_runs();
        let exists = |n: u32| completed.iter().any(|r| r.run_number == n);
        if let Some((l, r)) = tab.cmp_pair
            && exists(l)
            && exists(r)
        {
            return Some((l, r));
        }
        let saved = tab
            .project
            .comparison_workspace()
            .map(|c| (c.left, c.right));
        default_pair(&tab.runs, saved)
    }

    /// Text of the execution details (F41).
    pub fn nw5_details_text(&self, rec: &RunRecord, unit: DisplayUnit) -> (String, String) {
        let d = execution_details(rec, unit);
        let mut out = String::new();
        let section = |out: &mut String, title: &str| {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(title);
            out.push('\n');
        };
        section(&mut out, "Input data");
        for (k, v) in &d.input {
            out.push_str(&format!("  {k}: {v}\n"));
        }
        section(&mut out, "Execution");
        for (k, v) in &d.execution {
            out.push_str(&format!("  {k}: {v}\n"));
        }
        section(&mut out, "Requested parameters");
        if let Some(t) = d.requested_empty {
            out.push_str(&format!("  {t}\n"));
        }
        for (k, v) in &d.requested {
            out.push_str(&format!("  {k} = {v}\n"));
        }
        section(&mut out, "Resolved parameters");
        if let Some(t) = d.resolved_empty {
            out.push_str(&format!("  {t}\n"));
        }
        for (k, v, src) in &d.resolved {
            out.push_str(&format!("  {k} = {v}   [{src}]\n"));
        }
        section(&mut out, &d.pipe_state_title);
        for (id, st) in &d.pipe_states {
            out.push_str(&format!("  {id} {st}\n"));
        }
        if let Some(e) = &d.error {
            section(&mut out, "Error");
            out.push_str(&format!("  {e}\n"));
        }
        (d.title, out)
    }

    /// Rows of the open-project dialog (F5).
    pub fn nw5_refresh_open_list(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let q = s.get_op_search().to_lowercase();
        let all = Project::list(&self.nw5.dir);
        let list: Vec<ProjectEntry> = all
            .iter()
            .filter(|e| q.trim().is_empty() || e.name.to_lowercase().contains(q.trim()))
            .cloned()
            .collect();
        s.set_op_empty(ss(&if all.is_empty() {
            self.tr("Nw5.No_projects")
        } else if list.is_empty() {
            self.tr("Nw5.No_project_match")
        } else {
            String::new()
        }));
        let items = list
            .iter()
            .map(|e| Nw5ProjectItem {
                name: ss(&e.name),
                created: ss(&short_time(&e.created_at)),
                open: self.nw5.tabs.iter().any(|t| t.project.root() == e.path),
            })
            .collect();
        s.set_op_items(model(items));
        self.nw5.dialogs.open_list = list;
    }
}

pub fn nw5_layout_of(i: i32) -> Nw5Layout {
    match i {
        1 => Nw5Layout::Raw,
        2 => Nw5Layout::Linear,
        _ => Nw5Layout::Visual,
    }
}
