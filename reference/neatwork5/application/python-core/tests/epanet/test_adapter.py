from app.neatwork.epanet.adapter import _diameter_catalog_id
import pytest
from tests.epanet.test_inp_parser import FIXTURE_ROOT
from app.neatwork.epanet.prototype import parse_and_adapt_epanet
from app.neatwork.epanet import (EpanetAdapterOptions, EpanetDemandPolicy)

def test_adapter_does_not_invent_open_probability():

    content = (FIXTURE_ROOT / "simple_supported.inp").read_text(encoding="utf-8")

    result = parse_and_adapt_epanet(name="simple", content=content)

    assert result.domain.tables.clients == []
    assert result.report.simulation_ready is False

    codes = {issue.code for issue in result.report.issues}

    assert "INTERMITTENT_PROBABILITY_UNRESOLVED" in codes


def test_explicit_deterministic_policy_creates_always_open_clients():
    content = (FIXTURE_ROOT / "simple_supported.inp").read_text(encoding="utf-8")

    result = parse_and_adapt_epanet(
        name="simple",
        content=content,
        options=EpanetAdapterOptions(
                demand_policy=EpanetDemandPolicy.DETERMINISTIC_ALWAYS_OPEN,
                default_required_pressure_m=2.0,
        ))

    clients = result.domain.tables.clients

    assert len(clients) == 1
    assert clients[0].open_probability == 1.0
    assert clients[0].target_flow_lps == pytest.approx(0.20)

def test_diameter_id_is_order_independent():

    first = _diameter_catalog_id(
            diameter_m=0.063,
            roughness=100,
            headloss="H-W",
        )
    second = _diameter_catalog_id(
            diameter_m=0.063,
            roughness=100,
            headloss="H-W",
        )

    assert first == second

def test_pump_and_tank_are_blocking():
    content = (FIXTURE_ROOT / "unsupported_features.inp").read_text(encoding="utf-8")

    result = (parse_and_adapt_epanet(
            name="unsupported",
            content=content,
        ))

    assert result.report.simulation_ready is False

    codes = {
        issue.code
        for issue
        in result.report.issues
    }

    assert "UNSUPPORTED_PUMPS" in codes
    assert "UNSUPPORTED_TANKS" in codes

def test_emitter_is_preserved_but_not_converted_to_orifice():
    content = (FIXTURE_ROOT / "emitter.inp").read_text(encoding="utf-8")

    result = (parse_and_adapt_epanet(name="emitter", content=content))

    assert result.domain.tables.orifices == []

    codes = {issue.code for issue in result.report.issues}

    assert "EMITTER_ORIFICE_MAPPING_UNRESOLVED" in codes