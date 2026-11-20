//! NeatWork 5 commands of the command line.

use neatwork_core::project::design::{DEFAULT_DIAMETERS, DEFAULT_ORIFICES};
use std::path::Path;

use neatwork_nw5::Network;
use neatwork_nw5::read::{Read, read_dsg, read_tpo};
use neatwork_nw5_io::epanet::{DemandPolicy, InpImportOptions};
use neatwork_nw5_io::txt::FlowUnit;

fn ext(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
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
        let unit = if o.txt_cubic_metres {
            FlowUnit::CubicMetresPerSecond
        } else {
            FlowUnit::LitresPerSecond
        };
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
                    Read {
                        network: imp.network,
                        warnings,
                    }
                })
                .map_err(|e| e.to_string());
        }
        other => {
            return Err(format!(
                "{}: unknown input type `.{other}`",
                input.display()
            ));
        }
    };
    r.map_err(|e| e.to_string())
}

/// Catalogues of a `.tpo` without `!Diameters` or `!Orifices`, as MATLAB
/// finds them: explicit files, else `DiamBase<file>.db`,
/// `DiamBase<folder>.db` or `diameters.db`, and `orifices.db`, next to the
/// input. The last fallback is the catalog shipped with NeatWork 2024.
pub fn complete_catalogues(
    net: &mut Network,
    input: &Path,
    diameters: Option<&Path>,
    orifices: Option<&Path>,
) -> Result<(), String> {
    let dir = input.parent().unwrap_or(Path::new("."));
    let folder = dir
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    if net.pipes.is_empty() || diameters.is_some() {
        let candidates: Vec<std::path::PathBuf> = match diameters {
            Some(p) => vec![p.to_path_buf()],
            None => vec![
                dir.join(format!("DiamBase{}.db", stem(input))),
                dir.join(format!("DiamBase{folder}.db")),
                dir.join("diameters.db"),
            ],
        };
        if let Some(p) = candidates.iter().find(|p| p.is_file()) {
            net.pipes =
                neatwork_nw5::read::read_diameters_db(&text(p)?).map_err(|e| e.to_string())?;
            eprintln!("diameters: {}", p.display());
        } else if let Some(p) = diameters {
            return Err(format!("cannot read {}: no such file", p.display()));
        } else {
            net.pipes = neatwork_nw5::read::read_diameters_db(DEFAULT_DIAMETERS)
                .map_err(|e| e.to_string())?;
            eprintln!("diameters: NeatWork 2024 shipped catalog");
        }
    }
    if net.orifices.is_empty() || orifices.is_some() {
        let p = orifices.map_or_else(|| dir.join("orifices.db"), Path::to_path_buf);
        if p.is_file() {
            net.orifices = neatwork_nw5::read::read_orifices_db(&text(&p)?);
            eprintln!("orifices: {}", p.display());
        } else if orifices.is_some() {
            return Err(format!("cannot read {}: no such file", p.display()));
        } else {
            net.orifices = neatwork_nw5::read::read_orifices_db(DEFAULT_ORIFICES);
            eprintln!("orifices: NeatWork 2024 shipped catalog");
        }
    }
    Ok(())
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
        other => {
            return Err(format!(
                "{}: unknown output type `.{other}`",
                output.display()
            ));
        }
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

/// Options of `design` with the NW5 engine.
pub struct DesignArgs {
    pub diameters: Option<std::path::PathBuf>,
    pub orifices: Option<std::path::PathBuf>,
    pub cvar: bool,
    pub monte_carlo: bool,
    pub seed: u32,
    pub params: Vec<String>,
    pub pressures: bool,
}

/// `neatwork design` with the NW5 engine: a `.tpo` (or a `.dsg` to
/// reinforce) gives a design. The output extension gives the format
/// (`.dsg` by default, `.json`, `.inp`).
pub fn design(input: &Path, output: Option<&Path>, a: &DesignArgs) -> Result<(), String> {
    use neatwork_nw5_design::{DesignOptions, LoadMethod, RiskMeasure};
    let o = ConvertOptions {
        upgrade: false,
        txt_prefix: None,
        txt_cubic_metres: false,
        inp_open_demands: false,
        json_kind: "matlab".into(),
    };
    let mut r = read_any(input, &o)?;
    for w in &r.warnings {
        eprintln!("warning: {w}");
    }
    complete_catalogues(
        &mut r.network,
        input,
        a.diameters.as_deref(),
        a.orifices.as_deref(),
    )?;
    let mut overrides = Vec::new();
    for p in &a.params {
        let (k, v) = p
            .split_once('=')
            .ok_or_else(|| format!("--param {p}: expected NAME=VALUE"))?;
        let v: f64 = v
            .trim()
            .parse()
            .map_err(|_| format!("--param {p}: not a number"))?;
        overrides.push((k.trim().to_string(), v));
    }
    let opts = DesignOptions {
        params: overrides,
        load_method: if a.monte_carlo {
            LoadMethod::MonteCarlo
        } else {
            LoadMethod::Auto
        },
        risk: if a.cvar {
            Some(RiskMeasure::Cvar)
        } else {
            None
        },
        seed: a.seed,
        n_scen_load: None,
    };
    let res = neatwork_nw5_design::design(&r.network, &opts).map_err(|e| e.to_string())?;
    for w in &res.warnings {
        eprintln!("warning: {w}");
    }
    let name = r.network.name.clone();
    let default = input.with_file_name(format!("{name}.dsg"));
    let out = output.unwrap_or(&default);
    for w in write_any(&res.network, out, "matlab")? {
        eprintln!("warning: {w}");
    }
    eprintln!(
        "cost: optimal {:.2}, final {:.2}",
        res.costs.optim, res.costs.final_cost
    );
    if a.pressures {
        println!("succ,design flow (L/s),margin (m)");
        for (s, q, m) in res.tfact() {
            println!("{s},{q},{m}");
        }
    }
    Ok(())
}

pub struct SimulateArgs {
    /// Random scenarios asked (`--type random`, no `--open`).
    pub random: bool,
    pub nbsim: Option<String>,
    pub opentaps: Option<String>,
    pub orifice: Option<String>,
    pub seed: Option<u64>,
    pub diameters: Option<std::path::PathBuf>,
}

/// NW5 simulation: random scenarios on a design, statistics written as JSON
/// (python-core payload).
pub fn simulate(input: &Path, output: Option<&Path>, a: &SimulateArgs) -> Result<(), String> {
    use neatwork_nw5_sim::{OrificeMode, SimulationOptions};
    if !a.random {
        return Err(
            "the NW5 engine runs random scenarios only; use --engine nw4 for tap-by-tap and handmade simulations"
                .into(),
        );
    }
    let o = ConvertOptions {
        upgrade: false,
        txt_prefix: None,
        txt_cubic_metres: false,
        inp_open_demands: false,
        json_kind: "matlab".into(),
    };
    let mut r = read_any(input, &o)?;
    for w in &r.warnings {
        eprintln!("warning: {w}");
    }
    complete_catalogues(&mut r.network, input, a.diameters.as_deref(), None)?;
    let num = |name: &str, v: &Option<String>| -> Result<Option<f64>, String> {
        v.as_deref()
            .map(|t| {
                t.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("--{name} {t}: not a number"))
            })
            .transpose()
    };
    let mut opts = SimulationOptions {
        scenario_count: num("nbsim", &a.nbsim)?.map(|n| n.max(0.0) as usize),
        seed: a.seed,
        open_taps_probability: num("opentaps", &a.opentaps)?,
        ..SimulationOptions::default()
    };
    opts.overrides.orifice_mode = match a.orifice.as_deref() {
        Some("ideal") => Some(OrificeMode::Ideal),
        Some("commercial") => Some(OrificeMode::Commercial),
        _ => None,
    };
    let res =
        neatwork_nw5_sim::simulate(&r.network, &opts).map_err(|e| format!("{}: {e}", e.code()))?;
    let json = neatwork_nw5_sim::statistics_json(&res).map_err(|e| format!("{}: {e}", e.code()))?;
    let output = output.map_or_else(
        || input.with_file_name(format!("{}.sim.json", stem(input))),
        Path::to_path_buf,
    );
    std::fs::write(&output, json).map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    eprintln!(
        "scenarios: {} successful of {}, seed {}, {:.2}s -> {}",
        res.successful_count(),
        res.scenarios.len(),
        res.seed,
        res.duration_seconds,
        output.display()
    );
    Ok(())
}
