import pytest

from .neatwork_fixture import (
    HAZEN_WILLIAMS_C,
    HW_DIAMETER_EXPONENT,
    HW_FLOW_EXPONENT,
    HW_SI_CONSTANT,
    hazen_williams_bescal,
)

def test_neatwork_beta_can_represent_uniform_hazen_williams():
    length_m = 100.0
    diameter_m = 0.063
    flow_m3s = 0.001
    beta = (hazen_williams_bescal(HAZEN_WILLIAMS_C) * 
        length_m / diameter_m ** HW_DIAMETER_EXPONENT)
    neatwork_loss = (beta * flow_m3s ** HW_FLOW_EXPONENT)

    epanet_loss = (HW_SI_CONSTANT
        * HAZEN_WILLIAMS_C ** (-HW_FLOW_EXPONENT)
        * diameter_m ** (-HW_DIAMETER_EXPONENT)
        * length_m
        * flow_m3s ** HW_FLOW_EXPONENT)

    assert (neatwork_loss == pytest.approx(epanet_loss, rel=1e-12))

def test_bounded_flow_formula_matches_epanet_pda_special_case():
    full_demand = 1.0e-3
    required_pressure = 20.0
    actual_pressure = 10.0
    neatwork_delivery = (full_demand * (actual_pressure
        / required_pressure)** 0.5)

    epanet_delivery = (full_demand * ((actual_pressure - 0.0)
        / (required_pressure - 0.0))** 0.5)

    assert (neatwork_delivery == pytest.approx(epanet_delivery))

