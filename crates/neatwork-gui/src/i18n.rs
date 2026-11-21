//! Texts of the interface, read from the Java message bundles of NeatWork
//! 2024 (`neatwork_*.properties`), as `neatwork.Messages` does.
//!
//! The Rust interface adds a few features (the network drawing). Their
//! texts are in [`EXTRA`], with keys that start with `Gui.`.

use std::collections::HashMap;

use neatwork_core::java::JProperties;

/// Language index used by the `T` global of the `.slint` files.
pub const EN: i32 = 0;
pub const FR: i32 = 1;
pub const SP: i32 = 2;

const BASE: &[u8] = include_bytes!("../resources/neatwork.properties");
const BUNDLE_EN: &[u8] = include_bytes!("../resources/neatwork_en.properties");
const BUNDLE_FR: &[u8] = include_bytes!("../resources/neatwork_fr.properties");
const BUNDLE_SP: &[u8] = include_bytes!("../resources/neatwork_sp.properties");

/// Texts of the features that the Java program does not have:
/// (key, English, French, Spanish).
const EXTRA: &[(&str, &str, &str, &str)] = &[
    ("Gui.Network", "Network", "Réseau", "Red"),
    ("Gui.Layout", "Layout", "Disposition", "Disposición"),
    ("Gui.Layout_tree", "Tree", "Arbre", "Árbol"),
    ("Gui.Layout_profile", "Profile", "Profil", "Perfil"),
    (
        "Gui.Layout_coordinates",
        "Coordinates",
        "Coordonnées",
        "Coordenadas",
    ),
    ("Gui.Color", "Color", "Couleur", "Color"),
    ("Gui.Zoom", "Zoom", "Zoom", "Zoom"),
    ("Gui.Fit", "Fit", "Ajuster", "Ajustar"),
    ("Gui.Legend", "Legend", "Légende", "Leyenda"),
    (
        "Gui.Color_type",
        "Node type",
        "Type de nœud",
        "Tipo de nodo",
    ),
    ("Gui.Color_height", "Height", "Altitude", "Altura"),
    (
        "Gui.Color_diameter",
        "Pipe diameter",
        "Diamètre des tuyaux",
        "Diámetro de los tubos",
    ),
    (
        "Gui.Color_flow",
        "Mean flow at faucets",
        "Débit moyen aux robinets",
        "Caudal medio en los grifos",
    ),
    (
        "Gui.Color_pressure",
        "Mean pressure",
        "Pression moyenne",
        "Presión media",
    ),
    (
        "Gui.No_coordinates",
        "The nodes have no coordinates: tree layout shown.",
        "Les nœuds n'ont pas de coordonnées : disposition en arbre.",
        "Los nodos no tienen coordenadas: disposición en árbol.",
    ),
    (
        "Gui.No_simulation",
        "Select a simulation in the Simulation tab.",
        "Choisissez une simulation dans l'onglet Simulation.",
        "Elija una simulación en la pestaña Simulación.",
    ),
    (
        "Gui.Path_length",
        "Distance to the source",
        "Distance à la source",
        "Distancia a la fuente",
    ),
    ("Gui.Faucets", "Faucets", "Robinets", "Grifos"),
    ("Gui.Error", "Error", "Erreur", "Error"),
    (
        "Gui.Low",
        "below the low critical flow",
        "sous le débit critique bas",
        "bajo el caudal crítico bajo",
    ),
    (
        "Gui.Ok_flow",
        "between the critical flows",
        "entre les débits critiques",
        "entre los caudales críticos",
    ),
    (
        "Gui.High",
        "above the high critical flow",
        "au-dessus du débit critique haut",
        "sobre el caudal crítico alto",
    ),
];

/// Texts of the NeatWork 5 workspace (`Nw5.*` keys), in English for the
/// three languages until they are translated. Most are the English texts
/// of the NW5 application (`rust/nw5/specs/app.md`).
const EXTRA_EN: &[(&str, &str)] = &[
    ("Nw5.Active", "Active"),
    ("Nw5.Active_revision", "Active revision"),
    (
        "Nw5.Adaptive_precision",
        "Result values use adaptive display precision. Stored simulation values retain full numerical precision.",
    ),
    ("Nw5.Advanced_generation", "Advanced scenario generation"),
    ("Nw5.Apply", "Apply"),
    ("Nw5.Archive", "Archive"),
    (
        "Nw5.Archive_1",
        "This file will be removed from the active project library.",
    ),
    (
        "Nw5.Archive_2",
        "Historical file content and previous simulation results will be preserved.",
    ),
    (
        "Nw5.Archive_3",
        "This file will no longer be available for new imports.",
    ),
    ("Nw5.Archive_title", "Delete input file?"),
    ("Nw5.Archived", "Archived"),
    (
        "Nw5.Bad_seed",
        "The design seed must be a non-negative whole number.",
    ),
    ("Nw5.Bounded_flow", "Bounded flow"),
    ("Nw5.Browse", "Browse..."),
    ("Nw5.Cancel", "Cancel"),
    (
        "Nw5.Cancel_before_close",
        "Cancel the running simulation before closing the project.",
    ),
    (
        "Nw5.Cancel_before_delete",
        "Cancel the running simulation before deleting the project.",
    ),
    (
        "Nw5.Checking_compatibility",
        "Checking revision compatibility...",
    ),
    ("Nw5.Choose_file_name", "Choose file name"),
    ("Nw5.Client_demand", "Client demand"),
    ("Nw5.Close", "Close"),
    ("Nw5.Close_valve", "Close valve"),
    ("Nw5.Commercial", "Commercial"),
    ("Nw5.Comparison", "Comparison"),
    ("Nw5.Complementary_databases", "Complementary databases"),
    ("Nw5.Config_intro_1", "Values from the active"),
    (
        "Nw5.Config_intro_2",
        "input are prefilled. Run immediately to use them unchanged, or edit only the settings you want to override for this execution.",
    ),
    ("Nw5.Configure", "Simulation..."),
    ("Nw5.Configure_simulation", "Configure simulation"),
    ("Nw5.Costs", "Costs"),
    ("Nw5.Create_project", "Create project"),
    ("Nw5.Created", "Created"),
    ("Nw5.Currently_open", "Currently open"),
    ("Nw5.Custom_overrides", "Custom overrides"),
    ("Nw5.Defaults_and_sources", "Defaults and sources"),
    ("Nw5.Delete", "Delete"),
    ("Nw5.Delete_project", "Delete Project"),
    (
        "Nw5.Delete_project_1",
        "You are about to permanently delete",
    ),
    (
        "Nw5.Delete_project_2",
        "Deleting this project also removes all data associated with it.",
    ),
    (
        "Nw5.Delete_project_3",
        "This includes the project's network data, imported files, artifacts, domain snapshots, simulations and generated results.",
    ),
    ("Nw5.Delete_project_4", "This action cannot be undone."),
    (
        "Nw5.Delete_project_check",
        "I understand that this project and all its associated files, artifacts, imported data, simulations and results will be permanently deleted.",
    ),
    ("Nw5.Delete_project_title", "Delete Project?"),
    (
        "Nw5.Delete_run_1",
        "The simulation result will be permanently deleted.",
    ),
    (
        "Nw5.Delete_run_2",
        "Its source project, DSG/TPO file and domain snapshot will not be deleted.",
    ),
    ("Nw5.Delete_run_title", "Delete Simulation"),
    (
        "Nw5.Demand_note_1",
        "Demand values are read from the active Client demand table and cannot be overridden per simulation.",
    ),
    (
        "Nw5.Demand_note_2",
        "To change demand values, edit the Client demand table and save a new input revision before running the simulation.",
    ),
    ("Nw5.Design", "Design..."),
    ("Nw5.Design_completed", "Design completed"),
    ("Nw5.Design_failed", "Design failed"),
    ("Nw5.Design_input", "Input:"),
    ("Nw5.Design_seed", "Seed of the Monte Carlo loads"),
    ("Nw5.Design_title", "NW5 design"),
    ("Nw5.Details", "Details"),
    (
        "Nw5.Details_text",
        "Exact input and reproducibility parameters used by this execution.",
    ),
    ("Nw5.Diameters_database", "Diameters database"),
    ("Nw5.Discard", "Discard"),
    ("Nw5.Execution_failure_records", "Execution failure records"),
    ("Nw5.Export_as", "Export as"),
    ("Nw5.Export_json", "Export JSON"),
    (
        "Nw5.Extension_hint",
        "If the extension is omitted, NeatWork adds the correct one automatically.",
    ),
    ("Nw5.Find", "Find"),
    ("Nw5.Format_lock_1", "This project uses"),
    ("Nw5.Format_lock_2", "input."),
    (
        "Nw5.Format_lock_none",
        "No network imported: a DSG or TPO network can be imported.",
    ),
    (
        "Nw5.Fresh_seed",
        "A fresh random seed will be generated automatically when the simulation starts.",
    ),
    (
        "Nw5.Heterogeneous_note",
        "This network contains heterogeneous client demand. Bounded flow is required by the current solver, while the orifice model is still stored independently.",
    ),
    ("Nw5.Hydraulic_configuration", "Hydraulic configuration"),
    ("Nw5.Ideal", "Ideal"),
    (
        "Nw5.Import_hint",
        "Import a DSG or TPO network to start working with this project.",
    ),
    ("Nw5.Import_network", "Import Network"),
    ("Nw5.Import_tpo", "Import TPO"),
    ("Nw5.Import_warnings", "Import warnings"),
    ("Nw5.Imported_as", "imported as"),
    ("Nw5.Imported_defaults", "Imported defaults"),
    ("Nw5.Imported_from", "Imported from"),
    ("Nw5.Included", "Included in TPO"),
    ("Nw5.Json", "JSON"),
    (
        "Nw5.Json_description",
        "Parsed domain representation used by the application.",
    ),
    ("Nw5.Junction", "Junction"),
    ("Nw5.Left", "LEFT"),
    ("Nw5.Left_metric", "Left metric"),
    ("Nw5.Left_simulation", "Left simulation"),
    ("Nw5.Legend_faucet", "Faucet (terminal)"),
    ("Nw5.Legend_flash", "Found node"),
    ("Nw5.Legend_high", "Pressure ≥ 30 m"),
    ("Nw5.Legend_junction", "Junction without results"),
    ("Nw5.Legend_low", "Low pressure"),
    ("Nw5.Legend_medium", "Pressure ≥ 18 m"),
    ("Nw5.Legend_reservoir", "Reservoir (source)"),
    ("Nw5.Legend_valve_closed", "Valve closed"),
    ("Nw5.Legend_valve_open", "Valve open"),
    ("Nw5.Linear_graph", "Linear graph"),
    ("Nw5.Linear_header", "Linear topology"),
    ("Nw5.Linear_note", "Source-to-terminal hierarchy."),
    ("Nw5.Match_rows_by", "Match rows by"),
    ("Nw5.Max_batches", "Maximum generation batches"),
    ("Nw5.Max_draws", "Maximum draws per batch"),
    ("Nw5.Menu_delete_project", "Delete NW5 Project..."),
    ("Nw5.Menu_import_network", "Import Network..."),
    ("Nw5.Menu_new_project", "New NW5 Project..."),
    ("Nw5.Menu_open_project", "Open NW5 Project..."),
    ("Nw5.Menu_set_units", "Set global units..."),
    (
        "Nw5.Monte_carlo_loads",
        "Monte Carlo loads (also for uniform demand)",
    ),
    ("Nw5.New_file_name", "New file name"),
    ("Nw5.New_project", "New Project"),
    (
        "Nw5.No_common_table",
        "These sources do not expose a compatible table structure. Choose two simulation results that can be compared directly.",
    ),
    (
        "Nw5.No_completed_simulation",
        "No completed simulation is available yet.",
    ),
    ("Nw5.No_external_orifices", "No external orifices DB"),
    (
        "Nw5.No_identifier",
        "These tables do not expose a compatible entity identifier. Side-by-side inspection remains available above.",
    ),
    ("Nw5.No_network_imported", "No network imported"),
    ("Nw5.No_overrides", "No runtime overrides"),
    ("Nw5.No_project_match", "No project matches your search."),
    (
        "Nw5.No_projects",
        "No projects found in the projects folder.",
    ),
    (
        "Nw5.No_result_tables",
        "This simulation does not contain result tables.",
    ),
    ("Nw5.No_results", "No results"),
    ("Nw5.Nodes", "Nodes"),
    ("Nw5.Not_a_number", "not a number"),
    ("Nw5.Not_included", "Not included"),
    ("Nw5.Number_of_scenarios", "Number of scenarios"),
    ("Nw5.Numeric_comparison", "Numeric comparison"),
    ("Nw5.Nw_json", "NW-json"),
    (
        "Nw5.Nw_json_description",
        "MATLAB NW-json-v1 representation of the network.",
    ),
    ("Nw5.Open", "Open"),
    ("Nw5.Open_project", "Open Project"),
    (
        "Nw5.Open_project_text",
        "Select an existing project to continue working on it.",
    ),
    ("Nw5.Open_valve", "Open valve"),
    ("Nw5.Optional", "Optional"),
    ("Nw5.Orifice_model", "Orifice model"),
    ("Nw5.Orifices_database", "Orifices database"),
    ("Nw5.Original_file", "Original file"),
    ("Nw5.Original_unchanged", "This file will remain unchanged."),
    ("Nw5.Oversampling", "Oversampling factor"),
    (
        "Nw5.Parameter_overrides",
        "Parameter overrides (only changed values are sent)",
    ),
    ("Nw5.Parsed_json", "Parsed JSON · Domain snapshot"),
    ("Nw5.Pipes", "Pipes"),
    ("Nw5.Project_name", "Project name"),
    ("Nw5.Random_seed", "Random seed"),
    ("Nw5.Raw_graph", "Raw 2D graph"),
    ("Nw5.Raw_header", "Raw 2D network graph"),
    (
        "Nw5.Raw_identical",
        "All raw coordinates are identical. Nodes may overlap because the source file does not provide usable X/Y coordinates.",
    ),
    (
        "Nw5.Raw_note",
        "This view uses the original X/Y coordinates from the imported file.",
    ),
    ("Nw5.Reinforcement", "design to reinforce"),
    ("Nw5.Required", "Required"),
    ("Nw5.Reset_defaults", "Reset to imported defaults"),
    ("Nw5.Results", "Results"),
    ("Nw5.Results_of", "Results of"),
    ("Nw5.Review", "Review"),
    ("Nw5.Revision_of", "Revision of"),
    ("Nw5.Revisions", "Revisions"),
    ("Nw5.Right", "RIGHT"),
    ("Nw5.Right_metric", "Right metric"),
    ("Nw5.Right_simulation", "Right simulation"),
    ("Nw5.Risk_from_parameters", "From the parameters (use_cvar)"),
    ("Nw5.Risk_measure", "Risk measure of the loads"),
    ("Nw5.Run", "Run"),
    ("Nw5.Run_design", "Run design"),
    ("Nw5.Run_setup", "Run setup"),
    ("Nw5.Run_traceability", "Run traceability"),
    ("Nw5.Running", "running"),
    ("Nw5.Runtime_overrides", "runtime override(s)"),
    ("Nw5.Same_run", "Choose two different simulations."),
    ("Nw5.Save_automatically", "Save automatically"),
    ("Nw5.Save_new_version", "Save new version"),
    ("Nw5.Save_position", "Save position"),
    ("Nw5.Save_revision", "Save revision"),
    ("Nw5.Saved_as", "Saved as"),
    ("Nw5.Scenarios", "Scenarios"),
    ("Nw5.Search_node", "Search node..."),
    ("Nw5.Search_projects", "Search projects..."),
    ("Nw5.Seed", "Seed"),
    ("Nw5.Select_db_file", "Please select a .db file."),
    ("Nw5.Select_diameters_db", "Select diameters.db"),
    ("Nw5.Selected_execution", "Selected execution"),
    ("Nw5.Set_active", "Set active"),
    ("Nw5.Show_details", "Show Details"),
    ("Nw5.Show_details_columns", "Show details"),
    ("Nw5.Simulation", "Simulation"),
    ("Nw5.Simulation_history", "Simulation History"),
    ("Nw5.Source", "Source"),
    (
        "Nw5.Source_description",
        "Original network file stored in this project.",
    ),
    ("Nw5.Source_error", "Could not load this comparison source."),
    ("Nw5.Source_file", "Source File"),
    ("Nw5.Sources", "Sources"),
    ("Nw5.Stage_loads", "Design loads..."),
    ("Nw5.Stage_model", "Building the LP model..."),
    ("Nw5.Stage_post", "Post-processing..."),
    ("Nw5.Stage_prepare", "Preparing the design tables..."),
    ("Nw5.Stage_solve", "Solving with HiGHS..."),
    ("Nw5.Table", "Table"),
    ("Nw5.Tables", "Tables"),
    ("Nw5.Terminal", "Terminal"),
    ("Nw5.Text", "Text"),
    ("Nw5.Topography", "topography"),
    (
        "Nw5.Two_runs_required",
        "At least two completed simulations are required to enable comparison.",
    ),
    ("Nw5.Unit", "Unit"),
    ("Nw5.Unit_settings", "Unit Settings"),
    (
        "Nw5.Units_note",
        "Internal hydraulic calculations remain in canonical SI units. This setting changes display and user input only.",
    ),
    (
        "Nw5.Units_text",
        "Choose the unit used to display hydraulic lengths, pressure head, coordinates and velocities throughout this project.",
    ),
    ("Nw5.Use_diameters_from_tpo", "Use !Diameters from TPO"),
    ("Nw5.Use_specific_seed", "Use a specific seed"),
    ("Nw5.Visual_graph", "Visual graph"),
    (
        "Nw5.Visual_note",
        "This view uses generated visual coordinates when the source file has missing or zero coordinates.",
    ),
    ("Nw5.Visual_x", "Visual graph X"),
    ("Nw5.Visual_y", "Visual graph Y"),
    ("Nw5.Warnings", "Warnings"),
    ("Nw5.executions", "executions"),
];

/// The bundles of the three languages, with the base bundle as fallback.
pub struct Messages {
    bundles: [HashMap<String, String>; 3],
    base: HashMap<String, String>,
}

/// Decode a `.properties` file as `PropertyResourceBundle` does: UTF-8,
/// or ISO-8859-1 when the bytes are not valid UTF-8.
fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|b| char::from(*b)).collect(),
    }
}

fn load(bytes: &[u8]) -> HashMap<String, String> {
    let mut p = JProperties::new();
    p.load(&decode(bytes));
    p.entries().map(|(k, v)| (k.clone(), v.clone())).collect()
}

/// Remove HTML tags; turn line-breaking tags into new lines.
pub fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut tag = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match (in_tag, c) {
            (false, '<') => {
                in_tag = true;
                tag.clear();
            }
            (true, '>') => {
                in_tag = false;
                let t = tag.trim_start_matches('/').to_ascii_lowercase();
                let name = t.split_whitespace().next().unwrap_or("");
                if matches!(
                    name,
                    "br" | "p" | "h1" | "h2" | "h3" | "li" | "tr" | "ul" | "blockquote"
                ) && !out.ends_with('\n')
                    && !out.is_empty()
                {
                    out.push('\n');
                }
                if name == "li" && !tag.starts_with('/') {
                    out.push_str("  - ");
                }
            }
            (true, _) => tag.push(c),
            (false, _) => out.push(c),
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

impl Messages {
    pub fn new() -> Self {
        let mut bundles = [load(BUNDLE_EN), load(BUNDLE_FR), load(BUNDLE_SP)];
        for (k, en, fr, sp) in EXTRA {
            bundles[0].insert((*k).to_string(), (*en).to_string());
            bundles[1].insert((*k).to_string(), (*fr).to_string());
            bundles[2].insert((*k).to_string(), (*sp).to_string());
        }
        for (k, en) in EXTRA_EN {
            for b in &mut bundles {
                b.insert((*k).to_string(), (*en).to_string());
            }
        }
        Messages {
            bundles,
            base: load(BASE),
        }
    }

    /// Raw text of a key (with HTML), `!key!` when it is missing, as
    /// `Messages.getString` returns.
    pub fn raw(&self, lang: i32, key: &str) -> String {
        let b = &self.bundles[usize::try_from(lang).unwrap_or(0).min(2)];
        b.get(key)
            .or_else(|| self.base.get(key))
            .cloned()
            .unwrap_or_else(|| format!("!{key}!"))
    }

    /// Text for display: HTML removed, surrounding new lines trimmed.
    pub fn get(&self, lang: i32, key: &str) -> String {
        let raw = self.raw(lang, key);
        if raw.contains('<') {
            strip_html(&raw).trim().to_string()
        } else {
            raw.trim_end_matches('\n').to_string()
        }
    }
}

/// Code of a language in the file `neatwork.locale` (`appli.locale`).
pub fn locale_code(lang: i32) -> &'static str {
    match lang {
        FR => "fr",
        SP => "sp",
        _ => "en",
    }
}

pub fn lang_from_code(code: &str) -> i32 {
    match code {
        "fr" => FR,
        "sp" | "es" => SP,
        _ => EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles_load() {
        let m = Messages::new();
        assert_eq!(m.get(EN, "FrameNeatwork.File"), "File");
        assert_eq!(
            m.get(FR, "topo.watertemp.name"),
            "Température de l'eau (°C)"
        );
        assert_eq!(m.get(SP, "Network.Source"), "Tanque");
        assert_eq!(m.get(EN, "nope"), "!nope!");
        assert_eq!(m.get(EN, "Gui.Network"), "Network");
        assert!(!m.get(EN, "PresentationPanel.text").contains('<'));
    }

    /// Every `Nw5.*` key of the NW5 workspace has a text in each language.
    #[test]
    fn nw5_keys_have_texts() {
        let m = Messages::new();
        let sources = [
            include_str!("../ui/nw5.slint"),
            include_str!("../ui/nw5_dialogs.slint"),
            include_str!("../ui/main.slint"),
            include_str!("nw5.rs"),
            include_str!("nw5_actions.rs"),
        ];
        let mut count = 0;
        for src in sources {
            for part in src.split("\"Nw5.").skip(1) {
                let key: String = part
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                for lang in [EN, FR, SP] {
                    assert!(
                        !m.get(lang, &format!("Nw5.{key}")).starts_with('!'),
                        "Nw5.{key}"
                    );
                }
                count += 1;
            }
        }
        assert!(count > 100);
        assert_eq!(m.get(FR, "Nw5.Run"), "Run");
    }
}
