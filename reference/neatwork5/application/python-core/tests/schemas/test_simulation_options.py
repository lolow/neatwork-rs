import pytest
from app.schemas.simulation_options import ( SimulationOptionsInput)

@pytest.mark.parametrize(
    ("payload","expected_bounded","expected_orifice"),
    [
    ({"boundedFlow": True,"orificeMode": "IDEAL"}, True, "IDEAL"),
    ({"boundedFlow": True,"orificeMode": "COMMERCIAL"},True,"COMMERCIAL"),
    ({"boundedFlow": False,"orificeMode": "IDEAL"}, False,"IDEAL"),
    ({"orificeMode": "COMMERCIAL"},None,"COMMERCIAL"),
    ],
)

def test_bounded_flow_and_orifice_are_independent(payload, expected_bounded,expected_orifice):
    options = SimulationOptionsInput.model_validate(payload)
    assert options.bounded_flow is expected_bounded
    if expected_orifice is None:
        assert options.orifice_mode is None
    else:
        assert options.orifice_mode.value == expected_orifice