//! NeatWork 5 commands of the command line.

use std::path::Path;

use neatwork_nw5::Network;
use neatwork_nw5::read::{Read, read_dsg, read_tpo};
use neatwork_nw5_io::epanet::{DemandPolicy, InpImportOptions};
use neatwork_nw5_io::txt::FlowUnit;

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn text(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .map_err(|e| format!("cannot read {}: {e}", path.display()))
}

/// Options of `convert`.
pub struct ConvertOptions {
    /// Upgrade a NW4 `.dsg` (no `!Clients`) to NW5.
    pub upgrade: bool,
    /// Cruscade Txt folder: file prefix and flow unit of the clients.
    pub txt_prefix: Option<String>,
    pub txt_cubic_metres: bool,
    /// EPANET import: one always-open client per demand.
    pub inp_open_demands: bool,
    /// JSON output: `matlab` (NW-json-v1) or `domain` (application).
    pub json_kind: String,
}

/// Read a network in any NW5 input format.
pub fn read_any(input: &Path, o: &ConvertOptions) -> Result<Read, String> {
    if input.is_dir() {
        let prefix = o.txt_prefix.clone().unwrap_or_default();
        let unit = if o.txt_cubic_metres { FlowUnit::CubicMetresPerSecond } else { FlowUnit::LitresPerSecond };
        return neatwork_nw5_io::read_txt_folder(input, &prefix, unit).map_err(|e| e.to_string());
    }
    let t = text(input)?;
    let name = stem(input);
    let r = match ext(input).as_str() {
        "tpo" => read_tpo(&name, &t),
        "dsg" if o.upgrade => neatwork_nw5_io::upgrade_nw4_dsg(&name, &t),
        "dsg" => read_dsg(&name, &t),
        "json" if t.contains("NW-json-v1") => neatwork_nw5_io::read_nw_json(&name, &t),
        "json" => neatwork_nw5_io::read_domain_json(&t),
        "inp" => {
            let opts = InpImportOptions {
                name: name.clone(),
                demand_policy: if o.inp_open_demands {
                    DemandPolicy::DeterministicAlwaysOpen
                } else {
                    DemandPolicy::PreserveUnresolved
                },
                ..InpImportOptions::default()
            };
            return neatwork_nw5_io::read_inp(&t, &opts)
                .map(|imp| {
                    let warnings = imp.issues.iter().map(|i| format!("{i:?}")).collect();
                    Read { network: imp.network, warnings }
                })
                .map_err(|e| e.to_string());
        }
        other => return Err(format!("{}: unknown input type `.{other}`", input.display())),
    };
    r.map_err(|e| e.to_string())
}

/// Write a network in the format of the output extension.
pub fn write_any(net: &Network, output: &Path, json_kind: &str) -> Result<Vec<String>, String> {
    let (body, warnings) = match ext(output).as_str() {
        "tpo" => (neatwork_nw5_io::write_tpo(net), Vec::new()),
        "dsg" => (neatwork_nw5_io::write_dsg(net), Vec::new()),
        "json" if json_kind == "domain" => (neatwork_nw5_io::write_domain_json(net), Vec::new()),
        "json" => (neatwork_nw5_io::write_nw_json(net), Vec::new()),
        "inp" => {
            let e = neatwork_nw5_io::write_inp(net).map_err(|e| e.to_string())?;
            (e.text, e.warnings)
        }
        other => return Err(format!("{}: unknown output type `.{other}`", output.display())),
    };
    std::fs::write(output, body).map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    Ok(warnings)
}

/// `neatwork convert`.
pub fn convert(input: &Path, output: &Path, o: &ConvertOptions) -> Result<(), String> {
    let r = read_any(input, o)?;
    for w in &r.warnings {
        eprintln!("warning: {w}");
    }
    for w in write_any(&r.network, output, &o.json_kind)? {
        eprintln!("warning: {w}");
    }
    Ok(())
}
