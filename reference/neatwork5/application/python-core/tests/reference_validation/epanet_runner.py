from __future__ import annotations
from pathlib import Path
import wntr
from .models import HydraulicReferenceState


def run_epanet_reference(
    inp_path: Path,
    *,
    work_directory: Path,
    full_demand_lps: float,
) -> HydraulicReferenceState:
    """
    Execute one single-period reference case with the official EPANET 2.2 engine through WNTR.
    """
    network = wntr.network.WaterNetworkModel(str(inp_path))
    simulator = wntr.sim.EpanetSimulator(network)
    file_prefix = work_directory / "epanet_reference"

    results = simulator.run_sim(
            file_prefix=str(file_prefix),
            version=2.2,
            convergence_error=True,
        )
    
    time_index = results.node["pressure"].index[0]

    pressure_row = results.node["pressure"].loc[time_index]
    node_pressure_m = {str(node_id): float(pressure_row[node_id]) for node_id in ["J1", "P1"]}

    flow_row = results.link["flowrate"].loc[time_index]
    pipe_flow_m3s = {str(edge_id): float(flow_row[edge_id]) for edge_id in ["E1", "E2"]}

    velocity_row = results.link["velocity"].loc[time_index]
    pipe_velocity_mps = {str(edge_id): float(velocity_row[edge_id]) for edge_id in ["E1", "E2"]}

    demand_row = results.node["demand"].loc[time_index]
    delivered_p1_m3s = float(demand_row["P1"])
    full_demand_m3s = full_demand_lps * 1e-3

    satisfaction = delivered_p1_m3s / full_demand_m3s

    return HydraulicReferenceState(
        pipe_flow_m3s=pipe_flow_m3s,
        pipe_velocity_mps=pipe_velocity_mps,
        node_pressure_m=node_pressure_m,
        terminal_delivery_m3s={"P1":delivered_p1_m3s},
        terminal_satisfaction={"P1":satisfaction},
    )
    

