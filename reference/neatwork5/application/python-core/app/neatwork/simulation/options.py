"""Resolution and validation of simulation runtime options."""

from typing import TYPE_CHECKING

import numpy as np
import pandas as pd

from .exceptions import SimulationValidationError, ValidationIssue
from .models import ResolvedSimulationOptions, SimulationRunOptions, SolverOptions


if TYPE_CHECKING:
    from .data import SimulationData


def _resolve_uniform_target_flow(instance: "SimulationData") -> float | None:
    """
    Resolve uniform target flow from the synchronized domain and Client demand data only.
    """

    parameter_value = instance.param.get("targetflow")

    if parameter_value is not None:
        parameter_value = float(parameter_value)

        if parameter_value > 0:
            return parameter_value

    client_targets = pd.to_numeric(instance.clients["cible"],errors="coerce")

    positive_targets = np.unique(client_targets[np.isfinite(client_targets)& (client_targets > 0)])

    if len(positive_targets) == 1:
        return float(positive_targets[0])

    return None


def resolve_simulation_options(instance: "SimulationData", requested: SimulationRunOptions | None = None,) -> ResolvedSimulationOptions:
    """
    Merge runtime options with network parameters and validate the result.

    This function does not mutate SimulationData or its client table.
    """

    requested = requested or SimulationRunOptions()
    issues: list[ValidationIssue] = []

    uniform_demand = bool(int(instance.param.get("uniform_dem", 1)))

    scenario_count = (
        requested.scenario_count
        if requested.scenario_count is not None
        else int(instance.param.get("nbsim", 100))
    )

    if scenario_count <= 0:
        issues.append(
            ValidationIssue(
                code="INVALID_SCENARIO_COUNT",
                message=(
                    "The number of scenarios must be "
                    "strictly positive."
                ),
                entity_type="option",
                entity_id="scenario_count",
                details={"value": scenario_count},
            )
        )

    if requested.max_generation_batches <= 0:
        issues.append(
            ValidationIssue(
                code="INVALID_GENERATION_BATCH_COUNT",
                message=(
                    "max_generation_batches must be "
                    "strictly positive."
                ),
                entity_type="option",
                entity_id="max_generation_batches",
            )
        )

    if requested.oversampling_factor <= 1:
        issues.append(
            ValidationIssue(
                code="INVALID_OVERSAMPLING_FACTOR",
                message=(
                    "oversampling_factor must be greater than 1."
                ),
                entity_type="option",
                entity_id="oversampling_factor",
                details={
                    "value": requested.oversampling_factor,
                },
            )
        )

    if requested.max_draws_per_batch <= 0:
        issues.append(
            ValidationIssue(
                code="INVALID_MAX_DRAWS_PER_BATCH",
                message=(
                    "max_draws_per_batch must be "
                    "strictly positive."
                ),
                entity_type="option",
                entity_id="max_draws_per_batch",
            )
        )

    open_taps_probability: float | None = None
    target_flow_lps: float | None = None

    if uniform_demand:
        open_taps_probability = (float(instance.param.get("opentaps", 1.0)))

        if open_taps_probability < 0 or open_taps_probability > 1:
            issues.append(
                ValidationIssue(
                    code="OPEN_TAPS_PROBABILITY_OUT_OF_RANGE",
                    message=(
                        "open_taps_probability must be "
                        "between 0 and 1."
                    ),
                    entity_type="option",
                    entity_id="open_taps_probability",
                    details={
                        "value": open_taps_probability,
                    },
                )
            )

        if open_taps_probability == 0:
            issues.append(
                ValidationIssue(
                    code="ZERO_OPEN_TAPS_PROBABILITY",
                    message=(
                        "open_taps_probability must be greater "
                        "than zero because empty scenarios are "
                        "not retained."
                    ),
                    entity_type="option",
                    entity_id="open_taps_probability",
                )
            )

        target_flow_lps = _resolve_uniform_target_flow(instance)

        if (target_flow_lps is None or target_flow_lps <= 0):
            issues.append(
                ValidationIssue(
                    code="INVALID_UNIFORM_TARGET_FLOW",
                    message=(
                        "A strictly positive target flow is "
                        "required for uniform-demand simulation."
                    ),
                    entity_type="option",
                    entity_id="target_flow_lps",
                )
            )

    else:
        if requested.open_taps_probability is not None:
            issues.append(
                ValidationIssue(
                    code="UNSUPPORTED_UNIFORM_PROBABILITY_OVERRIDE",
                    message=(
                        "A global opening probability cannot "
                        "override heterogeneous client demands."
                    ),
                    entity_type="option",
                    entity_id="open_taps_probability",
                ))
        if (requested.target_flow_lps is not None):
            issues.append(
                ValidationIssue(
                    code="UNSUPPORTED_UNIFORM_FLOW_OVERRIDE",
                    message=(
                        "A global target flow cannot override "
                        "heterogeneous client demands."
                    ),
                    entity_type="option",
                    entity_id="target_flow_lps",
                ))

            open_taps_probability = None
            target_flow_lps = None

    if issues:
        raise SimulationValidationError(issues)

    return ResolvedSimulationOptions(
        scenario_count=scenario_count,
        seed=requested.seed,
        uniform_demand=uniform_demand,
        open_taps_probability=open_taps_probability,
        target_flow_lps=target_flow_lps,
        max_generation_batches=(
            requested.max_generation_batches
        ),
        oversampling_factor=requested.oversampling_factor,
        max_draws_per_batch=requested.max_draws_per_batch,
    )


def resolve_solver_options(instance: "SimulationData") -> SolverOptions:
    """Convert legacy IPM settings into validated solver options.

    Args:
        instance: Simulation data containing the historical ``options`` map.
    Returns:
        Validated typed options for the interior-point solver.
    Raises:
        SimulationValidationError: If one or more numerical options are invalid.
    """
    raw = instance.options

    options = SolverOptions(
        flow_tolerance=float(raw.get("tolx", 1e-8)),
        pressure_tolerance=float(raw.get("tolpress", 1e-6)),
        centering_tolerance=float(raw.get("tolcent", 1e-10)),
        max_iterations=int(raw.get("maxiter", 30)),
        barrier_reduction=float(raw.get("sigma", 0.1)),
        centering_exponent=float(raw.get("tau", 2.5)),
        step_safety=float(raw.get("a_secur", 0.999)),
    )

    issues: list[ValidationIssue] = []

    positive_values = {
        "flow_tolerance": options.flow_tolerance,
        "pressure_tolerance": options.pressure_tolerance,
        "centering_tolerance": options.centering_tolerance,
        "centering_exponent": options.centering_exponent,
    }

    for name, value in positive_values.items():
        if(not np.isfinite(value) or value <= 0):
            issues.append(
                ValidationIssue(
                    code="INVALID_SOLVER_OPTION",
                    message=(
                        f"{name} must be finite "
                        "and strictly positive."
                    ),
                    entity_type="solver_option",
                    entity_id=name,
                    details={
                        "value": value,
                    },
                )
            )
        
    if options.max_iterations <= 0:
        issues.append(
            ValidationIssue(
                code="INVALID_SOLVER_OPTION",
                message=(
                    "max_iterations must be "
                    "strictly positive."
            ),
            entity_type="solver_option",
            entity_id="max_iterations",
            details={
                "value": options.max_iterations,
            },
        )
    )

    if (not np.isfinite(options.barrier_reduction) or options.barrier_reduction >=1 or options.barrier_reduction <= 0):
        issues.append(
            ValidationIssue(
                code="INVALID_SOLVER_OPTION",
                message=(
                    "barrier_reduction must "
                    "be between 0 and 1."
                ),
                entity_type="solver_option",
                entity_id="barrier_reduction",
                details={
                    "value": options.barrier_reduction,
                },
            )
        )
    
    if (not np.isfinite(options.step_safety) or options.step_safety <= 0 or options.step_safety >= 1):
        issues.append(
            ValidationIssue(
                code="INVALID_SOLVER_OPTION",
                message=(
                    "step_safety must be "
                    "between 0 and 1."
                ),
                entity_type="solver_option",
                entity_id="step_safety",
                details={
                    "value": options.step_safety,
                },
            )
        )
    if issues:
        raise SimulationValidationError(issues)
        
    return options
    
