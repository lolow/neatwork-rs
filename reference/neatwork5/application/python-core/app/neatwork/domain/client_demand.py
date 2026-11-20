"""Canonical client-demand resolution."""

from __future__ import annotations
from dataclasses import dataclass
from collections import defaultdict
from app.neatwork.domain.client_behavior import materialize_effective_clients
from app.neatwork.domain.client_behavior import build_behavior_id
from app.schemas.simulation_domain import SimulationDomainInput


@dataclass(frozen=True,slots=True)
class ClientDemandProfile:
    """Resolved demand semantics for one domain."""
    client_count: int
    uniform: bool
    open_probability: float | None
    target_flow_lps: float | None

def resolve_client_demand_profile(domain) -> ClientDemandProfile:
    """
    Resolve demand exclusively from canonical client data.
    Legacy scalar parameters are never authoritative here.
    """
    clients = materialize_effective_clients(domain)
    if not clients:
        raise ValueError("Client demand table is empty.")

    probabilities:list[float] = []
    targets:list[float] = []
    for client in clients:
        probability = float(client["proba"])
        target = float(client["cible"])

        if not (0.0 <= probability <= 1.0):
            raise ValueError("Client opening probability must be between 0 and 1.")

        if target <= 0.0:
            raise ValueError("Client target flow must be strictly positive.")

        probabilities.append(probability)
        targets.append(target)

    unique_probabilities = set(probabilities)
    unique_targets = set(targets)

    uniform = (len(unique_probabilities) == 1 and len(unique_targets) == 1)

    return ClientDemandProfile(
        client_count=len(clients),
        uniform=uniform,
        open_probability=probabilities[0] if uniform else None,
        target_flow_lps=targets[0] if uniform else None,
    )

def synchronize_client_demand_params(domain, profile:ClientDemandProfile | None = None):
    """
    Synchronize legacy scalar parameters from the authoritative client-demand model.
    The original immutable domain is never modified.
    """
    resolved = profile or resolve_client_demand_profile(domain)
    updated = domain.model_copy(deep=True)
    updated.params["uniform_dem"] = (1 if resolved.uniform else 0)
    if resolved.uniform:
        updated.params["opentaps"] = resolved.open_probability
        updated.params["targetflow"] = resolved.target_flow_lps
    return updated

@dataclass(frozen=True, slots=True)
class TerminalDemandGroup:
    """Group consumers with identical effective demand behaviour."""
    group_id: str
    behavior_ids: tuple[str, ...]
    behavior_names: tuple[str, ...]
    client_count: int
    open_probability: float
    target_flow_lps: float
    profile_id: str | None

    def to_api_dict(self) -> dict:
        """Return the public JSON representation."""
        return {
            "groupId": self.group_id,
            "behaviorIds": list(self.behavior_ids),
            "behaviorNames": list(self.behavior_names),
            "clientCount": self.client_count,
            "openProbability": self.open_probability,
            "targetFlowLps": self.target_flow_lps,
            "profileId": self.profile_id,
        }

@dataclass(frozen=True, slots=True)
class TerminalDemandProfile:
    """Demand configuration of all clients attached to one terminal."""
    terminal_id: str
    client_count: int
    groups: tuple[TerminalDemandGroup,...]


    def to_api_dict(self) -> dict:
        return {
            "terminalId": self.terminal_id,
            "clientCount": self.client_count,
            "groups": [group.to_api_dict() for group in self.groups],
        }
    
def _terminal_order(domain:SimulationDomainInput) -> list[str]:
    """
    Return all terminal IDs in stable network order.

    Status-2 nodes define physical terminal outlets.
    Borne and client references are retained as fallback.
    """
    result:list[str] = []
    seen:set[str] = set()

    def append(terminal_id:str) -> None:
        normalized = str(terminal_id).strip()
        if not normalized or normalized in seen:
            return
        seen.add(normalized)
        result.append(normalized)

    for node in (domain.tables.nodes):
        if int(node.status) == 2:
            append(node.node_id)

    for borne in (domain.tables.bornes):
        append(borne.terminal_id)

    for client in (domain.tables.clients):
        append(client.terminal_id)

    return result

def _behavior_lookup(domain:SimulationDomainInput):
    """Index reusable behaviours by identifier."""
    return {
        behavior.behavior_id: behavior
        for behavior in domain.tables.client_behaviors
    }

def build_terminal_demand_profiles(domain: SimulationDomainInput) -> list[TerminalDemandProfile]:
    """
    Build grouped demand metadata for every terminal.
    Clients are grouped by their effective demand semantics, not merely by behaviour display name or identifier.
    Legacy immutable snapshots remain supported.
    """
    behavior_by_id = _behavior_lookup(domain)
    terminal_ids = _terminal_order(domain)
    groups_by_terminal: dict[str, dict[str, dict]] = (defaultdict(dict))

    for client in (domain.tables.clients):

        terminal_id = str(client.terminal_id).strip()
        original_behavior_id: str | None = None
        behavior_name: str | None = None
        profile_id: str | None = None        
        if client.behavior_id:
            behavior = behavior_by_id.get(client.behavior_id)
            if behavior is None:
                raise ValueError(f"Client {client.client_id!r} references unknown behavior {client.behavior_id!r}.")

            original_behavior_id = behavior.behavior_id
            behavior_name = (behavior.name.strip()
                if (behavior.name and behavior.name.strip()) else None
            )
            probability = float(behavior.open_probability)
            target_flow_lps = float(behavior.target_flow_lps)

            profile_id = (behavior.profile_id.strip()
                if (behavior.profile_id and behavior.profile_id.strip()) else None
            )

        else:
            if (client.open_probability is None or client.target_flow_lps is None):
                raise ValueError(f"Client {client.client_id!r} has no usable demand behavior.")
            probability = float(client.open_probability)
            target_flow_lps = float(client.target_flow_lps)

            if not (0.0 <= probability <= 1.0):
                raise ValueError(f"Client {client.client_id!r} has an invalid opening probability.")

        if (target_flow_lps <= 0.0):
            raise ValueError(f"Client {client.client_id!r} has a non-positive target flow.")

        group_id = (
            build_behavior_id(
                open_probability=probability,
                target_flow_lps=target_flow_lps,
                profile_id=profile_id,
            )
        )
        terminal_groups = (groups_by_terminal[terminal_id])
        group = (terminal_groups.get(group_id))
        if group is None:
            group = {
                "clientCount":0,
                "openProbability":probability,
                "targetFlowLps":target_flow_lps,
                "profileId":profile_id,
                "behaviorIds":set(),
                "behaviorNames":set(),
            }
            terminal_groups[group_id] = group
        group["clientCount"] += 1
        if original_behavior_id:
            group["behaviorIds"].add(original_behavior_id)
        if behavior_name:
            group["behaviorNames"].add(behavior_name)
        if (terminal_id not in terminal_ids):
            terminal_ids.append(terminal_id)

    profiles:list[TerminalDemandProfile] = []
    for terminal_id in (terminal_ids):
        raw_groups = (groups_by_terminal.get(terminal_id, {}))
        groups:list[TerminalDemandGroup] = []
        for (group_id,raw_group) in sorted(
            raw_groups.items(),
            key=lambda item:item[0],
        ):
            groups.append(TerminalDemandGroup(
                group_id=group_id,
                behavior_ids=tuple(sorted(raw_group["behaviorIds"])),
                behavior_names=tuple(sorted(raw_group["behaviorNames"])),
                client_count=int(raw_group["clientCount"]),
                open_probability=float(raw_group["openProbability"]),  
                target_flow_lps=float(raw_group["targetFlowLps"]),
                profile_id=raw_group["profileId"],
                )
            )

        client_count = sum(group.client_count for group in groups)
        profiles.append(TerminalDemandProfile(
                terminal_id=terminal_id,
                client_count=client_count,
                groups=tuple(groups),
            ))

    return profiles

def terminal_demand_profiles_payload(domain:SimulationDomainInput) -> list[dict]:
    """Return JSON-ready grouped terminal demand metadata."""
    return [profile.to_api_dict() for profile in build_terminal_demand_profiles(domain)]