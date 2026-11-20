"""Structured validation errors used by the hydraulic simulation core.

The classes in this module keep machine-readable error information separate
from human-readable messages.  They can therefore be used by Python callers,
serialized into API responses, or displayed directly in the user interface.
"""

from dataclasses import dataclass, field
from typing import Any, Iterable


@dataclass(frozen=True, slots=True)
class ValidationIssue:
    """Describe one validation problem found in simulation input data.

    Attributes
    ----------
    code:
        Stable machine-readable error code, such as ``DUPLICATE_NODE_ID``.
    message:
        Optional human-readable explanation of the problem.
    entity_type:
        Kind of affected object, for example ``node``, ``pipe`` or ``table``.
    entity_id:
        Identifier of the affected object or row.
    field_name:
        Name of the invalid field, when the issue is field-specific.
    details:
        Additional structured context useful to API clients or the UI.

    The dataclass is immutable (``frozen=True``) and slot-based to make issue
    objects lightweight and safe to pass between validation layers.
    """

    code: str
    message: str | None = None
    entity_type: str | None = None
    entity_id: str | None = None
    field_name: str | None = None
    details: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        """Return an API-friendly dictionary containing populated fields only."""
        result: dict[str, Any] = {
            "code": self.code,
            "message": self.message,
        }

        if self.entity_type is not None:
            result["entityType"] = self.entity_type

        if self.entity_id is not None:
            result["entityId"] = self.entity_id
        
        if self.field_name is not None:
            result["field"] = self.field_name

        if self.details:
            result["details"] = self.details

        return result

class SimulationValidationError(ValueError):
    """Aggregate and expose all issues detected during input validation.

    Attributes
    ----------
    error_code:
        Stable top-level code returned to API consumers.
    issues:
        Immutable tuple containing every collected :class:`ValidationIssue`.

    The exception message previews at most three issues to remain readable;
    callers needing the complete report should inspect ``issues`` or call
    :meth:`to_dict`.
    """

    # Shared top-level code used when serializing the exception.
    error_code = "SIMULATION_INPUT_INVALID"

    def __init__(self, issues: Iterable[ValidationIssue]) -> None:
        """Initialize the exception from a non-empty iterable of issues.

        Raises
        ------
        ValueError
            If the supplied iterable contains no validation issues.
        """
        collected_issues = tuple(issues)

        if not collected_issues:
            raise ValueError("SimulationValidationError requires at least one ValidationIssue.")
        
        self.issues = collected_issues

        preview = "; ".join(
            issue.message
            for issue in self.issues[:3]
        )

        remaining = len(self.issues) - 3

        if remaining > 0:
            preview = f"{preview}; and {remaining} more issue(s)"
        
        super().__init__(f"Simulation validation failed with {len(self.issues)} issue(s): {preview}")

    def to_dict(self) -> dict[str, Any]:
        """Serialize the aggregate error and all nested issues for an API response."""
        return {
            "code": self.error_code,
            "message": "Simulation input validation failed.", 
            "issues": [issue.to_dict() for issue in self.issues]
        }

class ScenarioGenerationError(RuntimeError):
    """
    Raised when valid probabilistic scenarios cannot be generated.

    This error occurs when the scenario generation process fails due to:
    - Zero probability of producing a valid scenario
    - Exhausted batch iteration limits
    - Excessive draw attempts
    - No active clients being generated

    Attributes:
        error_code (str): Machine-readable error code.
        details (dict[str, Any] | None): Additional failure context.

    Methods:
        to_dict(): Serializes the error to a structured dictionary.
    """

    error_code = "SCENARIO_GENERATION_FAILED"

    def __init__(self, message: str, *, details: dict[str, Any] | None = None) -> None:
        self.details = details or {}
        super().__init__(message)

    def to_dict(self) -> dict[str, Any]:
        """Returns error as a dict with code, message, and optional details."""
        result: dict[str, Any] = {
            "code": self.error_code,
            "message": str(self),
        }

        if self.details:
            result["details"] = self.details

        return result

class ScenarioPreparationError(RuntimeError):
    """
    Raised when a scenario cannot be converted into a solver problem.
    
    This may occur due to an invalid scenario index, inconsistent terminal
    definitions, missing open source pipes, NaN values in ``alpha`` or bounds,
    empty rows in the incidence matrix, or an incorrect number of active
    terminals.
    """

    error_code = "SCENARIO_PREPARATION_FAILED"

    def __init__(self, message: str, *, scenario_index: int | None = None, details: dict[str, Any] | None = None) -> None:
        self.scenario_index = scenario_index
        self.details = details or {}
        super().__init__(message)

    def to_dict(self) -> dict[str, Any]:
        """Returns error as a dict with code, message, and optional details."""
        result: dict[str, Any] = {
            "code": self.error_code,
            "message": str(self),
        }

        if self.scenario_index is not None:
            result["scenarioIndex"] = self.scenario_index

        if self.details:
            result["details"] = self.details

        return result

class SolverError(RuntimeError):
    """Base exception for failures produced by the hydraulic solver.

    Attributes:
        scenario_index: Scenario being solved when the failure occurred.
        details: Additional numerical information describing the failure.
    """

    error_code = "SOLVER_FAILED"

    def __init__(self, message: str, *, scenario_index: int | None = None, details: dict[str, Any] | None = None) -> None:
        """Create a structured solver failure."""
        self.scenario_index = scenario_index
        self.details = details or {}
        super().__init__(message)

    def to_dict(self) -> dict[str, Any]:
        """Returns error as a dict with code, message, and optional details."""
        result: dict[str, Any] = {
            "code": self.error_code,
            "message": str(self),
        }

        if self.scenario_index is not None:
            result["scenarioIndex"] = self.scenario_index

        if self.details:
            result["details"] = self.details

        return result

class SolverNumericalError(SolverError):
    """Raised when the IPM cannot continue because of numerical failure."""

    error_code = "SOLVER_NUMERICAL_FAILED"

class StatisticsProcessingError(RuntimeError):
    """Raised when simulation statistics cannot be computed.

    Attributes:
        details: Structured contextual information about the failure.
    """

    error_code = "STATISTICS_PROCESSING_FAILED"

    def __init__(self, message: str, *, details: dict[str, Any] | None = None) -> None:
        """Initialize a structured statistics-processing error.

        Args:
            message: Human-readable failure description.
            details: Optional machine-readable failure context.
        """

        self.details = details or {}
        super().__init__(message)

    def to_dict(self) -> dict[str, Any]:
        """Return an API-friendly error representation."""

        result: dict[str, Any] = {
            "code": self.error_code,
            "message": str(self),
        }

        if self.details:
            result["details"] = self.details

        return result

class SimulationCancelledError(RuntimeError):
    """Raised when a running simulation is cancelled by the caller."""

    error_code = "SIMULATION_CANCELLED"

    def __init__(self, message: str = "Simulation was cancelled.") -> None:
        super().__init__(message)

    def to_dict(self) -> dict[str, str]:
        """Return an API-friendly cancellation response."""
        return {
            "code": self.error_code,
            "message": str(self)
        }