import pytest

from app.neatwork.domain.client_demand import (
    build_terminal_demand_profiles,
    terminal_demand_profiles_payload,
)

from app.schemas.simulation_domain import SimulationDomainInput

def build_domain():
    return (
        SimulationDomainInput.model_validate({
            "name": "terminal-demand-test",
            "params": {},
            "tables": {
                "nodes": [
                        {
                            "nodeId": "P3",
                            "elevationM": 0,
                            "status": 2,
                        },
                        {
                            "nodeId": "P4",
                            "elevationM": 0,
                            "status": 2,
                        },
                        {
                            "nodeId": "P5",
                            "elevationM": 0,
                            "status": 2,
                        },
                    ],
                    "pipes": [],
                    "clientBehaviors": [
                        {
                            "behaviorId": "DOMESTIC",
                            "name": "Domestic",
                            "openProbability": 0.40,
                            "targetFlowLps": 0.20,
                        },
                        {
                            "behaviorId": "RESIDENTIAL",
                            "name": "Residential",
                            "openProbability": 0.40,
                            "targetFlowLps": 0.20,
                        },
                        {
                            "behaviorId": "SCHOOL",
                            "name": "School",
                            "openProbability": 0.75,
                            "targetFlowLps": 0.35,
                        },
                    ],
                    "clients": [
                        {
                            "clientId": "C1",
                            "terminalId": "P3",
                            "behaviorId": "DOMESTIC",
                        },
                        {
                            "clientId": "C2",
                            "terminalId": "P3",
                            "behaviorId": "DOMESTIC",
                        },
                        {
                            "clientId": "C3",
                            "terminalId": "P3",
                            "behaviorId": "SCHOOL",
                        },
                        {
                            "clientId": "C4",
                            "terminalId": "P4",
                            "behaviorId": "RESIDENTIAL",
                        },
                    ],
                    "bornes": [
                        {
                            "terminalId": "P3",
                            "requiredPressureM": 3,
                        },
                        {
                            "terminalId": "P4",
                            "requiredPressureM": 3,
                        },
                        {
                            "terminalId": "P5",
                            "requiredPressureM": 3,
                        },
                    ],
                    "diameters": [],
                    "orifices": [],
                },
            }
        )
    )

def test_groups_clients_by_effective_behavior():
    profiles = build_terminal_demand_profiles(build_domain())

    p3 = next(profile for profile in profiles if (profile.terminal_id =="P3"))

    assert p3.client_count == 3
    assert len(p3.groups) == 2

    domestic = next(group for group in p3.groups
        if (group.target_flow_lps == pytest.approx(0.20)) and group.open_probability
            == pytest.approx(0.40))

    assert domestic.client_count == 2
    assert domestic.target_flow_lps == pytest.approx(0.20)
    assert domestic.open_probability == pytest.approx(0.40)

def test_equivalent_behavior_ids_share_one_group():
    domain = build_domain()
    domain.tables.clients.append(
        type(domain.tables.clients[0]).model_validate(
            {
                "clientId": "C5",
                "terminalId": "P3",
                "behaviorId": "RESIDENTIAL",
            }
        )
    )
    profiles = build_terminal_demand_profiles(domain)
    p3 = next(item for item in profiles if item.terminal_id == "P3")
    equivalent = next(group for group in p3.groups if group.target_flow_lps
        == pytest.approx(0.20)
    )
    assert equivalent.client_count == 3
    assert set(equivalent.behavior_ids) == {"DOMESTIC", "RESIDENTIAL"}

def test_terminal_without_clients_is_still_exported():
    profiles = build_terminal_demand_profiles(build_domain())
    p5 = next(profile for profile in profiles
        if profile.terminal_id == "P5"
    )
    assert p5.client_count == 0
    assert p5.groups == ()

def test_legacy_client_rows_are_supported():
    domain = build_domain()
    domain.tables.client_behaviors = []
    domain.tables.clients = [type(domain.tables.clients[0]).model_validate(
            {
                "clientId": "LEGACY_C1",
                "terminalId": "P3",
                "openProbability": 0.25,
                "targetFlowLps": 0.15,
            }
        )
    ]

    profiles = build_terminal_demand_profiles(domain)

    p3 = next(profile for profile in profiles
        if profile.terminal_id == "P3"
    )

    assert p3.client_count == 1
    assert p3.groups[0].open_probability == pytest.approx(0.25)
    assert p3.groups[0].target_flow_lps == pytest.approx(0.15)

def test_api_payload_uses_camel_case_contract():
    payload = terminal_demand_profiles_payload(build_domain())
    p3 = next(row for row in payload
        if row["terminalId"] == "P3"
    )
    assert p3["clientCount"] == 3
    assert "groups" in p3
    assert "targetFlowLps" in p3["groups"][0]
    assert "openProbability" in p3["groups"][0]