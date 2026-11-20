//! Adapter between the windows and `neatwork_core::gui_support` (the logic
//! of the Java interface: tree view, statistics, reports, names).

use std::collections::HashMap;

use neatwork_core::gui_support::{self as gs, TreeCellKind, TreeView};
use neatwork_core::java::double_to_string;
use neatwork_core::project::network::Network;
use neatwork_core::project::{Design, SimulationFile, Topographie};
use slint::SharedString;

use crate::TreeCell;
use crate::app::{App, OpenProject, Proj};
use crate::i18n::strip_html;
use crate::table::{Col, Kind, Table};

pub use gs::{already_loaded_name, first_compatible_name, new_node_name};

pub fn is_tree(n: &Network) -> bool {
    gs::is_a_tree(n)
}

pub fn min_length(n: &Network) -> f64 {
    gs::network_stats(n).min_length
}

pub fn extract_topo_content(d: &mut Design) -> String {
    gs::extract_topo_content(d)
}

/// "Design total cost: 12,345" (`DesignTablePane`).
pub fn cost_label(app: &App, d: &Design) -> String {
    match gs::design_cost(d) {
        Ok(c) => app.tr("DesignTablePane.Design_total_cost") + " " + &gs::format_grouped(c),
        Err(e) => e.to_string(),
    }
}

/// Date as Java's `Date.toString()` writes it (in UTC).
pub fn java_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = (secs / 86_400) as i64;
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    // civil date from days since 1970-01-01 (H. Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let wd = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][days.rem_euclid(7) as usize];
    let mn = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1) as usize];
    format!("{wd} {mn} {day:02} {h:02}:{m:02}:{s:02} UTC {year}")
}

pub fn topo_report(_app: &App, t: &Topographie) -> String {
    gs::topography_report(t, &java_date())
}

pub fn design_report(_app: &App, d: &Design) -> String {
    gs::design_report(d, &java_date()).unwrap_or_else(|e| e.to_string())
}

pub fn simulation_report(_app: &App, d: &Design, f: &SimulationFile) -> String {
    gs::simulation_report(f, &d.name)
}

pub fn database_report(app: &App, diameters: bool) -> String {
    if diameters {
        gs::database_diameters_report(&app.db, &java_date())
    } else {
        gs::database_orifices_report(&app.db, &java_date())
    }
}

/// Text of the "Topography information" dialog.
pub fn topo_summary(_app: &App, t: &Topographie) -> String {
    strip_html(&gs::topo_stat_html(&t.name, &gs::network_stats(&t.network)))
        .trim()
        .to_string()
}

/// Text of the About dialog.
pub fn about_text(app: &App) -> String {
    let mut s = app.tr("AboutDialog.APLV") + "\n\n";
    for k in 1..=5 {
        s.push_str(&app.tr(&format!("PresentationPanel.text{k}")));
        s.push_str("\n\n");
    }
    let props = app.app_properties();
    s.push_str(&format!(
        "version: {} ({})\nRust port, based on NeatWork 2024.",
        props.get_property("appli.version").unwrap_or(""),
        props.get_property("appli.releasedate").unwrap_or("")
    ));
    s
}

/// Quick check table.
pub fn quick_check_table(app: &App, t: &Topographie) -> Table {
    match gs::quick_check(t) {
        Ok(q) => {
            let mut tb = Table::new(
                q.headers
                    .iter()
                    .map(|h| Col::new(h.clone(), 150.0, false, Kind::Text))
                    .collect(),
                false,
            );
            let ok = app.tr("QuickCheckTableModel.Ok");
            let no = app.tr("QuickCheckTableModel.No");
            tb.set_rows(
                q.rows
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|c| match c.as_str() {
                                "Ok" => ok.clone(),
                                "No" => no.clone(),
                                _ => c.clone(),
                            })
                            .collect()
                    })
                    .collect(),
            );
            tb
        }
        Err(e) => {
            let mut tb = Table::new(
                vec![Col::new(app.tr("Gui.Error"), 400.0, false, Kind::Text)],
                false,
            );
            tb.set_rows(vec![vec![e.to_string()]]);
            tb
        }
    }
}

/// Tree view of a network.
pub struct TreeViewData {
    pub message: String,
    pub rows: i32,
    pub cols: i32,
    pub cells: Vec<TreeCell>,
}

pub fn tree_layout(name: &str, n: &Network) -> Option<gs::TreeLayout> {
    match gs::build_tree(name, n) {
        Ok(TreeView::Tree(l)) => Some(l),
        _ => None,
    }
}

pub fn tree_view(app: &App, name: &str, n: &Network, selected: Option<&str>) -> TreeViewData {
    let path: Vec<String> = selected
        .map(|s| gs::path_to_source(n, s))
        .unwrap_or_default();
    match gs::build_tree(name, n) {
        Ok(TreeView::Tree(l)) => TreeViewData {
            message: String::new(),
            rows: l.rows as i32,
            cols: l.cols as i32,
            cells: l
                .cells
                .iter()
                .map(|c| {
                    let (kind, label, t) = match &c.kind {
                        TreeCellKind::Node { name, node_type } => {
                            (0, name.clone(), node_type.unwrap_or(1))
                        }
                        TreeCellKind::LineL => (1, String::new(), 0),
                        TreeCellKind::LineI => (2, String::new(), 0),
                        TreeCellKind::LineT => (3, String::new(), 0),
                    };
                    TreeCell {
                        row: c.row as i32,
                        col: c.col as i32,
                        kind,
                        highlight: kind == 0 && path.contains(&label),
                        label: SharedString::from(label),
                        node_type: t,
                    }
                })
                .collect(),
        },
        Ok(TreeView::NotATree(_)) => TreeViewData {
            message: name.to_string() + &app.msgs.raw(app.lang, "TreeNetwork._is_not_a_tree"),
            rows: 0,
            cols: 0,
            cells: Vec::new(),
        },
        Err(e) => TreeViewData {
            message: e.to_string(),
            rows: 0,
            cols: 0,
            cells: Vec::new(),
        },
    }
}

/// Name of a node in the simulation tables: `P2_a` is written `P2_1`.
pub fn sim_tap_name(name: &str) -> String {
    match name.rfind('_').filter(|k| *k > 0) {
        Some(k) => match name[k + 1..].encode_utf16().next() {
            Some(c) => format!("{}{}", &name[..=k], i32::from(c) - i32::from(b'a') + 1),
            None => name.to_string(),
        },
        None => name.to_string(),
    }
}

/// Details of a node for the drawing side panel.
pub fn node_info(
    app: &App,
    p: &OpenProject,
    name: &str,
    flows: &HashMap<String, f64>,
    pressures: &HashMap<String, f64>,
) -> String {
    let Some(n) = p.network().nodes.iter().find(|n| n.name == name) else {
        return String::new();
    };
    let mut s = format!(
        "{}\n{}: {}\n{}: {} m\n",
        n.name,
        app.tr("TopoNodeTableModel._Nature").trim(),
        app.node_type_name(&n.node_type.to_string()),
        app.tr("TopoNodeTableModel._Height").trim(),
        double_to_string(n.height)
    );
    if let Proj::Design(_) = &p.proj
        && n.node_type == 2
    {
        s.push_str(&format!(
            "{}: {}\n{}: {}\n",
            app.tr("DesignNodeModel.Ideal_Orifice"),
            double_to_string(n.orifice),
            app.tr("DesignNodeModel.Commercial_Orifice"),
            double_to_string(n.orifice_commercial)
        ));
    }
    if let Some(v) = flows.get(&sim_tap_name(name)) {
        s.push_str(&format!(
            "{}: {} l/s\n",
            app.tr("SimuTableModel.Average"),
            double_to_string(*v)
        ));
    }
    if let Some(v) = pressures.get(name) {
        s.push_str(&format!(
            "{}: {} m\n",
            app.tr("Gui.Color_pressure"),
            double_to_string(*v)
        ));
    }
    s
}

/// Parameter text of the simulation pane.
pub fn sim_params_text(app: &App, f: &SimulationFile) -> String {
    let p = |k: &str| f.property(k).unwrap_or("").to_string();
    let orif = if p("typeorifice") == "commercial" {
        "NewSimuDialog.commercial"
    } else {
        "NewSimuDialog.ideal"
    };
    let kind = match p("typesimu").as_str() {
        "random" => "NewSimuDialog.monte-carlo_sampling",
        "tapbytap" => "NewSimuDialog.individual_faucets",
        _ => "NewSimuDialog.user-defined",
    };
    format!(
        "{}:\t{}\n{}:\t{}\n{}:\t{}\n{}:\t{}",
        app.tr("simu.nbsim.name"),
        p("nbsim"),
        app.tr("simu.simopentaps.name"),
        p("simopentaps"),
        app.tr("simu.typeorifice.name"),
        app.tr(orif),
        app.tr("simu.typesimu.name"),
        app.tr(kind)
    )
}

/// Result table of the simulation pane (`SimuTableModel`).
pub fn sim_table(app: &App, f: &SimulationFile, kind: i32) -> Table {
    let col = |k: &str, w: f32, kind: Kind| Col::new(app.tr(k), w, false, kind);
    let cells = |rows: Vec<Vec<neatwork_core::project::Cell>>| -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(App::cell).collect())
            .collect()
    };
    let (cols, rows) = match kind {
        2 => (
            vec![
                col("SimuTableModel.Faucet_ID", 90.0, Kind::Text),
                col("SimuTableModel._#_of_occurences", 110.0, Kind::Text),
                col("SimuTableModel.Min", 70.0, Kind::Text),
                col("SimuTableModel.<10%", 70.0, Kind::Text),
                col("SimuTableModel.<25%", 70.0, Kind::Text),
                col("SimuTableModel.<50%", 70.0, Kind::Text),
                col("SimuTableModel.<75%", 70.0, Kind::Text),
                col("SimuTableModel.<90%", 70.0, Kind::Text),
                col("SimuTableModel.Max", 70.0, Kind::Text),
            ],
            cells(f.quartile_taps()),
        ),
        3 => {
            // The Java table shows "-" in every cell (SimuTableModel).
            let n = f.speed_pipes().len();
            (
                vec![
                    col("SimuTableModel.Pipe_ID", 110.0, Kind::Text),
                    col("SimuTableModel._#_of_simulation", 110.0, Kind::Text),
                    col("SimuTableModel.Average", 90.0, Kind::Text),
                    col("SimuTableModel.Maximum", 90.0, Kind::Text),
                ],
                vec![vec!["-".to_string(); 4]; n],
            )
        }
        4 => (
            vec![
                col("SimuTableModel.Node_ID", 110.0, Kind::Text),
                col("SimuTableModel.Minimum", 90.0, Kind::Text),
                col("SimuTableModel.Average", 90.0, Kind::Text),
                col("SimuTableModel.Maximum", 90.0, Kind::Text),
            ],
            cells(f.node_pressures()),
        ),
        _ => {
            let low = f.property("mincriticalflow").unwrap_or("");
            let high = f.property("maxcriticalflow").unwrap_or("");
            (
                vec![
                    col("SimuTableModel.Faucet_ID", 110.0, Kind::Text),
                    col("SimuTableModel._#_of_occurences", 110.0, Kind::Text),
                    col("SimuTableModel.Min", 70.0, Kind::Text),
                    col("SimuTableModel.Average", 70.0, Kind::Text),
                    col("SimuTableModel.Max", 70.0, Kind::Text),
                    col("SimuTableModel.Variability", 90.0, Kind::Bar),
                    Col::new(format!("< {low}"), 80.0, false, Kind::Bar),
                    Col::new(format!("> {high}"), 80.0, false, Kind::Bar),
                    col("SimuTableModel._#_of_failures", 100.0, Kind::Text),
                ],
                cells(f.flow_taps()),
            )
        }
    };
    let mut t = Table::new(cols, false);
    t.set_rows(rows);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_names() {
        assert_eq!(sim_tap_name("P2_a"), "P2_1");
        assert_eq!(sim_tap_name("P2_c"), "P2_3");
        assert_eq!(sim_tap_name("P4"), "P4");
    }

    #[test]
    fn date_shape() {
        let d = java_date();
        assert_eq!(d.split(' ').count(), 6);
    }
}
