//! NeatWork 5 file formats: writers of `.tpo`, `.dsg` and `.db` files, the
//! MATLAB `NW-json-v1` design JSON, the application domain JSON, EPANET
//! `.inp` import and export, the Cruscade Txt design reader and the NW4 to
//! NW5 `.dsg` upgrade (`rust/nw5/specs/formats.md`).
//!
//! The readers of `.tpo` and `.dsg` files are in `neatwork_nw5::read`.

pub mod domain;
pub mod epanet;
pub mod fmt;
pub mod json;
pub mod nwjson;
pub mod sha1;
pub mod text;
pub mod txt;
pub mod upgrade;
mod util;

pub use domain::{read_domain_json, write_domain_json};
pub use epanet::{read_inp, write_inp};
pub use nwjson::{read_nw_json, write_nw_json};
pub use text::{write_diameters_db, write_dsg, write_orifices_db, write_tpo};
pub use txt::{read_txt_design, read_txt_folder};
pub use upgrade::{upgrade_nw4_dsg, upgrade_nw4_network};
