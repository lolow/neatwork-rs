from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class HydraulicReferenceState:
    pipe_flow_m3s: dict[str, float]
    pipe_velocity_mps: dict[str, float]
    node_pressure_m: dict[str, float]
    terminal_delivery_m3s: dict[str, float]
    terminal_satisfaction: dict[str, float]
    