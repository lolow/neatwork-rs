"""Batch execution of hydraulic demand scenarios."""

from app.services.simulation_cancellation import is_cancel_requested
from app.neatwork.simulation.exceptions import SimulationCancelledError
from time import perf_counter

import numpy as np

from .exceptions import ScenarioPreparationError, SolverNumericalError

from .models import (
    ScenarioFailure,
    ScenarioFailureStage,
    ScenarioProblem,
    ScenarioRunStatus,
    SimulationBatchResult,
    SolverResult,
)

from .problem_builder import ScenarioProblemBuilder
from .solver import InteriorPointSolver
from collections.abc import Callable
from app.neatwork.hydraulic_units import lps_to_m3s


class SimulationBatchRunner:
    """Execute all generated scenarios and collect structured results.

    Attributes:
        problem_builder: Builder used to prepare one hydraulic problem
            per scenario.
        solver: Interior-point solver used for prepared scenarios.
        scenarios: Generated scenario set owned by the problem builder.
        target_flow_lps: Uniform target flow used by legacy unbounded-flow
            satisfaction calculations.
    """

    def __init__(self, problem_builder: ScenarioProblemBuilder, solver: InteriorPointSolver, *, target_flow_lps: float | None = None, cancel_requested: Callable[[], bool] | None = None) -> None:
        """Initialize a hydraulic batch runner.

        Args:
            problem_builder: Prepared builder containing the hydraulic instance
                and generated demand scenarios.
            solver: Configured interior-point solver used for every successfully
                prepared scenario.
            target_flow_lps: Optional uniform target flow per active client in
                litres per second. This value is required when satisfaction is
                calculated for an unbounded-flow problem.
            cancel_requested: Optional callback that returns True when the
                simulation should be cancelled.

        Raises:
            ValueError: If ``target_flow_lps`` is supplied but is non-finite or
                not strictly positive.
        """

        self.problem_builder = problem_builder
        self.solver = solver
        self.scenarios = problem_builder.scenarios
        self.cancel_requested = cancel_requested

        if target_flow_lps is not None:
            target_flow_lps = float(target_flow_lps)

            if not np.isfinite(target_flow_lps) or target_flow_lps <= 0:
                raise ValueError("target_flow_lps must be finite and strictly positive.")

        self.target_flow_lps = target_flow_lps

    def _compute_satisfaction(self, problem: ScenarioProblem, result: SolverResult) -> np.ndarray:
        """Calculate terminal satisfaction for one converged scenario.

        For bounded-flow problems, satisfaction is calculated as the delivered
        terminal flow divided by the terminal demand. For unbounded-flow problems,
        it is calculated using the number of active clients multiplied by the
        configured target flow per client.

        Only hydraulically served terminals receive calculated values. Active but
        unserved terminals and inactive terminals receive zero.

        Args:
            problem: Prepared scenario containing terminal masks, demands, client
                counts and flow-bound settings.
            result: Converged solver result whose flow vector follows the reduced
                open-pipe ordering.

        Returns:
            One-dimensional satisfaction array following
            ``problem.terminal_ids`` ordering.

        Raises:
            RuntimeError: If terminal flows are misaligned with served terminals,
                a denominator is invalid, or the calculation produces non-finite
                values.
            ValueError: If an unbounded-flow problem requires satisfaction
                calculation but ``target_flow_lps`` was not configured.
        """

        satisfaction = np.zeros(len(problem.terminal_ids), dtype=float)

        served_mask = problem.served_terminal_mask
        served_count = int(served_mask.sum())

        if served_count == 0:
            return satisfaction

        terminal_flows = result.flow_by_open_pipe_m3s[problem.terminal_pipe_mask_open]

        if len(terminal_flows) != served_count:
            raise RuntimeError("Solver terminal flows are not aligned with served terminals.")

        if problem.bounded_flow:
            denominator = problem.demand_by_terminal_m3s[served_mask]

        else:
            if self.target_flow_lps is None:
                raise ValueError("target_flow_lps is required for unbounded-flow satisfaction.")

            target_flow_m3s = lps_to_m3s(self.target_flow_lps)

            denominator = problem.active_terminal_client_counts.astype(float) * target_flow_m3s

        if not np.isfinite(denominator).all() or np.any(denominator <= 0):
            raise RuntimeError("Terminal satisfaction denominator contains invalid values.")

        satisfaction[served_mask] = terminal_flows / denominator

        if not np.isfinite(satisfaction).all():
            raise RuntimeError("Terminal satisfaction calculation produced invalid values.")

        return satisfaction

    def run(self) -> SimulationBatchResult:
        """Prepare and solve every generated demand scenario sequentially.

        For each scenario, the method:

        1. Builds a validated hydraulic problem.
        2. Executes the interior-point solver.
        3. Records preparation failures, numerical failures or non-convergence.
        4. Validates the shape of successful solver outputs.
        5. Expands reduced open-pipe results to the full network ordering.
        6. Calculates terminal satisfaction and execution duration.

        Expected scenario-level failures do not interrupt the batch. Failed
        scenario columns retain ``NaN`` numerical values and receive a structured
        status and failure record. Unexpected programming errors and broken solver
        contracts are intentionally allowed to propagate.

        Returns:
            Typed batch results containing scenario inputs, full-network numerical
            matrices, masks, statuses, failure information, iteration counts and
            execution durations.

            Entity-based result matrices follow the shape
            ``(entity_count, scenario_count)``. Scenario-based vectors follow the
            shape ``(scenario_count,)``.

        Raises:
            RuntimeError: If a successful solver result violates the expected
                vector shapes or batch execution leaves a scenario without a
                resolved status.
            ValueError: If satisfaction for an unbounded-flow scenario requires
                a target flow that was not configured.
            SimulationCancelledError: If cancellation is requested by the user
                before processing a scenario.
        """

        batch_started = perf_counter()

        scenario_count = self.scenarios.scenario_count

        terminal_count = len(self.scenarios.terminal_ids)

        pipe_count = self.problem_builder.pipe_count

        instance = self.problem_builder.instance

        transit_node_count = int(instance.dat["nbNtra"])

        edge_ids = np.asarray(instance.dat["edge_ids"], dtype=str).copy()

        transit_node_ids = np.asarray(instance.dat["transit_node_ids"], dtype=str).copy()

        flow_by_pipe = np.full((pipe_count, scenario_count), np.nan, dtype=float)

        charge_by_pipe = np.full((pipe_count, scenario_count), np.nan, dtype=float)

        node_dual = np.full((transit_node_count, scenario_count), np.nan, dtype=float)

        satisfaction = np.full((terminal_count, scenario_count), np.nan, dtype=float)

        alpha_by_pipe = np.full((pipe_count, scenario_count), np.nan, dtype=float)

        open_pipe_mask = np.zeros((pipe_count, scenario_count), dtype=bool)

        active_terminal_mask = self.scenarios.demand_by_terminal_lps > 0

        served_terminal_mask = np.zeros((terminal_count, scenario_count), dtype=bool)

        iterations = np.full(scenario_count, -1, dtype=int)

        success_mask = np.zeros(scenario_count, dtype=bool)

        scenario_duration = np.zeros(scenario_count, dtype=float)

        statuses: list[ScenarioRunStatus | None] = [None for _ in range(scenario_count)]

        failures: list[ScenarioFailure] = []

        # Iterate over each scenario and solve it.
        for scenario_index in range(scenario_count):
            # check for cancellation request before starting processing of the scenario
            if self.cancel_requested and self.cancel_requested():
                raise SimulationCancelledError(
                    f"Simulation cancelled before processing scenario {scenario_index + 1}/{scenario_count}"
                )

            scenario_started = perf_counter()

            try:
                try:
                    problem = self.problem_builder.build(scenario_index)

                except ScenarioPreparationError as exc:
                    statuses[scenario_index] = ScenarioRunStatus.PREPARATION_FAILED

                    failures.append(
                        ScenarioFailure(
                            scenario_index=scenario_index,
                            stage=ScenarioFailureStage.PREPARATION,
                            code=exc.error_code,
                            message=str(exc),
                            details=dict(exc.details)
                        )
                    )

                    continue

                open_pipe_mask[:, scenario_index] = problem.open_pipe_mask

                served_terminal_mask[:, scenario_index] = problem.served_terminal_mask

                alpha_by_pipe[:, scenario_index] = problem.alpha_by_pipe

                try:
                    result = self.solver.solve(problem, cancel_requested=self.cancel_requested)

                except SolverNumericalError as exc:
                    statuses[scenario_index] = ScenarioRunStatus.SOLVER_FAILED

                    failures.append(
                        ScenarioFailure(
                            scenario_index=scenario_index,
                            stage=ScenarioFailureStage.SOLVER,
                            code=exc.error_code,
                            message=str(exc),
                            details=dict(exc.details),
                        )
                    )

                    iteration_value = exc.details.get("iteration")

                    if isinstance(iteration_value, int):
                        iterations[scenario_index] = iteration_value

                    continue
            
                iterations[scenario_index] = result.iterations

                if not result.converged:
                    statuses[scenario_index] = ScenarioRunStatus.NOT_CONVERGED

                    failures.append(
                        ScenarioFailure(
                            scenario_index=scenario_index,
                            stage=ScenarioFailureStage.SOLVER,
                            code="SOLVER_MAX_ITERATIONS",
                            message="Solver did not converge within the iteration limit.",
                            details={
                                "iterations": result.iterations,
                                "primalResidualInf": result.diagnostics.primal_residual_inf,
                                "dualResidualInf": result.diagnostics.dual_residual_inf,
                                "barrierParameter": result.diagnostics.barrier_parameter,
                            }
                        )
                    )

                    continue

                expected_open_pipe_count = int(problem.open_pipe_mask.sum())

                if result.flow_by_open_pipe_m3s.shape != (expected_open_pipe_count,):
                    raise RuntimeError("Solver flow result has an invalid shape.")

                if result.charge_by_open_pipe_m.shape != (expected_open_pipe_count,):
                    raise RuntimeError("Solver charge result has an invalid shape.")

                if result.node_dual.shape != (transit_node_count,):
                    raise RuntimeError("Solver node-dual result has an invalid shape.")

                full_flow = np.zeros(pipe_count, dtype=float)

                full_flow[problem.open_pipe_mask] = result.flow_by_open_pipe_m3s

                flow_by_pipe[:, scenario_index] = full_flow

                charge_by_pipe[problem.open_pipe_mask, scenario_index] = result.charge_by_open_pipe_m

                node_dual[:, scenario_index] = result.node_dual

                satisfaction[:, scenario_index] = self._compute_satisfaction(problem, result)

                success_mask[scenario_index] = True

                statuses[scenario_index] = ScenarioRunStatus.SUCCESS
            
            finally:
                scenario_duration[scenario_index] = perf_counter() - scenario_started

        if any(status is None for status in statuses):
            raise RuntimeError("Batch execution finished with an unresolved scenario status.")

        typed_statuses = tuple(status for status in statuses if status is not None)

        successful_count = int(success_mask.sum())

        failed_count = scenario_count - successful_count

        duration_seconds = perf_counter() - batch_started

        return SimulationBatchResult(
            terminal_ids=(np.asarray(self.scenarios.terminal_ids, dtype=str).copy()),
            edge_ids=edge_ids,
            transit_node_ids=transit_node_ids,
            demand_by_terminal_lps=self.scenarios.demand_by_terminal_lps.copy(),
            open_client_count_by_terminal=self.scenarios.open_client_count_by_terminal.copy(),
            total_demand_lps=self.scenarios.total_demand_lps.copy(),
            flow_by_pipe_m3s=flow_by_pipe,
            charge_by_pipe_m=charge_by_pipe,
            node_dual_by_transit_node=node_dual,
            satisfaction_by_terminal=satisfaction,
            alpha_by_pipe=alpha_by_pipe,
            open_pipe_mask_by_scenario=open_pipe_mask,
            active_terminal_mask_by_scenario=active_terminal_mask.copy(),
            served_terminal_mask_by_scenario=served_terminal_mask,
            iterations=iterations,
            success_mask=success_mask,
            status_by_scenario=typed_statuses,
            failures=tuple(failures),
            scenario_duration_seconds=scenario_duration,
            scenario_count=scenario_count,
            successful_scenario_count=successful_count,
            failed_scenario_count=failed_count,
            seed=self.scenarios.seed,
            duration_seconds=float(duration_seconds),
        )


