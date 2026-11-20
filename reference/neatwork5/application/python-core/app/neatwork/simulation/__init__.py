"""Hydraulic simulation numerical core for NeatWork."""

from .data import SimulationData
from .domain_adapter import (
    build_simulation_data,
    canonical_parameter_name,
    normalize_simulation_params,
)
from .engine import Simulation
from .exceptions import (
    SimulationValidationError,
    ScenarioGenerationError,
    StatisticsProcessingError,
    ValidationIssue,
    ScenarioPreparationError,
    SolverError,
    SolverNumericalError,
    SimulationCancelledError,
)

from .batch_runner import SimulationBatchRunner
from .validation import validate_simulation_data
from .models import (
    PreparedHydraulicNetwork,
    ResolvedSimulationOptions,
    ScenarioFailure,
    ScenarioFailureStage,
    ScenarioRunStatus,
    ScenarioSet,
    SimulationBatchResult,
    SimulationRunOptions,
    ScenarioProblem,
    SolverDiagnostics,
    SolverOptions,
    SolverResult,
    SolverStatus
)

from .json_exporter import SimulationJsonExporter
from .statistics import SimulationStatistics, SimulationStatisticsProcessor
from .problem_builder import ScenarioProblemBuilder
from .options import resolve_simulation_options, resolve_solver_options
from .scenario_generator import ScenarioGenerator
from .preprocessor import HydraulicPreprocessor
from .solver import InteriorPointSolver
from .spreadsheet_exporter import SimulationSpreadsheetExporter

__all__ = [
    "SimulationData",
    "Simulation",
    
    "HydraulicPreprocessor",
    "PreparedHydraulicNetwork",

    "SimulationRunOptions",
    "ResolvedSimulationOptions",

    "ScenarioGenerator",
    "ScenarioSet",

    "ScenarioProblem",
    "ScenarioProblemBuilder",

    "SolverOptions",
    "SolverDiagnostics",
    "SolverResult",
    "SolverStatus",
    "InteriorPointSolver",
    "SimulationSpreadsheetExporter",
    
    "ValidationIssue",
    "SimulationValidationError",
    "ScenarioGenerationError",
    "ScenarioPreparationError",
    "SolverError",
    "SolverNumericalError",
    "SimulationCancelledError",

    "SimulationBatchRunner",
    "SimulationBatchResult",

    "SimulationStatistics",
    "SimulationStatisticsProcessor",
    "SimulationJsonExporter",
    "StatisticsProcessingError",

    "ScenarioFailure",
    "ScenarioFailureStage",
    "ScenarioRunStatus",

    "build_simulation_data",
    "canonical_parameter_name",
    "normalize_simulation_params",

    "resolve_simulation_options",
    "resolve_solver_options",
    
    "validate_simulation_data",
    "validate_simulation_options"
]