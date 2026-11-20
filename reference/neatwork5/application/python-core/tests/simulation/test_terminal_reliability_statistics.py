from __future__ import annotations

import numpy as np
import pytest

from app.neatwork.simulation.models import (SimulationBatchResult)
from app.neatwork.simulation.statistics import SimulationStatisticsProcessor


def build_batch() -> SimulationBatchResult:

    return SimulationBatchResult(
        terminal_ids=np.array(["P3", "P4"]),
        edge_ids=np.array([], dtype=str),
        transit_node_ids=np.array([], dtype=str),
        demand_by_terminal_lps=np.zeros((2, 5), dtype=float),
        open_client_count_by_terminal=np.array([
                [1,1,1,0,2],
                [0,1,0,1,1],
            ],
            dtype=int,
        ),
        total_demand_lps=np.zeros(5,dtype=float),
        flow_by_pipe_m3s=np.empty((0,5)),
        charge_by_pipe_m=np.empty((0,5)),
        node_dual_by_transit_node=np.empty((0,5)),
        satisfaction_by_terminal=np.array([
                [1.0,0.5,np.nan,0.0,1.2],
                [0.0,0.0,np.nan,1.0,0.8],
            ]
        ),
        alpha_by_pipe=np.empty((0,5)),
        open_pipe_mask_by_scenario=np.empty((0,5),dtype=bool),
        active_terminal_mask_by_scenario=np.array([
                [True,True,True,False,True],
                [False,True,False,True,True],
            ]
        ),
        served_terminal_mask_by_scenario=np.array([
                [True,True,False,False,True],
                [False,True,False,True,True],
            ]
        ),
        iterations=np.array([10,11,-1,9,10]),
        success_mask=np.array([True,True,False,True,True]),
        status_by_scenario=tuple(),
        failures=tuple(),
        scenario_duration_seconds=np.zeros(5),
        scenario_count=5,
        successful_scenario_count=4,
        failed_scenario_count=1,
        seed=42,
        duration_seconds=0.1,
    )

def test_terminal_activity_and_shortfall_are_separate_from_execution_failure():
    batch = build_batch()
    processor = SimulationStatisticsProcessor(instance=None)

    table = processor._compute_terminal_satisfaction(batch,batch.success_mask)
    p3 = table.loc["P3"]


    assert (
        p3[
            "activeScenarioCount"
        ]
        ==
        4
    )


    assert (
        p3[
            "evaluatedActiveScenarioCount"
        ]
        ==
        3
    )


    assert (
        p3[
            "waterShortfallCount"
        ]
        ==
        1
    )


    assert (
        p3[
            "maxSat"
        ]
        ==
        pytest.approx(
            1.2
        )
    )

    expected_values = np.array(
        [
            1.0,
            0.5,
            1.2,
        ]
    )


    expected_std = np.std(
        expected_values,
        ddof=0,
    )


    expected_mean = np.mean(
        expected_values
    )


    assert (
        p3[
            "stdSat"
        ]
        ==
        pytest.approx(
            expected_std
        )
    )


    assert (
        p3[
            "cvSat"
        ]
        ==
        pytest.approx(
            expected_std
            /
            expected_mean
        )
    )

    p4 = table.loc[
        "P4"
    ]


    assert (
        p4[
            "activeScenarioCount"
        ]
        ==
        3
    )


    assert (
        p4[
            "evaluatedActiveScenarioCount"
        ]
        ==
        3
    )


    assert (
        p4[
            "waterShortfallCount"
        ]
        ==
        2
    )

def test_satisfaction_above_one_is_not_clipped():

    batch = build_batch()


    table = (
        SimulationStatisticsProcessor(
            instance=None
        )
        ._compute_terminal_satisfaction(
            batch,
            batch.success_mask,
        )
    )


    assert (
        table.loc[
            "P3",
            "maxSat",
        ]
        ==
        pytest.approx(
            1.2
        )
    )

def test_statistics_can_be_computed_for_unserved_terminals():
    batch = build_batch()


    table = (
        SimulationStatisticsProcessor(
            instance=None
        )
        ._compute_terminal_satisfaction(
            batch,
            batch.success_mask,
        )
    )


    px = table.loc[
        "PX",
    ]


    assert (
        px[
            "activeScenarioCount"
        ]
        ==
        2
    )


    assert (
        px[
            "evaluatedActiveScenarioCount"
        ]
        ==
        0
    )


    assert (
        px[
            "waterShortfallCount"
        ]
        ==
        0
    )


    assert (
        np.isnan(
            px[
                "cvSat"
            ]
        )
    )
    