//! What the menus, buttons and tables do (Java `FrameNeatwork` actions and
//! the dialogs).

use std::collections::HashMap;

use neatwork_core::java::{JHashtable, double_to_string, parse_double};
use neatwork_core::project::network::TYPE_FAUCET;
use neatwork_core::project::{Design, SimulationFile, Topographie};
use neatwork_core::{
    Constraint, Database, SimulationParameters, SimulationType, SplitMix64, faucet_count,
    make_design, prepare_make_design, run_simulation, simulation_properties, solve_with_highs,
};
use slint::{ComponentHandle, SharedString};

use crate::app::{
    App, FdMode, FileDialogState, Finished, MakeDesignState, NewSimuState, OpenProject, Pending,
    Proj,
};
use crate::files::FileKind;
use crate::i18n;
use crate::support;
use crate::table::{Col, Edit, Kind, Table};
use crate::{AppState, with_app};

fn sstr(s: &str) -> SharedString {
    SharedString::from(s)
}

impl App {
    // ================================================================ menu

    pub fn menu(&mut self, action: &str) {
        match action {
            "new-topo" => self.open_file_dialog(FdMode::New, FileKind::Topo),
            "open-topo" => self.open_file_dialog(FdMode::Open, FileKind::Topo),
            "open-design" => self.open_file_dialog(FdMode::Open, FileKind::Design),
            "delete-topo" => self.open_file_dialog(FdMode::Delete, FileKind::Topo),
            "delete-design" => self.open_file_dialog(FdMode::Delete, FileKind::Design),
            "save" => self.save(),
            "save-as" => {
                if let Some(p) = self.cur() {
                    let (name, kind) = (p.name().to_string(), p.kind());
                    self.open_file_dialog(FdMode::Save, kind);
                    self.set_fd_name(&name);
                }
            }
            "close" => {
                if let Some(p) = self.cur() {
                    let text =
                        self.tr("FrameNeatwork.Do_you_want_to_close_the_file") + p.name() + " ?";
                    let title = self.tr("FrameNeatwork.Close_File");
                    self.message(&title, &text, 1, Pending::CloseCurrent);
                }
            }
            "close-all" => {
                let text = self.tr("FrameNeatwork.Do_you_want_to_close_all_the_files");
                let title = self.tr("FrameNeatwork.Close_All_The_Files");
                self.message(&title, &text, 1, Pending::CloseAll);
            }
            "exit" => {
                let text = self.tr("FrameNeatwork.Do_you_really_want_to_quit");
                let title = self.tr("FrameNeatwork.Confirmation_Dialog");
                self.message(&title, &text, 1, Pending::Exit);
            }
            "quick-check" => self.open_quick_check(),
            "topo-summary" => {
                if let Some(Proj::Topo(t)) = self.cur().map(|p| &p.proj) {
                    let text = support::topo_summary(self, t);
                    self.show_info("TopoStatDialog.Topography_information", &text, false);
                }
            }
            "topo-report" => {
                if let Some(Proj::Topo(t)) = self.cur().map(|p| &p.proj) {
                    let html = support::topo_report(self, t);
                    self.save_html(&html);
                }
            }
            "design-report" => {
                if let Some(Proj::Design(d)) = self.cur().map(|p| &p.proj) {
                    let html = support::design_report(self, d);
                    self.save_html(&html);
                }
            }
            "sim-report" => {
                if let Some(p) = self.cur()
                    && let (Proj::Design(d), Some(f)) = (&p.proj, &p.sim)
                {
                    let html = support::simulation_report(self, d, f);
                    self.save_html(&html);
                }
            }
            "start-design" => self.start_design(),
            "extract-topo" => self.extract_topography(),
            "design-params" => self.design_parameters(),
            "design-load-factors" => self.design_load_factors(),
            "database" => {
                self.db_diameters = self.db_diameter_table();
                self.db_orifices = self.db_orifice_table();
                self.push_database();
                if let Some(ui) = self.ui() {
                    ui.global::<AppState>().set_db_visible(true);
                }
            }
            "units" => self.units(),
            "about" => {
                let text = support::about_text(self);
                self.show_info("AboutDialog.About...", &text, true);
            }
            "lang-en" => self.set_language(i18n::EN),
            "lang-fr" => self.set_language(i18n::FR),
            "lang-sp" => self.set_language(i18n::SP),
            "sim-new" => self.new_simulation(),
            "sim-delete" => self.delete_simulation(),
            _ => {}
        }
    }

    /// One step of `NEATWORK_GUI_STEPS`.
    pub fn run_step(&mut self, key: &str, value: &str) {
        let n: i32 = value.parse().unwrap_or(0);
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<AppState>();
        match key {
            "tab" => s.set_pane_tab(n),
            "menu" => self.menu(value),
            "layout" => {
                s.set_net_layout(n);
                self.refresh_network();
            }
            "color" => {
                s.set_net_coloring(n);
                self.refresh_network();
            }
            "simtable" => {
                s.set_sim_table_kind(n);
                self.refresh_simulations();
            }
            "select" => {
                if let Some(p) = self.cur_mut() {
                    p.selected_node = Some(value.to_string());
                }
                self.refresh_tree();
                self.refresh_network();
            }
            "mdtab" => s.set_md_tab(n),
            "action" => {
                if let Some((id, a)) = value.split_once(':') {
                    self.table_action(id.parse().unwrap_or(-1), a);
                }
            }
            "mdrun" => self.md_run(),
            "pgclose" => self.pg_close(),
            "tdclose" => self.td_close(),
            "fdname" => self.set_fd_name(value),
            "fdaction" => self.fd_action(),
            "nstype" => s.set_ns_type(n),
            "nsrun" => self.ns_run(),
            "dbtab" => s.set_db_tab(n),
            _ => {}
        }
    }

    fn show_info(&mut self, title_key: &str, text: &str, about: bool) {
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_info_title(sstr(&self.tr(title_key)));
            s.set_info_text(sstr(text));
            s.set_info_about(about);
            s.set_info_visible(true);
        }
    }

    fn set_language(&mut self, lang: i32) {
        self.lang = lang;
        // The Java program asks for a restart; the Rust interface switches
        // at once and still writes the choice for the next start.
        let _ = crate::files::write_locale(i18n::locale_code(lang));
        for i in 0..self.projects.len() {
            let cur = self.current;
            self.current = Some(i);
            self.reload_tables();
            self.current = cur;
        }
        self.db_diameters = self.db_diameter_table();
        self.db_orifices = self.db_orifice_table();
        self.push_database();
        self.refresh();
    }

    /// Save an HTML report with a file chooser (`Tools.enregFich`).
    fn save_html(&mut self, html: &str) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML", &["html", "htm"])
            .set_directory(&self.files.project_dir)
            .save_file()
        else {
            return;
        };
        let name = path.to_string_lossy().to_string();
        let path = if name.ends_with("html") || name.ends_with("htm") {
            path
        } else {
            path.with_extension("html")
        };
        if let Err(e) = std::fs::write(&path, html) {
            self.message(&self.tr("Gui.Error"), &e.to_string(), 0, Pending::None);
        }
    }

    // ======================================================= message box

    pub fn msg_answer(&mut self, yes: bool) {
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_msg_visible(false);
        }
        let pending = std::mem::replace(&mut self.pending, Pending::None);
        if !yes {
            return;
        }
        match pending {
            Pending::CloseCurrent => self.remove_current(),
            Pending::CloseAll => {
                self.projects.clear();
                self.current = None;
                self.refresh();
            }
            Pending::Exit => {
                let _ = slint::quit_event_loop();
            }
            Pending::NewOverwrite(name) => self.create_topography(&name),
            Pending::SaveOverwrite(name) => {
                if let Some(p) = self.cur() {
                    let _ = self.files.delete(&name, p.kind());
                }
                self.write_as(&name);
            }
            Pending::None => {}
        }
    }

    // ======================================================== file dialog

    fn open_file_dialog(&mut self, mode: FdMode, kind: FileKind) {
        let files = self.files.list(kind);
        let what = if kind == FileKind::Design {
            "FileManagerDialog._design"
        } else {
            "FileManagerDialog._topography"
        };
        let (title, action, cancel) = match mode {
            FdMode::Open => (
                "FileManagerDialog.Open_a",
                "FileManagerDialog.Open",
                "FileManagerDialog.Cancel",
            ),
            FdMode::Save => (
                "FileManagerDialog.Save_a",
                "FileManagerDialog.Save",
                "FileManagerDialog.Cancel",
            ),
            FdMode::Delete => (
                "FileManagerDialog.Delete_a",
                "FileManagerDialog.Delete",
                "FileManagerDialog.Close",
            ),
            FdMode::New => (
                "FileManagerDialog.New0",
                "FileManagerDialog.New",
                "FileManagerDialog.Cancel",
            ),
        };
        let heading = if kind == FileKind::Design {
            "FileManagerDialog.List_of_your_designs"
        } else {
            "FileManagerDialog.List_of_your_topographies"
        };
        let raw_what = self.msgs.raw(self.lang, what);
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_fd_title(sstr(&(self.msgs.raw(self.lang, title) + &raw_what)));
            s.set_fd_path(sstr(&self.files.project_dir.to_string_lossy()));
            s.set_fd_heading(sstr(&self.tr(heading)));
            s.set_fd_action_label(sstr(&self.tr(action)));
            s.set_fd_cancel_label(sstr(&self.tr(cancel)));
            s.set_fd_name_editable(mode == FdMode::Save || mode == FdMode::New);
            s.set_fd_name(sstr(""));
            s.set_fd_status(sstr(&if files.is_empty() {
                self.tr("FileManagerDisk.No_files_found")
            } else {
                String::new()
            }));
            s.set_fd_files(crate::app_strings(files.clone()));
            s.set_fd_visible(true);
        }
        self.fd = Some(FileDialogState { mode, kind, files });
    }

    fn set_fd_name(&self, name: &str) {
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_fd_name(sstr(name));
        }
    }

    fn close_file_dialog(&mut self) {
        self.fd = None;
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_fd_visible(false);
        }
    }

    pub fn fd_pick(&mut self, index: usize, double: bool) {
        let Some(name) = self.fd.as_ref().and_then(|f| f.files.get(index).cloned()) else {
            return;
        };
        self.set_fd_name(&name);
        if double {
            self.fd_action();
        }
    }

    pub fn fd_browse(&mut self) {
        if let Some(dir) = rfd::FileDialog::new()
            .set_directory(&self.files.project_dir)
            .pick_folder()
        {
            self.files.project_dir = dir;
            if let Some(fd) = &self.fd {
                let (mode, kind) = (fd.mode, fd.kind);
                let name = self
                    .ui()
                    .map(|u| u.global::<AppState>().get_fd_name().to_string())
                    .unwrap_or_default();
                self.open_file_dialog(mode, kind);
                self.set_fd_name(&name);
            }
        }
    }

    pub fn fd_action(&mut self) {
        let Some(fd) = &self.fd else {
            return;
        };
        let (mode, kind) = (fd.mode, fd.kind);
        let exists = |n: &str| fd.files.iter().any(|f| f == n);
        let name = self
            .ui()
            .map(|u| u.global::<AppState>().get_fd_name().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            return;
        }
        match mode {
            FdMode::Open => {
                let content = self.files.read(&name, kind);
                self.close_file_dialog();
                let app = self.app_properties();
                let locale = self.locale();
                let new_name = support::already_loaded_name(&name, &self.open_names(kind));
                let proj = if kind == FileKind::Design {
                    let mut d = Design::new(&new_name, &content, &app, locale);
                    d.refresh(&self.db);
                    Proj::Design(d)
                } else {
                    Proj::Topo(Topographie::new(&new_name, &content, &app, locale))
                };
                self.add_project(proj);
            }
            FdMode::New => {
                let ex = exists(&name);
                self.close_file_dialog();
                if ex {
                    let text = format!(
                        "{}\n{}",
                        self.tr("FileManagerDialog.This_file_already_exists._It_will_be_overwritten_if_you_click_on_Yes."),
                        self.tr("FileManagerDialog.Click_no_to_abort.")
                    );
                    let title = self.tr("FileManagerDialog.Existing_File...");
                    self.message(&title, &text, 1, Pending::NewOverwrite(name));
                } else {
                    self.create_topography(&name);
                }
            }
            FdMode::Save => {
                let ex = exists(&name);
                self.close_file_dialog();
                if ex {
                    let text = self.tr("FileManagerDialog.The_file")
                        + &name
                        + &self.msgs.raw(
                            self.lang,
                            "FileManagerDialog._exists._Do_you_wish_to_overwrite_it",
                        );
                    let title = self.tr("FileManagerDialog.Overwrite_File");
                    self.message(&title, &text, 1, Pending::SaveOverwrite(name));
                } else {
                    self.write_as(&name);
                }
            }
            FdMode::Delete => {
                if let Err(e) = self.files.delete(&name, kind) {
                    if let Some(ui) = self.ui() {
                        ui.global::<AppState>().set_fd_status(sstr(&format!(
                            "{} ({e})",
                            self.tr("FileManagerDisk.Can__t_delete_this_file")
                        )));
                    }
                    return;
                }
                self.open_file_dialog(FdMode::Delete, kind);
            }
        }
    }

    pub fn fd_cancel(&mut self) {
        self.close_file_dialog();
    }

    /// Open a `.tpo` or `.dsg` file given by its path (command line).
    pub fn open_path(&mut self, path: &std::path::Path) {
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let name = neatwork_core::project_name(&file);
        let app = self.app_properties();
        let locale = self.locale();
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            self.files.project_dir = dir.to_path_buf();
        }
        if file.ends_with(".dsg") {
            let name = support::already_loaded_name(&name, &self.open_names(FileKind::Design));
            let mut d = Design::new(&name, &content, &app, locale);
            d.refresh(&self.db);
            self.add_project(Proj::Design(d));
        } else if file.ends_with(".tpo") {
            let name = support::already_loaded_name(&name, &self.open_names(FileKind::Topo));
            self.add_project(Proj::Topo(Topographie::new(&name, &content, &app, locale)));
        }
    }

    fn create_topography(&mut self, name: &str) {
        let app = self.app_properties();
        let t = Topographie::new(name, "", &app, self.locale());
        self.add_project(Proj::Topo(t));
    }

    /// Save: write over the file of the same name, or ask a name when there
    /// is no such file.
    fn save(&mut self) {
        let Some(p) = self.cur() else {
            return;
        };
        let existing = self.files.list(p.kind());
        if existing.iter().any(|f| f == p.name()) {
            let (name, content, kind) = (p.name().to_string(), p.content(), p.kind());
            if let Err(e) = self.files.write(&name, &content, kind) {
                self.message(&self.tr("Gui.Error"), &e.to_string(), 0, Pending::None);
            }
        } else {
            self.menu("save-as");
        }
    }

    fn write_as(&mut self, name: &str) {
        let Some(i) = self.current else {
            return;
        };
        let (content, kind) = (self.projects[i].content(), self.projects[i].kind());
        match self.files.write(name, &content, kind) {
            Ok(()) => {
                self.projects[i].set_name(name);
                if kind == FileKind::Design {
                    self.projects[i].sims = self.files.simulations(name);
                }
                self.refresh();
            }
            Err(e) => self.message(&self.tr("Gui.Error"), &e.to_string(), 0, Pending::None),
        }
    }

    // ============================================================ tables

    fn table_mut(&mut self, id: i32) -> Option<&mut Table> {
        match id {
            0 => self.cur_mut().map(|p| &mut p.nodes),
            1 => self.cur_mut().map(|p| &mut p.pipes),
            10 => Some(&mut self.db_diameters),
            11 => Some(&mut self.db_orifices),
            20 => self.md.as_mut().map(|m| &mut m.diameters),
            21 => self.md.as_mut().map(|m| &mut m.orifices),
            22 => self.md.as_mut().map(|m| &mut m.load_factors),
            30 => Some(&mut self.faucets),
            _ => None,
        }
    }

    pub fn push_table(&mut self, id: i32) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<AppState>();
        match id {
            0 | 1 => self.refresh_tables(),
            10 | 11 => self.push_database(),
            20..=23 => self.push_make_design(),
            30 => s.set_ts_table(self.faucets.plain()),
            _ => {}
        }
    }

    pub fn push_database(&self) {
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_db_diameters(self.db_diameters.plain());
            s.set_db_orifices(self.db_orifices.plain());
        }
    }

    pub fn table_row_clicked(&mut self, id: i32, row: usize, ctrl: bool, shift: bool) {
        if let Some(t) = self.table_mut(id) {
            t.click(row, ctrl, shift);
        }
        self.push_table(id);
    }

    pub fn table_edited(&mut self, id: i32, row: usize, col: usize, text: &str) {
        // diameter of a design pipe chosen in the list
        if let Some(idx) = text.strip_prefix('#').and_then(|n| n.parse::<usize>().ok()) {
            self.pick_diameter(row, col, idx);
            self.push_table(id);
            return;
        }
        let mut value = text.to_string();
        // TopoNodeTableModel: a node name stops at its first '_'
        if id == 0
            && col == 0
            && self.cur().map(OpenProject::kind) == Some(FileKind::Topo)
            && let Some(k) = value.find('_').filter(|k| *k > 0)
        {
            value.truncate(k);
        }
        let ok = match self.table_mut(id) {
            Some(t) => !matches!(t.edit(row, col, &value), Edit::Invalid),
            None => false,
        };
        if ok
            && id == 22
            && let Some(md) = &mut self.md
            && let Some(r) = md.load_factors.rows.get(row)
            && let Some(v) = parse_double(&r[4])
        {
            md.modified.put(r[1].clone(), v);
        }
        self.push_table(id);
    }

    /// `DesignPipeModel.setValueAt` for a diameter column: reuse the design
    /// reference of an equal diameter, or add the catalog entry.
    fn pick_diameter(&mut self, row: usize, col: usize, idx: usize) {
        let Some(d) = self.db.diameters().get(idx).cloned() else {
            return;
        };
        let Some(i) = self.current else {
            return;
        };
        let p = &mut self.projects[i];
        let Proj::Design(design) = &mut p.proj else {
            return;
        };
        let found = design
            .diam_table
            .entries()
            .find(|(_, x)| x.diameter == d.diameter && x.sdr == d.sdr && x.pipe_type == d.pipe_type)
            .map(|(k, _)| k.clone());
        let reference = match found {
            Some(r) => r,
            None => {
                let mut n = design.diam_table.len() + 1;
                while design.diam_table.contains_key(&format!("D{n}")) {
                    n += 1;
                }
                let r = format!("D{n}");
                design.diam_table.put(r.clone(), d);
                r
            }
        };
        let _ = p.pipes.edit(row, col, &reference);
    }

    pub fn table_action(&mut self, id: i32, action: &str) {
        match (id, action) {
            (0 | 1, "apply") => self.apply_tables(),
            (0 | 1, "undo") => {
                self.reload_tables();
                self.refresh_tables();
            }
            (0 | 1, "insert") => {
                let row = self.new_row(id);
                if let Some(t) = self.table_mut(id) {
                    t.insert(row);
                }
                self.push_table(id);
            }
            (10, "apply") | (11, "apply") => self.save_database(),
            (10, "undo") => {
                self.db_diameters = self.db_diameter_table();
                self.push_database();
            }
            (11, "undo") => {
                self.db_orifices = self.db_orifice_table();
                self.push_database();
            }
            (10, "insert") => {
                self.db_diameters.insert(
                    ["", "0.0", "0.0", "0.0", "0.0", "1", "0.0"]
                        .map(String::from)
                        .to_vec(),
                );
                self.push_database();
            }
            (11, "insert") => {
                self.db_orifices.insert(vec!["0.0".to_string()]);
                self.push_database();
            }
            (10, "export") | (11, "export") => {
                let html = support::database_report(self, id == 10);
                self.save_html(&html);
            }
            (20 | 21 | 30, "select-all") | (20 | 21 | 30, "select-none") => {
                let v = action == "select-all";
                if let Some(t) = self.table_mut(id) {
                    t.set_checks(0, v);
                }
                self.push_table(id);
            }
            (22, "undo") => {
                if let Some(md) = &mut self.md {
                    md.modified = JHashtable::from_hashtable(&md.suggested);
                }
                self.rebuild_load_factors();
            }
            (23, "apply") => self.md_apply_params(),
            (23, "undo") => {
                if let Some(t) = self.md_topo() {
                    let params = self.md_params(&t);
                    if let Some(md) = &mut self.md {
                        md.params = params;
                    }
                }
                self.push_make_design();
            }
            (_, "delete") => {
                if let Some(t) = self.table_mut(id) {
                    t.delete();
                }
                self.push_table(id);
            }
            (_, "copy") => {
                if let Some(t) = self.table_mut(id) {
                    let text = t.copy_text();
                    if let Ok(mut c) = arboard::Clipboard::new() {
                        let _ = c.set_text(text);
                    }
                }
            }
            (_, "paste") => self.paste(id),
            _ => {}
        }
    }

    fn paste(&mut self, id: i32) {
        let text = arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .unwrap_or_default();
        let design_pipes = id == 1 && self.cur().map(OpenProject::kind) == Some(FileKind::Design);
        let extra: &[&str] = if design_pipes { &["N"] } else { &[] };
        let ok = self.table_mut(id).map(|t| t.paste(&text, extra).is_ok());
        if ok == Some(false) {
            let title = self.tr("FancyTable.Format_error");
            let msg = self.tr("FancyTable.Clipboard_is_not_well_formated");
            self.message(&title, &msg, 0, Pending::None);
        }
        self.push_table(id);
    }

    /// New row of a project table (`getNewVector`).
    fn new_row(&self, id: i32) -> Vec<String> {
        let Some(p) = self.cur() else {
            return Vec::new();
        };
        let design = p.kind() == FileKind::Design;
        if id == 0 {
            let names: Vec<String> = p.nodes.rows.iter().map(|r| r[0].clone()).collect();
            let name = support::new_node_name(&names);
            let mut row = vec![name, "0.0".into(), "0.0".into(), "0.0".into()];
            if design {
                row.extend(["0.0".to_string(), "0.0".to_string(), "1".to_string()]);
            } else {
                row.extend(["0".to_string(), "1".to_string()]);
            }
            row
        } else {
            let last = p
                .network()
                .nodes
                .last()
                .map(|n| n.name.clone())
                .unwrap_or_default();
            if design {
                vec![
                    last.clone(),
                    last,
                    "0.0".into(),
                    "0.0".into(),
                    "0".into(),
                    "0.0".into(),
                    "0".into(),
                    "N".into(),
                ]
            } else {
                vec![last.clone(), last, "0.0".into()]
            }
        }
    }

    /// Apply of the Tables tab: rebuild the project text and parse it.
    fn apply_tables(&mut self) {
        let Some(i) = self.current else {
            return;
        };
        let p = &self.projects[i];
        let csv = |t: &Table| -> String { t.rows.iter().map(|r| r.join(",") + "\n").collect() };
        let content = match &p.proj {
            Proj::Topo(t) => t.properties_content() + &csv(&p.nodes) + &csv(&p.pipes),
            // The Java code drops the orifice list here; the Rust interface
            // keeps it.
            Proj::Design(d) => {
                d.properties_content()
                    + &csv(&p.nodes)
                    + &csv(&p.pipes)
                    + &d.diameters_content()
                    + &d.orifices_content()
            }
        };
        let info = match &mut self.projects[i].proj {
            Proj::Topo(t) => {
                t.set_content(&content);
                t.network.info.clone()
            }
            Proj::Design(d) => {
                d.set_content(&content);
                d.network.info.clone()
            }
        };
        self.reload_tables();
        self.refresh();
        if !info.is_empty() {
            let title = self.tr("TopoTablePane.error_in_your_project");
            self.message(&title, &info.join("\n"), 0, Pending::None);
        }
    }

    /// Apply of the Text tab.
    pub fn text_apply(&mut self, text: &str) {
        let Some(i) = self.current else {
            return;
        };
        let (info, error) = match &mut self.projects[i].proj {
            Proj::Topo(t) => {
                t.set_content(text);
                (t.network.info.clone(), t.network.has_error)
            }
            Proj::Design(d) => {
                d.set_content(text);
                (d.network.info.clone(), d.network.has_error)
            }
        };
        self.reload_tables();
        self.refresh_tables();
        self.refresh_tree();
        self.refresh_network();
        if error {
            self.refresh_text();
        }
        if !info.is_empty() {
            let title = self.tr("TextPane.error_in_your_project");
            self.message(&title, &info.join("\n"), 0, Pending::None);
        }
    }

    // ========================================================== database

    fn save_database(&mut self) {
        let csv = |t: &Table| -> String { t.rows.iter().map(|r| r.join(",") + "\n").collect() };
        let diam = csv(&self.db_diameters);
        let orif = csv(&self.db_orifices);
        self.db = Database::new(&diam, &orif);
        let r1 = self.files.write_db("diameters.db", &diam);
        let r2 = self.files.write_db("orifices.db", &orif);
        self.db_diameters = self.db_diameter_table();
        self.db_orifices = self.db_orifice_table();
        // FrameNeatwork: every open design takes the new catalog
        for p in &mut self.projects {
            if let Proj::Design(d) = &mut p.proj {
                d.refresh(&self.db);
            }
        }
        self.push_database();
        self.refresh();
        if let Err(e) = r1.and(r2) {
            self.message(&self.tr("Gui.Error"), &e.to_string(), 0, Pending::None);
        }
    }

    // ======================================================== quick check

    fn open_quick_check(&mut self) {
        let Some(Proj::Topo(t)) = self.cur().map(|p| &p.proj) else {
            return;
        };
        let threshold = t
            .properties
            .get_property("topo.seuil.value")
            .unwrap_or("0.1")
            .to_string();
        let faucet = t
            .properties
            .get_property("topo.faucetcoef.value")
            .unwrap_or("0.1")
            .to_string();
        self.quick_check = support::quick_check_table(self, t);
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_qc_threshold(sstr(&threshold));
            s.set_qc_faucet(sstr(&faucet));
            s.set_qc_table(self.quick_check.plain());
            s.set_qc_visible(true);
        }
    }

    pub fn qc_apply(&mut self) {
        let Some(ui) = self.ui() else {
            return;
        };
        let s = ui.global::<AppState>();
        let (th, fc) = (
            s.get_qc_threshold().to_string(),
            s.get_qc_faucet().to_string(),
        );
        if let Some(Proj::Topo(t)) = self.cur_mut().map(|p| &mut p.proj) {
            let mut p = neatwork_core::java::JProperties::new();
            p.set_property("topo.seuil.value", &th);
            p.set_property("topo.faucetcoef.value", &fc);
            t.properties.put_all(&p);
        }
        if let Some(Proj::Topo(t)) = self.cur().map(|p| &p.proj) {
            self.quick_check = support::quick_check_table(self, t);
        }
        s.set_qc_table(self.quick_check.plain());
        self.refresh_text();
    }

    // ===================================================== design dialogs

    fn units(&mut self) {
        let mut t = Table::new(
            vec![
                Col::new(self.tr("UnitDialog.Measure"), 200.0, false, Kind::Text),
                Col::new(self.tr("FrameNeatwork.Units"), 220.0, false, Kind::Text),
            ],
            false,
        );
        let app = self.app_properties();
        let rows = app
            .property_names()
            .into_iter()
            .filter(|k| k.starts_with("units."))
            .map(|k| {
                let base = k.trim_end_matches(".value").to_string();
                vec![
                    self.tr(&format!("{base}.name")),
                    self.tr(&format!("{base}.value")),
                ]
            })
            .collect();
        t.set_rows(rows);
        self.show_table("FrameNeatwork.Units", "", t);
    }

    fn show_table(&mut self, title_key: &str, text: &str, table: Table) {
        self.dialog_table = table;
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_td_title(sstr(&self.tr(title_key)));
            s.set_td_text(sstr(text));
            s.set_td_table(self.dialog_table.plain());
            s.set_td_visible(true);
        }
    }

    fn design_parameters(&mut self) {
        let Some(Proj::Design(d)) = self.cur().map(|p| &p.proj) else {
            return;
        };
        let mut t = Table::new(
            vec![
                Col::new(
                    self.tr("ParametersTableModel.Name"),
                    260.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ParametersTableModel.Value"),
                    120.0,
                    false,
                    Kind::Text,
                ),
            ],
            false,
        );
        t.set_rows(
            d.properties
                .property_names()
                .into_iter()
                .filter(|k| k.starts_with("topo.") && k.ends_with(".value"))
                .map(|k| {
                    let v = d.properties.get_property(&k).unwrap_or("").to_string();
                    vec![self.tr(&k.replace(".value", ".name")), v]
                })
                .collect(),
        );
        let text = self.tr("DesignParaDialog.These_are_the_parameters_used_by")
            + &self.tr("DesignParaDialog.the_MAKE_DESIGN_operation_for_building")
            + &self.msgs.raw(self.lang, "DesignParaDialog._this_design");
        self.show_table("DesignParaDialog.Building_Parameters", &text, t);
    }

    fn design_load_factors(&mut self) {
        let Some(Proj::Design(d)) = self.cur().map(|p| &p.proj) else {
            return;
        };
        let source = d
            .network
            .source()
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let mut rows: Vec<Vec<String>> = d
            .load_factor
            .entries()
            .filter(|(k, _)| **k != source)
            .map(|(k, v)| vec![k.clone(), v.clone()])
            .collect();
        // The Java dialog shows "!topo.<i>.name!" instead of the node
        // names; the Rust interface shows the names, sorted.
        rows.sort_by(|a, b| neatwork_core::java::compare_utf16(&a[0], &b[0]));
        let mut t = Table::new(
            vec![
                Col::new(
                    self.tr("DesignLoadFactorModel.Ending_Node"),
                    200.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("DesignLoadFactorModel.Load_Factor"),
                    140.0,
                    false,
                    Kind::Text,
                ),
            ],
            false,
        );
        t.set_rows(rows);
        let text = self.tr("DesignLoadFactorDialog.These_are_the_load_factors_used_by")
            + " "
            + &self.tr("DesignLoadFactorDialog.the_MAKE_DESIGN_operation_for_building")
            + &self
                .msgs
                .raw(self.lang, "DesignLoadFactorDialog._this_design");
        self.show_table("DesignLoadFactorDialog.Building_Load_Factors", &text, t);
    }

    fn extract_topography(&mut self) {
        let Some(Proj::Design(d)) = self.cur().map(|p| &p.proj) else {
            return;
        };
        if !support::is_tree(&d.network) {
            let text = self.tr("FrameNeatwork.This_feature_is_not_valid_for_non-tree_design");
            self.message("", &text, 0, Pending::None);
            return;
        }
        let mut d = d.clone();
        let content = support::extract_topo_content(&mut d);
        let name = support::already_loaded_name(&d.name, &self.open_names(FileKind::Topo));
        let t = Topographie::new(&name, &content, &self.app_properties(), self.locale());
        self.add_project(Proj::Topo(t));
    }

    // ======================================================= make design

    fn md_topo(&self) -> Option<Topographie> {
        let md = self.md.as_ref()?;
        match &self.projects.get(md.project)?.proj {
            Proj::Topo(t) => Some(t.clone()),
            Proj::Design(_) => None,
        }
    }

    fn start_design(&mut self) {
        let Some(i) = self.current else {
            return;
        };
        let Proj::Topo(t) = &mut self.projects[i].proj else {
            return;
        };
        let content = t.content();
        t.set_content(&content);
        if !support::is_tree(&t.network) {
            let (title, text) = (
                self.tr("FrameNeatwork.Not_a_tree"),
                self.tr("FrameNeatwork.The_topography_must_be_a_tree__!"),
            );
            self.message(&title, &text, 0, Pending::None);
            return;
        }
        if support::min_length(&t.network).partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            let (title, text) = (
                self.tr("FrameNeatwork.Bad_length"),
                self.tr("FrameNeatwork.All_pipe_lengths_must_be_greater_than_0__!"),
            );
            self.message(&title, &text, 0, Pending::None);
            return;
        }
        let setup = match prepare_make_design(t) {
            Ok(s) => s,
            Err(e) => {
                self.message(&self.tr("Gui.Error"), &e.to_string(), 0, Pending::None);
                return;
            }
        };
        let t = t.clone();
        let mut diameters = Table::new(
            vec![
                Col::new(" ".into(), 28.0, true, Kind::Check),
                Col::new(
                    self.tr("ChooseDiameterTableModel.Nominal"),
                    70.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ChooseDiameterTableModel.SDR"),
                    55.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ChooseDiameterTableModel.Diam"),
                    70.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ChooseDiameterTableModel.Cost"),
                    60.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ChooseDiameterTableModel.Max_Pressure"),
                    100.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ChooseDiameterTableModel.Type"),
                    50.0,
                    false,
                    Kind::Text,
                ),
                Col::new(
                    self.tr("ChooseDiameterTableModel.Roughness"),
                    85.0,
                    false,
                    Kind::Text,
                ),
            ],
            false,
        );
        diameters.set_rows(
            self.db
                .diameters()
                .iter()
                .map(|d| {
                    let mut r = vec!["false".to_string()];
                    r.extend(d.selection_line().split(',').map(str::to_string));
                    r
                })
                .collect(),
        );
        let mut orifices = Table::new(
            vec![
                Col::new(" ".into(), 28.0, true, Kind::Check),
                Col::new(
                    self.tr("ChooseOrificeTableModel.Diameter"),
                    120.0,
                    false,
                    Kind::Text,
                ),
            ],
            false,
        );
        orifices.set_rows(
            self.db
                .orifices()
                .iter()
                .map(|o| vec!["false".to_string(), double_to_string(*o)])
                .collect(),
        );
        let lf = |k: &str| t.properties.get_property(k).unwrap_or("").to_string();
        let mut md = MakeDesignState {
            project: i,
            hsource: setup.hsource,
            diameters,
            orifices,
            load_factors: Table::default(),
            modified: JHashtable::from_hashtable(&setup.load_factors),
            suggested: setup.load_factors,
            lf_params: (
                lf("topo.targetflow.value"),
                lf("topo.opentaps.value"),
                lf("topo.servicequal.value"),
            ),
            params: self.md_params(&t),
            pipes: t
                .expanded_pipes
                .iter()
                .map(|p| (p.begin.clone(), p.end.clone()))
                .collect(),
            constraints: Vec::new(),
        };
        md.load_factors = self.load_factor_table(&t, &md);
        self.md = Some(md);
        self.push_make_design();
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_md_tab(0);
            s.set_md_pipe(0);
            s.set_md_kind(0);
            s.set_md_value(sstr(""));
            s.set_md_length(sstr(""));
            s.set_md_constraint_selected(-1);
            s.set_md_visible(true);
        }
    }

    pub fn md_param_edited(&mut self, index: usize, text: &str) {
        if let Some(md) = &mut self.md
            && let Some(p) = md.params.get_mut(index)
        {
            p.1 = text.to_string();
        }
    }

    /// `ParametersPane` Apply: write the numbers to the topography, then
    /// recompute the load factors if their parameters changed.
    fn md_apply_params(&mut self) {
        let Some(md) = &self.md else {
            return;
        };
        let mut p = neatwork_core::java::JProperties::new();
        for ((h, k), (_, v)) in crate::app::PARAMS.iter().zip(&md.params) {
            if h.is_none()
                && let Some(x) = parse_double(v).filter(|x| *x >= 0.0)
            {
                p.set_property(k, &double_to_string(x));
            }
        }
        let project = md.project;
        if let Some(Proj::Topo(t)) = self.projects.get_mut(project).map(|p| &mut p.proj) {
            t.properties.put_all(&p);
        }
        self.rebuild_load_factors();
        self.refresh_text();
    }

    /// `LoadFactorTableModel.updateData`: new suggested factors when the
    /// target flow, open faucet fraction or service quality changed.
    fn rebuild_load_factors(&mut self) {
        let Some(t) = self.md_topo() else {
            return;
        };
        let lf = |k: &str| t.properties.get_property(k).unwrap_or("").to_string();
        let now = (
            lf("topo.targetflow.value"),
            lf("topo.opentaps.value"),
            lf("topo.servicequal.value"),
        );
        if let Some(md) = &mut self.md
            && md.lf_params != now
            && let Ok(taps) = t.load_taps()
            && let Ok(f) = t.load_factor(&taps)
        {
            md.modified = JHashtable::from_hashtable(&f);
            md.suggested = f;
            md.lf_params = now;
        }
        if let Some(md) = self.md.take() {
            let table = self.load_factor_table(&t, &md);
            self.md = Some(MakeDesignState {
                load_factors: table,
                ..md
            });
        }
        self.push_make_design();
    }

    pub fn md_add_constraint(&mut self) {
        let (Some(ui), Some(md)) = (self.ui(), &self.md) else {
            return;
        };
        let s = ui.global::<AppState>();
        let Some((begin, end)) = usize::try_from(s.get_md_pipe())
            .ok()
            .and_then(|i| md.pipes.get(i))
            .cloned()
        else {
            return;
        };
        let pipe = format!("Arc : {begin} -> {end}");
        let diams = self.db.diameters();
        let pick = |i: i32| {
            usize::try_from(i)
                .ok()
                .and_then(|i| diams.get(i))
                .map(|d| d.diameter)
        };
        let value = s.get_md_value().to_string();
        let item = match s.get_md_kind() {
            0 | 1 => {
                let Some(d) = parse_double(&value) else {
                    return;
                };
                let (key, c) = if s.get_md_kind() == 0 {
                    (
                        "ConstraintsListModel.]_pipe(s)_must_be_greater_than",
                        Constraint::MinDiameter {
                            begin,
                            end,
                            diameter: d,
                        },
                    )
                } else {
                    (
                        "ConstraintsListModel.]_pipe(s)_must_be_lower_than",
                        Constraint::MaxDiameter {
                            begin,
                            end,
                            diameter: d,
                        },
                    )
                };
                (
                    c,
                    format!("[ {pipe}{}{value}", self.msgs.raw(self.lang, key)),
                )
            }
            2 => {
                let Some(d) = pick(s.get_md_diam1()) else {
                    return;
                };
                let text = format!(
                    "[ {pipe}{}{}",
                    self.msgs
                        .raw(self.lang, "ConstraintsListModel.]_pipe(s)_must_be_equal_to"),
                    double_to_string(d)
                );
                (
                    Constraint::Fixed {
                        begin,
                        end,
                        diameter: d,
                    },
                    text,
                )
            }
            _ => {
                let (Some(d1), Some(d2)) = (pick(s.get_md_diam1()), pick(s.get_md_diam2())) else {
                    return;
                };
                let len = s.get_md_length().to_string();
                let Some(l1) = parse_double(&len) else {
                    return;
                };
                let text = format!(
                    "[ {pipe}{}{}{}{len}{}{})",
                    self.msgs
                        .raw(self.lang, "ConstraintsListModel.]_pipe_1_is_(diam"),
                    double_to_string(d1),
                    self.msgs.raw(self.lang, "ConstraintsListModel._and_length"),
                    self.msgs
                        .raw(self.lang, "ConstraintsListModel.)_and_pipe_2_is_(diam"),
                    double_to_string(d2)
                );
                (
                    Constraint::Split {
                        begin,
                        end,
                        diameter1: d1,
                        length1: l1,
                        diameter2: d2,
                    },
                    text,
                )
            }
        };
        if let Some(md) = &mut self.md {
            md.constraints.push(item);
        }
        self.push_make_design();
    }

    pub fn md_delete_constraint(&mut self) {
        let sel = self
            .ui()
            .map_or(-1, |u| u.global::<AppState>().get_md_constraint_selected());
        if let (Some(md), Ok(i)) = (&mut self.md, usize::try_from(sel))
            && i < md.constraints.len()
        {
            md.constraints.remove(i);
        }
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_md_constraint_selected(-1);
        }
        self.push_make_design();
    }

    pub fn md_abort(&mut self) {
        self.md = None;
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_md_visible(false);
        }
    }

    /// "Design": check the selection, then run the make design in the
    /// background with the progress dialog.
    pub fn md_run(&mut self) {
        let Some(md) = &self.md else {
            return;
        };
        let checked = |t: &Table| -> String {
            t.rows
                .iter()
                .filter(|r| r[0] == "true")
                .map(|r| r[1..].join(",") + "\n")
                .collect()
        };
        let diameters = checked(&md.diameters);
        if diameters.is_empty() {
            let msg = self.tr("MakeDesignDialog.You_must_choose_at_least_one_diameter");
            self.message("", &msg, 0, Pending::None);
            return;
        }
        let orifices = checked(&md.orifices);
        let Some(mut topo) = self.md_topo() else {
            return;
        };
        let (hsource, modified) = (md.hsource, md.modified.clone());
        let constraints: Vec<Constraint> = md.constraints.iter().map(|(c, _)| c.clone()).collect();
        let app = self.app_properties();
        self.md_abort();
        let name = topo.name.clone();
        self.start_progress("SolverDisk.Building_Design..");
        std::thread::spawn(move || {
            let mut optimal = true;
            let mut solve = |lp: &neatwork_core::Lp| {
                let s = lp.solve()?;
                optimal = s.optimal;
                Ok(s.x)
            };
            let result = make_design(
                &mut topo,
                hsource,
                &diameters,
                &orifices,
                &modified,
                &constraints,
                &app,
                &mut solve,
            )
            .map(|m| (Box::new(m), optimal))
            .map_err(|e| e.to_string());
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|app| {
                    app.finished = Some(Finished::Design {
                        project_name: name,
                        result,
                    });
                    app.set_progress(100, "SolverDisk.Completed.");
                });
            });
        });
        let _ = solve_with_highs;
    }

    fn start_progress(&self, status_key: &str) {
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_pg_value(0);
            s.set_pg_status(sstr(&self.tr(status_key)));
            s.set_pg_visible(true);
        }
    }

    pub fn set_progress(&self, value: i32, status_key: &str) {
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_pg_value(value);
            if !status_key.is_empty() {
                s.set_pg_status(sstr(&self.tr(status_key)));
            }
        }
    }

    /// Close of the progress dialog: show the result.
    pub fn pg_close(&mut self) {
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_pg_visible(false);
        }
        match self.finished.take() {
            Some(Finished::Design {
                project_name,
                result,
            }) => match result {
                Ok((made, true)) => {
                    // NodeTapPressureDialog, then the new design
                    let mut t = Table::new(
                        vec![
                            Col::new(
                                self.tr("NodeTapPressureModel.Node_ID"),
                                120.0,
                                false,
                                Kind::Text,
                            ),
                            Col::new(
                                self.tr("NodeTapPressureModel.Height"),
                                90.0,
                                false,
                                Kind::Text,
                            ),
                            Col::new(
                                self.tr("NodeTapPressureModel.Forecast_Pressure_Loss"),
                                170.0,
                                false,
                                Kind::Text,
                            ),
                            Col::new(
                                self.tr("NodeTapPressureModel.Potential_Problem"),
                                150.0,
                                false,
                                Kind::Text,
                            ),
                        ],
                        false,
                    );
                    t.set_rows(made.pressures.clone());
                    let mut design = made.design.clone();
                    let files = self.files.list(FileKind::Design);
                    let n1 = support::first_compatible_name(&project_name, &files);
                    design.name =
                        support::already_loaded_name(&n1, &self.open_names(FileKind::Design));
                    self.design_to_add = Some(design);
                    self.show_table("NodeTapPressureDialog.Nodes_&_Faucets_Pressure", "", t);
                }
                Ok((_, false)) => {
                    let (title, text) = (
                        self.tr("FrameNeatwork.Optimization_warning"),
                        self.tr("FrameNeatwork.there_is_no_feasible_solution__!"),
                    );
                    self.message(&title, &text, 0, Pending::None);
                }
                Err(e) => self.message(&self.tr("Gui.Error"), &e, 0, Pending::None),
            },
            Some(Finished::Simulation { project, result }) => match result {
                Ok(content) => self.store_simulation(project, &content),
                Err(e) => self.message(&self.tr("Gui.Error"), &e, 0, Pending::None),
            },
            None => {}
        }
    }

    pub fn td_close(&mut self) {
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_td_visible(false);
        }
        if let Some(d) = self.design_to_add.take() {
            self.add_project(Proj::Design(d));
        }
    }

    // ======================================================== simulation

    fn new_simulation(&mut self) {
        let Some(i) = self.current else {
            return;
        };
        let p = &self.projects[i];
        let Proj::Design(d) = &p.proj else {
            return;
        };
        if !self
            .files
            .list(FileKind::Design)
            .iter()
            .any(|f| f == &d.name)
        {
            let title = self.tr("SimulationPane.Design_not_saved");
            let text = self
                .tr("SimulationPane.You_have_to_save_this_design_in_order_to_run_a_simulation.");
            self.message(&title, &text, 0, Pending::None);
            return;
        }
        let get = |k: &str| d.properties.get_property(k).unwrap_or("").to_string();
        let orifice = i32::from(get("simu.typeorifice.value") == "commercial");
        let kind = match get("simu.typesimu.value").as_str() {
            "random" => 0,
            "tapbytap" => 1,
            _ => 2,
        };
        if let Some(ui) = self.ui() {
            let s = ui.global::<AppState>();
            s.set_ns_nbsim(sstr(&get("simu.nbsim.value")));
            s.set_ns_opentaps(sstr(&get("simu.simopentaps.value")));
            s.set_ns_low(sstr(&get("simu.mincriticalflow.value")));
            s.set_ns_high(sstr(&get("simu.maxcriticalflow.value")));
            s.set_ns_target(sstr(&get("simu.targetflow.value")));
            s.set_ns_orifice(orifice);
            s.set_ns_type(kind);
            s.set_ns_visible(true);
        }
        self.ns = Some(NewSimuState { project: i });
    }

    /// Values of the dialog, as `getParameters` builds them.
    fn ns_parameters(&self) -> Option<(usize, SimulationParameters)> {
        let ns = self.ns.as_ref()?;
        let ui = self.ui()?;
        let s = ui.global::<AppState>();
        let Proj::Design(d) = &self.projects.get(ns.project)?.proj else {
            return None;
        };
        let kind = match s.get_ns_type() {
            1 => SimulationType::TapByTap,
            2 => SimulationType::Handmade,
            _ => SimulationType::Random,
        };
        let (nb_sim, open_taps) = match kind {
            SimulationType::Random => (
                s.get_ns_nbsim().to_string(),
                s.get_ns_opentaps().to_string(),
            ),
            SimulationType::TapByTap => (faucet_count(d).to_string(), "-".to_string()),
            SimulationType::Handmade => ("1".to_string(), "-".to_string()),
        };
        // A stable code, not the translated label (the Java dialog stores
        // the label).
        let orifice = if s.get_ns_orifice() == 1 {
            "commercial"
        } else {
            "ideal"
        };
        Some((
            ns.project,
            SimulationParameters {
                kind,
                nb_sim,
                open_taps,
                orifice: orifice.to_string(),
                target_flow: Some(s.get_ns_target().to_string()),
                min_critical_flow: Some(s.get_ns_low().to_string()),
                max_critical_flow: Some(s.get_ns_high().to_string()),
            },
        ))
    }

    /// The dialog values become the design's defaults, run or not.
    fn ns_keep_values(&mut self) {
        let Some((project, prm)) = self.ns_parameters() else {
            return;
        };
        if let Some(Proj::Design(d)) = self.projects.get_mut(project).map(|p| &mut p.proj) {
            let p = simulation_properties(d, &prm);
            let mut own = neatwork_core::java::JProperties::new();
            for k in [
                "simu.simopentaps.value",
                "simu.typeorifice.value",
                "simu.typesimu.value",
                "simu.nbsim.value",
                "simu.targetflow.value",
                "simu.maxcriticalflow.value",
                "simu.mincriticalflow.value",
            ] {
                own.set_property(k, p.get_property(k).unwrap_or(""));
            }
            d.properties.put_all(&own);
        }
    }

    pub fn ns_close(&mut self) {
        self.ns_keep_values();
        self.ns = None;
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_ns_visible(false);
        }
        self.refresh_text();
    }

    pub fn ns_run(&mut self) {
        let Some((project, prm)) = self.ns_parameters() else {
            return;
        };
        if prm.kind == SimulationType::Handmade {
            // TapSelectionDialog
            let Proj::Design(d) = &self.projects[project].proj else {
                return;
            };
            let mut t = Table::new(
                vec![
                    Col::new(" ".into(), 28.0, true, Kind::Check),
                    Col::new(
                        self.tr("TapSelectionModel.Faucet"),
                        160.0,
                        false,
                        Kind::Text,
                    ),
                ],
                false,
            );
            t.set_rows(
                d.network
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == TYPE_FAUCET)
                    .map(|n| vec!["false".to_string(), n.name.clone()])
                    .collect(),
            );
            self.faucets = t;
            if let Some(ui) = self.ui() {
                let s = ui.global::<AppState>();
                s.set_ts_table(self.faucets.plain());
                s.set_ts_visible(true);
            }
            return;
        }
        self.launch_simulation(HashMap::new());
    }

    pub fn ts_ok(&mut self) {
        if let Some(ui) = self.ui() {
            ui.global::<AppState>().set_ts_visible(false);
        }
        let faucet_ref: HashMap<String, bool> = self
            .faucets
            .rows
            .iter()
            .map(|r| (r[1].clone(), r[0] == "true"))
            .collect();
        self.launch_simulation(faucet_ref);
    }

    fn launch_simulation(&mut self, faucet_ref: HashMap<String, bool>) {
        let Some((project, prm)) = self.ns_parameters() else {
            return;
        };
        self.ns_close();
        let Proj::Design(d) = &self.projects[project].proj else {
            return;
        };
        let mut design = d.clone();
        let p = simulation_properties(&design, &prm);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |t| t.as_nanos() as u64);
        self.start_progress("SolverDisk.Running_simulations..");
        std::thread::spawn(move || {
            let mut rng = SplitMix64(seed);
            let mut progress = |v: i32| {
                let _ = slint::invoke_from_event_loop(move || {
                    with_app(|app| app.set_progress(v.min(99), ""))
                });
            };
            let result = run_simulation(&mut design, &p, &faucet_ref, &mut rng, &mut progress)
                .map_err(|e| e.to_string());
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|app| {
                    app.finished = Some(Finished::Simulation { project, result });
                    app.set_progress(100, "SolverDisk.Completed.");
                });
            });
        });
    }

    /// Write `<design>.<n>.sim` with n one above the largest number.
    fn store_simulation(&mut self, project: usize, content: &str) {
        let Some(p) = self.projects.get(project) else {
            return;
        };
        let name = p.name().to_string();
        let n = self.files.simulations(&name).last().map_or(0, |m| m + 1);
        let file = format!("{name}.{n}");
        if let Err(e) = self.files.write(&file, content, FileKind::Simulation) {
            self.message(&self.tr("Gui.Error"), &e.to_string(), 0, Pending::None);
            return;
        }
        let sims = self.files.simulations(&name);
        let p = &mut self.projects[project];
        p.sim_current = sims.iter().position(|x| *x == n);
        p.sims = sims;
        p.sim = Some(SimulationFile::new(
            &file,
            &self.files.read(&file, FileKind::Simulation),
        ));
        self.refresh();
    }

    fn delete_simulation(&mut self) {
        let Some(i) = self.current else {
            return;
        };
        let p = &self.projects[i];
        let Some(n) = p.sim_current.and_then(|k| p.sims.get(k)) else {
            return;
        };
        let file = format!("{}.{n}", p.name());
        let _ = std::fs::remove_file(self.files.project_dir.join(format!("{file}.sim")));
        let name = p.name().to_string();
        let p = &mut self.projects[i];
        p.sims = self.files.simulations(&name);
        p.sim_current = None;
        p.sim = None;
        if !p.sims.is_empty() {
            self.select_simulation(0);
        }
        self.refresh();
    }
}
