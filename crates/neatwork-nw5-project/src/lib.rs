//! NeatWork 5 projects: the non-GUI logic of the NW5 desktop interface
//! (`rust/nw5/specs/app.md`, phase 7 of `rust/nw5/PLAN.md`, ADR-024 and
//! ADR-025).
//!
//! A project is a folder (spec section 14.2):
//!
//! ```text
//! <Project name>/
//!   project.json      name, createdAt, sourceFormat, displayUnit, activeRevision,
//!                     comparison workspaces, last view, last run options
//!   inputs/           revision files (immutable) and revisions.json
//!   db/               catalogues used by .tpo imports
//!   graph.json        node positions and arc states per revision family
//!   runs/NNNN.json    one record per simulation run
//!   exports/          exported statistics and revisions
//! ```
//!
//! Modules: [`project`] (folder, import, revisions, graph state, runs,
//! export), [`revision`] (naming rules), [`graph`] (graph data, node
//! search), [`run`] (options, run records, execution), [`review`] (result
//! tables), [`compare`] (comparison of two runs) and [`format`](mod@format) (number
//! formats and units).

pub mod compare;
pub mod format;
pub mod graph;
pub mod json;
pub mod project;
pub mod review;
pub mod revision;
pub mod run;
pub mod time;

pub use format::DisplayUnit;
pub use graph::{GraphData, GraphState};
pub use project::{ImportOptions, ImportOutcome, LoadedRevision, Project, ProjectEntry};
pub use revision::{Revision, RevisionName, SourceFormat};
pub use run::{CancelFlag, RequestedOptions, RunJob, RunRecord, RunStatus, SimulationDefaults};

/// Errors of the project crate.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// A file or folder operation failed.
    Io(String),
    /// A stored or imported file has a wrong format.
    Format(String),
    /// The request is refused; the text is the message for the user.
    Refused(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(m) | Error::Format(m) | Error::Refused(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

impl From<neatwork_nw5::Error> for Error {
    fn from(e: neatwork_nw5::Error) -> Error {
        match e {
            neatwork_nw5::Error::Io(m) => Error::Io(m),
            other => Error::Format(other.to_string()),
        }
    }
}

/// Result of the project crate.
pub type Result<T> = std::result::Result<T, Error>;

fn io_err(path: &std::path::Path, e: std::io::Error) -> Error {
    Error::Io(format!("{}: {e}", path.display()))
}

/// Write a text file through a temporary file in the same folder, so a
/// crash never leaves a half-written file.
pub(crate) fn write_text(path: &std::path::Path, text: &str) -> Result<()> {
    write_bytes(path, text.as_bytes())
}

/// Write a file through a temporary file in the same folder.
pub(crate) fn write_bytes(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| io_err(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| io_err(path, e))
}

/// Read a file.
pub(crate) fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| io_err(path, e))
}

/// Text of a file read as UTF-8, else as ISO-8859-1 (F14).
pub fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(t) => t.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_latin1() {
        assert_eq!(decode_text("é".as_bytes()), "é");
        assert_eq!(decode_text(&[0x41, 0xe9]), "Aé");
    }
}
