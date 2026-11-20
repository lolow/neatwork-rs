from __future__ import annotations
from app.neatwork import epanet
from app import neatwork

from pathlib import Path
import pytest
pytest.importorskip("wntr")

from .epanet_runner import run_epanet_reference
from .neatwork_runner import run_neatwork_reference

FIXTURE_ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "epanet"

@pytest.mark.reference
def test_pipe_flow_matches_epanet(tmp_path: Path):
    epanet = (run_epanet_reference(
        FIXTURE_ROOT / "reference_pda_hw.inp",
        work_directory=tmp_path,
        full_demand_lps=1.0,
    ))

    neatwork = run_neatwork_reference()
    for edge_id in ("E1", "E2"):
        assert (neatwork.pipe_flow_m3s[edge_id] == 
            pytest.approx(
                epanet.pipe_flow_m3s[edge_id],
                rel=1e-5,
                abs=1e-8,
        ))

@pytest.mark.reference
def test_node_pressure_matches_epanet(tmp_path: Path):
    epanet = (run_epanet_reference(
        FIXTURE_ROOT / "reference_pda_hw.inp",
        work_directory=tmp_path,
        full_demand_lps=1.0,
    ))
    neatwork = run_neatwork_reference()
    for node_id in ("J1","P1"):
        assert (neatwork.node_pressure_m[node_id] ==
            pytest.approx(
                epanet.node_pressure_m[node_id],
                rel=1e-5,
                abs=1e-4,
        ))

@pytest.mark.reference
def test_pipe_velocity_matches_epanet(tmp_path: Path):
    ...
    for edge_id in ("E1","E2"):
        assert (neatwork.pipe_velocity_mps[edge_id] ==
            pytest.approx(
                epanet.pipe_velocity_mps[edge_id],
                rel=1e-5,
                abs=1e-6,
            ))

@pytest.mark.reference
def test_bounded_flow_matches_epanet_pda_delivery(tmp_path:Path):
    epanet = ...
    neatwork = ...
    assert (neatwork.terminal_delivery_m3s["P1"] ==
        pytest.approx(epanet.terminal_delivery_m3s["P1"],
            rel=1e-5,
            abs=1e-8,
        ))

    assert (neatwork.terminal_satisfaction["P1"] ==
        pytest.approx(epanet.terminal_satisfaction["P1"],
            rel=1e-5,
            abs=1e-6,
        ))

        

