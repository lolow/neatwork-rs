"""Internal data models used by the hydraulic simulation core."""

from dataclasses import dataclass
from typing import Any

import pandas as pd
import numpy as np
from scipy import sparse
from enum import Enum


@dataclass(slots=True)
class PreparedHydraulicNetwork:
    """
    Result of hydraulic preprocessing.

    All tables are copied from the raw SimulationData instance before
    preprocessing. The original input tables are therefore not modified
    until preprocessing succeeds completely.
    """

    nodes: pd.DataFrame
    pipes: pd.DataFrame
    clients: pd.DataFrame
    bornes: pd.DataFrame
    diameters: pd.DataFrame
    solver_data: dict[str, Any]

@dataclass(frozen=True, slots=True)
class SimulationRunOptions:
    """
    Optional runtime settings supplied for one simulation run.

    Values set to None are resolved from the network parameters.
    """

    scenario_count: int | None = None
    seed: int | None = None

    open_taps_probability: float | None = None
    target_flow_lps: float | None = None

    max_generation_batches: int = 20
    oversampling_factor: float = 1.10
    max_draws_per_batch: int = 100_000


@dataclass(frozen=True, slots=True)
class ResolvedSimulationOptions:
    """Fully resolved and validated options for one simulation run."""

    scenario_count: int
    seed: int | None

    uniform_demand: bool
    open_taps_probability: float | None
    target_flow_lps: float | None

    max_generation_batches: int
    oversampling_factor: float
    max_draws_per_batch: int

class SolverStatus(str, Enum):
    """Describe how one interior-point solve terminated. => IPM result status"""

    CONVERGED = "converged"
    MAX_ITERATIONS = "max_iterations"

class ScenarioRunStatus(str, Enum):
    """Describe the execution outcome of one scenario."""

    SUCCESS = "success"
    PREPARATION_FAILED = "preparation_failed"
    SOLVER_FAILED = "solver_failed"
    NOT_CONVERGED = "not_converged"
    
class ScenarioFailureStage(str, Enum):
    """Identify the pipeline stage where one scenario failed."""

    PREPARATION = "preparation"
    SOLVER = "solver"

@dataclass(frozen=True, slots=True)
class ScenarioFailure:
    """Structured failure information for one scenario.

    Attributes:
        scenario_index: Zero-based index of the failed scenario.
        stage: Pipeline stage where execution failed.
        code: Stable machine-readable failure code.
        message: Human-readable failure description.
        details: Additional structured diagnostic information.
    """

    scenario_index: int
    stage: ScenarioFailureStage
    code: str
    message: str
    details: dict[str, Any]

    def to_dict(self) -> dict[str, Any]:
        """Return an API-friendly representation of the failure."""

        result: dict[str, Any] = {
            "scenarioIndex": self.scenario_index,
            "stage": self.stage.value,
            "code": self.code,
            "message": self.message,
        }

        if self.details:
            result["details"] = self.details

        return result

@dataclass(frozen=True, slots=True)
class SimulationBatchResult:
    """Results produced by executing all generated scenarios.

    Numerical output arrays use columns for scenarios. Failed scenarios keep
    ``NaN`` numerical outputs so they cannot be mistaken for valid zero-flow
    solutions.

    Attributes:
        terminal_ids: Terminal identifiers in scenario ordering.
        edge_ids: Pipe identifiers in full network ordering.
        transit_node_ids: Transit-node identifiers in dual-variable ordering.
        demand_by_terminal_lps: Generated terminal demand for every scenario.
        open_client_count_by_terminal: Number of open clients at each terminal for every generated scenario.
        total_demand_lps: Total generated demand for every scenario.
        flow_by_pipe_m3s: Full-network pipe flows by scenario.
        charge_by_pipe_m: Hydraulic charge on pipes by scenario.
        node_dual_by_transit_node: Solver node-dual values by scenario.
        satisfaction_by_terminal: Terminal satisfaction ratios by scenario.
        alpha_by_pipe: Scenario resistance coefficients in full pipe ordering.
        open_pipe_mask_by_scenario: Open-pipe mask for each prepared scenario.
        active_terminal_mask_by_scenario: Terminals with positive generated demand.
        served_terminal_mask_by_scenario: Active terminals whose terminal pipe is hydraulically available.
        iterations: Solver iterations for each scenario.
        success_mask: Scenarios that converged successfully.
        status_by_scenario: Structured execution status for each scenario.
        failures: Structured information about failed scenarios.
        scenario_duration_seconds: Execution duration of each scenario.
        scenario_count: Total number of generated scenarios.
        successful_scenario_count: Number of successful scenarios.
        failed_scenario_count: Number of unsuccessful scenarios.
        seed: Random seed used to generate scenarios.
        duration_seconds: Total batch execution time.
    """

    terminal_ids: np.ndarray
    edge_ids: np.ndarray
    transit_node_ids: np.ndarray

    demand_by_terminal_lps: np.ndarray
    open_client_count_by_terminal: np.ndarray
    total_demand_lps: np.ndarray

    flow_by_pipe_m3s: np.ndarray
    charge_by_pipe_m: np.ndarray
    node_dual_by_transit_node: np.ndarray

    satisfaction_by_terminal: np.ndarray
    alpha_by_pipe: np.ndarray

    open_pipe_mask_by_scenario: np.ndarray
    active_terminal_mask_by_scenario: np.ndarray
    served_terminal_mask_by_scenario: np.ndarray

    iterations: np.ndarray
    success_mask: np.ndarray

    status_by_scenario: tuple[
        ScenarioRunStatus,
        ...
    ]

    failures: tuple[
        ScenarioFailure,
        ...
    ]

    scenario_duration_seconds: np.ndarray

    scenario_count: int
    successful_scenario_count: int
    failed_scenario_count: int

    seed: int | None
    duration_seconds: float
    
@dataclass(frozen=True, slots=True)
class SolverOptions:
    """Numerical settings used by the interior-point solver.

    Attributes:
        flow_tolerance: Tolerance used for flow balance and active bounds.
        pressure_tolerance: Stationarity tolerance for hydraulic pressure.
        centering_tolerance: Centrality tolerance retained for compatibility.
        max_iterations: Maximum number of IPM iterations.
        barrier_reduction: Reduction factor applied to the barrier parameter.
        centering_exponent: Predictor-corrector centering exponent.
        step_safety: Safety factor applied to the feasible step length.
    """

    flow_tolerance: float = 1e-8
    pressure_tolerance: float = 1e-6
    centering_tolerance: float = 1e-10
    max_iterations: int = 30
    barrier_reduction: float = 0.1
    centering_exponent: float = 2.5
    step_safety: float = 0.999

@dataclass(frozen=True, slots=True)
class SolverDiagnostics:
    """Numerical diagnostics measured at the final solver iterate.

    Attributes:
        primal_residual_inf: Infinity norm of the flow-balance residual.
        dual_residual_inf: Infinity norm of the primal-dual residual.
        barrier_parameter: Final interior-point barrier parameter.
    """

    primal_residual_inf: float
    dual_residual_inf: float
    barrier_parameter: float

@dataclass(frozen=True, slots=True)
class SolverResult:
    """Result returned after solving one hydraulic scenario.

    Attributes:
        scenario_index: Zero-based index of the solved scenario.
        flow_by_open_pipe_m3s: Flow on open pipes in reduced-pipe order.
        node_dual: Dual variables associated with flow-balance equations.
        charge_by_open_pipe_m: Hydraulic charge on open pipes.
        lower_multiplier: Multipliers associated with lower flow bounds.
        upper_multiplier: Multipliers associated with upper flow bounds.
        iterations: Number of IPM iterations performed.
        converged: Whether the solver convergence criteria were satisfied.
        status: Structured termination status.
        objective_value: Final hydraulic objective value.
        diagnostics: Final numerical residuals and barrier parameter.
    """

    scenario_index: int

    flow_by_open_pipe_m3s: np.ndarray
    node_dual: np.ndarray
    charge_by_open_pipe_m: np.ndarray

    lower_multiplier: np.ndarray
    upper_multiplier: np.ndarray

    iterations: int
    converged: bool
    status: SolverStatus

    objective_value: float

    diagnostics: SolverDiagnostics

@dataclass(frozen=True, slots=True)
class ScenarioSet:
    """
    Probabilistic demand scenarios aligned with terminal-node ordering.

    Demand values are expressed in litres per second.
    """

    terminal_ids: np.ndarray
    demand_by_terminal_lps: np.ndarray
    open_client_count_by_terminal: np.ndarray
    total_demand_lps: np.ndarray

    scenario_count: int
    seed: int | None

    valid_scenario_probability: float
    generated_draw_count: int
    generation_batch_count: int

@dataclass(slots=True)
class ScenarioProblem:
    """
    Solver-ready hydraulic problem for one probabilistic scenario.

    Full-size arrays follow the complete pipe ordering.
    Reduced masks and the incidence matrix follow only open pipes.

    Attributes:
        scenario_index: Zero-based scenario index.
        terminal_ids: Terminal identifiers in scenario ordering.
        edge_ids: Pipe identifiers in full network ordering.
        demand_by_terminal_m3s: Requested demand for every terminal.
        open_client_count_by_terminal: Number of open clients per terminal.
        active_terminal_mask: Terminals with strictly positive demand.
        served_terminal_mask: Active terminals whose pipe is hydraulically open.
        alpha_by_pipe: Scenario-specific resistance coefficient per pipe.
        beta_by_pipe: Hydraulic friction coefficient per pipe.
        gravity_by_pipe: Gravity contribution per pipe.
        lower_bounds_by_pipe: Lower flow bounds in full pipe ordering.
        upper_bounds_by_pipe: Upper flow bounds in full pipe ordering.
        open_pipe_mask: Pipes included in the reduced optimization problem.
        closed_pipe_mask: Pipes fixed at zero flow.
        incidence_matrix: Incidence matrix reduced to open pipes.
        balance_rhs: Right-hand side of flow-balance constraints.
        terminal_pipe_mask_open: Terminal mask in reduced pipe ordering.
        transit_pipe_mask_open: Transit mask in reduced pipe ordering.
        source_pipe_mask_open: Source mask in reduced pipe ordering.
        active_terminal_client_counts: Open-client counts for served terminals.
        friction_exponent: Hydraulic friction exponent ``lambda``.
        bounded_flow: Whether terminal flows are explicitly demand-bounded.
    """

    scenario_index: int

    terminal_ids: np.ndarray
    edge_ids: np.ndarray

    demand_by_terminal_m3s: np.ndarray
    open_client_count_by_terminal: np.ndarray

    active_terminal_mask: np.ndarray
    served_terminal_mask: np.ndarray

    alpha_by_pipe: np.ndarray
    beta_by_pipe: np.ndarray
    gravity_by_pipe: np.ndarray

    lower_bounds_by_pipe: np.ndarray
    upper_bounds_by_pipe: np.ndarray

    open_pipe_mask: np.ndarray
    closed_pipe_mask: np.ndarray

    incidence_matrix: sparse.csr_matrix
    balance_rhs: np.ndarray

    terminal_pipe_mask_open: np.ndarray
    transit_pipe_mask_open: np.ndarray
    source_pipe_mask_open: np.ndarray

    active_terminal_client_counts: np.ndarray

    friction_exponent: float
    bounded_flow: bool

    def to_solver_work(self) -> dict[str, Any]:
        """
        Build the legacy solver dictionary.

        This adapter is kept temporarily for backward compatibility.
        New simulation code should pass ``ScenarioProblem`` directly to
        ``InteriorPointSolver.solve()``.
        """

        open_mask = self.open_pipe_mask

        return {
            "alpha": self.alpha_by_pipe[open_mask].copy(),
            "beta": self.beta_by_pipe[open_mask].copy(),
            "xsup": self.upper_bounds_by_pipe[open_mask].copy(),
            "xinf": self.lower_bounds_by_pipe[open_mask].copy(),
            "closed": self.closed_pipe_mask.copy(),
            "open": self.open_pipe_mask.copy(),
            "A": self.incidence_matrix.copy(),
            "b": self.balance_rhs.copy(),
            "ArcT": self.terminal_pipe_mask_open.copy(),
            "ArcB": self.transit_pipe_mask_open.copy(),
            "ArcS": self.source_pipe_mask_open.copy(),
            "clients": self.active_terminal_client_counts.copy(),
            "gravit": self.gravity_by_pipe[open_mask].copy(),
            "scenario_index": self.scenario_index,
            "friction_exponent": self.friction_exponent,
        }

