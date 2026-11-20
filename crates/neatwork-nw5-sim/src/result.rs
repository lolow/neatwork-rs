//! Raw results of a simulation run (`SimulationBatchResult`, spec 4.6).

use crate::json::Value;
use crate::stats::Statistics;

/// Pipe status (`statut` of a pipe).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipeKind {
    /// 0: the pipe leaves a source.
    Source,
    /// 1: transit pipe.
    Transit,
    /// 2: the pipe ends at a terminal.
    Terminal,
}

/// A pipe of the simulation, in sorted order (`statut, succ, pred, id`).
#[derive(Clone, Debug, PartialEq)]
pub struct PipeInfo {
    /// Arc ID (`external_edge_id`).
    pub id: String,
    /// Index in `Network::arcs`.
    pub arc_index: usize,
    /// `pred`, `succ`.
    pub from: String,
    /// Downstream node.
    pub to: String,
    /// Pipe status.
    pub kind: PipeKind,
    /// Open state used by the run.
    pub open: bool,
    /// Head-loss coefficient: loss = `beta * |q|^lambda` (q in m3/s).
    pub beta: f64,
    /// Gravity term: `-z(source)` on source pipes, `z(terminal)` on
    /// terminal pipes, 0 otherwise, m.
    pub gravity: f64,
    /// Static terminal loss coefficients (ideal and commercial orifice plus
    /// faucet), s2/m5. 0 on other pipes.
    pub alpha_ideal: f64,
    /// See `alpha_ideal`.
    pub alpha_com: f64,
    /// Diameter used for the velocity, m (segment 1, or segment 2 when
    /// segment 1 is empty).
    pub velocity_diameter: f64,
}

/// A node of the simulation, in sorted order (`statut, noeud`).
#[derive(Clone, Debug, PartialEq)]
pub struct NodeInfo {
    /// Node ID.
    pub id: String,
    /// Index in `Network::nodes`.
    pub node_index: usize,
    /// Elevation, m.
    pub elevation: f64,
}

/// Outcome of one scenario.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioStatus {
    /// `success`: converged; included in the statistics.
    Success,
    /// `preparation_failed`: the scenario problem could not be built.
    PreparationFailed,
    /// `solver_failed`: numerical failure of the interior-point method.
    SolverFailed,
    /// `not_converged`: iteration limit reached.
    NotConverged,
}

impl ScenarioStatus {
    /// python-core status text.
    pub fn as_str(self) -> &'static str {
        match self {
            ScenarioStatus::Success => "success",
            ScenarioStatus::PreparationFailed => "preparation_failed",
            ScenarioStatus::SolverFailed => "solver_failed",
            ScenarioStatus::NotConverged => "not_converged",
        }
    }
}

/// Stage of a scenario failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureStage {
    /// Problem construction.
    Preparation,
    /// Interior-point solver.
    Solver,
}

impl FailureStage {
    /// python-core stage text.
    pub fn as_str(self) -> &'static str {
        match self {
            FailureStage::Preparation => "preparation",
            FailureStage::Solver => "solver",
        }
    }
}

/// A failed scenario (`ScenarioFailure`).
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioFailure {
    /// Scenario index (0-based).
    pub scenario_index: usize,
    /// Stage.
    pub stage: FailureStage,
    /// `SCENARIO_PREPARATION_FAILED`, `SOLVER_NUMERICAL_FAILED` or
    /// `SOLVER_MAX_ITERATIONS`.
    pub code: &'static str,
    /// Message.
    pub message: String,
    /// Extra values.
    pub details: Vec<(String, Value)>,
}

impl ScenarioFailure {
    /// JSON record `{scenarioIndex, stage, code, message, details}`.
    pub fn to_value(&self) -> Value {
        Value::Map(vec![
            ("scenarioIndex".into(), Value::from(self.scenario_index)),
            ("stage".into(), Value::from(self.stage.as_str())),
            ("code".into(), Value::from(self.code)),
            ("message".into(), Value::from(self.message.as_str())),
            ("details".into(), Value::Map(self.details.clone())),
        ])
    }
}

/// Final residuals of a solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Diagnostics {
    /// `‖A x‖∞`, m3/s.
    pub primal_residual_inf: f64,
    /// `‖∇f + Aᵀu - s + t‖∞`, m.
    pub dual_residual_inf: f64,
    /// Barrier parameter.
    pub barrier_parameter: f64,
}

/// One scenario: demand and solution. Vectors follow the sorted orders of
/// `SimulationResult` (pipes, transit nodes, terminals). Values that do not
/// exist for the scenario are NaN, as in python-core.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioResult {
    /// Outcome.
    pub status: ScenarioStatus,
    /// Iterations of the solver; -1 when unknown.
    pub iterations: i64,
    /// Demand of each terminal, L/s.
    pub demand_lps: Vec<f64>,
    /// Open clients of each terminal.
    pub open_clients: Vec<u32>,
    /// Indices (in `SimulationResult::clients`) of the open clients.
    pub open_client_indices: Vec<usize>,
    /// Pipes open in the problem (empty if the preparation failed).
    pub open_pipes: Vec<bool>,
    /// Active terminals with an open terminal pipe.
    pub served: Vec<bool>,
    /// Terminal loss coefficient of each pipe (NaN if the preparation
    /// failed).
    pub alpha: Vec<f64>,
    /// Flow of each pipe, m3/s, positive from `from` to `to` (0 on closed
    /// pipes; NaN unless the scenario succeeded).
    pub flow: Vec<f64>,
    /// `∇f + Aᵀu` of each open pipe, m (NaN on closed pipes). On a terminal
    /// pipe `-charge` is the pressure left after the terminal loss.
    pub charge: Vec<f64>,
    /// Hydraulic head of each transit node, m (the multipliers `u`).
    pub head: Vec<f64>,
    /// Satisfaction of each terminal (0 when not served).
    pub satisfaction: Vec<f64>,
    /// Final residuals (absent if the solver did not return).
    pub diagnostics: Option<Diagnostics>,
}

/// Scenario-generation metadata (`ScenarioMetadata`).
#[derive(Clone, Debug, PartialEq)]
pub struct GenerationInfo {
    /// `1 - Π(1 - p)`.
    pub valid_scenario_probability: f64,
    /// Number of draw columns.
    pub generated_draw_count: usize,
    /// Number of batches.
    pub generation_batch_count: usize,
}

/// A client in sorted order (`borne, client`).
#[derive(Clone, Debug, PartialEq)]
pub struct ClientInfo {
    /// Client ID.
    pub id: String,
    /// Terminal index.
    pub terminal: usize,
    /// Opening probability used for the draws.
    pub probability: f64,
    /// Target flow used for the demand, L/s.
    pub target_flow_lps: f64,
}

/// Result of `simulate`.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationResult {
    /// Network name and version (JSON `network`).
    pub network_name: String,
    /// Network version.
    pub network_version: String,
    /// Seed of the draws.
    pub seed: u64,
    /// Uniform demand (`uniform_dem`).
    pub uniform_demand: bool,
    /// Bounded flow (`bounded_flow`).
    pub bounded_flow: bool,
    /// Friction exponent.
    pub lambda: f64,
    /// Uniform opening probability (uniform mode).
    pub open_taps_probability: Option<f64>,
    /// Uniform target flow, L/s (uniform mode).
    pub target_flow_lps: Option<f64>,
    /// Pipes in sorted order.
    pub pipes: Vec<PipeInfo>,
    /// Source nodes in sorted order.
    pub sources: Vec<NodeInfo>,
    /// Transit nodes in sorted order.
    pub transit_nodes: Vec<NodeInfo>,
    /// Terminal nodes in sorted order.
    pub terminals: Vec<NodeInfo>,
    /// Required pressure of each terminal, m.
    pub required_pressure_m: Vec<f64>,
    /// Clients in sorted order.
    pub clients: Vec<ClientInfo>,
    /// Scenario generation.
    pub generation: GenerationInfo,
    /// Scenarios in order.
    pub scenarios: Vec<ScenarioResult>,
    /// Failed scenarios in order.
    pub failures: Vec<ScenarioFailure>,
    /// Wall time of the batch, s.
    pub duration_seconds: f64,
    /// Statistics over the successful scenarios (`None` if none succeeded).
    pub statistics: Option<Statistics>,
}

impl SimulationResult {
    /// Number of successful scenarios.
    pub fn successful_count(&self) -> usize {
        self.scenarios
            .iter()
            .filter(|s| s.status == ScenarioStatus::Success)
            .count()
    }

    /// Pressure (head minus elevation) of transit node `i` in scenario `k`,
    /// m (NaN when out of range or not solved).
    pub fn transit_pressure(&self, k: usize, i: usize) -> f64 {
        match (
            self.scenarios.get(k).and_then(|s| s.head.get(i)),
            self.transit_nodes.get(i),
        ) {
            (Some(h), Some(n)) => h - n.elevation,
            _ => f64::NAN,
        }
    }

    /// Terminal pipe index of each terminal.
    pub fn terminal_pipes(&self) -> Vec<usize> {
        (0..self.pipes.len())
            .filter(|&j| self.pipes[j].kind == PipeKind::Terminal)
            .collect()
    }
}
