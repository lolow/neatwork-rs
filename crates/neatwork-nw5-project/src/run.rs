//! Simulation runs (`specs/app.md` F35–F41): defaults and overrides with
//! the source of each resolved value, the run record stored in
//! `runs/NNNN.json`, and the execution of a run on a background thread
//! with progress and a cancel flag.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use neatwork_nw5::Network;
use neatwork_nw5_sim::{OrificeMode, SimError, SimulationOptions, SolverOptions};

use crate::json::{
    Json, from_sim, get_array, get_bool, get_f64, get_str, get_u64, n, obj, opt_n, opt_s, s,
};
use crate::revision::SourceFormat;
use crate::{Error, Result};

/// Configuration options a user can change for one run (F36). `None`
/// keeps the default of the active revision.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RequestedOptions {
    /// Number of scenarios (≥ 1).
    pub scenario_count: Option<usize>,
    /// Explicit seed; `None` draws a fresh seed when the run starts.
    pub seed: Option<u64>,
    /// Bounded flow.
    pub bounded_flow: Option<bool>,
    /// Orifice model.
    pub orifice_mode: Option<OrificeMode>,
    /// Maximum generation batches (≥ 1).
    pub max_generation_batches: Option<usize>,
    /// Oversampling factor (> 1).
    pub oversampling_factor: Option<f64>,
    /// Maximum draws per batch (≥ 1).
    pub max_draws_per_batch: Option<usize>,
}

/// `IDEAL` / `COMMERCIAL`.
pub fn orifice_mode_str(m: OrificeMode) -> &'static str {
    match m {
        OrificeMode::Ideal => "IDEAL",
        OrificeMode::Commercial => "COMMERCIAL",
    }
}

/// "Ideal" / "Commercial".
pub fn orifice_mode_label(m: OrificeMode) -> &'static str {
    match m {
        OrificeMode::Ideal => "Ideal",
        OrificeMode::Commercial => "Commercial",
    }
}

fn parse_orifice_mode(text: &str) -> Option<OrificeMode> {
    match text.trim().to_ascii_uppercase().as_str() {
        "IDEAL" | "0" | "FALSE" => Some(OrificeMode::Ideal),
        "COMMERCIAL" | "1" | "TRUE" => Some(OrificeMode::Commercial),
        _ => None,
    }
}

fn usize_json(x: usize) -> Json {
    Json::Number(x as f64)
}

impl RequestedOptions {
    /// Keep only the values that differ from the defaults (F37: only those
    /// are sent). An explicit seed is always kept.
    pub fn compact(&self, d: &SimulationDefaults) -> RequestedOptions {
        RequestedOptions {
            scenario_count: self.scenario_count.filter(|v| *v != d.scenario_count),
            seed: self.seed,
            bounded_flow: self.bounded_flow.filter(|v| *v != d.bounded_flow),
            orifice_mode: self.orifice_mode.filter(|v| *v != d.orifice_mode),
            max_generation_batches: self
                .max_generation_batches
                .filter(|v| *v != d.max_generation_batches),
            oversampling_factor: self
                .oversampling_factor
                .filter(|v| *v != d.oversampling_factor),
            max_draws_per_batch: self
                .max_draws_per_batch
                .filter(|v| *v != d.max_draws_per_batch),
        }
    }

    /// Number of runtime overrides ("`<n>` runtime override(s)").
    pub fn override_count(&self) -> usize {
        [
            self.scenario_count.is_some(),
            self.seed.is_some(),
            self.bounded_flow.is_some(),
            self.orifice_mode.is_some(),
            self.max_generation_batches.is_some(),
            self.oversampling_factor.is_some(),
            self.max_draws_per_batch.is_some(),
        ]
        .iter()
        .filter(|b| **b)
        .count()
    }

    /// `Default` without overrides, else `Manual`.
    pub fn configuration_mode(&self) -> ConfigurationMode {
        if self.override_count() == 0 {
            ConfigurationMode::Default
        } else {
            ConfigurationMode::Manual
        }
    }

    /// Validation messages of the dialog (F36), empty when the request is
    /// valid.
    pub fn validate(&self, d: &SimulationDefaults) -> Vec<String> {
        let mut out = Vec::new();
        if self.scenario_count == Some(0) {
            out.push("Number of scenarios must be a positive whole number.".to_string());
        }
        if !d.demand.uniform && self.bounded_flow == Some(false) {
            out.push(
                "The current solver requires bounded flow for heterogeneous client demand."
                    .to_string(),
            );
        }
        if self.max_generation_batches == Some(0) {
            out.push("Maximum generation batches must be a positive whole number.".to_string());
        }
        if self
            .oversampling_factor
            .is_some_and(|f| f.is_nan() || f <= 1.0)
        {
            out.push("Oversampling factor must be greater than 1.".to_string());
        }
        if self.max_draws_per_batch == Some(0) {
            out.push("Maximum draws per batch must be a positive whole number.".to_string());
        }
        if !d.demand.complete() {
            out.push(
                "Some clients are missing an opening probability or target flow. Complete the Client demand table before simulation."
                    .to_string(),
            );
        }
        out
    }

    /// JSON object with the set values only (camelCase keys).
    pub fn to_json(&self) -> Json {
        let mut m: Vec<(&str, Json)> = Vec::new();
        if let Some(v) = self.scenario_count {
            m.push(("scenarioCount", usize_json(v)));
        }
        if let Some(v) = self.seed {
            m.push(("seed", Json::Number(v as f64)));
        }
        if let Some(v) = self.bounded_flow {
            m.push(("boundedFlow", Json::Bool(v)));
        }
        if let Some(v) = self.orifice_mode {
            m.push(("orificeMode", s(orifice_mode_str(v))));
        }
        if let Some(v) = self.max_generation_batches {
            m.push(("maxGenerationBatches", usize_json(v)));
        }
        if let Some(v) = self.oversampling_factor {
            m.push(("oversamplingFactor", n(v)));
        }
        if let Some(v) = self.max_draws_per_batch {
            m.push(("maxDrawsPerBatch", usize_json(v)));
        }
        obj(m)
    }

    /// Read the JSON object of [`RequestedOptions::to_json`].
    pub fn from_json(v: &Json) -> RequestedOptions {
        let us = |k: &str| get_u64(v, k).and_then(|x| usize::try_from(x).ok());
        RequestedOptions {
            scenario_count: us("scenarioCount"),
            seed: get_u64(v, "seed"),
            bounded_flow: get_bool(v, "boundedFlow"),
            orifice_mode: get_str(v, "orificeMode").and_then(|t| parse_orifice_mode(&t)),
            max_generation_batches: us("maxGenerationBatches"),
            oversampling_factor: get_f64(v, "oversamplingFactor"),
            max_draws_per_batch: us("maxDrawsPerBatch"),
        }
    }

    /// Rows "Parameter / Value" of the execution details (F41).
    pub fn rows(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(v) = self.scenario_count {
            out.push(("scenarioCount".to_string(), v.to_string()));
        }
        if let Some(v) = self.seed {
            out.push(("seed".to_string(), v.to_string()));
        }
        if let Some(v) = self.bounded_flow {
            out.push(("boundedFlow".to_string(), v.to_string()));
        }
        if let Some(v) = self.orifice_mode {
            out.push(("orificeMode".to_string(), orifice_mode_str(v).to_string()));
        }
        if let Some(v) = self.max_generation_batches {
            out.push(("maxGenerationBatches".to_string(), v.to_string()));
        }
        if let Some(v) = self.oversampling_factor {
            out.push(("oversamplingFactor".to_string(), v.to_string()));
        }
        if let Some(v) = self.max_draws_per_batch {
            out.push(("maxDrawsPerBatch".to_string(), v.to_string()));
        }
        out
    }
}

/// `DEFAULT` (no override) or `MANUAL`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfigurationMode {
    /// No runtime override.
    #[default]
    Default,
    /// At least one override.
    Manual,
}

impl ConfigurationMode {
    /// `DEFAULT` / `MANUAL`.
    pub fn as_str(self) -> &'static str {
        match self {
            ConfigurationMode::Default => "DEFAULT",
            ConfigurationMode::Manual => "MANUAL",
        }
    }

    /// Badge text "Default" / "Manual".
    pub fn label(self) -> &'static str {
        match self {
            ConfigurationMode::Default => "Default",
            ConfigurationMode::Manual => "Manual",
        }
    }
}

/// Client demand of the active revision (F36 "Client demand").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientDemand {
    /// Clients.
    pub client_count: usize,
    /// Clients with a finite probability and target flow.
    pub complete_client_count: usize,
    /// Complete and one shared probability and target flow.
    pub uniform: bool,
    /// Shared probability (uniform demand).
    pub open_probability: Option<f64>,
    /// Shared target flow, L/s (uniform demand).
    pub target_flow_lps: Option<f64>,
}

impl ClientDemand {
    /// Every client resolves to a probability and a target flow.
    pub fn complete(&self) -> bool {
        self.client_count > 0 && self.complete_client_count == self.client_count
    }
}

/// Defaults of the configuration dialog, from the active revision (F37).
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationDefaults {
    /// Number of scenarios (default 100).
    pub scenario_count: usize,
    /// Orifice model (default commercial).
    pub orifice_mode: OrificeMode,
    /// Bounded flow (default off; forced on when demand is not uniform).
    pub bounded_flow: bool,
    /// Service quality (0.65).
    pub service_quality: f64,
    /// Required pressure, m (the parameter, else the single terminal
    /// pressure, else 2).
    pub required_pressure_m: f64,
    /// Orifice coefficient (1).
    pub orifice_coefficient: f64,
    /// Faucet coefficient (1).
    pub faucet_coefficient: f64,
    /// Water temperature, °C (20).
    pub water_temperature_c: f64,
    /// Commercial pipe length, m (6).
    pub commercial_pipe_length_m: f64,
    /// Maximum generation batches (20).
    pub max_generation_batches: usize,
    /// Oversampling factor (1.1).
    pub oversampling_factor: f64,
    /// Maximum draws per batch (100000).
    pub max_draws_per_batch: usize,
    /// Client demand summary.
    pub demand: ClientDemand,
}

/// Number with a comma accepted as decimal separator.
fn parse_number(text: &str) -> Option<f64> {
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        t => parse_number(t).map(|v| v != 0.0),
    }
}

fn param_text<'a>(net: &'a Network, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| net.params.raw(k))
}

fn param_number(net: &Network, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|k| net.params.raw(k).and_then(parse_number))
}

fn param_count(net: &Network, keys: &[&str]) -> Option<usize> {
    param_number(net, keys)
        .filter(|v| *v >= 1.0)
        .map(|v| v.trunc() as usize)
}

impl SimulationDefaults {
    /// Defaults of a network. A `.tpo` or `.dsg` holds one parameter list,
    /// so the priority `params > designParams > technicalParams` of the
    /// domain JSON reduces to the key aliases.
    pub fn from_network(net: &Network) -> SimulationDefaults {
        let complete: Vec<&neatwork_nw5::Client> = net
            .clients
            .iter()
            .filter(|c| c.probability.is_finite() && c.target_flow.is_finite())
            .collect();
        let mut demand = ClientDemand {
            client_count: net.clients.len(),
            complete_client_count: complete.len(),
            ..ClientDemand::default()
        };
        if demand.complete() {
            let first = complete[0];
            demand.uniform = complete
                .iter()
                .all(|c| c.probability == first.probability && c.target_flow == first.target_flow);
            if demand.uniform {
                demand.open_probability = Some(first.probability);
                demand.target_flow_lps = Some(first.target_flow);
            }
        }
        let bounded = param_text(
            net,
            &[
                "bounded_flow",
                "bounded-flow",
                "boundedFlow",
                "boundedFlowEnabled",
                "bounded_flow_enabled",
            ],
        )
        .and_then(parse_bool)
        .unwrap_or(false);
        let mut pressures: Vec<f64> = Vec::new();
        for t in &net.terminals {
            if !pressures.contains(&t.pressure) {
                pressures.push(t.pressure);
            }
        }
        SimulationDefaults {
            scenario_count: param_count(
                net,
                &[
                    "nbsim",
                    "scenarioCount",
                    "scenario_count",
                    "numberOfScenarios",
                    "number_of_scenarios",
                ],
            )
            .unwrap_or(100),
            orifice_mode: param_text(
                net,
                &[
                    "orif_com",
                    "orificeCommercial",
                    "orifice_commercial",
                    "orificeMode",
                    "orifice_mode",
                ],
            )
            .and_then(parse_orifice_mode)
            .unwrap_or(OrificeMode::Commercial),
            bounded_flow: if demand.uniform { bounded } else { true },
            service_quality: param_number(
                net,
                &[
                    "servicequal",
                    "serviceequal",
                    "serviceQuality",
                    "service_quality",
                ],
            )
            .unwrap_or(0.65),
            required_pressure_m: param_number(
                net,
                &["pressure", "requiredPressureM", "required_pressure_m"],
            )
            .or(if pressures.len() == 1 {
                Some(pressures[0])
            } else {
                None
            })
            .unwrap_or(2.0),
            orifice_coefficient: param_number(
                net,
                &["orifcoef", "orificeCoefficient", "orifice_coefficient"],
            )
            .unwrap_or(1.0),
            faucet_coefficient: param_number(
                net,
                &["faucetcoef", "faucetCoefficient", "faucet_coefficient"],
            )
            .unwrap_or(1.0),
            water_temperature_c: param_number(
                net,
                &[
                    "watertemp",
                    "waterTemperatureC",
                    "water_temperature_c",
                    "waterTemperature",
                    "water_temperature",
                ],
            )
            .unwrap_or(20.0),
            commercial_pipe_length_m: param_number(
                net,
                &[
                    "pipelength",
                    "commercialPipeLengthM",
                    "commercial_pipe_length_m",
                    "pipeLength",
                    "pipe_length",
                ],
            )
            .unwrap_or(6.0),
            max_generation_batches: param_count(
                net,
                &["maxGenerationBatches", "max_generation_batches"],
            )
            .unwrap_or(20),
            oversampling_factor: param_number(net, &["oversamplingFactor", "oversampling_factor"])
                .unwrap_or(1.1),
            max_draws_per_batch: param_count(net, &["maxDrawsPerBatch", "max_draws_per_batch"])
                .unwrap_or(100_000),
            demand,
        }
    }

    /// Simulation options of a request: the requested values, else the
    /// defaults, with the open state of every arc.
    pub fn options(
        &self,
        requested: &RequestedOptions,
        arc_open: Vec<(String, bool)>,
    ) -> SimulationOptions {
        let mut o = SimulationOptions {
            scenario_count: Some(requested.scenario_count.unwrap_or(self.scenario_count)),
            seed: requested.seed,
            max_generation_batches: requested
                .max_generation_batches
                .unwrap_or(self.max_generation_batches),
            oversampling_factor: requested
                .oversampling_factor
                .unwrap_or(self.oversampling_factor),
            max_draws_per_batch: requested
                .max_draws_per_batch
                .unwrap_or(self.max_draws_per_batch),
            arc_open,
            ..SimulationOptions::default()
        };
        o.overrides.orifice_mode = requested.orifice_mode;
        o.overrides.bounded_flow = requested.bounded_flow;
        o
    }
}

/// Source of a resolved value (F37).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterSource {
    /// Set by the user for this run.
    UserOverride,
    /// From the revision (or the built-in default).
    ProjectDefault,
    /// Drawn when the run started (seed).
    Generated,
    /// Not used by this run.
    NotApplicable,
    /// From the client demand table.
    ClientDemandTable,
}

impl ParameterSource {
    /// Stored code (`USER_OVERRIDE`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            ParameterSource::UserOverride => "USER_OVERRIDE",
            ParameterSource::ProjectDefault => "PROJECT_DEFAULT",
            ParameterSource::Generated => "GENERATED",
            ParameterSource::NotApplicable => "NOT_APPLICABLE",
            ParameterSource::ClientDemandTable => "CLIENT_DEMAND_TABLE",
        }
    }

    /// Parse a stored code.
    pub fn parse(text: &str) -> Option<ParameterSource> {
        Some(match text {
            "USER_OVERRIDE" => ParameterSource::UserOverride,
            "PROJECT_DEFAULT" => ParameterSource::ProjectDefault,
            "GENERATED" => ParameterSource::Generated,
            "NOT_APPLICABLE" => ParameterSource::NotApplicable,
            "CLIENT_DEMAND_TABLE" => ParameterSource::ClientDemandTable,
            _ => return None,
        })
    }

    /// Label of the execution details; the client demand table has its own
    /// label (D7).
    pub fn label(self) -> &'static str {
        match self {
            ParameterSource::UserOverride => "Modified by user",
            ParameterSource::ProjectDefault => "Project default",
            ParameterSource::Generated => "Generated",
            ParameterSource::NotApplicable => "Not applicable",
            ParameterSource::ClientDemandTable => "Client demand table",
        }
    }
}

/// A resolved value of a run and its source.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedValue {
    /// Key (`scenarioCount`, `seed`, ...).
    pub key: String,
    /// Value.
    pub value: Json,
    /// Source.
    pub source: ParameterSource,
}

fn solver_json(o: &SolverOptions) -> Json {
    obj(vec![
        ("flowTolerance", n(o.flow_tolerance)),
        ("pressureTolerance", n(o.pressure_tolerance)),
        ("centeringTolerance", n(o.centering_tolerance)),
        ("maxIterations", usize_json(o.max_iterations)),
        ("barrierReduction", n(o.barrier_reduction)),
        ("centeringExponent", n(o.centering_exponent)),
        ("stepSafety", n(o.step_safety)),
    ])
}

/// Resolved values of a completed run, in the order of python-core
/// (`simulation_service.py:440-514`).
pub fn resolve_values(
    net: &Network,
    defaults: &SimulationDefaults,
    requested: &RequestedOptions,
    options: &SimulationOptions,
    result: &neatwork_nw5_sim::SimulationResult,
) -> Vec<ResolvedValue> {
    use ParameterSource::*;
    let src = |set: bool| if set { UserOverride } else { ProjectDefault };
    let uniform = result.uniform_demand;
    let demand_src = if uniform {
        ClientDemandTable
    } else {
        NotApplicable
    };
    let orifice = requested.orifice_mode.unwrap_or(defaults.orifice_mode);
    let mut v = vec![
        (
            "scenarioCount",
            usize_json(result.scenarios.len()),
            src(requested.scenario_count.is_some()),
        ),
        (
            "seed",
            Json::Number(result.seed as f64),
            if requested.seed.is_some() {
                UserOverride
            } else {
                Generated
            },
        ),
        ("uniformDemand", Json::Bool(uniform), ClientDemandTable),
        (
            "openTapsProbability",
            if uniform {
                opt_n(result.open_taps_probability)
            } else {
                Json::Null
            },
            demand_src,
        ),
        (
            "targetFlowLps",
            if uniform {
                opt_n(result.target_flow_lps)
            } else {
                Json::Null
            },
            demand_src,
        ),
        (
            "clientDemandCount",
            usize_json(net.clients.len()),
            ClientDemandTable,
        ),
        (
            "boundedFlow",
            Json::Bool(result.bounded_flow),
            src(requested.bounded_flow.is_some()),
        ),
        (
            "orificeMode",
            s(orifice_mode_str(orifice)),
            src(requested.orifice_mode.is_some()),
        ),
        (
            "serviceQuality",
            n(net.params.get("servicequal")),
            ProjectDefault,
        ),
        (
            "requiredPressureM",
            n(net.params.get("pressure")),
            ProjectDefault,
        ),
        (
            "terminalRequiredPressureM",
            Json::Object(
                net.terminals
                    .iter()
                    .map(|t| (t.id.clone(), n(t.pressure)))
                    .collect(),
            ),
            ProjectDefault,
        ),
        (
            "orificeCoefficient",
            n(net.params.get("orifcoef")),
            if result.bounded_flow {
                NotApplicable
            } else {
                ProjectDefault
            },
        ),
        (
            "faucetCoefficient",
            n(net.params.get("faucetcoef")),
            ProjectDefault,
        ),
        (
            "waterTemperatureC",
            n(net.params.get("watertemp")),
            ProjectDefault,
        ),
        (
            "commercialPipeLengthM",
            n(net.params.get("pipelength")),
            ProjectDefault,
        ),
        (
            "maxGenerationBatches",
            usize_json(options.max_generation_batches),
            src(requested.max_generation_batches.is_some()),
        ),
        (
            "oversamplingFactor",
            n(options.oversampling_factor),
            src(requested.oversampling_factor.is_some()),
        ),
        (
            "maxDrawsPerBatch",
            usize_json(options.max_draws_per_batch),
            src(requested.max_draws_per_batch.is_some()),
        ),
        (
            "solverOptions",
            solver_json(&options.solver),
            ProjectDefault,
        ),
    ];
    v.drain(..)
        .map(|(key, value, source)| ResolvedValue {
            key: key.to_string(),
            value,
            source,
        })
        .collect()
}

/// Run status (F38).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStatus {
    /// Created, not started.
    Pending,
    /// Running.
    Running,
    /// Every scenario succeeded.
    Completed,
    /// At least one scenario had an execution failure.
    CompletedWithErrors,
    /// The run stopped with an error.
    Failed,
    /// A cancel was requested.
    CancelRequested,
    /// Cancelled.
    Cancelled,
}

impl RunStatus {
    /// Stored code.
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::Pending => "PENDING",
            RunStatus::Running => "RUNNING",
            RunStatus::Completed => "COMPLETED",
            RunStatus::CompletedWithErrors => "COMPLETED_WITH_ERRORS",
            RunStatus::Failed => "FAILED",
            RunStatus::CancelRequested => "CANCEL_REQUESTED",
            RunStatus::Cancelled => "CANCELLED",
        }
    }

    /// Parse a stored code.
    pub fn parse(text: &str) -> Option<RunStatus> {
        Some(match text {
            "PENDING" => RunStatus::Pending,
            "RUNNING" => RunStatus::Running,
            "COMPLETED" => RunStatus::Completed,
            "COMPLETED_WITH_ERRORS" => RunStatus::CompletedWithErrors,
            "FAILED" => RunStatus::Failed,
            "CANCEL_REQUESTED" => RunStatus::CancelRequested,
            "CANCELLED" => RunStatus::Cancelled,
            _ => return None,
        })
    }

    /// Completed, with or without scenario failures (has results).
    pub fn is_completed(self) -> bool {
        matches!(self, RunStatus::Completed | RunStatus::CompletedWithErrors)
    }

    /// The run is over (it can be deleted).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RunStatus::Completed
                | RunStatus::CompletedWithErrors
                | RunStatus::Failed
                | RunStatus::Cancelled
        )
    }
}

/// One group of clients of a terminal with the same demand (F46).
#[derive(Clone, Debug, PartialEq)]
pub struct DemandGroup {
    /// Opening probability per client.
    pub open_probability: f64,
    /// Target flow per client, L/s.
    pub target_flow_lps: f64,
    /// Clients in the group.
    pub client_count: usize,
}

/// Demand profile of a terminal (statistics schema 1.2 `demand`).
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalDemand {
    /// Terminal ID.
    pub terminal_id: String,
    /// Clients attached to the terminal.
    pub client_count: usize,
    /// Groups in order of first client.
    pub groups: Vec<DemandGroup>,
}

/// Demand profile of every terminal of a network.
// PROPOSAL: the simulation payload of `neatwork-nw5-sim` is schema 1.1 with
// an empty `demand`; the run record stores the 1.2 demand profile next to
// it, built from the client table (groups by probability and target flow).
pub fn demand_profile(net: &Network) -> Vec<TerminalDemand> {
    net.terminals
        .iter()
        .map(|t| {
            let mut groups: Vec<DemandGroup> = Vec::new();
            let mut count = 0;
            for c in net.clients.iter().filter(|c| c.terminal == t.id) {
                count += 1;
                match groups.iter_mut().find(|g| {
                    g.open_probability == c.probability && g.target_flow_lps == c.target_flow
                }) {
                    Some(g) => g.client_count += 1,
                    None => groups.push(DemandGroup {
                        open_probability: c.probability,
                        target_flow_lps: c.target_flow,
                        client_count: 1,
                    }),
                }
            }
            TerminalDemand {
                terminal_id: t.id.clone(),
                client_count: count,
                groups,
            }
        })
        .collect()
}

/// A failed scenario of a run (F44 "Execution failure records").
#[derive(Clone, Debug, PartialEq)]
pub struct FailureRecord {
    /// Scenario index (0-based).
    pub scenario_index: usize,
    /// `preparation` or `solver`.
    pub stage: String,
    /// Code.
    pub code: String,
    /// Message.
    pub message: String,
}

/// Error that stopped a run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunError {
    /// Code (`SIMULATION_INPUT_INVALID`, ...).
    pub code: String,
    /// Message.
    pub message: String,
}

/// Everything about one run, stored in `runs/NNNN.json` (spec 14.2): the
/// review, the execution details and the comparison read it after a
/// restart.
#[derive(Clone, Debug, PartialEq)]
pub struct RunRecord {
    /// Project-wide run number (never reused).
    pub run_number: u32,
    /// Status.
    pub status: RunStatus,
    /// `DEFAULT` or `MANUAL`.
    pub configuration_mode: ConfigurationMode,
    /// Revision file in `inputs/`.
    pub revision: String,
    /// Version of the revision.
    pub revision_version: u32,
    /// Format of the revision.
    pub source_format: SourceFormat,
    /// Network name (file name without extension).
    pub network_name: String,
    /// Creation time.
    pub created_at: String,
    /// Start time.
    pub started_at: Option<String>,
    /// End time.
    pub completed_at: Option<String>,
    /// Requested overrides.
    pub requested: RequestedOptions,
    /// Resolved values with their sources (completed runs only).
    pub resolved: Vec<ResolvedValue>,
    /// Open state of every arc used by the run.
    pub edge_states: Vec<(String, bool)>,
    /// Demand profile per terminal.
    pub demand: Vec<TerminalDemand>,
    /// Statistics payload (`statistics_payload` of `neatwork-nw5-sim`).
    pub statistics: Option<Json>,
    /// Scenario failures.
    pub failures: Vec<FailureRecord>,
    /// Error of a failed or cancelled run.
    pub error: Option<RunError>,
}

/// File name of a run: `0001.json` (at least 4 digits).
pub fn run_file_name(run_number: u32) -> String {
    format!("{run_number:04}.json")
}

impl RunRecord {
    /// Resolved value of a key.
    pub fn resolved_value(&self, key: &str) -> Option<&ResolvedValue> {
        self.resolved.iter().find(|r| r.key == key)
    }

    /// Effective seed: resolved, else requested.
    pub fn effective_seed(&self) -> Option<u64> {
        match self.resolved_value("seed").map(|r| &r.value) {
            Some(Json::Number(x)) if *x >= 0.0 => Some(*x as u64),
            _ => self.requested.seed,
        }
    }

    /// Number of scenarios: resolved, else requested.
    pub fn scenario_count(&self) -> Option<usize> {
        match self.resolved_value("scenarioCount").map(|r| &r.value) {
            Some(Json::Number(x)) if *x >= 0.0 => Some(*x as usize),
            _ => self.requested.scenario_count,
        }
    }

    /// The JSON file.
    pub fn to_json(&self) -> Json {
        obj(vec![
            ("runNumber", Json::Number(f64::from(self.run_number))),
            ("status", s(self.status.as_str())),
            ("configurationMode", s(self.configuration_mode.as_str())),
            (
                "revision",
                obj(vec![
                    ("file", s(&self.revision)),
                    ("version", Json::Number(f64::from(self.revision_version))),
                    ("format", s(self.source_format.as_str())),
                    ("networkName", s(&self.network_name)),
                ]),
            ),
            ("createdAt", s(&self.created_at)),
            ("startedAt", opt_s(self.started_at.as_deref())),
            ("completedAt", opt_s(self.completed_at.as_deref())),
            ("requestedOptions", self.requested.to_json()),
            (
                "resolvedOptions",
                Json::Array(
                    self.resolved
                        .iter()
                        .map(|r| {
                            obj(vec![
                                ("key", s(&r.key)),
                                ("value", r.value.clone()),
                                ("source", s(r.source.as_str())),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "edgeStates",
                Json::Array(
                    self.edge_states
                        .iter()
                        .map(|(id, open)| {
                            obj(vec![("edgeId", s(id)), ("enabled", Json::Bool(*open))])
                        })
                        .collect(),
                ),
            ),
            (
                "demand",
                obj(vec![(
                    "terminals",
                    Json::Array(
                        self.demand
                            .iter()
                            .map(|t| {
                                obj(vec![
                                    ("terminalId", s(&t.terminal_id)),
                                    ("clientCount", usize_json(t.client_count)),
                                    (
                                        "groups",
                                        Json::Array(
                                            t.groups
                                                .iter()
                                                .map(|g| {
                                                    obj(vec![
                                                        ("openProbability", n(g.open_probability)),
                                                        ("targetFlowLps", n(g.target_flow_lps)),
                                                        ("clientCount", usize_json(g.client_count)),
                                                    ])
                                                })
                                                .collect(),
                                        ),
                                    ),
                                ])
                            })
                            .collect(),
                    ),
                )]),
            ),
            ("statistics", self.statistics.clone().unwrap_or(Json::Null)),
            (
                "failures",
                Json::Array(
                    self.failures
                        .iter()
                        .map(|f| {
                            obj(vec![
                                ("scenarioIndex", usize_json(f.scenario_index)),
                                ("stage", s(&f.stage)),
                                ("code", s(&f.code)),
                                ("message", s(&f.message)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "error",
                match &self.error {
                    Some(e) => obj(vec![("code", s(&e.code)), ("message", s(&e.message))]),
                    None => Json::Null,
                },
            ),
        ])
    }

    /// Read a run file.
    pub fn from_json(v: &Json) -> Result<RunRecord> {
        let bad = |what: &str| Error::Format(format!("run record: {what}"));
        let run_number = get_u64(v, "runNumber")
            .and_then(|x| u32::try_from(x).ok())
            .ok_or_else(|| bad("missing runNumber"))?;
        let status = get_str(v, "status")
            .and_then(|t| RunStatus::parse(&t))
            .ok_or_else(|| bad("unknown status"))?;
        let rev = v.get("revision").cloned().unwrap_or(Json::Null);
        let resolved = get_array(v, "resolvedOptions")
            .iter()
            .filter_map(|r| {
                Some(ResolvedValue {
                    key: get_str(r, "key")?,
                    value: r.get("value").cloned().unwrap_or(Json::Null),
                    source: get_str(r, "source").and_then(|t| ParameterSource::parse(&t))?,
                })
            })
            .collect();
        let edge_states = get_array(v, "edgeStates")
            .iter()
            .filter_map(|e| {
                Some((
                    get_str(e, "edgeId")?,
                    get_bool(e, "enabled").unwrap_or(true),
                ))
            })
            .collect();
        let demand = v
            .get("demand")
            .map(|d| {
                get_array(d, "terminals")
                    .iter()
                    .filter_map(|t| {
                        Some(TerminalDemand {
                            terminal_id: get_str(t, "terminalId")?,
                            client_count: get_u64(t, "clientCount").unwrap_or(0) as usize,
                            groups: get_array(t, "groups")
                                .iter()
                                .map(|g| DemandGroup {
                                    open_probability: get_f64(g, "openProbability")
                                        .unwrap_or(f64::NAN),
                                    target_flow_lps: get_f64(g, "targetFlowLps")
                                        .unwrap_or(f64::NAN),
                                    client_count: get_u64(g, "clientCount").unwrap_or(0) as usize,
                                })
                                .collect(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let failures = get_array(v, "failures")
            .iter()
            .map(|f| FailureRecord {
                scenario_index: get_u64(f, "scenarioIndex").unwrap_or(0) as usize,
                stage: get_str(f, "stage").unwrap_or_default(),
                code: get_str(f, "code").unwrap_or_default(),
                message: get_str(f, "message").unwrap_or_default(),
            })
            .collect();
        let error = match v.get("error") {
            Some(e @ Json::Object(_)) => Some(RunError {
                code: get_str(e, "code").unwrap_or_default(),
                message: get_str(e, "message").unwrap_or_default(),
            }),
            _ => None,
        };
        Ok(RunRecord {
            run_number,
            status,
            configuration_mode: if get_str(v, "configurationMode").as_deref() == Some("MANUAL") {
                ConfigurationMode::Manual
            } else {
                ConfigurationMode::Default
            },
            revision: get_str(&rev, "file").unwrap_or_default(),
            revision_version: get_u64(&rev, "version")
                .and_then(|x| u32::try_from(x).ok())
                .unwrap_or(1),
            source_format: get_str(&rev, "format")
                .and_then(|t| SourceFormat::parse(&t))
                .unwrap_or_default(),
            network_name: get_str(&rev, "networkName").unwrap_or_default(),
            created_at: get_str(v, "createdAt").unwrap_or_default(),
            started_at: get_str(v, "startedAt"),
            completed_at: get_str(v, "completedAt"),
            requested: v
                .get("requestedOptions")
                .map(RequestedOptions::from_json)
                .unwrap_or_default(),
            resolved,
            edge_states,
            demand,
            statistics: match v.get("statistics") {
                Some(Json::Null) | None => None,
                Some(x) => Some(x.clone()),
            },
            failures,
            error,
        })
    }
}

/// Write a record to its file (through a temporary file).
pub(crate) fn write_record(path: &std::path::Path, record: &RunRecord) -> Result<()> {
    crate::write_text(path, &record.to_json().pretty())
}

/// A cancel flag shared between the interface and the run thread.
#[derive(Clone, Debug, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    /// A new flag, not set.
    pub fn new() -> CancelFlag {
        CancelFlag::default()
    }

    /// Ask the run to stop (cooperative: checked before each scenario).
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// The run was asked to stop.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A prepared run: the network, the options and the record of a run that
/// [`crate::project::Project::start_run`] created. It is `Send`, so
/// [`RunJob::execute`] can run on a background thread.
#[derive(Debug)]
pub struct RunJob {
    pub(crate) path: PathBuf,
    pub(crate) network: Network,
    pub(crate) defaults: SimulationDefaults,
    pub(crate) options: SimulationOptions,
    pub(crate) record: RunRecord,
}

impl RunJob {
    /// Run number.
    pub fn run_number(&self) -> u32 {
        self.record.run_number
    }

    /// The record as created (status `PENDING`).
    pub fn record(&self) -> &RunRecord {
        &self.record
    }

    /// The simulation options the run will use.
    pub fn options(&self) -> &SimulationOptions {
        &self.options
    }

    /// Run the simulation and store the final record. `progress(done,
    /// total)` is called before each scenario and at the end. A cancel stops
    /// the run before the next scenario; a run that ends after a cancel
    /// request is `CANCELLED` too (F38). The returned record is the one on
    /// disk; an error means it could not be written.
    pub fn execute(
        self,
        cancel: &CancelFlag,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<RunRecord> {
        let RunJob {
            path,
            network,
            defaults,
            options,
            mut record,
        } = self;
        record.status = RunStatus::Running;
        record.started_at = Some(crate::time::now_iso());
        write_record(&path, &record)?;
        let outcome =
            neatwork_nw5_sim::simulate_with_progress(&network, &options, |done, total| {
                progress(done, total);
                !cancel.is_cancelled()
            });
        record.completed_at = Some(crate::time::now_iso());
        match outcome {
            Ok(result) => {
                record.failures = result
                    .failures
                    .iter()
                    .map(|f| FailureRecord {
                        scenario_index: f.scenario_index,
                        stage: f.stage.as_str().to_string(),
                        code: f.code.to_string(),
                        message: f.message.clone(),
                    })
                    .collect();
                if cancel.is_cancelled() {
                    record.status = RunStatus::Cancelled;
                    record.error = Some(RunError {
                        code: "SIMULATION_CANCELLED".into(),
                        message: "Simulation cancelled.".into(),
                    });
                } else {
                    match neatwork_nw5_sim::statistics_payload(&result) {
                        Ok(payload) => {
                            record.statistics = Some(from_sim(&payload));
                            record.resolved = resolve_values(
                                &network,
                                &defaults,
                                &record.requested,
                                &options,
                                &result,
                            );
                            record.status = if result.failures.is_empty() {
                                RunStatus::Completed
                            } else {
                                RunStatus::CompletedWithErrors
                            };
                        }
                        Err(e) => fail(&mut record, &e),
                    }
                }
            }
            Err(e @ SimError::Cancelled(_)) => {
                record.status = RunStatus::Cancelled;
                record.error = Some(RunError {
                    code: e.code().to_string(),
                    message: e.to_string(),
                });
            }
            Err(e) => fail(&mut record, &e),
        }
        write_record(&path, &record)?;
        Ok(record)
    }
}

fn fail(record: &mut RunRecord, e: &SimError) {
    record.status = RunStatus::Failed;
    record.error = Some(RunError {
        code: e.code().to_string(),
        message: e.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use neatwork_nw5::{Client, Terminal};

    fn net(clients: &[(f64, f64)]) -> Network {
        let mut net = Network::default();
        for (k, (p, q)) in clients.iter().enumerate() {
            net.clients.push(Client {
                id: format!("C{k}"),
                terminal: if k < 2 { "T1".into() } else { "T2".into() },
                probability: *p,
                target_flow: *q,
            });
        }
        net.terminals = vec![
            Terminal {
                id: "T1".into(),
                pressure: 3.0,
            },
            Terminal {
                id: "T2".into(),
                pressure: 3.0,
            },
        ];
        net
    }

    #[test]
    fn defaults_from_parameters() {
        let mut n1 = net(&[(0.3, 0.1), (0.3, 0.1), (0.3, 0.1)]);
        n1.params.set("nbsim", "250");
        n1.params.set("orif_com", "0");
        n1.params.set("watertemp", "12,5");
        let d = SimulationDefaults::from_network(&n1);
        assert_eq!(d.scenario_count, 250);
        assert_eq!(d.orifice_mode, OrificeMode::Ideal);
        assert_eq!(d.water_temperature_c, 12.5);
        assert_eq!(d.required_pressure_m, 3.0);
        assert!(d.demand.uniform && !d.bounded_flow);
        assert_eq!(d.demand.open_probability, Some(0.3));
        let d2 = SimulationDefaults::from_network(&net(&[(0.3, 0.1), (0.5, 0.1), (0.3, 0.1)]));
        assert!(!d2.demand.uniform && d2.bounded_flow);
        assert_eq!(d2.scenario_count, 100);
        assert_eq!(d2.max_draws_per_batch, 100_000);
    }

    #[test]
    fn overrides_and_validation() {
        let d = SimulationDefaults::from_network(&net(&[(0.3, 0.1), (0.5, 0.1), (0.3, f64::NAN)]));
        let r = RequestedOptions {
            scenario_count: Some(0),
            bounded_flow: Some(false),
            oversampling_factor: Some(1.0),
            max_draws_per_batch: Some(0),
            max_generation_batches: Some(0),
            ..RequestedOptions::default()
        };
        assert_eq!(r.validate(&d).len(), 6);
        let ok = SimulationDefaults::from_network(&net(&[(0.3, 0.1); 3]));
        let same = RequestedOptions {
            scenario_count: Some(100),
            seed: Some(42),
            orifice_mode: Some(OrificeMode::Commercial),
            ..RequestedOptions::default()
        };
        let c = same.compact(&ok);
        assert_eq!(c.scenario_count, None);
        assert_eq!(c.orifice_mode, None);
        assert_eq!(c.seed, Some(42));
        assert_eq!(c.configuration_mode(), ConfigurationMode::Manual);
        assert_eq!(
            RequestedOptions::default().configuration_mode(),
            ConfigurationMode::Default
        );
        assert!(c.validate(&ok).is_empty());
        let o = ok.options(&c, vec![("A1".into(), false)]);
        assert_eq!(o.scenario_count, Some(100));
        assert_eq!(o.seed, Some(42));
        assert_eq!(o.arc_open, vec![("A1".to_string(), false)]);
        assert_eq!(RequestedOptions::from_json(&same.to_json()), same);
    }

    #[test]
    fn labels_and_demand() {
        assert_eq!(
            ParameterSource::ClientDemandTable.label(),
            "Client demand table"
        );
        assert_eq!(
            ParameterSource::parse("GENERATED"),
            Some(ParameterSource::Generated)
        );
        assert_eq!(
            RunStatus::parse("COMPLETED_WITH_ERRORS"),
            Some(RunStatus::CompletedWithErrors)
        );
        assert!(RunStatus::Cancelled.is_terminal() && !RunStatus::Running.is_terminal());
        let p = demand_profile(&net(&[(0.3, 0.1), (0.5, 0.1), (0.3, 0.1)]));
        assert_eq!(p[0].client_count, 2);
        assert_eq!(p[0].groups.len(), 2);
        assert_eq!(p[1].groups[0].client_count, 1);
        assert_eq!(run_file_name(7), "0007.json");
        assert_eq!(run_file_name(12345), "12345.json");
    }

    #[test]
    fn run_types_are_send() {
        fn send<T: Send + 'static>() {}
        send::<RunJob>();
        send::<CancelFlag>();
        send::<RunRecord>();
    }
}
