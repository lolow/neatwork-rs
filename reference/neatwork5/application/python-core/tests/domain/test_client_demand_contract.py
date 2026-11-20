from app.neatwork.domain.client_demand import (
    resolve_client_demand_profile,
    synchronize_client_demand_params,
)

from app.schemas.simulation_domain import SimulationDomainInput

def build_domain():
    return (
        SimulationDomainInput
        .model_validate(
            {
                "name": "demand-contract",
                "params": {
                    # Deliberately stale legacy values.
                    "opentaps": 0.10,
                    "targetflow": 9.00,
                },
                "tables": {
                    "nodes": [],
                    "pipes": [],
                    "clientBehaviors": [
                        {
                            "behaviorId": "DOMESTIC",
                            "openProbability": 0.80,
                            "targetFlowLps": 3.00,
                        }
                    ],
                    "clients": [
                        {
                            "clientId": "C1",
                            "terminalId": "P1",
                            "behaviorId": "DOMESTIC",
                        }
                    ],
                    "bornes": [],
                    "diameters": [],
                    "orifices": [],
                },
            }
        )
    )

def test_client_behavior_is_demand_source_of_truth():
    domain = build_domain()
    profile = resolve_client_demand_profile(domain)
    assert profile.open_probability == 0.80
    assert profile.target_flow_lps == 3.00

def test_legacy_params_are_synchronized_from_behavior():
    domain = build_domain()
    synchronized = synchronize_client_demand_params(domain)
    assert synchronized.params["opentaps"] == 0.80
    assert synchronized.params["targetflow"] == 3.00

    # Immutable input remains unchanged.
    assert domain.params["targetflow"] == 9.00
    
    