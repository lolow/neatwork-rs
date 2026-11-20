"""Application service connecting FastAPI to the hydraulic numerical core."""

from __future__ import annotations
from app.neatwork.simulation import domain_adapter

from dataclasses import replace, asdict, dataclass
import re
import secrets
from typing import Any

import numpy as np

from app.neatwork.simulation import (
    Simulation,
    SimulationBatchResult,
    SimulationData,
    SimulationJsonExporter,
    SimulationStatistics,
    SimulationValidationError,
    ValidationIssue,
    build_simulation_data,
    SimulationCancelledError
)
from app.services.simulation_cancellation import is_cancel_requested

from app.schemas.simulation_execution import (
    SimulationEdgeOverrideInput,
    SimulationEdgeResult,
    SimulationExecutionRequest,
    SimulationExecutionResponse,
    SimulationExecutionSummary,
    SimulationNodeResult,
    SimulationExecutionParameters,
)

from app.schemas.simulation_options import SimulationOptionsInput, OrificeMode
from app.neatwork.simulation import SimulationSpreadsheetExporter
from app.neatwork.domain.client_behavior import materialize_effective_clients
from app.neatwork.hydraulic_units import m3s_to_lps
from app.neatwork.domain.client_demand import resolve_client_demand_profile,synchronize_client_demand_params, terminal_demand_profiles_payload, ClientDemandProfile


def _apply_edge_overrides(domain,overrides: list[SimulationEdgeOverrideInput]):
    """Apply current project edge states to a copied simulation domain.

    The persisted domain snapshot remains immutable. Missing pipe IDs are
    normalized to the same stable ``A1``, ``A2``, ... identifiers used by the
    graph importer and numerical adapter.

    Args:
        domain: Imported hydraulic domain snapshot.
        overrides: Current graph OPEN/CLOSED states.

    Returns:
        Deep-copied domain containing the runtime edge states.

    Raises:
        SimulationValidationError: If override IDs are duplicated or do not exist in the hydraulic domain.
    """

    updated_domain = domain.model_copy(deep=True)

    pipe_by_edge_id = {}

    issues: list[ValidationIssue] = []

    for index, pipe in enumerate(updated_domain.tables.pipes):
        edge_id = pipe.edge_id or f"A{index + 1}"

        # Persist the generated stable ID inside this runtime copy as well.
        pipe.edge_id = edge_id

        if edge_id in pipe_by_edge_id:
            issues.append(
                ValidationIssue(
                    code="DUPLICATE_EDGE_ID",
                    message=f"Multiple hydraulic pipes use edge ID '{edge_id}'.",
                    entity_type="pipe",
                    entity_id=edge_id,
                )
            )

            continue

        pipe_by_edge_id[edge_id] = pipe

    if issues:
        raise SimulationValidationError(issues)

    seen_override_ids: set[str] = set()

    for override in overrides:
        edge_id = override.edge_id

        if edge_id in seen_override_ids:
            issues.append(
                ValidationIssue(
                    code="DUPLICATE_EDGE_OVERRIDE",
                    message=f"The same edge override was provided more than once: '{edge_id}'.",
                    entity_type="pipe",
                    entity_id=edge_id,
                )
            )

            continue

        seen_override_ids.add(edge_id)

        pipe = pipe_by_edge_id.get(edge_id)

        if pipe is None:
            issues.append(
                ValidationIssue(
                    code="UNKNOWN_EDGE_OVERRIDE",
                    message=f"Edge override does not match any hydraulic pipe: '{edge_id}'.",
                    entity_type="pipe",
                    entity_id=edge_id,
                )
            )

            continue

        pipe.enabled = override.enabled

        # Current graph state must override any status stored in the snapshot.
        pipe.operational_status = "OPEN" if override.enabled else "CLOSED"

    if issues:
        raise SimulationValidationError(issues)

    return updated_domain

def _resolve_core_options(options: SimulationOptionsInput, demand_profile: "ClientDemandProfile"):
    """
    Build runtime simulation options from non-demand overrides and demand values derived from the Client demand table.

    A user-provided seed is preserved exactly. When no seed is supplied, a fresh 32-bit seed is generated for execution.
    """

    core_options = options.to_core_options(
        uniform_demand=demand_profile.uniform,
        open_taps_probability=demand_profile.open_probability,
        target_flow_lps=demand_profile.target_flow_lps,
    )

    if core_options.seed is not None:
        return core_options

    generated_seed = secrets.randbelow(2**32)

    return replace(core_options, seed=generated_seed)

def _build_node_results(instance: SimulationData, batch: SimulationBatchResult, statistics: SimulationStatistics) -> list[SimulationNodeResult]:
    """Build graph-compatible aggregate node results.

    Source nodes receive zero gauge pressure. Transit and terminal nodes use
    the mean statistical pressure calculated.
    """

    results: list[SimulationNodeResult] = []

    # ---------------------------------
    # Source nodes
    # ---------------------------------

    source_node_ids = np.asarray(instance.dat["source_node_ids"], dtype=str)

    for source_id in source_node_ids:
        results.append(
            SimulationNodeResult(
                id=str(source_id),
                pressureM=0.0,
                lowPressure=False,
            )
        )

    # ---------------------------------
    # Transit nodes
    # ---------------------------------

    for node_id, row in statistics.transit_pressure.iterrows():
        results.append(
            SimulationNodeResult(
                id=str(node_id),
                pressureM=float(row["meanPressureM"]),
                lowPressure=False,
            )
        )

    # ---------------------------------
    # Terminal nodes
    # ---------------------------------

    terminal_position = {
        str(terminal_id): index
        for index, terminal_id in enumerate(batch.terminal_ids)
    }

    successful_served_mask = (
        batch.served_terminal_mask_by_scenario[:, batch.success_mask]
    )

    required_pressure_by_terminal = (instance.bornes.set_index("borne")["pressure"].astype(float).to_dict())

    for terminal_id, row in statistics.terminal_pressure.iterrows():
        terminal_id = str(terminal_id)

        position = terminal_position[terminal_id]

        has_pressure_observation = bool(successful_served_mask[position,:].any())

        minimum_pressure = float(row["minPressureM"])

        mean_pressure = float(row["meanPressureM"])

        required_pressure = float(required_pressure_by_terminal.get(terminal_id, 0.0))

        low_pressure = (has_pressure_observation and minimum_pressure < required_pressure)

        results.append(
            SimulationNodeResult(
                id=terminal_id,
                pressureM=mean_pressure if has_pressure_observation else 0.0,
                lowPressure=low_pressure,
            )
        )

    return results

def _build_edge_results(instance: SimulationData, batch: SimulationBatchResult, statistics: SimulationStatistics) -> list[SimulationEdgeResult]:
    """Build mean flow and velocity values for every project edge."""

    successful_flow = batch.flow_by_pipe_m3s[:, batch.success_mask]

    mean_flow_lps = m3s_to_lps(np.mean(successful_flow,axis=1))

    edge_ids = np.asarray(instance.dat["edge_ids"],dtype=str)

    velocity_by_edge: dict[str, float,] = {}

    for _, row in statistics.pipe_velocity.iterrows():
        velocity_by_edge[str(row["edgeId"])] = float(row["meanVelocityMps"])

    return [
        SimulationEdgeResult(
            id=str(edge_id),
            flowRateLps=float(mean_flow_lps[index]),
            velocityMps=float(velocity_by_edge.get(str(edge_id),0.0)),
        )
        for index, edge_id in enumerate(edge_ids)
    ]

def _build_summary(instance: SimulationData, batch: SimulationBatchResult, node_results: list[SimulationNodeResult]) -> SimulationExecutionSummary:
    """Build a compact UI-oriented summary from one completed batch."""

    if batch.seed is None:
        raise RuntimeError("Simulation batch is missing its reproducibility seed.")

    successful_iterations = batch.iterations[batch.success_mask]

    min_iterations = int(np.min(successful_iterations))

    mean_iterations = float(np.mean(successful_iterations))

    max_iterations = int(np.max(successful_iterations))

    open_pipe_mask = instance.pipes["is_open"].to_numpy(dtype=bool)

    if batch.failed_scenario_count == 0:
        status = "COMPLETED"
    else:
        status = "COMPLETED_WITH_ERRORS"

    return SimulationExecutionSummary(
        status=status,

        scenarioCount=batch.scenario_count,

        successfulScenarios=batch.successful_scenario_count,

        failedScenarios=batch.failed_scenario_count,

        successRate=(batch.successful_scenario_count / batch.scenario_count),

        seed=int(batch.seed),

        durationSeconds=float(batch.duration_seconds),

        minIterations=min_iterations,

        meanIterations=mean_iterations,

        maxIterations=max_iterations,

        openEdges=int(open_pipe_mask.sum()),

        closedEdges=int((~open_pipe_mask).sum()),

        lowPressureNodes=sum(1 for node in node_results if node.lowPressure),
    )

def _apply_parameter_overrides(domain, options: SimulationOptionsInput):
    """Apply one-run hydraulic parameter overrides to a copied domain.
    The persisted project snapshot remains unchanged.
    """
    updated_domain = domain.model_copy(deep=True)
    params = updated_domain.params

    # Bounded-flow control and terminal-orifice model are independent runtime parameters. Either one may be overridden without forcing an override of the other.
    if options.orifice_mode is not None:
        params["orif_com"] = (
            1 if options.orifice_mode == OrificeMode.COMMERCIAL
            else 0
        )

    if options.bounded_flow is not None:
        params["bounded_flow"] = int(options.bounded_flow)

    if options.service_quality is not None:
        params["servicequal"] = options.service_quality

    if options.required_pressure_m is not None:
        pressure = float(options.required_pressure_m)

        params["pressure"] = pressure

        # Important concept: ScenarioProblemBuilder reads pressure from bornes.
        for borne in updated_domain.tables.bornes:
            borne.required_pressure_m = pressure

    if options.orifice_coefficient is not None:
        params["orifcoef"] = options.orifice_coefficient

    if options.faucet_coefficient is not None:
        params["faucetcoef"] = options.faucet_coefficient

    if options.water_temperature_c is not None:
        params["watertemp"] = options.water_temperature_c

    if options.commercial_pipe_length_m is not None:
        params["pipelength"] = options.commercial_pipe_length_m

    return updated_domain

def _execute_pipeline(request: SimulationExecutionRequest) -> tuple[SimulationData,SimulationBatchResult,SimulationStatistics,dict[str, Any],SimulationExecutionParameters]:
    """Execute the complete numerical and statistical simulation pipeline."""

    cancel_requested = None

    if request.execution_id is not None:
        cancel_requested = lambda: is_cancel_requested(request.execution_id)

    if cancel_requested and cancel_requested():
        raise SimulationCancelledError()

    runtime_domain = _apply_parameter_overrides(request.domain, request.options)
    demand_profile = resolve_client_demand_profile(runtime_domain)
    runtime_domain = synchronize_client_demand_params(runtime_domain,demand_profile)
    terminal_demand_profiles = terminal_demand_profiles_payload(runtime_domain)
    runtime_domain = _apply_edge_overrides(runtime_domain, request.edge_overrides)

    core_options = _resolve_core_options(request.options, demand_profile)

    instance = build_simulation_data(runtime_domain)

    simulation = Simulation(inst=instance,options=core_options)
    
    if cancel_requested and cancel_requested():
        raise SimulationCancelledError()

    batch = simulation.run(cancel_requested=cancel_requested)

    statistics = simulation.compute_stats()

    statistics_payload = simulation.statistics_payload()
    statistics_payload["demand"] = {"terminals": terminal_demand_profiles}

    execution_parameters = _build_execution_parameters(request,instance,simulation,batch, demand_profile)

    return (instance,batch,statistics,statistics_payload,execution_parameters)

def run_hydraulic_simulation(request: SimulationExecutionRequest) -> SimulationExecutionResponse:
    """Execute one hydraulic simulation and return graph plus JSON statistics."""

    (instance,batch,statistics,statistics_payload,execution_parameters) = _execute_pipeline(request)

    node_results = _build_node_results(instance,batch,statistics)

    edge_results = _build_edge_results(instance,batch,statistics)

    summary = _build_summary(instance,batch,node_results)

    return SimulationExecutionResponse(
        projectId=request.project_id,
        nodes=node_results,
        edges=edge_results,
        summary=summary,
        statistics=statistics_payload,
        executionParameters=execution_parameters,
    )

def export_hydraulic_statistics_json(request: SimulationExecutionRequest) -> tuple[bytes, str]:
    """Execute a simulation and serialize its statistics as strict JSON."""

    (instance, _, _, statistics_payload, _) = _execute_pipeline(request)

    exporter = SimulationJsonExporter()

    content = exporter.export_payload_bytes(statistics_payload, indent=2)

    safe_name = re.sub(r"[^a-zA-Z0-9._-]+", "_", instance.name).strip("._")

    if not safe_name:
        safe_name = "network"

    filename = f"simulation_statistics_{safe_name}.json"

    return (content,filename)

def export_statistics_spreadsheet(payload: dict[str, object]) -> bytes:
    """Convert canonical statistics JSON into XLSX bytes."""
    exporter = SimulationSpreadsheetExporter()
    
    return exporter.export_bytes(payload)

def _parameter_source(requested_fields: set[str], field_name: str) -> str:
    """
    Identify whether one runtime value was explicitly supplied by the user or resolved from project data.
    """
    if field_name in requested_fields:
        return "USER_OVERRIDE"

    return "PROJECT_DEFAULT"

def _build_execution_parameters(request: SimulationExecutionRequest, instance: SimulationData, simulation: Simulation, batch: SimulationBatchResult, demand_profile: ClientDemandProfile) -> SimulationExecutionParameters:
    """
    Build the complete reproducibility metadata for one finished hydraulic simulation.
    Values are taken after project defaults and runtime overrides have been resolved.
    """

    requested_fields = set(request.options.model_fields_set)
    params = instance.param
    bounded_flow = bool(int(params.get("bounded_flow", 0)))
    orifice_mode = ("COMMERCIAL" if int(params.get("orif_com",1,)) == 1 else "IDEAL")

    terminal_required_pressure = {
        str(row["borne"]): float(row["pressure"])

        for _, row in instance.bornes.iterrows()
    }

    values: dict[str, Any] = {
        "scenarioCount": simulation.options.scenario_count,
        "seed": int(batch.seed),
        #"uniformDemand": simulation.options.uniform_demand,
        "uniformDemand":demand_profile.uniform,
        "openTapsProbability": (demand_profile.open_probability if demand_profile.uniform else None),
        "targetFlowLps": (demand_profile.target_flow_lps if demand_profile.uniform else None),
        "clientDemandCount": demand_profile.client_count,
        "boundedFlow": bounded_flow,
        "orificeMode": orifice_mode,
        "serviceQuality": float(params.get("servicequal", 0.65)),
        "requiredPressureM": float(params.get("pressure", 0.0)),
        "terminalRequiredPressureM": terminal_required_pressure,
        "orificeCoefficient": float(params.get("orifcoef", 1.0)),
        "faucetCoefficient": float(params.get("faucetcoef", 1.0)),
        "waterTemperatureC": float(params.get("watertemp", 20.0)),
        "commercialPipeLengthM": float(params.get("pipelength", 6.0)),
        "maxGenerationBatches": simulation.options.max_generation_batches,
        "oversamplingFactor": simulation.options.oversampling_factor,
        "maxDrawsPerBatch": simulation.options.max_draws_per_batch,
         # Solver parameters are part of the actual numerical execution and therefore belong to reproducibility metadata.
        "solverOptions": asdict(simulation.solver_options),
    }

    sources: dict[str, str] = {
        "scenarioCount":_parameter_source(requested_fields,"scenario_count"),
        "seed":("USER_OVERRIDE" if "seed" in requested_fields else "GENERATED"),
        "uniformDemand":"CLIENT_DEMAND_TABLE",
        "openTapsProbability":(
            "CLIENT_DEMAND_TABLE" 
            if demand_profile.uniform
            else "NOT_APPLICABLE"
            ),
        "targetFlowLps":(
                "CLIENT_DEMAND_TABLE"
                if demand_profile.uniform
                else "NOT_APPLICABLE"
            ),
        "clientDemandCount":"CLIENT_DEMAND_TABLE",
        "boundedFlow":_parameter_source(requested_fields,"bounded_flow"),
        "orificeMode":_parameter_source(requested_fields,"orifice_mode"),
        "serviceQuality":_parameter_source(requested_fields,"service_quality"),
        "requiredPressureM":_parameter_source(requested_fields,"required_pressure_m"),
        "terminalRequiredPressureM":_parameter_source(requested_fields,"required_pressure_m"),

        "orificeCoefficient":(
            "NOT_APPLICABLE"
            if bounded_flow
            else _parameter_source(requested_fields,"orifice_coefficient")
        ),
        "faucetCoefficient":
            _parameter_source(requested_fields,"faucet_coefficient"),
        "waterTemperatureC":
            _parameter_source(requested_fields,"water_temperature_c"),
        "commercialPipeLengthM":
            _parameter_source(requested_fields,"commercial_pipe_length_m"),
        "maxGenerationBatches":
            _parameter_source(requested_fields,"max_generation_batches"),
        "oversamplingFactor":
            _parameter_source(requested_fields,"oversampling_factor"),
        "maxDrawsPerBatch":
            _parameter_source(requested_fields,"max_draws_per_batch"),
        "solverOptions": "PROJECT_DEFAULT",
    }

    return SimulationExecutionParameters(values=values, sources=sources)
 
def _client_value(client, key: str):
    """
    Read one canonical client-demand field from either a Pydantic model or a mapping-like row.
    """
    if isinstance(client, dict):
        return client.get(key)
    return getattr(client, key, None)


