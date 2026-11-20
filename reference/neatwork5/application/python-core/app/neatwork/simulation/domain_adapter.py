"""Convert typed API payloads into numerical-core simulation structures.

The API uses descriptive Python/JSON names while the legacy numerical engine
expects DSG-style column and parameter names. This module is the translation
boundary: it normalizes parameter aliases, builds the five pandas tables, and
creates a validated :class:`SimulationData` instance without reading a file.
"""

import re
from typing import Any

import pandas as pd

from app.schemas.simulation_domain import SimulationDomainInput

from .constants import (
    DEFAULT_SIMULATION_PARAMS,
    INTEGER_SIMULATION_PARAMS,
    PARAMETER_ALIASES,
)
from .data import SimulationData

from app.neatwork.domain.client_behavior import materialize_effective_clients
from app.neatwork.domain.client_demand import resolve_client_demand_profile, synchronize_client_demand_params


# Matches the position immediately before an uppercase character, allowing API
# names such as ``targetFlow`` to be normalized to ``target_flow``.
_CAMEL_CASE_BOUNDARY = re.compile(r"(?<!^)(?=[A-Z])")


def _to_snake_case(value: str) -> str:
    """Normalize camelCase, kebab-case, and spaced names to snake_case."""
    normalized = value.strip().replace("-", "_").replace(" ", "_")
    normalized = _CAMEL_CASE_BOUNDARY.sub("_", normalized)

    while "__" in normalized:
        normalized = normalized.replace("__", "_")

    return normalized.lower()


def canonical_parameter_name(parameter_name: str) -> str:
    """Return the numerical-core name for a legacy or API parameter name.

    Unknown names are still normalized to snake_case and retained so newer
    parameters can pass through the adapter without requiring an alias entry.
    """
    snake_name = _to_snake_case(parameter_name)
    return PARAMETER_ALIASES.get(snake_name, snake_name)


def _convert_parameter_value(parameter_name: str, value: Any) -> float | int:
    """Convert a raw parameter value to the numeric type required by the core.

    Boolean values become ``0`` or ``1``. Parameters listed in
    :data:`INTEGER_SIMULATION_PARAMS` become integers; all others become floats.

    Raises
    ------
    ValueError
        If a string or other value cannot be converted to a number.
    """
    if isinstance(value, bool):
        numeric_value: float | int = int(value)
    elif isinstance(value, (int, float)):
        numeric_value = value
    else:
        numeric_value = float(str(value).strip())

    if parameter_name in INTEGER_SIMULATION_PARAMS:
        return int(numeric_value)

    return float(numeric_value)


def normalize_simulation_params(domain: SimulationDomainInput,) -> dict[str, float | int]:
    """Merge, rename, and type-convert all simulation parameter sources.

    Precedence from lowest to highest is: default values, technical parameters,
    design parameters, and general domain parameters. ``None`` values are
    ignored so they do not overwrite a usable default.
    """
    raw_params: dict[str, Any] = {}

    # General precedence:
    # defaults < technical parameters < design parameters < domain params
    raw_params.update(domain.technical_params)
    raw_params.update(domain.design_params)
    raw_params.update(domain.params)

    normalized = dict(DEFAULT_SIMULATION_PARAMS)

    for raw_name, raw_value in raw_params.items():
        if raw_value is None:
            continue

        # Debug: Print non-numeric parameters
        if not isinstance(raw_value, (int, float, bool)):
            print(f"Parameter '{raw_name}' has non-numeric value: {raw_value} (type: {type(raw_value)})")

        canonical_name = canonical_parameter_name(str(raw_name))
        normalized[canonical_name] = _convert_parameter_value(
            canonical_name,
            raw_value,
        )

    return normalized


def _build_nodes_table(domain: SimulationDomainInput,) -> pd.DataFrame:
    """Build the legacy ``nodes`` DataFrame from typed node models."""
    records = [
        {
            "noeud": node.node_id,
            "haut": node.elevation_m,
            "xdata": node.x,
            "ydata": node.y,
            "orif_ideal": node.ideal_orifice_m,
            "orif_com": node.commercial_orifice_m,
            "statut": node.status,
        }
        for node in domain.tables.nodes
    ]

    return pd.DataFrame(
        records,
        columns=[
            "noeud",
            "haut",
            "xdata",
            "ydata",
            "orif_ideal",
            "orif_com",
            "statut",
        ],
    )


def _build_pipes_table(domain: SimulationDomainInput,) -> pd.DataFrame:
    """Build the legacy ``pipes`` DataFrame and derive missing pipe lengths.

    Missing external edge IDs receive stable sequential IDs (``A1``, ``A2``,
    ...). If the first segment length is omitted, total length is used; if total
    length is omitted, it is calculated as the sum of both segments.
    """
    records: list[dict[str, Any]] = []

    for index, pipe in enumerate(domain.tables.pipes):
        first_length = (
            pipe.first_length_m
            if pipe.first_length_m is not None
            else pipe.length_m
        )

        first_length = 0.0 if first_length is None else first_length
        second_length = pipe.second_length_m

        total_length = (
            pipe.length_m
            if pipe.length_m is not None
            else first_length + second_length
        )

        records.append(
            {
                # Stable identifier used to map results back to the UI edge.
                "external_edge_id": pipe.edge_id or f"A{index + 1}",
                "pred": pipe.predecessor_id,
                "succ": pipe.successor_id,
                "lon": total_length,
                "lon1": first_length,
                "ID1": pipe.first_diameter_id,
                "lon2": second_length,
                "ID2": pipe.second_diameter_id,
                "is_open": pipe.is_hydraulically_open,
                "N": pipe.metadata_n,
            }
        )

    return pd.DataFrame(
        records,
        columns=[
            "external_edge_id",
            "pred",
            "succ",
            "lon",
            "lon1",
            "ID1",
            "lon2",
            "ID2",
            "is_open",
            "N",
        ],
    )


def _build_clients_table(domain: SimulationDomainInput) -> pd.DataFrame:
    """Build legacy client rows for the numerical core."""
    records = materialize_effective_clients(domain)

    return pd.DataFrame(
        records,
        columns=["client", "borne", "proba", "cible"],
    )


def _build_bornes_table(domain: SimulationDomainInput,) -> pd.DataFrame:
    """Build the terminal-pressure (``bornes``) DataFrame."""
    records = [
        {
            "borne": borne.terminal_id,
            "pressure": borne.required_pressure_m,
            "N": borne.metadata_n,
        }
        for borne in domain.tables.bornes
    ]

    return pd.DataFrame(
        records,
        columns=[
            "borne",
            "pressure",
            "N",
        ],
    )


def _build_diameters_table(domain: SimulationDomainInput,) -> pd.DataFrame:
    """Build the available-diameter catalogue DataFrame."""
    records = [
        {
            "ID": diameter.diameter_id,
            "Nominal": diameter.nominal_size,
            "SDR": diameter.sdr,
            "diam": diameter.inner_diameter,
            "cost": diameter.cost,
            "Pmax": diameter.maximum_pressure,
            "Type": diameter.type_code,
            "Roughness": diameter.roughness,
            "N": diameter.metadata_n,
        }
        for diameter in domain.tables.diameters
    ]

    return pd.DataFrame(
        records,
        columns=[
            "ID",
            "Nominal",
            "SDR",
            "diam",
            "cost",
            "Pmax",
            "Type",
            "Roughness",
            "N",
        ],
    )


def build_simulation_data(domain: SimulationDomainInput,) -> SimulationData:
    """Convert a typed domain payload into validated simulation data.

    The function creates all legacy tables, normalizes parameters, determines
    whether demand is uniform, and delegates final construction and validation
    to :meth:`SimulationData.from_tables`.

    Returns
    -------
    SimulationData
        An in-memory simulation object ready for hydraulic preprocessing.
    """

    demand_profile = resolve_client_demand_profile(domain)
    domain = synchronize_client_demand_params(domain,demand_profile)
    
    nodes = _build_nodes_table(domain)
    pipes = _build_pipes_table(domain)
    clients = _build_clients_table(domain)
    bornes = _build_bornes_table(domain)
    diameters = _build_diameters_table(domain)
    params = normalize_simulation_params(domain)

    # Keep the behavior of the original DSG loader:
    # heterogeneous client demand implies bounded-flow simulation.
    if (clients["proba"].nunique() != 1 or clients["cible"].nunique() != 1):
        params["uniform_dem"] = 0
        params["bounded_flow"] = 1
    else:
        params["uniform_dem"] = 1

    return SimulationData.from_tables(
        name=domain.name,
        version=domain.version,
        source_format=domain.source_format,
        nodes=nodes,
        pipes=pipes,
        clients=clients,
        bornes=bornes,
        diameters=diameters,
        params=params,
    )