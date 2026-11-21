//! What the NW5 menus, buttons and dialogs do: projects, import,
//! revisions, network edits, design, runs on a background thread, export,
//! review and comparison (`rust/nw5/specs/app.md`).

use std::path::{Path, PathBuf};

use neatwork_core::project::design::{DEFAULT_DIAMETERS, DEFAULT_ORIFICES};
use neatwork_nw5_design::{DesignOptions, LoadMethod, RiskMeasure, Stage, design_with_progress};
use neatwork_nw5_project::project::RevisionExport;
use neatwork_nw5_project::review::{SortDirection, TableKind, next_sort};
use neatwork_nw5_project::revision::suggested_custom_name;
use neatwork_nw5_project::run::orifice_mode_label;
use neatwork_nw5_project::{
    CancelFlag, DisplayUnit, ImportOptions, Project, RequestedOptions, RevisionName, RunRecord,
};
use neatwork_nw5_sim::OrificeMode;
use slint::ComponentHandle;

use crate::app::{App, Finished, Pending};
use crate::drawing::nw5_picture;
use crate::nw5::{ActiveRun, DesignOutcome, Nw5Tab, Selection, model, nw5_layout_of, run_line, ss};
use crate::{MainWindow, Nw5, Nw5Option, Nw5Pair, idx, with_app};

/// Common design parameters offered as overrides (`DesignOptions::params`).
const DESIGN_PARAMS: [&str; 10] = [
    "servicequal",
    "opentaps",
    "targetflow",
    "pressure",
    "vmax",
    "secu",
    "limitbudget",
    "pipelength",
    "nScenLoad",
    "use_cvar",
];

/// Connect the callbacks of the `Nw5` global.
pub fn connect(ui: &MainWindow) {
    let s = ui.global::<Nw5>();
    s.on_import_network(|| with_app(|app| app.nw5_import_dialog()));
    s.on_design(|| with_app(|app| app.nw5_design_dialog()));
    s.on_configure(|| with_app(|app| app.nw5_configure()));
    s.on_run(|| with_app(|app| app.nw5_run()));
    s.on_cancel_run(|| with_app(|app| app.nw5_cancel_run()));
    s.on_revision_activate(|i| with_app(|app| app.nw5_revision_activate(idx(i))));
    s.on_revision_archive(|i| with_app(|app| app.nw5_revision_archive(idx(i))));
    s.on_revision_export(|i, f| with_app(|app| app.nw5_revision_export(idx(i), &f, None)));
    s.on_table_select(|i| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_table_index(i);
            }
            app.refresh_nw5();
        });
    });
    s.on_node_clicked(|i| with_app(|app| app.nw5_node_clicked(idx(i))));
    s.on_canvas_clicked(|x, y| with_app(|app| app.nw5_canvas_clicked(f64::from(x), f64::from(y))));
    s.on_node_dragged(|i, dx, dy| {
        with_app(|app| app.nw5_node_dragged(idx(i), f64::from(dx), f64::from(dy)))
    });
    s.on_toggle_valve(|| with_app(|app| app.nw5_toggle_valve()));
    s.on_save_position(|| with_app(|app| app.nw5_save_position()));
    s.on_close_inspector(|| {
        with_app(|app| {
            if let Some(t) = app.nw5_cur_mut() {
                t.selection = None;
            }
            app.nw5_refresh_network();
        });
    });
    s.on_search(|q| with_app(|app| app.nw5_search(&q)));
    s.on_view_changed(|| with_app(|app| app.nw5_refresh_network()));
    s.on_text_mode_changed(|| {
        with_app(|app| {
            if let Some(t) = app.nw5_cur() {
                app.nw5_refresh_text(t);
            }
        });
    });
    s.on_run_review(|n| with_app(|app| app.nw5_review(n as u32)));
    s.on_run_details(|n| with_app(|app| app.nw5_details(n as u32)));
    s.on_run_delete(|n| with_app(|app| app.nw5_run_delete(n as u32)));
    s.on_run_export(|n| with_app(|app| app.nw5_run_export(n as u32, None)));
    s.on_review_changed(|| with_app(|app| app.nw5_review_changed()));
    s.on_review_sort(|c| with_app(|app| app.nw5_review_sort(idx(c))));
    s.on_cmp_changed(|| with_app(|app| app.nw5_cmp_changed()));
    s.on_cmp_sort(|c| with_app(|app| app.nw5_cmp_sort(idx(c))));
    s.on_cmp_metric_changed(|| with_app(|app| app.nw5_cmp_metric_changed()));
    // dialogs
    s.on_np_create(|| with_app(|app| app.nw5_np_create()));
    s.on_np_cancel(|| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_np_visible(false);
            }
        });
    });
    s.on_op_search_changed(|| with_app(|app| app.nw5_refresh_open_list()));
    s.on_op_open(|i| with_app(|app| app.nw5_op_open(idx(i))));
    s.on_op_delete(|i| with_app(|app| app.nw5_op_delete(idx(i))));
    s.on_op_close(|| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_op_visible(false);
            }
        });
    });
    s.on_dp_confirm(|| with_app(|app| app.nw5_dp_confirm()));
    s.on_dp_cancel(|| {
        with_app(|app| {
            app.nw5.dialogs.delete_target = None;
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_dp_visible(false);
            }
        });
    });
    s.on_tc_browse_diameters(|| with_app(|app| app.nw5_tc_browse(true)));
    s.on_tc_browse_orifices(|| with_app(|app| app.nw5_tc_browse(false)));
    s.on_tc_import(|| with_app(|app| app.nw5_tc_import()));
    s.on_tc_cancel(|| {
        with_app(|app| {
            app.nw5.dialogs.tpo_path = None;
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_tc_visible(false);
            }
        });
    });
    s.on_dd_param_edited(|i, t| {
        with_app(|app| {
            if let Some(p) = app.nw5.dialogs.design_params.get_mut(idx(i)) {
                p.2 = t.to_string();
            }
        });
    });
    s.on_dd_run(|| with_app(|app| app.nw5_dd_run()));
    s.on_dd_cancel(|| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_dd_visible(false);
            }
        });
    });
    s.on_dr_save(|| with_app(|app| app.nw5_dr_save()));
    s.on_dr_discard(|| {
        with_app(|app| {
            app.nw5.dialogs.design_result = None;
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_dr_visible(false);
            }
        });
    });
    s.on_sc_apply(|run| with_app(|app| app.nw5_sc_apply(run)));
    s.on_sc_reset(|| {
        with_app(|app| {
            if let Some(t) = app.nw5_cur_mut() {
                t.requested = RequestedOptions::default();
            }
            app.nw5_configure();
        });
    });
    s.on_sc_cancel(|| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_sc_visible(false);
            }
        });
    });
    s.on_un_apply(|| with_app(|app| app.nw5_units_apply()));
    s.on_un_cancel(|| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_un_visible(false);
            }
        });
    });
    s.on_rd_close(|| {
        with_app(|app| {
            if let Some(ui) = app.ui() {
                ui.global::<Nw5>().set_rd_visible(false);
            }
        });
    });
}

fn parse_number(text: &str) -> Option<f64> {
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn parse_count(text: &str) -> Option<usize> {
    text.trim().parse::<usize>().ok().filter(|v| *v >= 1)
}

fn save_dialog(name: &str, ext: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(ext.to_ascii_uppercase(), &[ext])
        .set_file_name(name)
        .save_file()
}

impl App {
    fn nw5_error(&mut self, text: &str) {
        let title = self.tr("Gui.Error");
        self.message(&title, text, 0, Pending::None);
    }

    /// Index of the current NW5 tab.
    fn nw5_index(&self) -> Option<usize> {
        if self.current.is_some() {
            return None;
        }
        self.nw5.current.filter(|k| *k < self.nw5.tabs.len())
    }

    /// Menu actions `nw5-*`.
    pub fn nw5_menu(&mut self, action: &str) {
        match action {
            "nw5-new" => {
                if let Some(ui) = self.ui() {
                    let s = ui.global::<Nw5>();
                    s.set_np_name(ss(""));
                    s.set_np_error(ss(""));
                    s.set_np_visible(true);
                }
            }
            "nw5-open" => {
                self.nw5_refresh_open_list();
                if let Some(ui) = self.ui() {
                    ui.global::<Nw5>().set_op_visible(true);
                }
            }
            "nw5-import" => self.nw5_import_dialog(),
            "nw5-delete" => {
                if let Some(t) = self.nw5_cur() {
                    let (root, name) =
                        (t.project.root().to_path_buf(), t.project.name().to_string());
                    self.nw5_ask_delete(root, &name);
                }
            }
            "nw5-units" => {
                if let (Some(ui), Some(t)) = (self.ui(), self.nw5_cur()) {
                    let s = ui.global::<Nw5>();
                    let k = DisplayUnit::ALL
                        .iter()
                        .position(|u| *u == t.project.display_unit())
                        .unwrap_or(0) as i32;
                    s.set_un_current(k);
                    s.set_un_choice(k);
                    s.set_un_visible(true);
                }
            }
            _ => {}
        }
    }

    // ============================================================ projects

    /// Add an opened or created project as a tab and show it.
    pub fn nw5_add_tab(&mut self, project: Project) {
        let id = self.nw5.next_id;
        self.nw5.next_id += 1;
        self.nw5.tabs.push(Nw5Tab::new(id, project));
        self.nw5.current = Some(self.nw5.tabs.len() - 1);
        self.current = None;
        if let Some(ui) = self.ui() {
            ui.global::<Nw5>().set_pane_tab(0);
        }
        self.refresh();
    }

    pub fn nw5_np_create(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        match Project::create(&self.nw5.dir, &s.get_np_name()) {
            Ok(p) => {
                s.set_np_visible(false);
                self.nw5_add_tab(p);
            }
            Err(e) => s.set_np_error(ss(&e.to_string())),
        }
    }

    /// Open a project folder, or show its tab when it is open (F5).
    pub fn nw5_open_path(&mut self, path: &Path) {
        if let Some(k) = self.nw5.tabs.iter().position(|t| t.project.root() == path) {
            self.nw5.current = Some(k);
            self.current = None;
            self.refresh();
            return;
        }
        match Project::open(path) {
            Ok(p) => self.nw5_add_tab(p),
            Err(e) => self.nw5_error(&e.to_string()),
        }
    }

    fn nw5_op_open(&mut self, i: usize) {
        let Some(e) = self.nw5.dialogs.open_list.get(i).cloned() else {
            return;
        };
        if let Some(ui) = self.ui() {
            ui.global::<Nw5>().set_op_visible(false);
        }
        self.nw5_open_path(&e.path);
    }

    fn nw5_op_delete(&mut self, i: usize) {
        if let Some(e) = self.nw5.dialogs.open_list.get(i).cloned() {
            self.nw5_ask_delete(e.path, &e.name);
        }
    }

    fn nw5_ask_delete(&mut self, root: PathBuf, name: &str) {
        self.nw5.dialogs.delete_target = Some(root);
        if let Some(ui) = self.ui() {
            let s = ui.global::<Nw5>();
            s.set_dp_name(ss(name));
            s.set_dp_checked(false);
            s.set_dp_error(ss(""));
            s.set_dp_visible(true);
        }
    }

    /// Delete a project folder after the confirmation (F6).
    fn nw5_dp_confirm(&mut self) {
        let Some(root) = self.nw5.dialogs.delete_target.clone() else {
            return;
        };
        let result = match self.nw5.tabs.iter().position(|t| t.project.root() == root) {
            Some(k) if self.nw5.tabs[k].job.is_some() => Err(self.tr("Nw5.Cancel_before_delete")),
            Some(k) => {
                let tab = self.nw5.tabs.remove(k);
                self.nw5.current = match self.nw5.current {
                    Some(c) if c > k => Some(c - 1),
                    Some(c) if c == k => None,
                    c => c,
                };
                tab.project.delete().map_err(|e| e.to_string())
            }
            None => Project::open(&root)
                .and_then(Project::delete)
                .map_err(|e| e.to_string()),
        };
        if let Some(ui) = self.ui() {
            let s = ui.global::<Nw5>();
            match result {
                Ok(()) => {
                    s.set_dp_visible(false);
                    self.nw5.dialogs.delete_target = None;
                }
                Err(e) => s.set_dp_error(ss(&e)),
            }
            if s.get_op_visible() {
                self.nw5_refresh_open_list();
            }
        }
        self.refresh();
    }

    /// Close the current NW5 tab ("Close" of the File menu).
    pub fn nw5_close_current(&mut self) {
        let Some(k) = self.nw5_index() else {
            return;
        };
        if self.nw5.tabs[k].job.is_some() {
            // PROPOSAL: a project with a run in progress stays open.
            let text = self.tr("Nw5.Cancel_before_close");
            self.nw5_error(&text);
            return;
        }
        self.nw5.tabs.remove(k);
        self.nw5.current = if self.nw5.tabs.is_empty() {
            None
        } else {
            Some(k.min(self.nw5.tabs.len() - 1))
        };
        self.refresh();
    }

    /// "Close All": the NW5 tabs without a run in progress close too.
    pub fn nw5_close_all(&mut self) {
        self.nw5.tabs.retain(|t| t.job.is_some());
        self.nw5.current = if self.nw5.tabs.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    // ============================================================== import

    /// File > Import Network: the native file chooser (F12).
    pub fn nw5_import_dialog(&mut self) {
        if self.nw5_cur().is_none() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NeatWork network", &["tpo", "dsg", "json", "inp"])
            .pick_file()
        else {
            return;
        };
        self.nw5_import_path(&path);
    }

    /// Import a file; a `.tpo` without `!Diameters` asks for its catalogues
    /// first (F15).
    pub fn nw5_import_path(&mut self, path: &Path) {
        let is_tpo = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("tpo"));
        if is_tpo {
            let text = std::fs::read(path)
                .map(|b| neatwork_nw5_project::decode_text(&b))
                .unwrap_or_default();
            let present: Vec<String> = neatwork_nw5::read::sections(&text)
                .0
                .into_iter()
                .map(|s| s.name)
                .collect();
            let has = |n: &str| present.iter().any(|p| p == n);
            if !has("diameters") {
                self.nw5_tpo_dialog(path, has("orifices"));
                return;
            }
        }
        self.nw5_do_import(path, &ImportOptions::default());
    }

    fn nw5_tpo_dialog(&mut self, path: &Path, has_orifices: bool) {
        let Some(ui) = self.ui() else {
            return;
        };
        let dir = path.parent().unwrap_or(Path::new("."));
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folder = dir
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let found = [
            dir.join(format!("DiamBase{stem}.db")),
            dir.join(format!("DiamBase{folder}.db")),
            dir.join("diameters.db"),
        ]
        .into_iter()
        .find(|p| p.is_file());
        let orifices = dir.join("orifices.db");
        let s = ui.global::<Nw5>();
        s.set_tc_file(ss(&path.display().to_string()));
        s.set_tc_analysis(ss(&format!(
            "!Diameters — {}\n!Orifices — {}",
            self.tr("Nw5.Not_included"),
            if has_orifices {
                self.tr("Nw5.Included")
            } else {
                self.tr("Nw5.Not_included")
            }
        )));
        s.set_tc_required(true);
        s.set_tc_diameters(ss(&found
            .map(|p| p.display().to_string())
            .unwrap_or_default()));
        s.set_tc_orifices(ss(&if !has_orifices && orifices.is_file() {
            orifices.display().to_string()
        } else {
            String::new()
        }));
        s.set_tc_error(ss(""));
        s.set_tc_visible(true);
        self.nw5.dialogs.tpo_path = Some(path.to_path_buf());
    }

    fn nw5_tc_browse(&mut self, diameters: bool) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("Database", &["db"])
            .pick_file()
        else {
            return;
        };
        if let Some(ui) = self.ui() {
            let s = ui.global::<Nw5>();
            if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("db")) {
                s.set_tc_error(ss(&self.tr("Nw5.Select_db_file")));
                return;
            }
            let text = ss(&p.display().to_string());
            if diameters {
                s.set_tc_diameters(text);
            } else {
                s.set_tc_orifices(text);
            }
        }
    }

    pub fn nw5_tc_import(&mut self) {
        let (Some(ui), Some(path)) = (self.ui(), self.nw5.dialogs.tpo_path.clone()) else {
            return;
        };
        let s = ui.global::<Nw5>();
        let opt = |t: slint::SharedString| {
            let t = t.trim().to_string();
            (!t.is_empty()).then(|| PathBuf::from(t))
        };
        let options = ImportOptions {
            diameters: opt(s.get_tc_diameters()),
            orifices: opt(s.get_tc_orifices()),
        };
        match self.nw5_import(&path, &options) {
            Ok(()) => {
                s.set_tc_visible(false);
                self.nw5.dialogs.tpo_path = None;
                self.refresh();
            }
            Err(e) => s.set_tc_error(ss(&e)),
        }
    }

    fn nw5_do_import(&mut self, path: &Path, options: &ImportOptions) {
        match self.nw5_import(path, options) {
            Ok(()) => self.refresh(),
            Err(e) => self.nw5_error(&e),
        }
    }

    fn nw5_import(&mut self, path: &Path, options: &ImportOptions) -> Result<(), String> {
        let label = self.tr("Nw5.Imported_as");
        let Some(tab) = self.nw5_cur_mut() else {
            return Err("No NW5 project".into());
        };
        let out = tab
            .project
            .import_network(path, options)
            .map_err(|e| e.to_string())?;
        tab.warnings = out.warnings;
        tab.banner.clear();
        tab.notice = format!(
            "{} {label} {}",
            path.file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            out.revision.file
        );
        tab.selection = None;
        tab.reload();
        if let Some(ui) = self.ui() {
            // F3: the view switches to the graph after an import
            ui.global::<Nw5>().set_pane_tab(0);
        }
        Ok(())
    }

    // =========================================================== revisions

    fn nw5_revision_file(&self, i: usize) -> Option<String> {
        self.nw5_cur()?
            .project
            .revisions()
            .get(i)
            .map(|r| r.file.clone())
    }

    fn nw5_revision_activate(&mut self, i: usize) {
        let Some(file) = self.nw5_revision_file(i) else {
            return;
        };
        self.nw5_set_active(&file);
    }

    pub fn nw5_set_active(&mut self, file: &str) {
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        match tab.project.set_active_revision(file) {
            Ok(()) => {
                tab.notice.clear();
                tab.reload();
                self.refresh_nw5();
            }
            Err(e) => self.nw5_error(&e.to_string()),
        }
    }

    fn nw5_revision_archive(&mut self, i: usize) {
        let Some(file) = self.nw5_revision_file(i) else {
            return;
        };
        let text = format!(
            "{file}\n{}\n{}\n{}",
            self.tr("Nw5.Archive_1"),
            self.tr("Nw5.Archive_2"),
            self.tr("Nw5.Archive_3")
        );
        let title = self.tr("Nw5.Archive_title");
        self.message(&title, &text, 1, Pending::Nw5ArchiveRevision(file));
    }

    /// Answer of a NW5 confirmation of the message box.
    pub fn nw5_answer(&mut self, pending: Pending) {
        let result = match (pending, self.nw5_cur_mut()) {
            (Pending::Nw5ArchiveRevision(file), Some(tab)) => {
                let r = tab.project.archive_revision(&file);
                tab.reload();
                r
            }
            (Pending::Nw5DeleteRun(n), Some(tab)) => {
                let r = tab.project.delete_run(n);
                tab.reload_runs();
                tab.cmp_pair = None;
                if tab.results_run == Some(n) {
                    tab.results_run = None;
                }
                r
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            self.nw5_error(&e.to_string());
        }
        self.refresh_nw5();
    }

    /// Export a revision as `.tpo`, `.dsg`, NW-json or `.inp` (F40,
    /// ADR-025 item 10): written to `exports/`, then copied where the save
    /// dialog says (`to` for the test steps).
    pub fn nw5_revision_export(&mut self, i: usize, ext: &str, to: Option<PathBuf>) {
        let Some(file) = self.nw5_revision_file(i) else {
            return;
        };
        let format = match ext {
            ".tpo" => RevisionExport::Tpo,
            ".dsg" => RevisionExport::Dsg,
            ".inp" => RevisionExport::Inp,
            _ => RevisionExport::NwJson,
        };
        let stem = file.rsplit_once('.').map_or(file.as_str(), |(s, _)| s);
        let Some(target) = to.or_else(|| save_dialog(&format!("{stem}{ext}"), &ext[1..])) else {
            return;
        };
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let result = tab
            .project
            .export_revision(&file, format)
            .map_err(|e| e.to_string())
            .and_then(|out| {
                std::fs::copy(&out.path, &target)
                    .map(|_| out.warnings)
                    .map_err(|e| format!("{}: {e}", target.display()))
            });
        match result {
            Ok(w) => {
                tab.notice = format!("{} {}", file, target.display());
                if !w.is_empty() {
                    tab.notice.push_str(&format!(" ({})", w.join("; ")));
                }
                self.refresh_nw5();
            }
            Err(e) => self.nw5_error(&e),
        }
    }

    fn nw5_units_apply(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let unit = DisplayUnit::ALL
            .get(usize::try_from(s.get_un_choice()).unwrap_or(0))
            .copied()
            .unwrap_or_default();
        s.set_un_visible(false);
        if let Some(tab) = self.nw5_cur_mut()
            && let Err(e) = tab.project.set_display_unit(unit)
        {
            let e = e.to_string();
            self.nw5_error(&e);
        }
        self.refresh_nw5();
    }

    // ============================================================= network

    fn nw5_select(&mut self, sel: Option<Selection>) {
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        tab.selection = sel;
        let pos = match (&tab.selection, &tab.graph) {
            (Some(Selection::Node(id)), Some(g)) => {
                g.nodes.iter().find(|n| &n.id == id).map(|n| (n.x, n.y))
            }
            _ => None,
        };
        if let (Some(ui), Some((x, y))) = (self.ui(), pos) {
            let s = ui.global::<Nw5>();
            s.set_inspector_x(ss(&format!("{x}")));
            s.set_inspector_y(ss(&format!("{y}")));
        }
        self.nw5_refresh_network();
    }

    fn nw5_node_clicked(&mut self, i: usize) {
        let id = self
            .nw5_cur()
            .and_then(|t| t.graph.as_ref())
            .and_then(|g| g.nodes.get(i))
            .map(|n| n.id.clone());
        if let Some(id) = id {
            self.nw5_select(Some(Selection::Node(id)));
        }
    }

    /// A click on the canvas selects the nearest edge (7 px) or clears the
    /// selection (F24).
    fn nw5_canvas_clicked(&mut self, x: f64, y: f64) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let zoom = f64::from(s.get_zoom()).max(1e-6);
        let layout = nw5_layout_of(s.get_layout());
        let id = self.nw5_cur().and_then(|t| t.graph.as_ref()).and_then(|g| {
            let pic = nw5_picture(g, layout);
            crate::drawing::nw5_nearest_edge(g, &pic.points, x, y, 7.0 / zoom)
                .map(|k| g.edges[k].id.clone())
        });
        self.nw5_select(id.map(Selection::Edge));
    }

    /// Node drag in the Visual layout: the new position is stored (F25).
    fn nw5_node_dragged(&mut self, i: usize, dx: f64, dy: f64) {
        let Some(ui) = self.ui() else {
            return;
        };
        let layout = nw5_layout_of(ui.global::<Nw5>().get_layout());
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let Some(g) = &tab.graph else {
            return;
        };
        let Some(frame) = nw5_picture(g, layout).frame else {
            return;
        };
        let Some(n) = g.nodes.get(i) else {
            return;
        };
        let (mx, my) = frame.to_model_delta(dx, dy);
        let (id, x, y) = (n.id.clone(), n.x + mx, n.y + my);
        match tab.project.set_node_position(&id, x, y) {
            Ok(()) => {
                tab.reload_graph();
                tab.selection = Some(Selection::Node(id));
                self.nw5_select(tab_selection(self));
            }
            Err(e) => self.nw5_error(&e.to_string()),
        }
    }

    fn nw5_save_position(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let (x, y) = (
            parse_number(&s.get_inspector_x()),
            parse_number(&s.get_inspector_y()),
        );
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let Some(Selection::Node(id)) = tab.selection.clone() else {
            return;
        };
        let result = match (x, y) {
            (Some(x), Some(y)) => tab.project.set_node_position(&id, x, y),
            _ => Err(neatwork_nw5_project::Error::Refused(
                "Coordinates must be finite numbers.".into(),
            )),
        };
        tab.reload_graph();
        if let Err(e) = result {
            self.nw5_error(&e.to_string());
        }
        self.nw5_refresh_network();
    }

    /// Open or close the selected edge (F25); the results shown are
    /// cleared (D6).
    fn nw5_toggle_valve(&mut self) {
        let id = match self.nw5_cur().and_then(|t| t.selection.clone()) {
            Some(Selection::Edge(id)) => id,
            _ => return,
        };
        self.nw5_set_arc(&id, None);
    }

    /// Set an arc open, closed (`open`) or toggle it (`None`).
    pub fn nw5_set_arc(&mut self, id: &str, open: Option<bool>) {
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let current = tab
            .graph
            .as_ref()
            .and_then(|g| g.edges.iter().find(|e| e.id == id))
            .map(|e| e.open);
        let Some(current) = current else {
            let text = format!("Edge not found: {id}");
            self.nw5_error(&text);
            return;
        };
        let open = open.unwrap_or(!current);
        match tab.project.set_arc_open(id, open) {
            Ok(()) => {
                tab.results_run = None;
                tab.reload_graph();
                tab.selection = Some(Selection::Edge(id.to_string()));
                self.refresh_nw5();
            }
            Err(e) => self.nw5_error(&e.to_string()),
        }
    }

    /// Node search (F26): select, center and flash the node for 2.5 s.
    pub fn nw5_search(&mut self, query: &str) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let layout = nw5_layout_of(s.get_layout());
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let Some(g) = &tab.graph else {
            return;
        };
        match g.find_node(query) {
            Ok(n) => {
                let id = n.id.clone();
                let k = g.nodes.iter().position(|m| m.id == id).unwrap_or(0);
                let p = nw5_picture(g, layout).points[k];
                tab.flash = Some(id.clone());
                s.set_search_message(ss(""));
                s.set_focus_x(p.0 as f32);
                s.set_focus_y(p.1 as f32);
                s.set_focus_serial(s.get_focus_serial() + 1);
                let tab_id = tab.id;
                self.nw5_select(Some(Selection::Node(id)));
                slint::Timer::single_shot(std::time::Duration::from_millis(2500), move || {
                    with_app(|app| {
                        if let Some(t) = app.nw5_tab_mut(tab_id) {
                            t.flash = None;
                        }
                        app.nw5_refresh_network();
                    });
                });
            }
            Err(msg) => s.set_search_message(ss(&msg)),
        }
    }

    // ============================================================== design

    /// Open the design dialog with the parameters of the active revision.
    pub fn nw5_design_dialog(&mut self) {
        let Some(tab) = self.nw5_cur() else {
            return;
        };
        let Some(l) = &tab.loaded else {
            return;
        };
        let params: Vec<(String, String, String)> = DESIGN_PARAMS
            .iter()
            .map(|k| {
                let v = l.network.params.raw(k).map_or_else(
                    || neatwork_nw5::read::g15(l.network.params.get(k)),
                    str::to_string,
                );
                ((*k).to_string(), v.clone(), v)
            })
            .collect();
        let input = format!(
            "{} {} ({})",
            self.tr("Nw5.Design_input"),
            l.revision.file,
            if l.network.is_design() {
                self.tr("Nw5.Reinforcement")
            } else {
                self.tr("Nw5.Topography")
            }
        );
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        s.set_dd_input(ss(&input));
        s.set_dd_params(model(
            params
                .iter()
                .map(|(k, _, v)| Nw5Pair {
                    key: ss(k),
                    value: ss(v),
                })
                .collect(),
        ));
        if s.get_dd_seed().is_empty() {
            s.set_dd_seed(ss("0"));
        }
        s.set_dd_error(ss(""));
        s.set_dd_visible(true);
        self.nw5.dialogs.design_params = params;
    }

    /// Run the design on a background thread with the progress dialog.
    pub fn nw5_dd_run(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let Some(tab) = self.nw5_cur() else {
            return;
        };
        let Some(l) = &tab.loaded else {
            return;
        };
        let Ok(seed) = s.get_dd_seed().trim().parse::<u32>() else {
            s.set_dd_error(ss(&self.tr("Nw5.Bad_seed")));
            return;
        };
        let mut params = Vec::new();
        for (k, initial, current) in &self.nw5.dialogs.design_params {
            if current.trim() == initial.trim() {
                continue;
            }
            match parse_number(current) {
                Some(v) => params.push((k.clone(), v)),
                None => {
                    s.set_dd_error(ss(&format!("{k}: {}", self.tr("Nw5.Not_a_number"))));
                    return;
                }
            }
        }
        let options = DesignOptions {
            params,
            load_method: if s.get_dd_monte_carlo() {
                LoadMethod::MonteCarlo
            } else {
                LoadMethod::Auto
            },
            risk: match s.get_dd_risk() {
                1 => Some(RiskMeasure::Var),
                2 => Some(RiskMeasure::Cvar),
                _ => None,
            },
            seed,
            n_scen_load: None,
        };
        let mut network = l.network.clone();
        // PROPOSAL: as `neatwork design`, a network without a catalogue
        // takes the catalogue shipped with NeatWork 2024.
        let mut notes = Vec::new();
        if network.pipes.is_empty() {
            match neatwork_nw5::read::read_diameters_db(DEFAULT_DIAMETERS) {
                Ok(p) => network.pipes = p,
                Err(e) => {
                    s.set_dd_error(ss(&e.to_string()));
                    return;
                }
            }
            notes.push("diameters: NeatWork 2024 shipped catalog".to_string());
        }
        if network.orifices.is_empty() {
            network.orifices = neatwork_nw5::read::read_orifices_db(DEFAULT_ORIFICES);
            notes.push("orifices: NeatWork 2024 shipped catalog".to_string());
        }
        let id = tab.id;
        s.set_dd_visible(false);
        self.start_progress("Nw5.Stage_prepare");
        std::thread::spawn(move || {
            let mut progress = |stage: Stage| {
                let (v, key) = match stage {
                    Stage::Prepare => (5, "Nw5.Stage_prepare"),
                    Stage::Loads => (20, "Nw5.Stage_loads"),
                    Stage::Model => (50, "Nw5.Stage_model"),
                    Stage::Solve => (65, "Nw5.Stage_solve"),
                    Stage::PostProcess => (90, "Nw5.Stage_post"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    with_app(|app| app.set_progress(v, key));
                });
            };
            let result = design_with_progress(&network, &options, &mut progress)
                .map(|r| {
                    Box::new(DesignOutcome {
                        network: r.network,
                        costs: r.costs,
                        warnings: r.warnings,
                        notes,
                    })
                })
                .map_err(|e| e.to_string());
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|app| {
                    app.finished = Some(Finished::Nw5Design { tab: id, result });
                    app.set_progress(100, "SolverDisk.Completed.");
                });
            });
        });
    }

    /// Close of the progress dialog after a design: show the costs and
    /// warnings, and the revision save (F9, F10).
    pub fn nw5_design_finished(&mut self, tab: u64, result: Result<Box<DesignOutcome>, String>) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let Some(t) = self.nw5.tabs.iter().find(|t| t.id == tab) else {
            return;
        };
        let source = t.active_file().unwrap_or_default();
        match result {
            Ok(out) => {
                let c = &out.costs;
                let mut text = format!(
                    "{}\n  Optimal pipe cost (solver): {:.2}\n  Cost after mixture reduction: {:.2}\n  Cost before rounding: {:.2}\n  Final pipe cost: {:.2}\n  Fixed cost: {:.2}\n  LP objective: {:.2}\n",
                    self.tr("Nw5.Costs"),
                    c.optim,
                    c.after_reduction,
                    c.inter,
                    c.final_cost,
                    c.fixed,
                    c.lp
                );
                text.push_str(&format!(
                    "\n{}: {} · {}: {}\n",
                    self.tr("Nw5.Nodes"),
                    out.network.nodes.len(),
                    self.tr("Nw5.Pipes"),
                    out.network.arcs.len()
                ));
                if !out.warnings.is_empty() || !out.notes.is_empty() {
                    text.push_str(&format!("\n{}\n", self.tr("Nw5.Warnings")));
                    for w in out.notes.iter().chain(&out.warnings) {
                        text.push_str(&format!("  • {w}\n"));
                    }
                }
                let auto = t
                    .project
                    .design_revision_name(&RevisionName::Automatic)
                    .unwrap_or_default();
                s.set_dr_title(ss(&self.tr("Nw5.Design_completed")));
                s.set_dr_text(ss(&text));
                s.set_dr_ok(true);
                s.set_dr_source(ss(&source));
                s.set_dr_mode(0);
                s.set_dr_auto_name(ss(&auto));
                s.set_dr_custom_name(ss(&suggested_custom_name(&auto)));
                self.nw5.dialogs.design_result = Some((tab, out));
            }
            Err(e) => {
                s.set_dr_title(ss(&self.tr("Nw5.Design_failed")));
                s.set_dr_text(ss(&e));
                s.set_dr_ok(false);
                self.nw5.dialogs.design_result = None;
            }
        }
        s.set_dr_error(ss(""));
        s.set_dr_visible(true);
    }

    /// Save the design as a new `.dsg` revision; a refused name shows the
    /// message of the project crate (ADR-025 item 2).
    pub fn nw5_dr_save(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let name = if s.get_dr_mode() == 0 {
            RevisionName::Automatic
        } else {
            RevisionName::Custom(s.get_dr_custom_name().to_string())
        };
        let Some((id, out)) = self.nw5.dialogs.design_result.take() else {
            return;
        };
        let saved_as = self.tr("Nw5.Saved_as");
        let Some(tab) = self.nw5_tab_mut(id) else {
            return;
        };
        match tab.project.save_design_revision(&out.network, &name) {
            Ok(r) => {
                tab.notice = format!("{saved_as} {} (v{})", r.file, r.version);
                tab.banner.clear();
                tab.reload();
                s.set_dr_visible(false);
                self.refresh();
            }
            Err(e) => {
                s.set_dr_error(ss(&e.to_string()));
                self.nw5.dialogs.design_result = Some((id, out));
            }
        }
    }

    // ========================================================== simulation

    /// Fill and open the configuration dialog (F36, F37).
    pub fn nw5_configure(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let Some(tab) = self.nw5_cur() else {
            return;
        };
        let d = match tab.project.simulation_defaults() {
            Ok(d) => d,
            Err(e) => {
                let e = e.to_string();
                self.nw5_error(&e);
                return;
            }
        };
        let r = &tab.requested;
        let unit = tab.project.display_unit();
        let format = tab
            .project
            .active_revision()
            .map_or("network", |r| r.format.as_str());
        s.set_sc_intro(ss(&format!(
            "{} {format} {}",
            self.tr("Nw5.Config_intro_1"),
            self.tr("Nw5.Config_intro_2")
        )));
        let n = r.override_count();
        s.set_sc_badge(ss(&if n == 0 {
            format!(
                "{} · {}",
                self.tr("Nw5.Imported_defaults"),
                self.tr("Nw5.No_overrides")
            )
        } else {
            format!(
                "{} · {n} {}",
                self.tr("Nw5.Custom_overrides"),
                self.tr("Nw5.Runtime_overrides")
            )
        }));
        let src = |set: bool| {
            if set {
                "Modified by user"
            } else {
                "Project default"
            }
        };
        let on = |b: bool| if b { "Enabled" } else { "Disabled" };
        let rows = vec![
            (
                "Number of scenarios",
                d.scenario_count.to_string(),
                src(r.scenario_count.is_some()),
            ),
            (
                "Random seed",
                r.seed.map_or("—".into(), |x| x.to_string()),
                if r.seed.is_some() {
                    "Modified by user"
                } else {
                    "Generated"
                },
            ),
            (
                "Bounded flow",
                on(d.bounded_flow).to_string(),
                src(r.bounded_flow.is_some()),
            ),
            (
                "Orifice model",
                orifice_mode_label(d.orifice_mode).to_string(),
                src(r.orifice_mode.is_some()),
            ),
            (
                "Service quality",
                format!("{}", d.service_quality),
                "Project default",
            ),
            (
                "Required pressure",
                format!(
                    "{} {}",
                    unit.from_meters(d.required_pressure_m),
                    unit.symbol()
                ),
                "Project default",
            ),
            (
                "Orifice coefficient",
                format!("{}", d.orifice_coefficient),
                "Project default",
            ),
            (
                "Faucet coefficient",
                format!("{}", d.faucet_coefficient),
                "Project default",
            ),
            (
                "Water temperature (°C)",
                format!("{}", d.water_temperature_c),
                "Project default",
            ),
            (
                "Commercial pipe length",
                format!(
                    "{} {}",
                    unit.from_meters(d.commercial_pipe_length_m),
                    unit.symbol()
                ),
                "Project default",
            ),
            (
                "Maximum generation batches",
                d.max_generation_batches.to_string(),
                src(r.max_generation_batches.is_some()),
            ),
            (
                "Oversampling factor",
                format!("{}", d.oversampling_factor),
                src(r.oversampling_factor.is_some()),
            ),
            (
                "Maximum draws per batch",
                d.max_draws_per_batch.to_string(),
                src(r.max_draws_per_batch.is_some()),
            ),
        ];
        s.set_sc_defaults(model(
            rows.iter()
                .map(|(l, v, so)| Nw5Option {
                    label: ss(l),
                    value: ss(v),
                    source: ss(so),
                })
                .collect(),
        ));
        s.set_sc_scenarios(ss(&r
            .scenario_count
            .unwrap_or(d.scenario_count)
            .to_string()));
        s.set_sc_use_seed(r.seed.is_some());
        s.set_sc_seed(ss(&r.seed.unwrap_or(42).to_string()));
        s.set_sc_bounded_locked(!d.demand.uniform);
        s.set_sc_bounded(!d.demand.uniform || r.bounded_flow.unwrap_or(d.bounded_flow));
        s.set_sc_orifice(match r.orifice_mode.unwrap_or(d.orifice_mode) {
            OrificeMode::Ideal => 0,
            OrificeMode::Commercial => 1,
        });
        s.set_sc_batches(ss(&r
            .max_generation_batches
            .unwrap_or(d.max_generation_batches)
            .to_string()));
        s.set_sc_oversampling(ss(&format!(
            "{}",
            r.oversampling_factor.unwrap_or(d.oversampling_factor)
        )));
        s.set_sc_draws(ss(&r
            .max_draws_per_batch
            .unwrap_or(d.max_draws_per_batch)
            .to_string()));
        let dm = &d.demand;
        let mut demand = format!(
            "{}\nSource: Client demand table · Clients {} · Demand profile {}",
            self.tr("Nw5.Demand_note_1"),
            dm.client_count,
            if dm.uniform { "Uniform" } else { "Per client" }
        );
        if dm.uniform
            && let (Some(p), Some(q)) = (dm.open_probability, dm.target_flow_lps)
        {
            demand.push_str(&format!(
                "\nOpen faucets {} ({}%) · Target flow {} L/s",
                neatwork_nw5_project::format::fixed(p, 2),
                neatwork_nw5_project::format::fixed(p * 100.0, 0),
                neatwork_nw5_project::format::fixed(q, 2)
            ));
        }
        if !dm.uniform {
            demand.push_str(&format!("\n{}", self.tr("Nw5.Heterogeneous_note")));
        }
        demand.push_str(&format!("\n{}", self.tr("Nw5.Demand_note_2")));
        s.set_sc_demand(ss(&demand));
        s.set_sc_can_run(dm.complete());
        s.set_sc_error(ss(&if dm.complete() {
            String::new()
        } else {
            "Some clients are missing an opening probability or target flow. Complete the Client demand table before simulation.".to_string()
        }));
        s.set_sc_visible(true);
    }

    /// Read the configuration dialog, store the request and run it when
    /// `run` (F36).
    fn nw5_sc_apply(&mut self, run: bool) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let Some(tab) = self.nw5_cur() else {
            return;
        };
        let Ok(d) = tab.project.simulation_defaults() else {
            return;
        };
        let mut errors: Vec<String> = Vec::new();
        let scenarios = parse_count(&s.get_sc_scenarios());
        if scenarios.is_none() {
            errors.push("Number of scenarios must be a positive whole number.".into());
        }
        let seed = if s.get_sc_use_seed() {
            let v = s.get_sc_seed().trim().parse::<u64>().ok();
            if v.is_none() {
                errors.push("Random seed must be a non-negative whole number.".into());
            }
            v
        } else {
            None
        };
        let batches = parse_count(&s.get_sc_batches());
        if batches.is_none() {
            errors.push("Maximum generation batches must be a positive whole number.".into());
        }
        let oversampling = parse_number(&s.get_sc_oversampling()).filter(|f| *f > 1.0);
        if oversampling.is_none() {
            errors.push("Oversampling factor must be greater than 1.".into());
        }
        let draws = parse_count(&s.get_sc_draws());
        if draws.is_none() {
            errors.push("Maximum draws per batch must be a positive whole number.".into());
        }
        let requested = RequestedOptions {
            scenario_count: scenarios,
            seed,
            bounded_flow: Some(s.get_sc_bounded()),
            orifice_mode: Some(if s.get_sc_orifice() == 0 {
                OrificeMode::Ideal
            } else {
                OrificeMode::Commercial
            }),
            max_generation_batches: batches,
            oversampling_factor: oversampling,
            max_draws_per_batch: draws,
        }
        .compact(&d);
        if errors.is_empty() {
            errors = requested.validate(&d);
        }
        if !errors.is_empty() {
            s.set_sc_error(ss(&errors.join("\n")));
            return;
        }
        s.set_sc_visible(false);
        if let Some(t) = self.nw5_cur_mut() {
            t.requested = requested;
        }
        if run {
            self.nw5_run();
        }
    }

    /// Start a run of the active revision on a background thread (F38).
    pub fn nw5_run(&mut self) {
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        if tab.job.is_some() {
            return;
        }
        let job = match tab.project.start_run(&tab.requested) {
            Ok(j) => j,
            Err(e) => {
                let e = e.to_string();
                self.nw5_error(&e);
                return;
            }
        };
        let cancel = CancelFlag::new();
        let total = job.options().scenario_count.unwrap_or(0);
        tab.job = Some(ActiveRun {
            run_number: job.run_number(),
            cancel: cancel.clone(),
            done: 0,
            total,
        });
        tab.banner.clear();
        tab.notice.clear();
        tab.reload_runs();
        let id = tab.id;
        std::thread::spawn(move || {
            let mut last = usize::MAX;
            let result = job.execute(&cancel, |done, total| {
                let pct = done * 100 / total.max(1);
                if pct != last {
                    last = pct;
                    let _ = slint::invoke_from_event_loop(move || {
                        with_app(|app| app.nw5_run_progress(id, done, total));
                    });
                }
            });
            let result = result.map_err(|e| e.to_string());
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|app| app.nw5_run_finished(id, result));
            });
        });
        self.refresh_nw5();
    }

    fn nw5_cancel_run(&mut self) {
        if let Some(j) = self.nw5_cur().and_then(|t| t.job.as_ref()) {
            j.cancel.cancel();
        }
    }

    fn nw5_run_progress(&mut self, id: u64, done: usize, total: usize) {
        let Some(tab) = self.nw5_tab_mut(id) else {
            return;
        };
        if let Some(j) = &mut tab.job {
            j.done = done;
            j.total = total;
        }
        if let Some(t) = self.nw5_cur().filter(|t| t.id == id) {
            self.nw5_refresh_progress(t);
        }
    }

    /// The run thread ended: store the state, show the result (F38).
    fn nw5_run_finished(&mut self, id: u64, result: Result<RunRecord, String>) {
        let is_current = self.nw5_cur().is_some_and(|t| t.id == id);
        let Some(tab) = self.nw5_tab_mut(id) else {
            return;
        };
        tab.project.finish_run();
        tab.job = None;
        tab.reload_runs();
        let mut open_review = false;
        match result {
            Ok(rec) => {
                if rec.status.is_completed() && rec.statistics.is_some() {
                    if Some(&rec.revision) == tab.active_file().as_ref() {
                        tab.results_run = Some(rec.run_number);
                    }
                    tab.review_run = Some(rec.run_number);
                    tab.review_sort = None;
                    open_review = true;
                }
                let low = tab
                    .graph_with_results(true)
                    .map_or(0, |g| g.nodes.iter().filter(|n| n.low_pressure).count());
                tab.last_summary = run_line(&rec, low);
                if let Some(e) = &rec.error {
                    tab.banner = format!("{}: {}", rec.status.as_str(), e.message);
                }
            }
            Err(e) => tab.banner = e,
        }
        if is_current {
            if open_review && let Some(ui) = self.ui() {
                // F38: a completed run opens its review
                ui.global::<Nw5>().set_pane_tab(5);
            }
            self.refresh_nw5();
        }
    }

    // ================================================================ runs

    pub fn nw5_review(&mut self, n: u32) {
        if let Some(t) = self.nw5_cur_mut() {
            t.review_run = Some(n);
            t.review_sort = None;
        }
        if let Some(ui) = self.ui() {
            ui.global::<Nw5>().set_pane_tab(5);
        }
        self.refresh_nw5();
    }

    pub fn nw5_details(&mut self, n: u32) {
        let Some(tab) = self.nw5_cur() else {
            return;
        };
        let Some(rec) = tab.run(n) else {
            return;
        };
        let (title, text) = self.nw5_details_text(rec, tab.project.display_unit());
        if let Some(ui) = self.ui() {
            let s = ui.global::<Nw5>();
            s.set_rd_title(ss(&title));
            s.set_rd_text(ss(&text));
            s.set_rd_visible(true);
        }
    }

    fn nw5_run_delete(&mut self, n: u32) {
        let Some(rec) = self.nw5_cur().and_then(|t| t.run(n)) else {
            return;
        };
        let text = format!(
            "{}\n{}\nStatus {} · Seed {} · Scenarios {}",
            self.tr("Nw5.Delete_run_1"),
            self.tr("Nw5.Delete_run_2"),
            rec.status.as_str(),
            rec.effective_seed().map_or("—".into(), |s| s.to_string()),
            rec.scenario_count().map_or("—".into(), |s| s.to_string()),
        );
        let title = format!("{} #{n}?", self.tr("Nw5.Delete_run_title"));
        self.message(&title, &text, 1, Pending::Nw5DeleteRun(n));
    }

    /// Export the statistics JSON of a run (F40) with a save dialog (`to`
    /// for the test steps).
    pub fn nw5_run_export(&mut self, n: u32, to: Option<PathBuf>) {
        let Some(target) = to.or_else(|| save_dialog("simulation_statistics.json", "json")) else {
            return;
        };
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let result = tab
            .project
            .export_statistics(n)
            .map_err(|e| e.to_string())
            .and_then(|out| {
                std::fs::copy(&out.path, &target)
                    .map(|_| ())
                    .map_err(|e| format!("{}: {e}", target.display()))
            });
        match result {
            Ok(()) => {
                tab.notice = format!("{} #{n} → {}", "Simulation statistics", target.display());
                self.refresh_nw5();
            }
            Err(e) => self.nw5_error(&e),
        }
    }

    // ===================================================== review, compare

    fn nw5_review_changed(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let k = idx(ui.global::<Nw5>().get_review_run());
        if let Some(t) = self.nw5_cur_mut() {
            t.review_run = t.completed_runs().get(k).map(|r| r.run_number);
            t.review_sort = None;
        }
        self.refresh_nw5();
    }

    fn nw5_review_sort(&mut self, visible: usize) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let details = s.get_review_details();
        let kind = TableKind::ALL
            .get(idx(s.get_review_kind()))
            .copied()
            .unwrap_or(TableKind::TerminalSatisfaction);
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        if visible == 0 {
            return;
        }
        let rec = tab
            .review_run
            .and_then(|n| tab.run(n))
            .or_else(|| tab.completed_runs().last().copied());
        let Some(table) = rec.and_then(|r| {
            neatwork_nw5_project::review::result_table(r, kind, tab.project.display_unit())
        }) else {
            return;
        };
        if let Some(&c) = table.visible_columns(details).get(visible - 1) {
            tab.review_sort = next_sort(tab.review_sort, c);
        }
        self.refresh_nw5();
    }

    /// Load the two revisions of the compared runs and their
    /// compatibility (F49).
    fn nw5_update_compat(&mut self) {
        let Some(k) = self.nw5_index() else {
            return;
        };
        let pair = self.nw5_cmp_pair(&self.nw5.tabs[k]);
        let tab = &mut self.nw5.tabs[k];
        let Some((l, r)) = pair else {
            return;
        };
        if tab.compat.as_ref().is_some_and(|(key, _)| *key == (l, r)) {
            return;
        }
        let (Some(lr), Some(rr)) = (tab.run(l).cloned(), tab.run(r).cloned()) else {
            return;
        };
        let c = tab
            .project
            .load_revision(&lr.revision)
            .and_then(|a| Ok((a, tab.project.load_revision(&rr.revision)?)))
            .map(|(a, b)| {
                neatwork_nw5_project::compare::compatibility(
                    &a.network,
                    &lr.revision,
                    &b.network,
                    &rr.revision,
                )
            })
            .map_err(|e| format!("Could not load this comparison source. {e}"));
        tab.compat = Some(((l, r), c));
    }

    fn nw5_cmp_changed(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let (li, ri, ti) = (
            idx(s.get_cmp_left()),
            idx(s.get_cmp_right()),
            idx(s.get_cmp_table()),
        );
        let Some(tab) = self.nw5_cur_mut() else {
            return;
        };
        let active = tab.active_file();
        let sources: Vec<u32> =
            neatwork_nw5_project::compare::comparison_sources(&tab.runs, active.as_deref())
                .iter()
                .map(|r| r.run_number)
                .collect();
        let (Some(&l), Some(&r)) = (sources.get(li), sources.get(ri)) else {
            return;
        };
        if l == r {
            // an option chosen on one side is not offered on the other
            let text = self.tr("Nw5.Same_run");
            self.nw5_error(&text);
            self.refresh_nw5();
            return;
        }
        let (lr, rr) = (tab.run(l).cloned(), tab.run(r).cloned());
        let kind = match (&lr, &rr) {
            (Some(a), Some(b)) => neatwork_nw5_project::compare::common_tables(a, b)
                .get(ti)
                .copied(),
            _ => None,
        };
        if tab.cmp_pair != Some((l, r)) || tab.cmp_table != kind {
            tab.cmp_sort = None;
            tab.cmp_metric = None;
        }
        tab.cmp_pair = Some((l, r));
        tab.cmp_table = kind;
        let _ = tab.project.set_comparison_workspace(
            l,
            r,
            kind.unwrap_or(TableKind::TerminalSatisfaction).key(),
        );
        self.nw5_update_compat();
        self.refresh_nw5();
    }

    fn nw5_cmp_sort(&mut self, c: usize) {
        if let Some(t) = self.nw5_cur_mut() {
            t.cmp_sort = match t.cmp_sort {
                Some((k, SortDirection::Ascending)) if k == c => {
                    Some((c, SortDirection::Descending))
                }
                Some((k, SortDirection::Descending)) if k == c => None,
                _ => Some((c, SortDirection::Ascending)),
            };
        }
        self.refresh_nw5();
    }

    fn nw5_cmp_metric_changed(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        let m = (
            idx(s.get_cmp_strategy()),
            idx(s.get_cmp_left_metric()),
            idx(s.get_cmp_right_metric()),
        );
        if let Some(t) = self.nw5_cur_mut() {
            t.cmp_metric = Some(m);
        }
        self.refresh_nw5();
    }

    /// Show the comparison of two runs.
    pub fn nw5_compare(&mut self, l: u32, r: u32) {
        if let Some(t) = self.nw5_cur_mut() {
            t.cmp_pair = Some((l, r));
            t.cmp_sort = None;
            t.cmp_metric = None;
        }
        self.nw5_update_compat();
        if let Some(ui) = self.ui() {
            ui.global::<Nw5>().set_pane_tab(6);
        }
        self.refresh_nw5();
    }

    // ========================================================== test steps

    /// Steps `nw5*` of `NEATWORK_GUI_STEPS` (see `docs/gui.md`).
    pub fn nw5_step(&mut self, key: &str, value: &str) {
        let n: i32 = value.parse().unwrap_or(0);
        let two = |v: &str| -> Option<(String, String)> {
            v.split_once(':')
                .map(|(a, b)| (a.to_string(), b.to_string()))
        };
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<Nw5>();
        match key {
            "nw5new" => match Project::create(&self.nw5.dir, value) {
                Ok(p) => self.nw5_add_tab(p),
                Err(e) => self.nw5_error(&e.to_string()),
            },
            "nw5open" => {
                let p = PathBuf::from(value);
                let p = if p.is_absolute() {
                    p
                } else {
                    self.nw5.dir.join(p)
                };
                self.nw5_open_path(&p);
            }
            "nw5import" => {
                self.nw5_import_path(Path::new(value));
                if s.get_tc_visible() {
                    self.nw5_tc_import();
                }
            }
            "nw5tab" => s.set_pane_tab(n),
            "nw5table" => {
                s.set_table_index(n);
                self.refresh_nw5();
            }
            "nw5text" => {
                s.set_text_mode(n);
                self.refresh_nw5();
            }
            "nw5layout" => {
                s.set_layout(n);
                self.nw5_refresh_network();
            }
            "nw5details" if value.is_empty() => {
                s.set_show_details(!s.get_show_details());
                self.nw5_refresh_network();
            }
            "nw5details" => self.nw5_details(n as u32),
            "nw5select" => self.nw5_search(value),
            "nw5zoom" => s.set_zoom(value.parse().unwrap_or(1.0)),
            "nw5edge" => self.nw5_select(Some(Selection::Edge(value.to_string()))),
            "nw5valve" => self.nw5_set_arc(value, None),
            "nw5design" => {
                self.nw5_design_dialog();
                match value {
                    "var" => s.set_dd_risk(1),
                    "cvar" => s.set_dd_risk(2),
                    _ => {}
                }
                self.nw5_dd_run();
            }
            "nw5save" => {
                if value.is_empty() {
                    s.set_dr_mode(0);
                } else {
                    s.set_dr_mode(1);
                    s.set_dr_custom_name(ss(value));
                }
                self.nw5_dr_save();
            }
            "nw5active" => self.nw5_set_active(value),
            "nw5configure" => self.nw5_configure(),
            "nw5run" => {
                let (count, seed) = match two(value) {
                    Some((c, sd)) => (c.parse().ok(), sd.parse().ok()),
                    None => (value.parse().ok(), None),
                };
                if let Some(t) = self.nw5_cur_mut() {
                    t.requested = RequestedOptions {
                        scenario_count: count,
                        seed,
                        ..t.requested.clone()
                    };
                }
                self.nw5_run();
            }
            "nw5review" => self.nw5_review(n as u32),
            "nw5reviewtable" => {
                s.set_review_kind(n);
                self.refresh_nw5();
            }
            "nw5compare" => {
                if let Some((a, b)) = two(value)
                    && let (Ok(a), Ok(b)) = (a.parse(), b.parse())
                {
                    self.nw5_compare(a, b);
                }
            }
            "nw5units" => {
                if let (Some(unit), Some(t)) = (DisplayUnit::parse(value), self.nw5_cur_mut()) {
                    let _ = t.project.set_display_unit(unit);
                }
                self.refresh_nw5();
            }
            "nw5export" => {
                if let Some((run, path)) = two(value) {
                    self.nw5_run_export(run.parse().unwrap_or(0), Some(PathBuf::from(path)));
                }
            }
            "nw5close" => {
                for set in [
                    Nw5::set_np_visible,
                    Nw5::set_op_visible,
                    Nw5::set_tc_visible,
                    Nw5::set_dd_visible,
                    Nw5::set_dr_visible,
                    Nw5::set_sc_visible,
                    Nw5::set_un_visible,
                    Nw5::set_rd_visible,
                    Nw5::set_dp_visible,
                ] {
                    set(&s, false);
                }
            }
            "nw5menu" => self.nw5_menu(value),
            _ => {}
        }
    }
}

/// Current selection of the current NW5 tab.
fn tab_selection(app: &App) -> Option<Selection> {
    app.nw5_cur().and_then(|t| t.selection.clone())
}
