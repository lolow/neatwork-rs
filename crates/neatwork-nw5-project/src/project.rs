//! Project folders (`specs/app.md` F4–F7, F11, F14, F15, F25, F38–F40,
//! F53 and section 14.2).
//!
//! Every change is written to disk at once, so opening a project restores
//! the active revision, the graph state, the runs and the comparison
//! workspace (D1).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use neatwork_nw5::Network;
use neatwork_nw5::read::{read_diameters_db, read_dsg, read_orifices_db, read_tpo, sections};

use crate::format::DisplayUnit;
use crate::graph::{GraphData, GraphState, graph_states_from_json, graph_states_to_json};
use crate::json::{Json, get_array, get_str, get_u64, obj, opt_s, s};
use crate::revision::{
    Revision, RevisionName, SourceFormat, default_revision_name, import_file_name, next_version,
    revisions_from_json, revisions_to_json, split_name, validate_custom_name,
};
use crate::run::{
    RequestedOptions, RunError, RunJob, RunRecord, RunStatus, SimulationDefaults, demand_profile,
    run_file_name, write_record,
};
use crate::{Error, Result, decode_text, io_err, read_bytes, write_bytes, write_text};

/// Longest project name (F4).
pub const MAX_NAME_LENGTH: usize = 120;

/// Saved choice of the comparison dialog for one active revision (F48).
#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonWorkspace {
    /// Active revision when the choice was saved.
    pub revision: String,
    /// Left run number.
    pub left: u32,
    /// Right run number.
    pub right: u32,
    /// Table key (`terminalSatisfaction`, ...).
    pub table: String,
}

/// Content of `project.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectMeta {
    /// Project name (trimmed, 1 to 120 characters).
    pub name: String,
    /// Creation time.
    pub created_at: String,
    /// Source-format lock (F7).
    pub source_format: SourceFormat,
    /// Display unit (F53).
    pub display_unit: DisplayUnit,
    /// Active revision file.
    pub active_revision: Option<String>,
    /// Saved comparison choices, one per active revision.
    pub comparison: Vec<ComparisonWorkspace>,
    /// Last view of the workspace (free text for the interface).
    pub last_view: Option<String>,
    /// Requested options of the last run ("Run Again").
    pub last_run_options: Option<RequestedOptions>,
}

impl ProjectMeta {
    fn to_json(&self) -> Json {
        obj(vec![
            ("name", s(&self.name)),
            ("createdAt", s(&self.created_at)),
            ("sourceFormat", s(self.source_format.as_str())),
            ("displayUnit", s(self.display_unit.symbol())),
            ("activeRevision", opt_s(self.active_revision.as_deref())),
            (
                "comparison",
                Json::Array(
                    self.comparison
                        .iter()
                        .map(|c| {
                            obj(vec![
                                ("revision", s(&c.revision)),
                                ("left", Json::Number(f64::from(c.left))),
                                ("right", Json::Number(f64::from(c.right))),
                                ("table", s(&c.table)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("lastView", opt_s(self.last_view.as_deref())),
            (
                "lastRunOptions",
                self.last_run_options
                    .as_ref()
                    .map_or(Json::Null, RequestedOptions::to_json),
            ),
        ])
    }

    fn from_json(v: &Json) -> Result<ProjectMeta> {
        let run = |c: &Json, k: &str| get_u64(c, k).and_then(|x| u32::try_from(x).ok());
        Ok(ProjectMeta {
            name: get_str(v, "name")
                .ok_or_else(|| Error::Format("project.json: no name".into()))?,
            created_at: get_str(v, "createdAt").unwrap_or_default(),
            source_format: get_str(v, "sourceFormat")
                .and_then(|t| SourceFormat::parse(&t))
                .unwrap_or_default(),
            display_unit: get_str(v, "displayUnit")
                .and_then(|t| DisplayUnit::parse(&t))
                .unwrap_or_default(),
            active_revision: get_str(v, "activeRevision"),
            comparison: get_array(v, "comparison")
                .iter()
                .filter_map(|c| {
                    Some(ComparisonWorkspace {
                        revision: get_str(c, "revision")?,
                        left: run(c, "left")?,
                        right: run(c, "right")?,
                        table: get_str(c, "table")?,
                    })
                })
                .collect(),
            last_view: get_str(v, "lastView"),
            last_run_options: match v.get("lastRunOptions") {
                Some(o @ Json::Object(_)) => Some(RequestedOptions::from_json(o)),
                _ => None,
            },
        })
    }
}

/// A project found in a folder of projects (F5).
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectEntry {
    /// Project name.
    pub name: String,
    /// Creation time.
    pub created_at: String,
    /// Project folder.
    pub path: PathBuf,
}

/// Catalogues chosen for a `.tpo` import (F15). `None` looks next to the
/// file: `DiamBase<file>.db`, `DiamBase<folder>.db`, `diameters.db` and
/// `orifices.db` (as `neatwork convert`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportOptions {
    /// Diameter catalogue.
    pub diameters: Option<PathBuf>,
    /// Orifice catalogue.
    pub orifices: Option<PathBuf>,
}

/// Result of an import.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportOutcome {
    /// The revision, now active.
    pub revision: Revision,
    /// Warnings of the reader (F18).
    pub warnings: Vec<String>,
    /// The same file was already in the project: it was made active again.
    pub reused: bool,
}

/// A parsed revision.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedRevision {
    /// The revision.
    pub revision: Revision,
    /// The network, with the catalogues of `db/` for a `.tpo`.
    pub network: Network,
    /// Warnings of the reader.
    pub warnings: Vec<String>,
}

/// Export formats of a revision (ADR-025 item 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionExport {
    /// `.tpo`.
    Tpo,
    /// `.dsg`.
    Dsg,
    /// MATLAB `NW-json-v1`.
    NwJson,
    /// EPANET `.inp`.
    Inp,
}

/// Result of an export.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportOutcome {
    /// Written file in `exports/`.
    pub path: PathBuf,
    /// Warnings of the writer (EPANET).
    pub warnings: Vec<String>,
}

const MSG_RUNNING_IMPORT: &str = "Cancel the running simulation before importing a new domain.";
const MSG_RUNNING_STATE: &str = "Cancel the running simulation before changing network state.";

/// A NW5 project folder.
#[derive(Clone, Debug)]
pub struct Project {
    root: PathBuf,
    meta: ProjectMeta,
    revisions: Vec<Revision>,
    graph: BTreeMap<String, GraphState>,
    active_run: Option<u32>,
}

/// Folder name of a project name: characters that file systems refuse
/// become `_`.
// PROPOSAL: names need not be unique (F4), folders must be: a taken folder
// name gets " (2)", " (3)", ...
fn folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if "/\\:*?\"<>|".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim().to_string();
    if cleaned.is_empty() {
        "Project".to_string()
    } else {
        cleaned
    }
}

fn file_name_of(path: &Path) -> Result<String> {
    path.file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .ok_or_else(|| Error::Refused(format!("{}: not a file", path.display())))
}

fn create_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|e| io_err(path, e))
}

/// Read a `.json` (NW-json-v1 or application domain JSON) or `.inp`
/// (EPANET) network and write it as a `.dsg` (a design) or a `.tpo` (a
/// topography): the visible name, the file bytes and the reader warnings.
/// `None` for any other file.
// PROPOSAL: NW5 does not reach its JSON import (F16, D12) and has no INP
// import in the interface. The port converts these files on import, with
// the readers of `neatwork convert`; the format of the converted file is
// DSG for a network with pipes chosen, else TPO, so the format lock (F7)
// applies as for a native file.
pub fn converted_input(
    path: &Path,
    original: &str,
) -> Result<Option<(String, Vec<u8>, Vec<String>)>> {
    let (stem, ext) = split_name(original);
    let ext = ext.to_ascii_lowercase();
    if ext != ".json" && ext != ".inp" {
        return Ok(None);
    }
    let text = decode_text(&read_bytes(path)?);
    let refused = |e: String| Error::Refused(format!("{original}: {e}"));
    let read = if ext == ".inp" {
        let opts = neatwork_nw5_io::epanet::InpImportOptions {
            name: stem.to_string(),
            ..neatwork_nw5_io::epanet::InpImportOptions::default()
        };
        let imp = neatwork_nw5_io::read_inp(&text, &opts).map_err(|e| refused(e.to_string()))?;
        neatwork_nw5::read::Read {
            network: imp.network,
            warnings: imp.issues.iter().map(|i| i.message.clone()).collect(),
        }
    } else if text.contains("NW-json-v1") {
        neatwork_nw5_io::read_nw_json(stem, &text).map_err(|e| refused(e.to_string()))?
    } else {
        neatwork_nw5_io::read_domain_json(&text).map_err(|e| refused(e.to_string()))?
    };
    let (body, ext) = if read.network.is_design() {
        (neatwork_nw5_io::write_dsg(&read.network), ".dsg")
    } else {
        (neatwork_nw5_io::write_tpo(&read.network), ".tpo")
    };
    Ok(Some((
        format!("{stem}{ext}"),
        body.into_bytes(),
        read.warnings,
    )))
}

impl Project {
    /// Create a project folder in `parent` (F4): format `NONE`, display
    /// unit m, no revision.
    pub fn create(parent: &Path, name: &str) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Refused("The project could not be created.".into()));
        }
        if name.chars().count() > MAX_NAME_LENGTH {
            return Err(Error::Refused(format!(
                "The project name must have at most {MAX_NAME_LENGTH} characters."
            )));
        }
        create_dir(parent)?;
        let base = folder_name(name);
        let mut root = parent.join(&base);
        let mut k = 2;
        while root.exists() {
            root = parent.join(format!("{base} ({k})"));
            k += 1;
        }
        for sub in ["inputs", "db", "runs", "exports"] {
            create_dir(&root.join(sub))?;
        }
        let project = Project {
            root,
            meta: ProjectMeta {
                name: name.to_string(),
                created_at: crate::time::now_iso(),
                source_format: SourceFormat::None,
                display_unit: DisplayUnit::Meter,
                active_revision: None,
                comparison: Vec::new(),
                last_view: None,
                last_run_options: None,
            },
            revisions: Vec::new(),
            graph: BTreeMap::new(),
            active_run: None,
        };
        project.save_meta()?;
        project.save_revisions()?;
        project.save_graph()?;
        Ok(project)
    }

    /// Open a project folder and restore its state (D1). Runs left
    /// `PENDING` or `RUNNING` by a previous session become `FAILED`.
    pub fn open(root: &Path) -> Result<Project> {
        let meta_path = root.join("project.json");
        let meta = Json::parse(&decode_text(&read_bytes(&meta_path)?))
            .map_err(|e| Error::Format(format!("{}: {e}", meta_path.display())))?;
        let meta = ProjectMeta::from_json(&meta)?;
        let rev_path = root.join("inputs").join("revisions.json");
        let revisions = if rev_path.is_file() {
            revisions_from_json(&decode_text(&read_bytes(&rev_path)?))?
        } else {
            Vec::new()
        };
        let graph_path = root.join("graph.json");
        let graph = if graph_path.is_file() {
            graph_states_from_json(&decode_text(&read_bytes(&graph_path)?))?
        } else {
            BTreeMap::new()
        };
        for sub in ["inputs", "db", "runs", "exports"] {
            create_dir(&root.join(sub))?;
        }
        let project = Project {
            root: root.to_path_buf(),
            meta,
            revisions,
            graph,
            active_run: None,
        };
        for mut r in project.runs()? {
            if !r.status.is_terminal() {
                r.status = RunStatus::Failed;
                // PROPOSAL: NW5 says "...because the backend restarted."
                r.error = Some(RunError {
                    code: "SIMULATION_INTERRUPTED".into(),
                    message: "Simulation interrupted because the application closed.".into(),
                });
                write_record(&project.run_path(r.run_number), &r)?;
            }
        }
        Ok(project)
    }

    /// Projects in a folder, newest first (F5). Folders without a readable
    /// `project.json` are skipped.
    pub fn list(parent: &Path) -> Vec<ProjectEntry> {
        let mut out = Vec::new();
        let Ok(dir) = std::fs::read_dir(parent) else {
            return out;
        };
        for entry in dir.flatten() {
            let path = entry.path();
            let Ok(bytes) = std::fs::read(path.join("project.json")) else {
                continue;
            };
            let Ok(meta) = Json::parse(&decode_text(&bytes)).and_then(|v| {
                ProjectMeta::from_json(&v).map_err(|e| neatwork_nw5::Error::Format(e.to_string()))
            }) else {
                continue;
            };
            out.push(ProjectEntry {
                name: meta.name,
                created_at: meta.created_at,
                path,
            });
        }
        out.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.name.cmp(&b.name))
        });
        out
    }

    /// Delete the project folder and everything in it (F6).
    pub fn delete(self) -> Result<()> {
        if self.active_run.is_some() {
            return Err(Error::Refused(
                "Cancel the running simulation before deleting the project.".into(),
            ));
        }
        std::fs::remove_dir_all(&self.root).map_err(|e| io_err(&self.root, e))
    }

    /// Project folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Name.
    pub fn name(&self) -> &str {
        &self.meta.name
    }

    /// Content of `project.json`.
    pub fn meta(&self) -> &ProjectMeta {
        &self.meta
    }

    /// Source-format lock.
    pub fn source_format(&self) -> SourceFormat {
        self.meta.source_format
    }

    /// Display unit.
    pub fn display_unit(&self) -> DisplayUnit {
        self.meta.display_unit
    }

    /// Set the display unit (F53).
    pub fn set_display_unit(&mut self, unit: DisplayUnit) -> Result<()> {
        self.meta.display_unit = unit;
        self.save_meta()
    }

    /// Remember the last view of the workspace.
    pub fn set_last_view(&mut self, view: Option<&str>) -> Result<()> {
        self.meta.last_view = view.map(str::to_string);
        self.save_meta()
    }

    /// All revisions in creation order (archived ones included).
    pub fn revisions(&self) -> &[Revision] {
        &self.revisions
    }

    /// A revision by file name.
    pub fn revision(&self, file: &str) -> Option<&Revision> {
        self.revisions.iter().find(|r| r.file == file)
    }

    /// The active revision.
    pub fn active_revision(&self) -> Option<&Revision> {
        self.meta
            .active_revision
            .as_deref()
            .and_then(|f| self.revision(f))
    }

    /// Number of the run in progress (one at a time, ADR-025 item 4).
    pub fn active_run(&self) -> Option<u32> {
        self.active_run
    }

    fn save_meta(&self) -> Result<()> {
        write_text(
            &self.root.join("project.json"),
            &self.meta.to_json().pretty(),
        )
    }

    fn save_revisions(&self) -> Result<()> {
        write_text(
            &self.root.join("inputs").join("revisions.json"),
            &revisions_to_json(&self.revisions),
        )
    }

    fn save_graph(&self) -> Result<()> {
        write_text(
            &self.root.join("graph.json"),
            &graph_states_to_json(&self.graph),
        )
    }

    // ----- source-format lock (F7) ---------------------------------------

    /// Format a new import may have: the project format, or any when the
    /// project has none (`None`).
    pub fn allowed_import_format(&self) -> Option<SourceFormat> {
        match self.meta.source_format {
            SourceFormat::None => None,
            f => Some(f),
        }
    }

    /// Check that a file of format `format` may be imported (F7).
    pub fn check_import_format(&self, format: SourceFormat) -> Result<()> {
        match self.allowed_import_format() {
            Some(f) if f != format => Err(Error::Refused(format!(
                "This project uses {} input. A {} file cannot be added to it.",
                f.as_str(),
                format.as_str()
            ))),
            _ => Ok(()),
        }
    }

    // ----- import (F12–F15) ----------------------------------------------

    /// Store a catalogue in `db/` and return its name there. A file with
    /// the same name and content is reused; a different content gets
    /// `<base>_v<N>.db` with N = number of `.db` files + 1 (F8).
    fn store_catalogue(&self, path: &Path) -> Result<String> {
        let bytes = read_bytes(path)?;
        let name = file_name_of(path)?;
        let db = self.root.join("db");
        let target = db.join(&name);
        if !target.exists() {
            write_bytes(&target, &bytes)?;
            return Ok(name);
        }
        if read_bytes(&target)? == bytes {
            return Ok(name);
        }
        let count = std::fs::read_dir(&db)
            .map_err(|e| io_err(&db, e))?
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("db"))
            })
            .count();
        let (base, ext) = split_name(&name);
        let mut version = count + 1;
        loop {
            let candidate = format!("{base}_v{version}{ext}");
            let p = db.join(&candidate);
            if !p.exists() {
                write_bytes(&p, &bytes)?;
                return Ok(candidate);
            }
            if read_bytes(&p)? == bytes {
                return Ok(candidate);
            }
            version += 1;
        }
    }

    /// Import a `.dsg` or `.tpo` file from anywhere (F12–F15): it is copied
    /// into `inputs/` as `<base>_v<N>.<ext>`, the catalogues a `.tpo` needs
    /// are copied into `db/`, and the revision becomes active. The first
    /// import sets the project format. A NW-json or application JSON file
    /// and an EPANET `.inp` file are converted first (see
    /// [`converted_input`]).
    pub fn import_network(
        &mut self,
        path: &Path,
        options: &ImportOptions,
    ) -> Result<ImportOutcome> {
        if self.active_run.is_some() {
            return Err(Error::Refused(MSG_RUNNING_IMPORT.into()));
        }
        let original = file_name_of(path)?;
        let (original, bytes, mut warnings) = match converted_input(path, &original)? {
            Some(converted) => converted,
            None => (original, Vec::new(), Vec::new()),
        };
        let format = SourceFormat::of_file(&original).ok_or_else(|| {
            Error::Refused("Unsupported network file. Drop a .dsg or .tpo file.".into())
        })?;
        self.check_import_format(format)?;
        let bytes = if bytes.is_empty() {
            read_bytes(path)?
        } else {
            bytes
        };
        let text = decode_text(&bytes);
        let file = import_file_name(&original);
        let (stem, _) = split_name(&file);
        let present: Vec<String> = sections(&text).0.into_iter().map(|s| s.name).collect();
        let has = |name: &str| present.iter().any(|p| p == name);
        let (mut diameters_db, mut orifices_db) = (None, None);
        let read = match format {
            SourceFormat::Dsg => {
                if !has("nodes") {
                    return Err(Error::Refused(
                        "DSG import failed: missing !Nodes section".into(),
                    ));
                }
                if !has("pipes") {
                    return Err(Error::Refused(
                        "DSG import failed: missing !Pipes section".into(),
                    ));
                }
                if !has("clients") {
                    warnings.push(
                        "DSG has no !Clients section. Simulation requires client demand data before running."
                            .to_string(),
                    );
                }
                read_dsg(stem, &text)?
            }
            _ => {
                let dir = path.parent().unwrap_or(Path::new("."));
                let folder = dir
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let (src_stem, _) = split_name(&original);
                if !has("diameters") {
                    let candidates = match &options.diameters {
                        Some(p) => vec![p.clone()],
                        None => vec![
                            dir.join(format!("DiamBase{src_stem}.db")),
                            dir.join(format!("DiamBase{folder}.db")),
                            dir.join("diameters.db"),
                        ],
                    };
                    let found = candidates.into_iter().find(|p| p.is_file()).ok_or_else(|| {
                        Error::Refused(
                            "TPO import requires a diameters database. This TPO has no !Diameters section. Select or upload a diameters database."
                                .into(),
                        )
                    })?;
                    read_diameters_db(&decode_text(&read_bytes(&found)?))?;
                    diameters_db = Some(found);
                }
                if !has("orifices") {
                    let p = options
                        .orifices
                        .clone()
                        .unwrap_or_else(|| dir.join("orifices.db"));
                    if p.is_file() {
                        orifices_db = Some(p);
                    } else if options.orifices.is_some() {
                        return Err(Error::Refused(format!("{}: file not found", p.display())));
                    }
                }
                read_tpo(stem, &text)?
            }
        };
        if read.network.nodes.is_empty() {
            return Err(Error::Refused(format!("{original}: the file has no node.")));
        }
        warnings.extend(read.warnings);
        // PROPOSAL: a file whose `inputs/` name exists is made active again
        // when its content is identical and not archived, else refused
        // (NW5 numbered stored copies with hidden internal names).
        if let Some(existing) = self.revision(&file).cloned() {
            let same = read_bytes(&self.root.join("inputs").join(&file))? == bytes;
            if !same || existing.archived {
                return Err(Error::Refused(format!(
                    "A revision named {file} already exists in this project."
                )));
            }
            self.meta.active_revision = Some(file);
            self.save_meta()?;
            return Ok(ImportOutcome {
                revision: existing,
                warnings,
                reused: true,
            });
        }
        let diameters_db = diameters_db.map(|p| self.store_catalogue(&p)).transpose()?;
        let orifices_db = orifices_db.map(|p| self.store_catalogue(&p)).transpose()?;
        write_bytes(&self.root.join("inputs").join(&file), &bytes)?;
        let revision = Revision {
            version: crate::revision::initial_version(&file),
            family: file.clone(),
            parent: None,
            format,
            created_at: crate::time::now_iso(),
            archived: false,
            diameters_db,
            orifices_db,
            imported_from: Some(path.display().to_string()),
            file: file.clone(),
        };
        self.revisions.push(revision.clone());
        self.save_revisions()?;
        if self.meta.source_format == SourceFormat::None {
            self.meta.source_format = format;
        }
        self.meta.active_revision = Some(file);
        self.save_meta()?;
        Ok(ImportOutcome {
            revision,
            warnings,
            reused: false,
        })
    }

    // ----- revisions (F8–F11) --------------------------------------------

    fn catalogue(&self, name: &str) -> Result<String> {
        Ok(decode_text(&read_bytes(&self.root.join("db").join(name))?))
    }

    /// Parse a revision, with the catalogues of `db/` for a `.tpo` that has
    /// none. Archived revisions can be read (runs refer to them).
    pub fn load_revision(&self, file: &str) -> Result<LoadedRevision> {
        let revision = self
            .revision(file)
            .cloned()
            .ok_or_else(|| Error::Refused(format!("Unknown revision {file}.")))?;
        let text = decode_text(&read_bytes(&self.root.join("inputs").join(file))?);
        let name = revision.network_name();
        let mut read = match revision.format {
            SourceFormat::Tpo => read_tpo(&name, &text)?,
            _ => read_dsg(&name, &text)?,
        };
        if read.network.pipes.is_empty()
            && let Some(db) = &revision.diameters_db
        {
            read.network.pipes = read_diameters_db(&self.catalogue(db)?)?;
        }
        if read.network.orifices.is_empty()
            && let Some(db) = &revision.orifices_db
        {
            read.network.orifices = read_orifices_db(&self.catalogue(db)?);
        }
        Ok(LoadedRevision {
            revision,
            network: read.network,
            warnings: read.warnings,
        })
    }

    /// Parse the active revision.
    pub fn load_active(&self) -> Result<LoadedRevision> {
        let file = self
            .meta
            .active_revision
            .clone()
            .ok_or_else(|| Error::Refused("No network imported".into()))?;
        self.load_revision(&file)
    }

    /// Text of a revision file (F52). An archived file cannot be read.
    pub fn revision_text(&self, file: &str) -> Result<String> {
        match self.revision(file) {
            Some(r) if !r.archived => Ok(decode_text(&read_bytes(
                &self.root.join("inputs").join(file),
            )?)),
            _ => Err(Error::Refused(
                "Could not read source file — The original source file could not be read.".into(),
            )),
        }
    }

    /// Name a new revision of the active revision would get.
    pub fn revision_name(&self, name: &RevisionName) -> Result<String> {
        let source = self
            .active_revision()
            .ok_or_else(|| Error::Refused("No network imported".into()))?;
        let ext = source.format.extension();
        match name {
            RevisionName::Automatic => Ok(default_revision_name(
                &source.file,
                next_version(&self.revisions, &source.family),
                ext,
            )),
            RevisionName::Custom(n) => validate_custom_name(n, ext),
        }
    }

    /// Save an edited network as a new revision of the active revision
    /// (F8–F10). The file is written with the `neatwork-nw5-io` writer of
    /// the source format (unknown sections are kept), read again to check
    /// it, and refused when its name exists (ADR-025 item 2). The source
    /// file never changes. The new revision becomes active; the graph state
    /// of the family is kept (D11).
    pub fn save_revision(&mut self, network: &Network, name: &RevisionName) -> Result<Revision> {
        let source = self
            .active_revision()
            .cloned()
            .ok_or_else(|| Error::Refused("No network imported".into()))?;
        if source.archived {
            // PROPOSAL: message of the refusal (NW5 only hides the editor).
            return Err(Error::Refused(
                "This input file is archived and cannot be edited.".into(),
            ));
        }
        if source.format != self.meta.source_format {
            return Err(Error::Refused(
                "Revision format does not match project format.".into(),
            ));
        }
        let file = self.revision_name(name)?;
        let target = self.root.join("inputs").join(&file);
        if self
            .revisions
            .iter()
            .any(|r| r.file.eq_ignore_ascii_case(&file))
            || target.exists()
        {
            // PROPOSAL: message of ADR-025 item 2.
            return Err(Error::Refused(format!(
                "A revision named {file} already exists. Choose another file name."
            )));
        }
        // A .tpo keeps its link to a db/ catalogue while the catalogue is
        // unchanged; an edited catalogue is written into the file.
        // PROPOSAL: NW5 has no rule for catalogues edited in a TPO revision.
        let mut written = network.clone();
        let (mut diameters_db, mut orifices_db) = (None, None);
        if source.format == SourceFormat::Tpo {
            if let Some(db) = &source.diameters_db
                && read_diameters_db(&self.catalogue(db)?)? == network.pipes
            {
                written.pipes.clear();
                diameters_db = Some(db.clone());
            }
            if let Some(db) = &source.orifices_db
                && read_orifices_db(&self.catalogue(db)?) == network.orifices
            {
                written.orifices.clear();
                orifices_db = Some(db.clone());
            }
        }
        let (stem, _) = split_name(&file);
        let text = match source.format {
            SourceFormat::Tpo => neatwork_nw5_io::write_tpo(&written),
            _ => neatwork_nw5_io::write_dsg(&written),
        };
        let check = match source.format {
            SourceFormat::Tpo => read_tpo(stem, &text),
            _ => read_dsg(stem, &text),
        }
        .map_err(|e| Error::Refused(format!("The revision could not be saved: {e}")))?;
        if check.network.nodes.len() != network.nodes.len()
            || check.network.arcs.len() != network.arcs.len()
        {
            return Err(Error::Refused(
                "The revision could not be saved: the written file does not read back to the same network."
                    .into(),
            ));
        }
        write_text(&target, &text)?;
        let revision = Revision {
            file: file.clone(),
            version: next_version(&self.revisions, &source.family),
            family: source.family.clone(),
            parent: Some(source.file.clone()),
            format: source.format,
            created_at: crate::time::now_iso(),
            archived: false,
            diameters_db,
            orifices_db,
            imported_from: None,
        };
        self.revisions.push(revision.clone());
        self.save_revisions()?;
        self.meta.active_revision = Some(file);
        self.save_meta()?;
        Ok(revision)
    }

    /// Name a design of the active revision would get as a `.dsg` revision
    /// (see [`Project::save_design_revision`]).
    pub fn design_revision_name(&self, name: &RevisionName) -> Result<String> {
        let source = self
            .active_revision()
            .ok_or_else(|| Error::Refused("No network imported".into()))?;
        let ext = SourceFormat::Dsg.extension();
        match name {
            RevisionName::Automatic => Ok(default_revision_name(
                &source.file,
                next_version(&self.revisions, &source.family),
                ext,
            )),
            RevisionName::Custom(n) => validate_custom_name(n, ext),
        }
    }

    /// Save the network of a NW5 design of the active revision as a new
    /// `.dsg` revision of the same family (default name
    /// `<base>_v<next>.dsg`). The file is read again to check it, a taken
    /// name is refused (ADR-025 item 2), and the new revision becomes
    /// active. The catalogues are written into the file.
    // PROPOSAL: NW5 keeps one format per project (F7) and has no design in
    // its interface. A design is a `.dsg`, so a design of a `.tpo` project
    // is a `.dsg` revision of the `.tpo` family; the format lock still
    // applies to imports, and the editor saves only revisions of the
    // project format.
    pub fn save_design_revision(
        &mut self,
        network: &Network,
        name: &RevisionName,
    ) -> Result<Revision> {
        if self.active_run.is_some() {
            return Err(Error::Refused(MSG_RUNNING_IMPORT.into()));
        }
        let source = self
            .active_revision()
            .cloned()
            .ok_or_else(|| Error::Refused("No network imported".into()))?;
        let file = self.design_revision_name(name)?;
        let target = self.root.join("inputs").join(&file);
        if self
            .revisions
            .iter()
            .any(|r| r.file.eq_ignore_ascii_case(&file))
            || target.exists()
        {
            return Err(Error::Refused(format!(
                "A revision named {file} already exists. Choose another file name."
            )));
        }
        let (stem, _) = split_name(&file);
        let text = neatwork_nw5_io::write_dsg(network);
        let check = read_dsg(stem, &text)
            .map_err(|e| Error::Refused(format!("The revision could not be saved: {e}")))?;
        if check.network.nodes.len() != network.nodes.len()
            || check.network.arcs.len() != network.arcs.len()
        {
            return Err(Error::Refused(
                "The revision could not be saved: the written file does not read back to the same network."
                    .into(),
            ));
        }
        write_text(&target, &text)?;
        let revision = Revision {
            file: file.clone(),
            version: next_version(&self.revisions, &source.family),
            family: source.family.clone(),
            parent: Some(source.file.clone()),
            format: SourceFormat::Dsg,
            created_at: crate::time::now_iso(),
            archived: false,
            diameters_db: None,
            orifices_db: None,
            imported_from: None,
        };
        self.revisions.push(revision.clone());
        self.save_revisions()?;
        self.meta.active_revision = Some(file);
        self.save_meta()?;
        Ok(revision)
    }

    /// Archive an input file (F11): it stays on disk, runs keep it, it
    /// cannot be edited or imported again.
    pub fn archive_revision(&mut self, file: &str) -> Result<()> {
        let r = self
            .revisions
            .iter_mut()
            .find(|r| r.file == file)
            .ok_or_else(|| Error::Refused(format!("Unknown revision {file}.")))?;
        r.archived = true;
        self.save_revisions()
    }

    /// Choose a revision as the active input (ADR-025 item 5).
    pub fn set_active_revision(&mut self, file: &str) -> Result<()> {
        if self.active_run.is_some() {
            return Err(Error::Refused(MSG_RUNNING_IMPORT.into()));
        }
        if self.revision(file).is_none() {
            return Err(Error::Refused(format!("Unknown revision {file}.")));
        }
        self.meta.active_revision = Some(file.to_string());
        self.save_meta()
    }

    // ----- graph state (F19, F25) -----------------------------------------

    /// User state of the family of a revision.
    pub fn graph_state(&self, file: &str) -> GraphState {
        self.revision(file)
            .and_then(|r| self.graph.get(&r.family))
            .cloned()
            .unwrap_or_default()
    }

    /// Graph of the active revision with the user state of its family.
    pub fn graph(&self) -> Result<GraphData> {
        let loaded = self.load_active()?;
        Ok(GraphData::build(
            &loaded.network,
            loaded.revision.format,
            &self.graph_state(&loaded.revision.file),
        ))
    }

    fn active_family_state(&mut self) -> Result<(&mut GraphState, Network)> {
        if self.active_run.is_some() {
            return Err(Error::Refused(MSG_RUNNING_STATE.into()));
        }
        let loaded = self.load_active()?;
        Ok((
            self.graph.entry(loaded.revision.family).or_default(),
            loaded.network,
        ))
    }

    /// Store the visual position of a node of the active revision (F25).
    pub fn set_node_position(&mut self, node_id: &str, x: f64, y: f64) -> Result<()> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::Refused("Coordinates must be finite numbers.".into()));
        }
        let (state, net) = self.active_family_state()?;
        if net.node(node_id).is_none() {
            return Err(Error::Refused(format!("Node not found: {node_id}")));
        }
        state.positions.insert(node_id.to_string(), (x, y));
        self.save_graph()
    }

    /// Open or close an arc of the active revision (F25). Any arc is
    /// accepted, as in the NW5 backend; the interface offers it for valves.
    /// The state applies to the next run. The interface clears the results
    /// it shows ([`GraphData::clear_results`], D6).
    // PROPOSAL: the API does not restrict the state to `VALVE` edges (the
    // NW5 inspector shows the button for valves only).
    pub fn set_arc_open(&mut self, arc_id: &str, open: bool) -> Result<()> {
        let (state, net) = self.active_family_state()?;
        if !net.arcs.iter().any(|a| a.id == arc_id) {
            return Err(Error::Refused(format!("Edge not found: {arc_id}")));
        }
        state.arc_open.insert(arc_id.to_string(), open);
        self.save_graph()
    }

    // ----- runs (F35–F41) -------------------------------------------------

    fn run_path(&self, run_number: u32) -> PathBuf {
        self.root.join("runs").join(run_file_name(run_number))
    }

    /// All runs, by run number.
    pub fn runs(&self) -> Result<Vec<RunRecord>> {
        let dir = self.root.join("runs");
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| io_err(&dir, e))?
            .flatten()
        {
            let path = entry.path();
            let is_run = path.extension().is_some_and(|x| x == "json")
                && path.file_stem().is_some_and(|s| {
                    !s.is_empty() && s.to_string_lossy().chars().all(|c| c.is_ascii_digit())
                });
            if !is_run {
                continue;
            }
            let v = Json::parse(&decode_text(&read_bytes(&path)?))
                .map_err(|e| Error::Format(format!("{}: {e}", path.display())))?;
            out.push(RunRecord::from_json(&v)?);
        }
        out.sort_by_key(|r| r.run_number);
        Ok(out)
    }

    /// One run.
    pub fn run(&self, run_number: u32) -> Result<RunRecord> {
        let path = self.run_path(run_number);
        let v = Json::parse(&decode_text(&read_bytes(&path)?))
            .map_err(|e| Error::Format(format!("{}: {e}", path.display())))?;
        RunRecord::from_json(&v)
    }

    /// Defaults of the configuration dialog for the active revision (F37).
    pub fn simulation_defaults(&self) -> Result<SimulationDefaults> {
        Ok(SimulationDefaults::from_network(
            &self.load_active()?.network,
        ))
    }

    /// Create a run of the active revision with the current arc states
    /// (F38): run number = highest + 1, record `PENDING` written at once.
    /// The request is compacted against the defaults first. Execute the
    /// returned job (on any thread), then call [`Project::finish_run`].
    pub fn start_run(&mut self, requested: &RequestedOptions) -> Result<RunJob> {
        if self.active_run.is_some() {
            // PROPOSAL: one run at a time (ADR-025 item 4); message of the port.
            return Err(Error::Refused("A simulation is already running.".into()));
        }
        let loaded = self.load_active()?;
        let defaults = SimulationDefaults::from_network(&loaded.network);
        let requested = requested.compact(&defaults);
        let problems = requested.validate(&defaults);
        if !problems.is_empty() {
            return Err(Error::Refused(problems.join(" ")));
        }
        let run_number = self.runs()?.iter().map(|r| r.run_number).max().unwrap_or(0) + 1;
        let edge_states = self
            .graph_state(&loaded.revision.file)
            .arc_states(&loaded.network);
        let options = defaults.options(&requested, edge_states.clone());
        let record = RunRecord {
            run_number,
            status: RunStatus::Pending,
            configuration_mode: requested.configuration_mode(),
            revision: loaded.revision.file.clone(),
            revision_version: loaded.revision.version,
            source_format: loaded.revision.format,
            network_name: loaded.revision.network_name(),
            created_at: crate::time::now_iso(),
            started_at: None,
            completed_at: None,
            requested: requested.clone(),
            resolved: Vec::new(),
            edge_states,
            demand: demand_profile(&loaded.network),
            statistics: None,
            failures: Vec::new(),
            error: None,
        };
        let path = self.run_path(run_number);
        write_record(&path, &record)?;
        self.meta.last_run_options = Some(requested);
        self.save_meta()?;
        self.active_run = Some(run_number);
        Ok(RunJob {
            path,
            network: loaded.network,
            defaults,
            options,
            record,
        })
    }

    /// Mark the run in progress as finished (after [`RunJob::execute`]
    /// returned, whatever its result).
    pub fn finish_run(&mut self) {
        self.active_run = None;
    }

    /// Delete a run (ADR-025 item 3). Active runs are refused. The saved
    /// comparison choices are cleared (F39).
    pub fn delete_run(&mut self, run_number: u32) -> Result<()> {
        let r = self.run(run_number)?;
        if self.active_run == Some(run_number) || !r.status.is_terminal() {
            return Err(Error::Refused(
                "Cancel the simulation before deleting it.".into(),
            ));
        }
        let path = self.run_path(run_number);
        std::fs::remove_file(&path).map_err(|e| io_err(&path, e))?;
        self.meta.comparison.clear();
        self.save_meta()
    }

    // ----- comparison workspace (F48) ------------------------------------

    /// Saved comparison choice of the active revision.
    pub fn comparison_workspace(&self) -> Option<&ComparisonWorkspace> {
        let active = self.meta.active_revision.as_deref()?;
        self.meta.comparison.iter().find(|c| c.revision == active)
    }

    /// Save the comparison choice for the active revision.
    pub fn set_comparison_workspace(&mut self, left: u32, right: u32, table: &str) -> Result<()> {
        let Some(active) = self.meta.active_revision.clone() else {
            return Err(Error::Refused("No network imported".into()));
        };
        self.meta.comparison.retain(|c| c.revision != active);
        self.meta.comparison.push(ComparisonWorkspace {
            revision: active,
            left,
            right,
            table: table.to_string(),
        });
        self.save_meta()
    }

    // ----- export (F40, ADR-025 item 10) ---------------------------------

    /// Next free `exports/<base>_v<N><ext>`.
    fn export_path(&self, base: &str, ext: &str) -> Result<PathBuf> {
        let dir = self.root.join("exports");
        create_dir(&dir)?;
        let mut k = 1;
        loop {
            let p = dir.join(format!("{base}_v{k}{ext}"));
            if !p.exists() {
                return Ok(p);
            }
            k += 1;
        }
    }

    /// Write the statistics of a completed run to
    /// `exports/simulation_statistics_v<N>.json` (pretty JSON).
    pub fn export_statistics(&self, run_number: u32) -> Result<ExportOutcome> {
        let r = self.run(run_number)?;
        let stats = match (&r.statistics, r.status.is_completed()) {
            (Some(st), true) => st,
            _ => {
                return Err(Error::Refused(
                    "This simulation does not contain result tables.".into(),
                ));
            }
        };
        let path = self.export_path("simulation_statistics", ".json")?;
        write_text(&path, &stats.pretty())?;
        Ok(ExportOutcome {
            path,
            warnings: Vec::new(),
        })
    }

    /// Write a revision in another format to `exports/<name>_v<N>.<ext>`
    /// (with its `db/` catalogues).
    // PROPOSAL: export files are named after the revision with a version
    // suffix, as the statistics exports.
    pub fn export_revision(&self, file: &str, format: RevisionExport) -> Result<ExportOutcome> {
        let loaded = self.load_revision(file)?;
        let net = &loaded.network;
        let base = loaded.revision.network_name();
        let (text, ext, warnings) = match format {
            RevisionExport::Tpo => (neatwork_nw5_io::write_tpo(net), ".tpo", Vec::new()),
            RevisionExport::Dsg => (neatwork_nw5_io::write_dsg(net), ".dsg", Vec::new()),
            RevisionExport::NwJson => (neatwork_nw5_io::write_nw_json(net), ".json", Vec::new()),
            RevisionExport::Inp => {
                let e =
                    neatwork_nw5_io::write_inp(net).map_err(|e| Error::Refused(e.to_string()))?;
                (e.text, ".inp", e.warnings)
            }
        };
        let path = self.export_path(&base, ext)?;
        write_text(&path, &text)?;
        Ok(ExportOutcome { path, warnings })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names() {
        assert_eq!(folder_name(" a/b:c "), "a_b_c");
        assert_eq!(folder_name(".."), "Project");
        assert_eq!(folder_name("Toy network"), "Toy network");
    }
}
