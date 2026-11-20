//! NeatWork 5 simulation engine: Monte-Carlo demand scenarios solved by a
//! predictor-corrector interior-point method, as python-core does it
//! (`rust/nw5/specs/simulation.md`).
//!
//! ```no_run
//! use neatwork_nw5_sim::{simulate, statistics_json, SimulationOptions};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let read = neatwork_nw5::read::read_file(std::path::Path::new("net.dsg"))?;
//! let result = simulate(&read.network, &SimulationOptions::new(100, 42))?;
//! println!("{}", statistics_json(&result)?);
//! # Ok(())
//! # }
//! ```
//!
//! Corrections of python-core defects (ADR-021):
//! - a missing `lambda` uses 1.781, not the DSG reader default 0;
//! - a network without `!Borne` is simulated: the reader gives every borne
//!   the `pressure` parameter;
//! - source flows are reported per source node (sum of its source pipes)
//!   instead of pairing sorted source nodes with sorted source pipes;
//! - the velocity of a pipe whose first segment is empty uses the diameter of
//!   the second segment instead of 1 m;
//! - a negative centering ratio (python `TypeError`) and any Cholesky failure
//!   are scenario solver failures;
//! - a parameter with a text value is NaN (reported by validation) when the
//!   simulation uses it and ignored otherwise (python raises `ValueError`);
//! - the catalogue `Type` column is not validated (unused by the simulation).

mod chol;
mod error;
mod json;
mod options;
mod prep;
mod result;
mod rng;
mod scenario;
mod solver;
mod stats;
mod validate;

use std::time::Instant;

use neatwork_nw5::Network;

pub use error::{SimError, ValidationIssue};
pub use json::Value;
pub use options::{ModeOverrides, OrificeMode, SimulationOptions, SolverOptions};
pub use result::{
    ClientInfo, Diagnostics, FailureStage, GenerationInfo, NodeInfo, PipeInfo, PipeKind,
    ScenarioFailure, ScenarioResult, ScenarioStatus, SimulationResult,
};
pub use stats::{
    PipeVelocity, PressureStats, SourceFlow, Statistics, Summary, TerminalSatisfaction,
    statistics_payload,
};

/// Run a simulation.
pub fn simulate(
    network: &Network,
    options: &SimulationOptions,
) -> Result<SimulationResult, SimError> {
    simulate_with_progress(network, options, |_, _| true)
}

/// Check the network and the options without solving (the validation and
/// option errors of `simulate`).
pub fn validate(network: &Network, options: &SimulationOptions) -> Result<(), SimError> {
    let tables = prep::build_tables(network, options)?;
    let issues = validate::validate_tables(&tables);
    if !issues.is_empty() {
        return Err(SimError::Validation(issues));
    }
    let model = prep::preprocess(tables)?;
    validate::resolve_options(&model.params, &model.clients, options)
        .map_err(SimError::Validation)?;
    Ok(())
}

/// A seed below 2^32 from OS entropy (python-core API: `secrets.randbelow(2**32)`).
fn entropy_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.finish() & 0xFFFF_FFFF
}

/// Run a simulation and report progress. `progress(done, total)` is called
/// before each scenario and once at the end; returning `false` cancels the
/// run (`SIMULATION_CANCELLED`).
pub fn simulate_with_progress(
    network: &Network,
    options: &SimulationOptions,
    mut progress: impl FnMut(usize, usize) -> bool,
) -> Result<SimulationResult, SimError> {
    let tables = prep::build_tables(network, options)?;
    let issues = validate::validate_tables(&tables);
    if !issues.is_empty() {
        return Err(SimError::Validation(issues));
    }
    let model = prep::preprocess(tables)?;
    let resolved = validate::resolve_options(&model.params, &model.clients, options)
        .map_err(SimError::Validation)?;
    let lambda = model.param("lambda");
    if !lambda.is_finite() || lambda <= 0.0 {
        return Err(SimError::ScenarioPreparation {
            message: "Friction exponent 'lambda' must be finite and strictly positive.".into(),
            details: Vec::new(),
        });
    }
    let seed = options.seed.unwrap_or_else(entropy_seed);
    let scenarios = scenario::generate(&model, &resolved, options, seed)?;
    let bounded = model.param("bounded_flow") != 0.0;
    let ctx = solver::Context::new(
        &model,
        resolved.uniform,
        bounded,
        model.param("orif_com") == 1.0,
        lambda,
    );

    let started = Instant::now();
    let total = scenarios.demand.len();
    let npipe = model.pipes.len();
    let nterm = model.terminals.len();
    let ntransit = model.transit.len();
    let mut out = Vec::with_capacity(total);
    let mut failures = Vec::new();
    let mut ws = solver::Workspace::default();
    for k in 0..total {
        if !progress(k, total) {
            return Err(SimError::Cancelled(format!(
                "Simulation cancelled before processing scenario {}/{}",
                k + 1,
                total
            )));
        }
        let mut sr = ScenarioResult {
            status: ScenarioStatus::PreparationFailed,
            iterations: -1,
            demand_lps: scenarios.demand[k].clone(),
            open_clients: scenarios.count[k].clone(),
            open_client_indices: scenarios.open[k].clone(),
            open_pipes: Vec::new(),
            served: vec![false; nterm],
            alpha: vec![f64::NAN; npipe],
            flow: vec![f64::NAN; npipe],
            charge: vec![f64::NAN; npipe],
            head: vec![f64::NAN; ntransit],
            satisfaction: vec![f64::NAN; nterm],
            diagnostics: None,
        };
        let problem = match solver::build(&ctx, &scenarios.demand[k], &scenarios.count[k]) {
            Ok(p) => p,
            Err(e) => {
                failures.push(ScenarioFailure {
                    scenario_index: k,
                    stage: FailureStage::Preparation,
                    code: "SCENARIO_PREPARATION_FAILED",
                    message: e.message,
                    details: e.details,
                });
                out.push(sr);
                continue;
            }
        };
        sr.open_pipes = problem.open_mask.clone();
        sr.served = problem.served.clone();
        sr.alpha = problem.alpha.clone();
        let solution = match solver::solve(&ctx, &problem, &options.solver, &mut ws) {
            Ok(s) => s,
            Err(e) => {
                sr.status = ScenarioStatus::SolverFailed;
                if let Some((_, Value::Int(i))) =
                    e.details.iter().find(|(key, _)| key == "iteration")
                {
                    sr.iterations = *i;
                }
                failures.push(ScenarioFailure {
                    scenario_index: k,
                    stage: FailureStage::Solver,
                    code: "SOLVER_NUMERICAL_FAILED",
                    message: e.message,
                    details: e.details,
                });
                out.push(sr);
                continue;
            }
        };
        sr.iterations = i64::try_from(solution.iterations).unwrap_or(i64::MAX);
        sr.diagnostics = Some(solution.diagnostics);
        if !solution.converged {
            sr.status = ScenarioStatus::NotConverged;
            let d = solution.diagnostics;
            failures.push(ScenarioFailure {
                scenario_index: k,
                stage: FailureStage::Solver,
                code: "SOLVER_MAX_ITERATIONS",
                message: "Solver did not converge within the iteration limit.".into(),
                details: vec![
                    ("iterations".into(), Value::Int(sr.iterations)),
                    (
                        "primalResidualInf".into(),
                        Value::Float(d.primal_residual_inf),
                    ),
                    ("dualResidualInf".into(), Value::Float(d.dual_residual_inf)),
                    ("barrierParameter".into(), Value::Float(d.barrier_parameter)),
                ],
            });
            out.push(sr);
            continue;
        }
        let sat = solver::satisfaction(&ctx, &problem, &solution, resolved.target_flow_lps)
            .map_err(|e| SimError::StatisticsProcessing {
                message: e.message,
                details: e.details,
            })?;
        let mut flow = vec![0.0; npipe];
        for (o, &j) in problem.open.iter().enumerate() {
            flow[j] = solution.x[o];
            sr.charge[j] = solution.charge[o];
        }
        sr.flow = flow;
        sr.head = solution.u;
        sr.satisfaction = sat;
        sr.status = ScenarioStatus::Success;
        out.push(sr);
    }
    progress(total, total);
    let duration_seconds = started.elapsed().as_secs_f64();

    let node_info = |i: usize| NodeInfo {
        id: model.nodes[i].id.clone(),
        node_index: model.nodes[i].index,
        elevation: model.nodes[i].haut,
    };
    let pipes = (0..npipe)
        .map(|j| {
            let p = &model.pipes[j];
            PipeInfo {
                id: p.eid.clone(),
                arc_index: p.index,
                from: p.pred.clone(),
                to: p.succ.clone(),
                kind: model.kind[j],
                open: p.open,
                beta: model.beta[j],
                gravity: model.gravity[j],
                alpha_ideal: model.alpha_ideal[j],
                alpha_com: model.alpha_com[j],
                velocity_diameter: if p.lon1 > 0.0 {
                    model.diam1[j]
                } else {
                    model.diam2[j]
                },
            }
        })
        .collect();
    let clients = model
        .clients
        .iter()
        .enumerate()
        .map(|(c, row)| ClientInfo {
            id: row.id.clone(),
            terminal: model.client_terminal[c],
            probability: scenarios.probability[c],
            target_flow_lps: scenarios.target[c],
        })
        .collect();
    let mut result = SimulationResult {
        network_name: network.name.clone(),
        network_version: String::new(),
        seed,
        uniform_demand: resolved.uniform,
        bounded_flow: bounded,
        lambda,
        open_taps_probability: resolved.open_taps_probability,
        target_flow_lps: resolved.target_flow_lps,
        pipes,
        sources: model.sources.iter().map(|&i| node_info(i)).collect(),
        transit_nodes: model.transit.iter().map(|&i| node_info(i)).collect(),
        terminals: model.terminals.iter().map(|&i| node_info(i)).collect(),
        required_pressure_m: model.preq.clone(),
        clients,
        generation: scenarios.info.clone(),
        scenarios: out,
        failures,
        duration_seconds,
        statistics: None,
    };
    result.statistics = stats::compute(&result);
    Ok(result)
}

/// The statistics payload of schema 1.1 as JSON text (python-core field
/// names, `indent=2`). Fails with `STATISTICS_PROCESSING_FAILED` when no
/// scenario succeeded.
pub fn statistics_json(result: &SimulationResult) -> Result<String, SimError> {
    Ok(statistics_payload(result)?.to_json())
}
