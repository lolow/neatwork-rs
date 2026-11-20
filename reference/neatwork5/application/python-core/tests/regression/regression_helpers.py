"""Helpers for deterministic hydraulic regression tests."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path
from typing import Any
import pytest

from app.neatwork.parsers.dsg_parser import parse_dsg
from app.neatwork.simulation import Simulation, SimulationRunOptions, build_simulation_data
from app.schemas.simulation_domain import SimulationDomainInput


def load_dsg_domain(path: Path) -> SimulationDomainInput:
    """Parse one regression DSG into the canonical domain."""

    if not path.exists():
        raise FileNotFoundError(f"Regression DSG does not exist: {path}")

    parsed_domain, _warnings = parse_dsg(name=path.stem,
        content=path.read_text(encoding="utf-8"),
    )

    return SimulationDomainInput.model_validate(
        parsed_domain.model_dump(mode="json", by_alias=True)
    )

def run_regression_case(*, dsg_path: Path, scenario_count: int, seed: int) -> dict[str, Any]:
    """Run the real hydraulic core and return strict statistics JSON."""
    domain = load_dsg_domain(dsg_path)
    instance = build_simulation_data(domain)
    simulation = Simulation(inst=instance, 
    options=SimulationRunOptions(scenario_count=scenario_count, seed=seed))

    simulation.run()
    simulation.compute_stats()
    payload = simulation.statistics_payload()

    return deepcopy(payload)

VOLATILE_SUMMARY_FIELDS = {"durationSeconds"}

def canonicalize_regression_payload(payload: dict[str, Any]) -> dict[str, Any]:
    """Remove non-deterministic execution metadata."""
    result = deepcopy(payload)
    summary = result.get("summary")

    if isinstance(summary, dict):
        for field in (VOLATILE_SUMMARY_FIELDS):
            summary.pop(field, None)

    return result

DEFAULT_REL_TOLERANCE = 1.0e-6

DEFAULT_ABS_TOLERANCE = 1.0e-8

def assert_regression_value(actual: Any, expected: Any, *, path: str = "$") -> None:
    """Recursively compare canonical simulation output."""

    if expected is None:
        assert actual is None, f"{path}: expected None, got {actual!r}"
        return

    if isinstance(expected, bool):
        assert actual is expected, f"{path}: expected {expected!r}, got {actual!r}"
        return

    if isinstance(expected, (int, float)) and not isinstance(expected, bool):
        assert isinstance(actual, (int, float)), f"{path}: expected numeric value, got {type(actual).__name__}"
        assert actual == pytest.approx(expected,
            rel= DEFAULT_REL_TOLERANCE,
            abs= DEFAULT_ABS_TOLERANCE
        ), f"{path}: expected {expected!r}, got {actual!r}"

        return

    if isinstance(expected, dict):
        assert isinstance(actual, dict), f"{path}: expected dict"
        assert set(actual.keys()) == set(expected.keys()), (
            f"{path}: key mismatch\n"
            f"expected={sorted(expected.keys())}\n"
            f"actual={sorted(actual.keys())}")

        for key in expected:
            assert_regression_value(
                actual[key],
                expected[key],
                path=f"{path}.{key}",
            )

        return

    if isinstance(expected, list):
        assert isinstance(actual, list), f"{path}: expected list"
        assert len(actual) == len(expected), (
            f"{path}: expected "
            f"{len(expected)} rows, "
            f"got {len(actual)}")
            
        for index, (actual_item, expected_item) in enumerate(zip(
                actual,
                expected,
                strict=True,
            )
        ):

            assert_regression_value(actual_item,expected_item,path=f"{path}[{index}]")

        return

    assert actual == expected, (
        f"{path}: "
        f"expected {expected!r}, "
        f"got {actual!r}"
    )

def load_reference(path: Path) -> dict[str, Any]:
    """Load one reviewed hydraulic baseline."""
    if not path.exists():
        raise AssertionError("Regression reference is missing: " f"{path}")

    return json.loads(path.read_text(encoding="utf-8"))