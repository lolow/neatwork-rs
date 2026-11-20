//! `neatwork`: command line front end of the NeatWork engine.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod nw5;

use clap::{Parser, Subcommand, ValueEnum};
use neatwork_core::engine::lp::{parse_solution, solution_text};
use neatwork_core::{
    Database, DesignOptions, Locale, Lp, SimulationOptions, SimulationType, design, project_name,
    simulate, solve_with_highs,
};

#[derive(Parser)]
#[command(
    name = "neatwork",
    version,
    about = "Design and simulate gravity-driven water networks (NeatWork engine)"
)]
struct Cli {
    /// Engine: NeatWork 5 (default) or NeatWork 4 (the 2024 Java engine,
    /// byte-identical output).
    #[arg(long, global = true, value_enum, default_value_t = Engine::Nw5)]
    engine: Engine,
    /// Language of the localized strings written to files.
    #[arg(long, global = true, value_enum, default_value_t = Lang::En)]
    locale: Lang,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Engine {
    Nw5,
    Nw4,
}

#[derive(Clone, Copy, ValueEnum)]
enum Lang {
    En,
    Fr,
    /// Spanish (`es` is also accepted).
    #[value(alias = "es")]
    Sp,
}

impl From<Lang> for Locale {
    fn from(l: Lang) -> Locale {
        match l {
            Lang::En => Locale::En,
            Lang::Fr => Locale::Fr,
            Lang::Sp => Locale::Sp,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Kind {
    Random,
    Tapbytap,
    Handmade,
}

#[derive(Subcommand)]
enum Command {
    /// Make a design (.dsg) from a topography (.tpo).
    Design {
        /// Topography file.
        input: PathBuf,
        /// Output file [default: <input name>.dsg next to the input].
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Diameter catalog (lines: nominal,SDR,diameter,cost,max head,type,roughness).
        #[arg(long)]
        diameters: Option<PathBuf>,
        /// Orifice catalog (one diameter per line).
        #[arg(long)]
        orifices: Option<PathBuf>,
        /// Write the LP to this file (JSON, MOSEK column layout).
        #[arg(long)]
        lp_dump: Option<PathBuf>,
        /// Write the LP solution to this file (one value per line).
        #[arg(long)]
        lp_solution_out: Option<PathBuf>,
        /// Do not solve: read the LP solution from this file.
        #[arg(long)]
        lp_solution: Option<PathBuf>,
        /// Print the node pressure table.
        #[arg(long)]
        pressures: bool,
        /// NW5: conditional value at risk for the loads (else VaR).
        #[arg(long)]
        cvar: bool,
        /// NW5: Monte Carlo loads also for uniform demand.
        #[arg(long)]
        monte_carlo: bool,
        /// NW5: seed of the Monte Carlo draws.
        #[arg(long, default_value_t = 0)]
        seed: u32,
        /// NW5: parameter override NAME=VALUE (repeatable).
        #[arg(long = "param", value_name = "NAME=VALUE")]
        params: Vec<String>,
    },
    /// Solve an LP file written by `design --lp-dump` (or by the Java
    /// reference harness) with HiGHS; print one column value per line.
    LpSolve {
        /// LP file (JSON).
        input: PathBuf,
    },
    /// Convert a NeatWork 5 network between formats: .tpo, .dsg, .json
    /// (MATLAB NW-json-v1 or application domain JSON), EPANET .inp, or a
    /// Cruscade Txt folder as input.
    Convert {
        /// Input file or Txt folder.
        input: PathBuf,
        /// Output file; its extension gives the format.
        #[arg(short, long)]
        output: PathBuf,
        /// Upgrade a NeatWork 4 .dsg (no !Clients section) to NeatWork 5.
        #[arg(long)]
        upgrade: bool,
        /// Kind of JSON written.
        #[arg(long, value_parser = ["matlab", "domain"], default_value = "matlab")]
        json: String,
        /// Txt folder: prefix of the five files (for example CruscadeDet2406).
        #[arg(long)]
        txt_prefix: Option<String>,
        /// Txt folder: client target flows are in m3/s (Cruscade files).
        #[arg(long)]
        txt_m3s: bool,
        /// EPANET import: one always-open client per demand.
        #[arg(long)]
        inp_open_demands: bool,
    },
    /// Simulate a design (.dsg) and write a simulation (.sim).
    Simulate {
        /// Design file.
        input: PathBuf,
        /// Output file [default: NW4 <design>.<next number>.sim, NW5
        /// <design>.sim.json, next to the input].
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Kind of simulation [default: random]. NW5 runs random scenarios
        /// only.
        #[arg(long = "type", value_enum)]
        kind: Option<Kind>,
        /// Number of random runs [default: simu.nbsim of default.properties, 100].
        #[arg(long)]
        nbsim: Option<String>,
        /// Probability that a tap is open in a random run [default: 0.4].
        #[arg(long)]
        opentaps: Option<String>,
        /// Orifices to use.
        #[arg(long, value_parser = ["commercial", "ideal"])]
        orifice: Option<String>,
        /// Open taps of a handmade simulation, comma separated.
        #[arg(long, value_delimiter = ',')]
        open: Vec<String>,
        /// Seed of the random generator [default: NW4 0, NW5 drawn and
        /// printed].
        #[arg(long)]
        seed: Option<u64>,
        /// Diameter catalog used to refresh the design.
        #[arg(long)]
        diameters: Option<PathBuf>,
    },
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn database(
    diameters: Option<&PathBuf>,
    orifices: Option<&PathBuf>,
) -> Result<Option<Database>, String> {
    if diameters.is_none() && orifices.is_none() {
        return Ok(None);
    }
    let d = diameters.map(|p| read(p)).transpose()?.unwrap_or_default();
    let o = orifices.map(|p| read(p)).transpose()?.unwrap_or_default();
    Ok(Some(Database::new(&d, &o)))
}

/// The GUI names simulations `<design>.<N>.sim` with N one above the
/// largest existing number, starting at 0.
fn next_simulation_path(input: &Path) -> PathBuf {
    let dir = input.parent().unwrap_or(Path::new("."));
    let name = project_name(&file_name(input));
    let prefix = format!("{name}.");
    let mut next = 0u64;
    if let Ok(entries) = std::fs::read_dir(if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    }) {
        for e in entries.flatten() {
            let f = file_name(&e.path());
            if let Some(n) = f
                .strip_prefix(&prefix)
                .and_then(|r| r.strip_suffix(".sim"))
                .and_then(|n| n.parse::<u64>().ok())
            {
                next = next.max(n + 1);
            }
        }
    }
    dir.join(format!("{name}.{next}.sim"))
}

fn run(cli: Cli) -> Result<(), String> {
    let locale: Locale = cli.locale.into();
    match cli.command {
        Command::Design {
            input,
            output,
            diameters,
            orifices,
            lp_dump,
            lp_solution_out,
            lp_solution,
            pressures,
            cvar,
            monte_carlo,
            seed,
            params,
        } => {
            if cli.engine == Engine::Nw5 {
                if lp_dump.is_some() || lp_solution_out.is_some() || lp_solution.is_some() {
                    return Err(
                        "--lp-dump, --lp-solution-out and --lp-solution need --engine nw4".into(),
                    );
                }
                return nw5::design(
                    &input,
                    output.as_deref(),
                    &nw5::DesignArgs {
                        diameters: diameters.clone(),
                        orifices: orifices.clone(),
                        cvar,
                        monte_carlo,
                        seed,
                        params,
                        pressures,
                    },
                );
            }
            let text = read(&input)?;
            let name = project_name(&file_name(&input));
            let opts = DesignOptions {
                locale,
                database: database(diameters.as_ref(), orifices.as_ref())?,
                constraints: Vec::new(),
            };
            let given = lp_solution.as_ref().map(|p| read(p)).transpose()?;
            let mut solved: Vec<f64> = Vec::new();
            let mut solve = |lp: &Lp| {
                let x = match &given {
                    Some(t) => parse_solution(t, lp.numvar)?,
                    None => solve_with_highs(lp)?,
                };
                solved = x.clone();
                Ok(x)
            };
            let out = design(&name, &text, &opts, &mut solve).map_err(|e| e.to_string())?;
            for m in &out.info {
                eprintln!("{m}");
            }
            if let Some(p) = &lp_dump {
                write(p, &out.lp.to_json())?;
            }
            if let Some(p) = &lp_solution_out {
                write(p, &solution_text(&solved))?;
            }
            let output = output.unwrap_or_else(|| input.with_file_name(format!("{name}.dsg")));
            write(&output, &out.dsg)?;
            if pressures {
                println!("node,height,pressure,suction");
                for r in &out.pressures {
                    println!("{}", r.join(","));
                }
            }
            Ok(())
        }
        Command::Convert {
            input,
            output,
            upgrade,
            json,
            txt_prefix,
            txt_m3s,
            inp_open_demands,
        } => nw5::convert(
            &input,
            &output,
            &nw5::ConvertOptions {
                upgrade,
                txt_prefix,
                txt_cubic_metres: txt_m3s,
                inp_open_demands,
                json_kind: json,
            },
        ),
        Command::LpSolve { input } => {
            let lp = Lp::from_json(&read(&input)?).map_err(|e| e.to_string())?;
            let x = solve_with_highs(&lp).map_err(|e| e.to_string())?;
            print!("{}", solution_text(&x));
            Ok(())
        }
        Command::Simulate {
            input,
            output,
            kind,
            nbsim,
            opentaps,
            orifice,
            open,
            seed,
            diameters,
        } => {
            if cli.engine == Engine::Nw5 {
                return nw5::simulate(
                    &input,
                    output.as_deref(),
                    &nw5::SimulateArgs {
                        random: kind.is_none_or(|k| k == Kind::Random) && open.is_empty(),
                        nbsim,
                        opentaps,
                        orifice,
                        seed,
                        diameters,
                    },
                );
            }
            let text = read(&input)?;
            let name = project_name(&file_name(&input));
            let opts = SimulationOptions {
                locale,
                kind: match kind.unwrap_or(Kind::Random) {
                    Kind::Random => SimulationType::Random,
                    Kind::Tapbytap => SimulationType::TapByTap,
                    Kind::Handmade => SimulationType::Handmade,
                },
                nb_sim: nbsim,
                open_taps: opentaps,
                orifice,
                open,
                seed: seed.unwrap_or(0),
                database: database(diameters.as_ref(), None)?,
            };
            let sim = simulate(&name, &text, &opts).map_err(|e| e.to_string())?;
            let output = output.unwrap_or_else(|| next_simulation_path(&input));
            write(&output, &sim)?;
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("neatwork: {e}");
            ExitCode::FAILURE
        }
    }
}
