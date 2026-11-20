import pytest

from app.neatwork.domain.client_behavior import build_behavior_id, normalize_client_behaviors, materialize_effective_clients

from app.neatwork.schemas import NeatworkDomainPayload
from app.schemas.simulation_domain import SimulationDomainInput


def test_legacy_clients_are_deduplicated():
    domain = NeatworkDomainPayload.model_validate(
        {
            "name": "test",
            "tables": {
                "clients": [
                    {
                        "client": "C1",
                        "borne": "P1",
                        "proba": 0.4,
                        "cible": 0.2,
                    },
                    {
                        "client": "C2",
                        "borne": "P2",
                        "proba": 0.4,
                        "cible": 0.2,
                    },
                ]
            },
        }
    )

    normalized = normalize_client_behaviors(domain)

    assert len(normalized.tables.clientBehaviors) == 1
    assert normalized.tables.clients[0]["behaviorId"] == normalized.tables.clients[1]["behaviorId"]

def test_same_values_create_same_behavior_id():
    first = build_behavior_id(open_probability=0.4, target_flow_lps=0.20)
    second = build_behavior_id(open_probability=0.40, target_flow_lps=0.2000)

    assert first == second

def test_materialized_clients_match_legacy_values():

    domain = SimulationDomainInput.model_validate(
        {
            "name": "behavior-test",

            "tables": {
                "nodes": [],
                "pipes": [],

                "clientBehaviors": [
                    {
                        "behaviorId": "DOMESTIC",
                        "name": "Domestic",
                        "openProbability": 0.4,
                        "targetFlowLps": 0.2,
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

    rows = materialize_effective_clients(domain)

    assert rows == [
        {
            "client": "C1",
            "borne": "P1",
            "proba": 0.4,
            "cible": 0.2,
        }
    ]

def test_missing_behavior_is_rejected():

    domain = SimulationDomainInput.model_validate(
        {
            "name": "invalid-test",

            "tables": {
                "nodes": [],
                "pipes": [],

                "clientBehaviors": [],

                "clients": [
                    {
                        "clientId": "C1",
                        "terminalId": "P1",
                        "behaviorId": "MISSING",
                    }
                ],

                "bornes": [],
                "diameters": [],
                "orifices": [],
            },
        }
    )

    with pytest.raises(ValueError, match="unknown behavior"):

        materialize_effective_clients(domain)


