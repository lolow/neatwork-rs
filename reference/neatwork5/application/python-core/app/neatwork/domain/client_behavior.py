from __future__ import annotations

import hashlib
from decimal import Decimal
from app.neatwork.schemas import NeatworkDomainPayload
from app.schemas.simulation_domain import SimulationDomainInput

def _canonical_decimal(value: float) -> str:
    """Return one stable decimal representation."""
    return format(Decimal(str(value)).normalize(), "f")

def build_behavior_id(*, open_probability: float,
    target_flow_lps: float, profile_id: str | None = None) -> str:
    """Build a stable client-behaviour identifier."""

    canonical = "|".join(
        [
            _canonical_decimal(open_probability),
            _canonical_decimal(target_flow_lps),
            profile_id or "",
        ]
    )

    digest = (
        hashlib.sha1(canonical.encode("utf-8"))
        .hexdigest()[:10].upper()
    )

    return f"CB_{digest}"

def normalize_client_behaviors(domain: NeatworkDomainPayload) -> NeatworkDomainPayload:
    """
    Convert legacy client demand rows into reusable client behaviours.

    The original domain is not mutated.
    """

    result = domain.model_copy(deep=True)

    behavior_by_id: dict[str, dict] = {
        str(behavior.get("behaviorId")):
            dict(behavior)
        for behavior in result.tables.clientBehaviors
        if behavior.get("behaviorId")
    }

    normalized_clients: list[dict] = []

    for client in result.tables.clients:
        row = dict(client)

        existing_behavior_id = (
            row.get("behaviorId")
            or
            row.get("behavior_id")
        )

        if existing_behavior_id:
            normalized_clients.append(row)
            continue

        probability = row.get("proba")
        target_flow = row.get("cible")

        if probability is None or target_flow is None:
            raise ValueError(
                f"Client {row.get('client')!r} "
                "has neither behaviorId nor "
                "complete legacy demand values."
            )

        behavior_id = (
            build_behavior_id(
                open_probability=float(probability),
                target_flow_lps=float(target_flow),
                profile_id=None,
            ))

        if behavior_id not in behavior_by_id:
            behavior_by_id[behavior_id] = {
                "behaviorId": behavior_id,
                "name":
                    f"Imported behavior "f"{len(behavior_by_id) + 1}",
                "openProbability": float(probability),
                "targetFlowLps": float(target_flow),
                "profileId": None,
            }

        normalized_clients.append(
            {
                "client": row.get("client"),
                "borne": row.get("borne"),
                "behaviorId": behavior_id,
            }
        )

    result.tables.clients = normalized_clients
    result.tables.clientBehaviors = list(behavior_by_id.values())

    return result

def materialize_effective_clients(domain: SimulationDomainInput) -> list[dict]:
    """
    Resolve canonical client behaviours into
    legacy rows expected by the numerical core.
    """
    behavior_by_id = {
        behavior.behavior_id: behavior
        for behavior in domain.tables.client_behaviors
    }
    rows: list[dict] = []

    for client in domain.tables.clients:
        if client.behavior_id:
            behavior = behavior_by_id.get(client.behavior_id)
            if behavior is None:
                raise ValueError(
                    f"Client {client.client_id!r} "
                    f"references unknown behavior "
                    f"{client.behavior_id!r}."
                )

            rows.append(
                {
                    "client": client.client_id,
                    "borne": client.terminal_id,
                    "proba": behavior.open_probability,
                    "cible": behavior.target_flow_lps,
                }
            )
            continue

        # Legacy immutable snapshot.
        if (client.open_probability is not None
            and
            client.target_flow_lps is not None
        ):
            rows.append(
                {
                    "client": client.client_id,
                    "borne": client.terminal_id,
                    "proba": client.open_probability,
                    "cible": client.target_flow_lps,
                }
            )
            continue
        raise ValueError(
            f"Client {client.client_id!r} "
            "has no usable demand behaviour."
        )
    return rows

def materialize_serializable_clients(domain: NeatworkDomainPayload) -> list[dict]:
    """Build legacy client rows for DSG/TPO serialization."""

    behavior_by_id = {
        str(row.get("behaviorId")): row
        for row in domain.tables.clientBehaviors
        if row.get("behaviorId")
    }
    result: list[dict] = []
    for client in domain.tables.clients:
        behavior_id = client.get("behaviorId") or client.get("behavior_id")
        if behavior_id:
            behavior = behavior_by_id.get(str(behavior_id))
            if behavior is None:
                raise ValueError(
                    f"Unknown client behavior {behavior_id!r}."
                )
            result.append(
                {
                    "client":(client.get("client") or client.get("clientId") or client.get("client_id")),
                    "borne":(client.get("borne") or client.get("terminalId") or client.get("terminal_id")),
                    "proba": behavior.get("openProbability"),
                    "cible": behavior.get("targetFlowLps")
                }
            )
            continue
        result.append(
            {
                "client":(client.get("client") or client.get("clientId") or client.get("client_id")),
                "borne":(client.get("borne") or client.get("terminalId") or client.get("terminal_id")),
                "proba": client.get("proba"),
                "cible": client.get("cible")
            }
        )
    return result