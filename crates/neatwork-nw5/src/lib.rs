//! NeatWork 5 in Rust: network model and readers of the NW5 files.
//!
//! The engines are in separate crates: `neatwork-nw5-io` (writers, JSON,
//! EPANET, Cruscade), `neatwork-nw5-sim` (simulation) and
//! `neatwork-nw5-design` (design). Specifications: `rust/nw5/specs/`.

pub mod model;
pub mod read;

pub use model::{Arc, ArcState, Client, Network, Node, NodeStatus, Params, Pipe, Terminal};

/// Errors of the NW5 crates.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// A file cannot be read.
    Io(String),
    /// A file has a wrong format.
    Format(String),
    /// The network data is inconsistent (code and message).
    Validation(String),
    /// A solver failed.
    Solver(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(m) => write!(f, "file error: {m}"),
            Error::Format(m) => write!(f, "format error: {m}"),
            Error::Validation(m) => write!(f, "validation error: {m}"),
            Error::Solver(m) => write!(f, "solver error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
