"""Canonical hydraulic regression cases."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[4]

@dataclass(frozen=True, slots=True)
class RegressionCase:
    """One deterministic hydraulic regression case."""
    case_id: str
    dsg_path: Path
    reference_path: Path
    scenario_count: int
    seed: int

REFERENCE_ROOT = (
    PROJECT_ROOT
    / "application"
    / "python-core"
    / "tests"
    / "fixtures"
    / "reference"
)

REGRESSION_CASES = (
    RegressionCase(
        case_id="toy_blu",
        dsg_path=(
            PROJECT_ROOT
            / "examples"
            / "dsg_examples"
            / "Toy"
            / "Toy_blu_Secu1.00_fusion0_split0.dsg"
        ),
        reference_path=(
            REFERENCE_ROOT
            / "toy_blu_seed42.json"
        ),
        scenario_count=100,
        seed=42,
    ),

    RegressionCase(
        case_id="toy_bbu",
        dsg_path=(
            PROJECT_ROOT
            / "examples"
            / "dsg_examples"
            / "Toy"
            / "Toy_bbu_Secu1.00_fusion0_split0.dsg"
        ),
        reference_path=(
            REFERENCE_ROOT
            / "toy_bbu_seed42.json"
        ),
        scenario_count=100,
        seed=42,
    ),

    RegressionCase(
        case_id="toy_bbd",
        dsg_path=(
            PROJECT_ROOT
            / "examples"
            / "dsg_examples"
            / "Toy"
            / "Toy_bbd_Secu1.00_fusion0_split0.dsg"
        ),
        reference_path=(
            REFERENCE_ROOT
            / "toy_bbd_seed42.json"
        ),
        scenario_count=100,
        seed=42,
    ),
)