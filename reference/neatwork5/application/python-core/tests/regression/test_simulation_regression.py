"""Hydraulic numerical regression suite."""

from __future__ import annotations

import pytest

from .regression_cases import (REGRESSION_CASES, RegressionCase)
from .regression_helpers import (
    assert_regression_value,
    canonicalize_regression_payload,
    load_reference,
    run_regression_case,
)

@pytest.mark.regression
@pytest.mark.parametrize("case", REGRESSION_CASES, ids=lambda case: case.case_id)
def test_simulation_matches_reference(case: RegressionCase):
    actual = (
        run_regression_case(
            dsg_path=case.dsg_path,
            scenario_count=case.scenario_count,
            seed=case.seed,
        )
    )

    actual = canonicalize_regression_payload(actual)
    expected = load_reference(case.reference_path)

    assert_regression_value(actual, expected)

@pytest.mark.regression
def test_same_seed_produces_same_results():
    case = REGRESSION_CASES[0]
    first = (
        canonicalize_regression_payload(
            run_regression_case(
                dsg_path=case.dsg_path,
                scenario_count=case.scenario_count,
                seed=case.seed,
            )
        )
    )


    second = (
        canonicalize_regression_payload(
            run_regression_case(
                dsg_path=case.dsg_path,
                scenario_count=case.scenario_count,
                seed=case.seed,
            )
        )
    )

    assert_regression_value(first, second)