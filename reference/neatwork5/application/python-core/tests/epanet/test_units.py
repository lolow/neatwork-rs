from app.neatwork.epanet import EpanetSemanticModel
import pytest
from app.neatwork.epanet.units import (
    EpanetUnitContext,
)


def test_metric_epanet_units():

    model = EpanetSemanticModel(options={ "UNITS": "LPS", "HEADLOSS": "H-W" })

    units = EpanetUnitContext.from_model(model)

    assert units.length_to_m(10) == 10
    assert units.pipe_diameter_to_m(63) == 0.063
    assert units.flow_to_lps(2) == 2

def test_us_epanet_units():
    model = EpanetSemanticModel(options={ "UNITS": "GPM", "HEADLOSS": "H-W" })

    units = EpanetUnitContext.from_model(model)

    assert units.length_to_m(10) == pytest.approx(3.048)
    assert units.pipe_diameter_to_m(12) == pytest.approx(0.3048)
    assert units.flow_to_lps(1) == pytest.approx(0.0630901964)