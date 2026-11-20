"""Build solver-ready hydraulic problems for demand scenarios."""

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any

import numpy as np
import pandas as pd

from .constants import UNBOUNDED_FLOW_MULTIPLIER
from .exceptions import ScenarioPreparationError
from .models import ScenarioProblem, ScenarioSet
from app.neatwork.hydraulic_units import lps_to_m3s


if TYPE_CHECKING:
    from .data import SimulationData

class ScenarioProblemBuilder:
    """Build solver-ready hydraulic optimization problems for demand scenarios.

    The builder combines a prepared hydraulic network, a set of demand
    scenarios, and runtime simulation parameters. It validates terminal
    alignment, derives scenario-specific terminal pressures and resistances,
    constructs flow bounds, removes closed pipes from the incidence matrix,
    and validates each :class:`ScenarioProblem` before it is sent to the
    solver.

    Attributes:
        instance: Prepared simulation data containing the hydraulic network,
            pipe table, borne table, and derived matrices.
        scenarios: Demand scenarios indexed by terminal and scenario.
        params: Copy of the runtime simulation parameters.
        pipe_count: Total number of pipes in the hydraulic network.
        terminal_pipe_mask: Boolean mask identifying terminal pipes in the
            full pipe ordering.
        transit_pipe_mask: Boolean mask identifying transit pipes in the full
            pipe ordering.
        source_pipe_mask: Boolean mask identifying source pipes in the full
            pipe ordering.
        terminal_pipe_indices: Integer indices of terminal pipes in the full
            pipe ordering.
        static_open_pipe_mask: Boolean mask describing the user-configured
            open/closed state of every pipe.
        required_pressure_by_terminal: Required pressure for each terminal,
            aligned with ``scenarios.terminal_ids``.
        friction_exponent: Hydraulic friction exponent used by the solver.
    """

    def __init__(self, instance: "SimulationData", scenarios: ScenarioSet, runtime_params: Mapping[str, Any]) -> None:
        """Initialize a scenario problem builder.

        Args:
            instance: Prepared hydraulic simulation data. ``instance.dat`` is
                expected to contain the pipe counts, pipe-type masks, edge
                identifiers, incidence matrix, and hydraulic coefficients used
                by the builder.
            scenarios: Scenario data containing terminal identifiers, demand
                values, and open-client counts for each scenario.
            runtime_params: Runtime simulation parameters. The builder reads
                ``uniform_dem``, ``bounded_flow``, ``orif_com`` and ``lambda`` from this mapping.

        Raises:
            ScenarioPreparationError: If the hydraulic network has not been
                prepared, terminal ordering is inconsistent, or terminal
                pressure requirements cannot be constructed.
        """
        if not instance.is_prepared:
            raise ScenarioPreparationError(
                "The hydraulic network must be prepared "
                "before building scenario problems."
            )

        self.instance = instance
        self.scenarios = scenarios
        self.params = dict(runtime_params)

        self.pipe_count = int(instance.dat["nbA"])

        self.terminal_pipe_mask = np.asarray(instance.dat["idAter"],dtype=bool)

        self.transit_pipe_mask = np.asarray(instance.dat["idAtra"],dtype=bool)

        self.source_pipe_mask = np.asarray(instance.dat["idAsrc"],dtype=bool)

        self.terminal_pipe_indices = np.flatnonzero(self.terminal_pipe_mask)

        self.static_open_pipe_mask = instance.pipes["is_open"].to_numpy(dtype=bool)

        self.friction_exponent = float(self.params["lambda"])
        if(not np.isfinite(self.friction_exponent) or self.friction_exponent <= 0):
            raise ScenarioPreparationError(
                "The hydraulic friction exponent must be finite and positive.",
                details={
                    "lambda": self.friction_exponent,
                },
            )

        self._check_terminal_alignment()

        self.required_pressure_by_terminal = self._build_terminal_pressure_vector()

    def _check_terminal_alignment(self) -> None:
        """Validate terminal ordering between scenarios and terminal pipes.

        Scenario vectors are positional, so their terminal order must exactly
        match the successor-node order of terminal pipes in the network. This
        check prevents demand and client-count values from being assigned to
        the wrong terminal pipe.

        Raises:
            ScenarioPreparationError: If the number of scenario terminals does
                not match the number of terminal pipes, or if their identifier
                ordering differs.
        """

        terminal_ids = np.asarray(self.scenarios.terminal_ids,dtype=str)

        terminal_pipe_successors = (self.instance.pipes.loc[self.terminal_pipe_mask, "succ",].astype(str).to_numpy())

        if len(terminal_ids) != len(terminal_pipe_successors):
            raise ScenarioPreparationError(
                "The number of scenario terminals does not match "
                "the number of terminal pipes.",
                details={
                    "scenarioTerminalCount": len(terminal_ids),
                    "terminalPipeCount": len(
                        terminal_pipe_successors
                    ),
                },
            )

        if not np.array_equal(terminal_ids, terminal_pipe_successors):
            raise ScenarioPreparationError(
                "Scenario terminal ordering does not match "
                "terminal-pipe ordering.",
                details={
                    "scenarioTerminalIds": terminal_ids.tolist(),
                    "terminalPipeSuccessors": (
                        terminal_pipe_successors.tolist()
                    ),
                },
            )

    def _build_terminal_pressure_vector(self) -> np.ndarray:
        """Build terminal pressure requirements in scenario-terminal order.

        Pressure values are read from ``instance.bornes`` using the ``borne``
        identifier and reordered to match ``scenarios.terminal_ids``. This
        allows bounded-flow scenarios to use the pressure requirement of each
        individual borne instead of a single global pressure value.

        Returns:
            A one-dimensional float array containing the required pressure for
            each scenario terminal, in the same order as
            ``scenarios.terminal_ids``.

        Raises:
            ScenarioPreparationError: If one or more scenario terminals cannot
                be matched to a borne pressure, or if any aligned pressure is
                not finite.
        """

        pressure_by_terminal = self.instance.bornes.set_index("borne")["pressure"]

        try:
            aligned_pressures = pressure_by_terminal.loc[self.scenarios.terminal_ids]
        except KeyError as exc:
            raise ScenarioPreparationError(
                "Required pressure cannot be aligned with "
                "scenario terminals."
            ) from exc

        pressures = aligned_pressures.to_numpy(
            dtype=float
        )

        if not np.isfinite(pressures).all():
            raise ScenarioPreparationError(
                "Terminal pressure requirements contain "
                "invalid values."
            )

        return pressures 

    def _get_scenario_vectors(self, scenario_index: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        """Extract and validate terminal vectors for one demand scenario.

        Demand is converted from litres per second to cubic metres per second.
        A terminal is considered active when its converted demand is strictly
        positive.

        Args:
            scenario_index: Zero-based index of the scenario to extract.

        Returns:
            A tuple ``(demand_m3s, client_counts, active_terminal_mask)`` where
            ``demand_m3s`` contains terminal demand in m³/s, ``client_counts``
            contains the number of open clients per terminal, and
            ``active_terminal_mask`` marks terminals with positive demand.

        Raises:
            ScenarioPreparationError: If ``scenario_index`` is out of range,
                demand or client counts contain invalid values, a positive
                demand has no open client, or the scenario contains no active
                terminal.
        """

        if (scenario_index < 0 or scenario_index >= self.scenarios.scenario_count):
            raise ScenarioPreparationError(
                "Scenario index is outside the available range.",
                scenario_index=scenario_index,
                details={
                    "scenarioCount": (
                        self.scenarios.scenario_count
                    ),
                },
            )

        demand_lps = self.scenarios.demand_by_terminal_lps[:, scenario_index]

        demand_m3s = lps_to_m3s(np.asarray(demand_lps, dtype=float))

        client_counts = np.asarray(
            self.scenarios.open_client_count_by_terminal[:, scenario_index],
            dtype=int,
        )

        if not np.isfinite(demand_m3s).all():
            raise ScenarioPreparationError("Scenario demand contains invalid values.", scenario_index=scenario_index)

        if (demand_m3s < 0).any():
            raise ScenarioPreparationError("Scenario demand cannot be negative.", scenario_index=scenario_index)

        if (client_counts < 0).any():
            raise ScenarioPreparationError("Open-client counts cannot be negative.", scenario_index=scenario_index)

        active_terminal_mask = demand_m3s > 0

        inconsistent_mask = (active_terminal_mask & (client_counts <= 0))

        if inconsistent_mask.any():
            raise ScenarioPreparationError(
                "A terminal has positive demand but no open client.",
                scenario_index=scenario_index,
                details={
                    "terminalIds": (
                        self.scenarios.terminal_ids[
                            inconsistent_mask
                        ].tolist()
                    ),
                },
            )

        if not active_terminal_mask.any():
            raise ScenarioPreparationError("An empty scenario cannot be prepared.", scenario_index=scenario_index)

        return (
            demand_m3s,
            client_counts,
            active_terminal_mask,
        )

    def _compute_alpha_by_pipe(self, demand_m3s: np.ndarray, client_counts: np.ndarray, active_terminal_mask: np.ndarray,) -> np.ndarray:
        """Compute the scenario-specific resistance coefficient for each pipe.

        Only active terminal pipes receive a scenario-specific ``alpha`` value;
        all other entries remain zero. For uniform, unbounded-flow simulations,
        terminal resistance is derived from ``alpha_com`` or ``alpha_ideal``
        and divided by the square of the open-client count. For bounded-flow
        simulations, resistance is computed as required pressure divided by
        squared terminal demand. Heterogeneous unbounded flow is intentionally
        unsupported.

        Args:
            demand_m3s: Demand per scenario terminal in m³/s.
            client_counts: Number of open clients per scenario terminal.
            active_terminal_mask: Boolean mask selecting terminals with positive
                demand.

        Returns:
            A full-size float array of ``alpha`` coefficients aligned with the
            network pipe ordering.

        Raises:
            ScenarioPreparationError: If a required terminal-alpha column is
                missing, heterogeneous unbounded flow is requested, or the
                computed alpha values are non-finite or negative.
        """

        alpha_by_pipe = np.zeros(self.pipe_count,dtype=float,)

        uniform_demand = bool(int(self.params["uniform_dem"]))

        bounded_flow = bool(int(self.params["bounded_flow"]))

        active_terminal_indices = self.terminal_pipe_indices[active_terminal_mask]

        if uniform_demand and not bounded_flow:
            alpha_column = (
                "alpha_com"
                if int(self.params["orif_com"]) == 1
                else "alpha_ideal"
            )

            if alpha_column not in self.instance.pipes.columns:
                raise ScenarioPreparationError(
                    f"Pipe column '{alpha_column}' is required "
                    "for uniform unbounded-flow simulation."
                )

            base_terminal_alpha = (
                self.instance.pipes.loc[
                    self.terminal_pipe_mask,
                    alpha_column,
                ].to_numpy(dtype=float)
            )

            active_client_counts = client_counts[active_terminal_mask].astype(float)

            alpha_by_pipe[active_terminal_indices] = (
                base_terminal_alpha[active_terminal_mask]
                / active_client_counts**2
            )

        elif not uniform_demand and not bounded_flow:
            raise ScenarioPreparationError(
                "Heterogeneous unbounded-flow simulation "
                "is not supported."
            )

        else:
            active_demand = demand_m3s[active_terminal_mask]

            active_pressures = self.required_pressure_by_terminal[active_terminal_mask]

            alpha_by_pipe[active_terminal_indices] = (
                active_pressures
                / active_demand**2
            )

        if not np.isfinite(alpha_by_pipe).all():
            raise ScenarioPreparationError(
                "Scenario alpha calculation produced "
                "invalid values."
            )

        if (alpha_by_pipe < 0).any():
            raise ScenarioPreparationError(
                "Scenario alpha values cannot be negative."
            )

        return alpha_by_pipe

    def _build_flow_bounds(self, demand_m3s: np.ndarray,) -> tuple[np.ndarray, np.ndarray]:
        """Build lower and upper flow bounds for every pipe.

        In bounded-flow mode, the total scenario demand is used as the global
        upper bound and each terminal is capped by its own demand. In unbounded
        mode, both limits are scaled by ``UNBOUNDED_FLOW_MULTIPLIER``. Terminal
        and source pipes are constrained to non-negative flow. A terminal with
        zero demand, or any pipe explicitly closed by the user, receives fixed
        zero flow.

        Args:
            demand_m3s: Demand per scenario terminal in m³/s.

        Returns:
            A tuple ``(lower_bounds, upper_bounds)`` containing full-size float
            arrays aligned with the network pipe ordering.

        Raises:
            ScenarioPreparationError: If total scenario demand is not positive,
                a bound is non-finite, or any lower bound exceeds its matching
                upper bound.
        """

        bounded_flow = bool(int(self.params["bounded_flow"]))

        total_demand_m3s = float(np.sum(demand_m3s))

        if total_demand_m3s <= 0:
            raise ScenarioPreparationError(
                "Total scenario demand must be positive."
            )

        if bounded_flow:
            global_upper_bound = total_demand_m3s
            terminal_upper_bounds = demand_m3s
        else:
            global_upper_bound = (
                UNBOUNDED_FLOW_MULTIPLIER
                * total_demand_m3s
            )

            terminal_upper_bounds = (
                UNBOUNDED_FLOW_MULTIPLIER
                * demand_m3s
            )

        upper_bounds = np.full(self.pipe_count, global_upper_bound, dtype=float,)

        lower_bounds = -upper_bounds.copy()

        upper_bounds[self.terminal_pipe_mask] = terminal_upper_bounds

        lower_bounds[self.terminal_pipe_mask] = 0.0

        lower_bounds[self.source_pipe_mask] = 0.0

        # Explicitly closed pipes have fixed zero flow.
        closed_by_user_mask = ~self.static_open_pipe_mask

        lower_bounds[closed_by_user_mask] = 0.0
        upper_bounds[closed_by_user_mask] = 0.0

        if not np.isfinite(lower_bounds).all():
            raise ScenarioPreparationError(
                "Lower flow bounds contain invalid values."
            )

        if not np.isfinite(upper_bounds).all():
            raise ScenarioPreparationError(
                "Upper flow bounds contain invalid values."
            )

        if (lower_bounds > upper_bounds).any():
            invalid_indices = np.flatnonzero(lower_bounds > upper_bounds)

            raise ScenarioPreparationError(
                "Some pipe lower bounds exceed upper bounds.",
                details={
                    "edgeIds": self.instance.dat[
                        "edge_ids"
                    ][invalid_indices].tolist(),
                },
            )

        return lower_bounds, upper_bounds

    def build(self, scenario_index: int) -> ScenarioProblem:
        """Build and validate a solver-ready problem for one scenario.

        The method extracts scenario vectors, computes terminal resistance and
        flow bounds, determines open and closed pipes, reduces the incidence
        matrix to open pipes, derives terminal/source/transit masks in the
        reduced ordering, and packages all data into a :class:`ScenarioProblem`.
        The resulting object is validated before being returned.

        Args:
            scenario_index: Zero-based index of the demand scenario to build.

        Returns:
            A validated :class:`ScenarioProblem` containing the full-pipe
            hydraulic vectors and the open-pipe solver matrices for the selected
            scenario.

        Raises:
            ScenarioPreparationError: If scenario extraction, coefficient or
                bound construction, or final problem validation fails.
        """

        (demand_m3s, client_counts, active_terminal_mask) = self._get_scenario_vectors(scenario_index)

        alpha_by_pipe = self._compute_alpha_by_pipe(demand_m3s, client_counts, active_terminal_mask)

        (lower_bounds, upper_bounds) = self._build_flow_bounds(demand_m3s)

        closed_pipe_mask = np.isclose(lower_bounds, upper_bounds, rtol=0.0, atol=0.0)

        open_pipe_mask = ~closed_pipe_mask

        terminal_static_open_mask = self.static_open_pipe_mask[self.terminal_pipe_mask]

        served_terminal_mask = active_terminal_mask & terminal_static_open_mask

        incidence_matrix = self.instance.dat["M_incid"][:, open_pipe_mask].tocsr()

        terminal_pipe_mask_open = self.terminal_pipe_mask[open_pipe_mask]

        transit_pipe_mask_open = self.transit_pipe_mask[open_pipe_mask]

        source_pipe_mask_open = self.source_pipe_mask[open_pipe_mask]

        active_terminal_client_counts = client_counts[served_terminal_mask]

        problem = ScenarioProblem(
            scenario_index=scenario_index,
            terminal_ids=np.asarray(self.scenarios.terminal_ids, dtype=str).copy(),
            edge_ids=np.asarray(self.instance.dat["edge_ids"], dtype=str).copy(),
            demand_by_terminal_m3s=demand_m3s.copy(),
            open_client_count_by_terminal=client_counts.copy(),
            active_terminal_mask=active_terminal_mask.copy(),
            served_terminal_mask=served_terminal_mask.copy(),
            alpha_by_pipe=alpha_by_pipe,
            beta_by_pipe=np.asarray(self.instance.dat["beta"], dtype=float,).copy(),
            gravity_by_pipe=np.asarray(self.instance.dat["gravite"], dtype=float,).copy(),
            lower_bounds_by_pipe=lower_bounds,
            upper_bounds_by_pipe=upper_bounds,
            open_pipe_mask=open_pipe_mask,
            closed_pipe_mask=closed_pipe_mask,
            incidence_matrix=incidence_matrix,
            balance_rhs=np.zeros(incidence_matrix.shape[0], dtype=float).copy(),
            terminal_pipe_mask_open=terminal_pipe_mask_open,
            transit_pipe_mask_open=transit_pipe_mask_open,
            source_pipe_mask_open=source_pipe_mask_open,
            active_terminal_client_counts=active_terminal_client_counts,
            friction_exponent=self.friction_exponent,
            bounded_flow=bool(int(self.params["bounded_flow"])),
        )

        self._check_problem(problem)

        return problem 

    def _check_problem(self, problem: ScenarioProblem) -> None:
        """Validate the internal consistency of a solver-ready scenario problem.

        The validation checks that at least one pipe and one source pipe remain
        open, the reduced incidence matrix has the expected shape, served
        terminals align with open terminal pipes and client counts, every open
        pipe has a non-empty flow interval, all numerical arrays are finite, and
        no transit-node row becomes empty after pipe closures.

        Args:
            problem: Scenario problem to validate before solver execution.

        Raises:
            ScenarioPreparationError: If any structural, dimensional, flow-bound,
                numerical, source-connectivity, terminal-alignment, or
                transit-node consistency check fails.
        """

        open_pipe_count = int(problem.open_pipe_mask.sum())

        if open_pipe_count == 0:
            raise ScenarioPreparationError(
                "The scenario contains no open pipe.",
                scenario_index=problem.scenario_index,
            )

        if not problem.source_pipe_mask_open.any():
            raise ScenarioPreparationError(
                "The scenario contains no open source pipe.",
                scenario_index=problem.scenario_index,
            )

        if problem.incidence_matrix.shape != (
            int(self.instance.dat["nbNtra"]),
            open_pipe_count,
        ):
            raise ScenarioPreparationError(
                "Reduced incidence matrix has an invalid shape.",
                scenario_index=problem.scenario_index,
                details={
                    "actualShape": list(
                        problem.incidence_matrix.shape
                    ),
                    "expectedShape": [
                        int(self.instance.dat["nbNtra"]),
                        open_pipe_count,
                    ],
                },
            )

        open_terminal_count = int(problem.terminal_pipe_mask_open.sum())

        served_terminal_count = int(problem.served_terminal_mask.sum())

        if open_terminal_count != served_terminal_count:
            raise ScenarioPreparationError(
                "The number of open terminal pipes does not match "
                "the number of served terminals.",
                scenario_index=problem.scenario_index,
                details={
                    "openTerminalPipes": open_terminal_count,
                    "servedTerminals": served_terminal_count,
                },
            )

        if (len(problem.active_terminal_client_counts) != served_terminal_count):
            raise ScenarioPreparationError(
                "Active client counts are not aligned with "
                "served terminals.",
                scenario_index=problem.scenario_index,
            )

        open_lower_bounds = problem.lower_bounds_by_pipe[problem.open_pipe_mask]

        open_upper_bounds = problem.upper_bounds_by_pipe[problem.open_pipe_mask]

        if not np.all(open_lower_bounds < open_upper_bounds):
            raise ScenarioPreparationError(
                "Every open pipe must have a non-empty "
                "flow interval.",
                scenario_index=problem.scenario_index,
            )

        numerical_arrays = {
            "alpha": problem.alpha_by_pipe,
            "beta": problem.beta_by_pipe,
            "gravity": problem.gravity_by_pipe,
            "lowerBounds": problem.lower_bounds_by_pipe,
            "upperBounds": problem.upper_bounds_by_pipe,
        }

        for field_name, values in numerical_arrays.items():
            if not np.isfinite(values).all():
                raise ScenarioPreparationError(
                    f"Scenario field '{field_name}' contains "
                    "invalid values.",
                    scenario_index=problem.scenario_index,
                )

        row_nonzero_counts = np.diff(problem.incidence_matrix.indptr)

        empty_row_mask = row_nonzero_counts == 0

        if empty_row_mask.any():
            transit_node_ids = np.asarray(
                self.instance.dat["transit_node_ids"],
                dtype=str,
            )

            raise ScenarioPreparationError(
                "Closing pipes isolated one or more "
                "transit nodes.",
                scenario_index=problem.scenario_index,
                details={
                    "transitNodeIds": (
                        transit_node_ids[
                            empty_row_mask
                        ].tolist()
                    ),
                },
            )  
