from __future__ import annotations

import math
import numpy as np
from app.neatwork.simulation import (
    Simulation,
    SimulationRunOptions,
    build_simulation_data,
)
from .models import HydraulicReferenceState
from .neatwork_fixture import build_reference_neatwork_domain

def run_neatwork_reference() -> HydraulicReferenceState:
    """
    Execute the exact NeatWork counterpart of the controlled EPANET validation case.
    """

    domain = build_reference_neatwork_domain()
    instance = build_simulation_data(domain)

    simulation = Simulation(
        inst=instance,
        options=SimulationRunOptions(
            scenario_count=1,
            seed=42,
        ),
    )

    batch = simulation.run()
    assert batch.successful_scenario_count == 1
    flow_column = batch.flow_by_pipe_m3s[:, 0]
    pipe_flow_m3s = {
        str(edge_id): float(flow) 
        for (edge_id, flow) 
        in zip(batch.edge_ids, flow_column, strict=True)
    }

    diameter_by_edge = {
        str(row["external_edge_id"]): float(row["diam1"]) 
        for _, row in instance.pipes.iterrows()
    }

    pipe_velocity_mps: dict[str, float] = {}

    for (edge_id, flow_m3s) in pipe_flow_m3s.items():
        diameter_m = diameter_by_edge[edge_id]
        area_m2 = math.pi * diameter_m ** 2 / 4.0
        pipe_velocity_mps[edge_id] = abs(flow_m3s) / area_m2

    node_pressure_m: dict[str, float] = {}
    transit_dual = batch.node_dual_by_transit_node[:, 0]
    elevation_by_node = instance.nodes.set_index("noeud")["haut"].astype(float).to_dict()

    for (node_id, dual) in zip(batch.transit_node_ids, transit_dual, strict=True):
        node_pressure_m[str(node_id)] = (float(dual)
            -
            float(elevation_by_node[str(node_id)]))

    terminal_pipe_mask = np.asarray(instance.dat["idAter"], dtype=bool)
    terminal_pipe_indices = np.flatnonzero(terminal_pipe_mask)
    terminal_ids = batch.terminal_ids

    for (terminal_id, pipe_index) in zip(terminal_ids, terminal_pipe_indices, strict=True):
        node_pressure_m[str(terminal_id)] = -float(batch.charge_by_pipe_m[pipe_index, 0])

    p1_index = int(np.flatnonzero(batch.terminal_ids == "P1")[0])
    p1_delivery_m3s = (batch.demand_by_terminal_lps[p1_index, 0]
        *
        batch.satisfaction_by_terminal[p1_index, 0]
        *1e-3
    )

    return HydraulicReferenceState(
        pipe_flow_m3s=pipe_flow_m3s,
        pipe_velocity_mps=pipe_velocity_mps,
        node_pressure_m=node_pressure_m,
        terminal_delivery_m3s={"P1": float(p1_delivery_m3s)},

        terminal_satisfaction={
            "P1":float(batch.satisfaction_by_terminal[p1_index, 0]),
        },
    )

    assert pipe_flow_m3s["E2"] == pytest.approx(p1_delivery_m3s,rel=1e-8,abs=1e-10)
    