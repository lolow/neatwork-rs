import numpy as np
import pytest

from app.neatwork.hydraulic_units import lps_to_m3s, m3s_to_lps


def test_lps_to_m3s():
    assert lps_to_m3s(1.0) == pytest.approx(0.001)

def test_m3s_to_lps():
    assert m3s_to_lps(0.001) == pytest.approx(1.0)

def test_flow_conversion_round_trip():
    values = np.array([0.2, 1.0, 3.0])
    restored = m3s_to_lps(lps_to_m3s(values))
    assert np.allclose(restored,values)