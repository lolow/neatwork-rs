from app.schemas.simulation_options import (SimulationOptionsInput)
from app.services.simulation_service import (_apply_parameter_overrides)


def test_legacy_runtime_demand_override_is_ignored(sample_domain):
    original_open_taps = sample_domain.params["opentaps"]
    original_target_flow = sample_domain.params["targetflow"]
    options = (SimulationOptionsInput.model_validate({
                "openTapsProbability": 0.99,
                "targetFlowLps": 9.99
                } ))

    updated = _apply_parameter_overrides(sample_domain, options)

    assert (updated.params["opentaps"] == original_open_taps)
    assert (updated.params["targetflow"] == original_target_flow)