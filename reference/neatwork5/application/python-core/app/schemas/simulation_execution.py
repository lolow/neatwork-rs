"""API models for executing hydraulic simulations."""

from __future__ import annotations

from typing import Any

from pydantic import (AliasChoices, BaseModel, ConfigDict, Field)
from app.schemas.simulation_domain import (SimulationDomainInput)
from app.schemas.simulation_options import (SimulationOptionsInput)


class SimulationEdgeOverrideInput(BaseModel):
    """Override the current hydraulic state of one network edge.

    Attributes:
        edge_id: External edge identifier shared with the project graph.
        enabled: Whether the edge is hydraulically open.
    """

    model_config = ConfigDict(populate_by_name=True, extra="forbid")

    edge_id: str = Field(min_length=1, validation_alias=AliasChoices("edge_id", "edgeId"))

    enabled: bool


class SimulationExecutionRequest(BaseModel):
    """Describe one complete hydraulic simulation execution.

    Attributes:
        project_id: Project identifier used for request traceability.
        domain: Complete imported hydraulic network domain.
        options: Runtime scenario-generation options.
        edge_overrides: Current OPEN/CLOSED project edge states.
    """

    model_config = ConfigDict(populate_by_name=True, extra="forbid")

    project_id: str = Field(min_length=1, validation_alias=AliasChoices("project_id", "projectId"))

    domain: SimulationDomainInput

    options: SimulationOptionsInput = Field(default_factory=SimulationOptionsInput)

    edge_overrides: list[SimulationEdgeOverrideInput] = Field(
        default_factory=list,
        validation_alias=AliasChoices("edge_overrides", "edgeOverrides")
    )
    
    execution_id: str | None = Field(default=None, validation_alias=AliasChoices("execution_id", "executionId"))


class SimulationNodeResult(BaseModel):
    """Expose aggregate hydraulic pressure for one graph node.

    Attributes:
        id: External node identifier.
        pressureM: Mean simulated pressure in metres.
        lowPressure: Whether the minimum simulated terminal pressure violates
            its configured pressure requirement.
    """

    id: str
    pressureM: float
    lowPressure: bool


class SimulationEdgeResult(BaseModel):
    """Expose aggregate hydraulic values for one graph edge.

    Attributes:
        id: External edge identifier.
        flowRateLps: Mean simulated flow rate in litres per second.
        velocityMps: Mean simulated velocity in metres per second.
    """

    id: str
    flowRateLps: float
    velocityMps: float


class SimulationExecutionSummary(BaseModel):
    """Summarize one complete hydraulic batch execution.

    Attributes:
        status: COMPLETED or COMPLETED_WITH_ERRORS.
        scenarioCount: Total generated scenarios.
        successfulScenarios: Successfully solved scenarios.
        failedScenarios: Scenarios that failed during preparation, numerical solving or convergence. Hydraulic water shortfall is not an execution failure.
        successRate: Fraction of successful scenarios.
        seed: Effective reproducibility seed.
        durationSeconds: Complete batch execution duration.
        minIterations: Minimum successful solver iteration count.
        meanIterations: Mean successful solver iteration count.
        maxIterations: Maximum successful solver iteration count.
        openEdges: Number of currently enabled project edges.
        closedEdges: Number of currently disabled project edges.
        lowPressureNodes: Number of terminals below required pressure.
    """

    status: str

    scenarioCount: int
    successfulScenarios: int
    failedScenarios: int
    successRate: float

    seed: int

    durationSeconds: float

    minIterations: int
    meanIterations: float
    maxIterations: int

    openEdges: int
    closedEdges: int
    lowPressureNodes: int

class SimulationExecutionParameters(BaseModel):
    """
    Describe the exact parameters used by one completed simulation execution.

    values contains resolved numerical values.
    sources indicates whether each value came from the project, a user override or runtime generation.
    """
    values: dict[str, Any]
    sources: dict[str, str]

class SimulationExecutionResponse(BaseModel):
    """Return graph-compatible values and the complete statistics JSON.

    Attributes:
        projectId: Project identifier received in the request.
        nodes: Aggregate node values used by the visual graph.
        edges: Aggregate edge values used by the visual graph.
        summary: Compact execution summary used by the UI.
        statistics: Strict JSON payload produced.
    """

    projectId: str

    nodes: list[SimulationNodeResult]
    edges: list[SimulationEdgeResult]

    summary: SimulationExecutionSummary

    statistics: dict[str, Any]
    executionParameters: SimulationExecutionParameters
