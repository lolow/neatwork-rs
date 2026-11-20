"""Validate the tabular input used by the hydraulic simulation engine.

The module validates the five input tables (nodes, pipes, clients, bornes and
diameters), simulation parameters, and the directed topology of the network.
Validation is deliberately non-destructive: every detected problem is added to
an issue list so callers receive one comprehensive error report instead of
having to fix errors one at a time.

Functions whose names start with ``_`` are internal validation helpers.  The
public entry point is :func:`validate_simulation_data`.
"""

from collections import defaultdict, deque
from typing import Any

import numpy as np
import pandas as pd

from .exceptions import (ValidationIssue, SimulationValidationError)

# Minimum schema expected for every simulation DataFrame. Extra columns are
# accepted because preprocessing and UI layers may attach additional metadata.
REQUIRED_COLUMNS: dict[str, set[str]] = {
    "nodes": {"noeud", "haut", "xdata", "ydata", "orif_ideal", "orif_com", "statut"},

    "pipes": {"external_edge_id", "pred", "succ", "lon", "lon1", "ID1", "lon2", "ID2"},

    "clients": {"client", "borne", "proba", "cible"},

    "bornes": {"borne", "pressure"},

    "diameters": {"ID", "diam", "cost", "Pmax", "Type", "Roughness"},
}

# Parameters that behave as feature flags and must contain only 0/1 (or the
# equivalent boolean values False/True).
BINARY_PARAMETERS = {
    "bounded_flow",
    "fusion",
    "orif_com",
    "split",
    "uniform_dem",
    "use_efficient_pipes",
}

def _is_blank(value: Any) -> bool:
    """Return whether *value* represents a missing or empty table cell.

    ``None``, pandas/NumPy missing values, and whitespace-only strings are all
    treated as blank. This gives identifier validation consistent behaviour
    regardless of the original file format.
    """
    if value is None:
        return True
    
    if pd.isna(value):
        return True
    
    return str(value).strip() == ""

def _entity_id(row: pd.Series, column: str, fallback: str) -> str:
    """Return a normalized entity ID for use in validation messages.

    Parameters
    ----------
    row:
        Row containing the entity.
    column:
        Name of the identifier column.
    fallback:
        Diagnostic ID (usually ``row-<index>``) used when the cell is blank.
    """
    value = row.get(column)

    if _is_blank(value):
        return fallback

    return str(value).strip()

def _numeric_series(series: pd.Series) -> pd.Series:
    """Convert a Series to numbers, representing invalid values as ``NaN``."""
    return pd.to_numeric(series, errors="coerce")

def _is_finite(value: Any) -> bool:
    """Return ``True`` only if *value* can be converted to a finite float."""
    try:
        return bool(np.isfinite(float(value)))
    except (TypeError, ValueError):
        return False

def _add_issue(
    issues: list[ValidationIssue],
    *,
    code: str,
    message: str,
    entity_type: str | None = None,
    entity_id: str | None = None,
    field_name: str | None = None,
    details: dict[str, Any] | None = None,
) -> None:
    """Create a :class:`ValidationIssue` and append it to ``issues``.

    This centralizes issue construction so all validators report errors with a
    consistent code, entity context, field name, and optional details payload.
    """
    issues.append(
        ValidationIssue(code=code, message=message, entity_type=entity_type, entity_id=entity_id, field_name=field_name, details=details or {})
    )

def _has_required_columns(table_name: str, table: pd.DataFrame, issues: list[ValidationIssue]) -> bool:
    """Check a table schema and report every missing required column.

    Returns ``True`` when the schema is complete. Validators use this return
    value to stop safely before accessing columns that do not exist.
    """
    missing_columns = REQUIRED_COLUMNS[table_name] - set(table.columns)

    if not missing_columns:
        return True
    
    for column in sorted(missing_columns):
        _add_issue(
            issues,
            code="MISSING_TABLE_COLUMN",
            message=(
                f"Table '{table_name}' is missing "
                f"required column '{column}'."
            ),
            entity_type="table",
            entity_id=table_name,
            field_name=column
        )

    return False

def _validate_tables(
        instance: Any,
        issues: list[ValidationIssue],
) -> None:
    """Verify that every required table exists, is a DataFrame, and has its schema."""
    for table_name in REQUIRED_COLUMNS:
        table = getattr(instance, table_name, None)

        if table is None:
            _add_issue(
                issues,
                code="MISSING_TABLE",
                message=(
                    f"Simulation table '{table_name}' is missing."
                ),
                entity_type="table",
                entity_id=table_name,
            )
            continue

        if not isinstance(table, pd.DataFrame):
            _add_issue(
                issues,
                code="INVALID_TABLE_TYPE",
                message=(
                    f"Simulation table '{table_name}' "
                    "must be a pandas DataFrame."
                ),
                entity_type="table",
                entity_id=table_name,
                details={
                    "actualType": type(table).__name__,
                },
            )
            continue

        _has_required_columns(table_name, table, issues)

def _validate_nodes(nodes: pd.DataFrame, issues: list[ValidationIssue]) -> None:
    """Validate node identifiers, numeric fields, types, and required roles.

    Node status values have the following meaning: ``0`` is a source, ``1`` is
    an intermediate junction, and ``2`` is a terminal/borne. The network must
    contain at least one source and at least one terminal.
    """
    if not _has_required_columns("nodes", nodes, issues):
        return
    
    if nodes.empty:
        _add_issue(
            issues,
            code="EMPTY_NODE_TABLE",
            message="The simulation network has no nodes.",
            entity_type="table",
            entity_id="nodes"
        )
        return
    
    # Normalize IDs once so blank and duplicate checks use identical values.
    node_ids = nodes["noeud"].astype("string").fillna("").str.strip()

    for index, node_id in node_ids.items():
        if node_id:
            continue

        _add_issue(
            issues,
            code="EMPTY_NODE_ID",
            message=f"Node at row {index} has an empty ID.",
            entity_type="node",
            entity_id=f"row-{index}",
            field_name="noeud",
        )

    duplicate_mask = (node_ids.duplicated(keep=False) & node_ids.ne(""))

    for node_id in sorted(node_ids[duplicate_mask].unique()):
        _add_issue(
            issues,
            code="DUPLICATE_NODE_ID",
            message=f"Node ID '{node_id}' is duplicated.",
            entity_type="node",
            entity_id=str(node_id),
            field_name="noeud",
        )

    # Hydraulic and coordinate fields must all be finite numeric values.
    for column in ["haut", "xdata", "ydata", "orif_ideal", "orif_com"]:
        numeric_values = _numeric_series(nodes[column])

        invalid_mask = ~np.isfinite(
            numeric_values.to_numpy(dtype=float)
        )

        for index in nodes.index[invalid_mask]:
            node_id = _entity_id(nodes.loc[index], "noeud", f"row-{index}")

            _add_issue(
                issues,
                code="INVALID_NODE_NUMBER",
                message=(
                    f"Node '{node_id}' has an invalid "
                    f"numeric value for '{column}'."
                ),
                entity_type="node",
                entity_id=node_id,
                field_name=column,
            )
    
    # Validate both the numeric representation and the allowed status domain.
    statuses = _numeric_series(nodes["statut"])
    status_values = statuses.to_numpy(dtype=float)
    finite_status = np.isfinite(status_values)

    for index in nodes.index[~finite_status]:
        node_id = _entity_id(nodes.loc[index], "noeud", f"row-{index}")

        _add_issue(
            issues,
            code="INVALID_NODE_STATUS",
            message=f"Node '{node_id}' has an invalid status.",
            entity_type="node",
            entity_id=node_id,
            field_name="statut",
        )
    
    valid_status_mask = (finite_status & np.isin(status_values, [0,1, 2]))

    for index in nodes.index[finite_status & ~valid_status_mask]:
        node_id = _entity_id(nodes.loc[index], "noeud", f"row-{index}")

        _add_issue(
            issues,
            code="UNSUPPORTED_NODE_STATUS",
            message=(
                f"Node '{node_id}' has unsupported status "
                f"'{nodes.loc[index, 'statut']}'."
            ),
            entity_type="node",
            entity_id=node_id,
            field_name="statut",
            details={
                "allowedValues": [0, 1, 2],
            },
        )

    source_count = int(np.sum(statuses == 0))
    terminal_count = int(np.sum(statuses == 2))

    if source_count == 0:
        _add_issue(
            issues,
            code="MISSING_SOURCE_NODE",
            message="The network must contain at least one source node.",
            entity_type="table",
            entity_id="nodes",
        )

    if terminal_count == 0:
        _add_issue(
            issues,
            code="MISSING_TERMINAL_NODE",
            message=(
                "The network must contain at least one terminal node."
            ),
            entity_type="table",
            entity_id="nodes",
        )

def _validate_pipes(pipes: pd.DataFrame, nodes: pd.DataFrame, diameters: pd.DataFrame, issues: list[ValidationIssue]) -> None:
    """Validate pipe IDs, endpoints, lengths, and diameter references.

    Every pipe must join two different known nodes. Its total length must equal
    ``lon1 + lon2`` and each positive segment must reference a diameter present
    in the diameter catalogue.
    """
    if not _has_required_columns("pipes", pipes, issues):
        return
    
    if pipes.empty:
        _add_issue(
            issues,
            code="EMPTY_PIPE_TABLE",
            message="The simulation network has no pipes.",
            entity_type="table",
            entity_id="pipes",
        )
        return
    
    # Reference sets allow fast foreign-key checks for every pipe row.
    node_ids = (
        set(nodes["noeud"].astype(str))
        if "noeud" in nodes.columns
        else set()
        )
    
    diameter_ids = (
        set(diameters["ID"].astype(str))
        if "ID" in diameters.columns
        else set()
    )

    pipe_ids = (pipes["external_edge_id"].astype("string").fillna("").str.strip())

    duplicate_mask = (pipe_ids.duplicated(keep=False) & pipe_ids.ne(""))

    for pipe_id in sorted(pipe_ids[duplicate_mask].unique()):
        _add_issue(
            issues,
            code="DUPLICATE_PIPE_ID",
            message=f"Pipe ID '{pipe_id}' is duplicated.",
            entity_type="pipe",
            entity_id=str(pipe_id),
            field_name="external_edge_id",
        )

    for index, row in pipes.iterrows():
        pipe_id = _entity_id(row, "external_edge_id", f"row-{index}")

        if _is_blank(row["external_edge_id"]):
            _add_issue(
                issues,
                code="EMPTY_PIPE_ID",
                message=f"Pipe at row {index} has an empty ID.",
                entity_type="pipe",
                entity_id=f"row-{index}",
                field_name="external_edge_id",
            )
        
        pred = str(row["pred"]).strip()
        succ = str(row["succ"]).strip()

        if pred not in node_ids:
            _add_issue(
                issues,
                code="UNKNOWN_PIPE_NODE",
                message=(
                    f"Pipe '{pipe_id}' references "
                    f"unknown predecessor node '{pred}'."
                ),
                entity_type="pipe",
                entity_id=pipe_id,
                field_name="pred",
                details={"nodeId": pred},
            )

        if succ not in node_ids:
            _add_issue(
                issues,
                code="UNKNOWN_PIPE_NODE",
                message=(
                    f"Pipe '{pipe_id}' references "
                    f"unknown successor node '{succ}'."
                ),
                entity_type="pipe",
                entity_id=pipe_id,
                field_name="succ",
                details={"nodeId": succ},
            )

        if pred == succ:
            _add_issue(
                issues,
                code="PIPE_SELF_LOOP",
                message=(
                    f"Pipe '{pipe_id}' connects node "
                    f"'{pred}' to itself."
                ),
                entity_type="pipe",
                entity_id=pipe_id,
            )
        
        # Keep converted values so cross-field length rules run only after all
        # three cells have independently passed numeric validation.
        length_values: dict[str, float] = {}
        length_valid = True

        for column in ["lon", "lon1", "lon2"]:
            value = row[column]

            if not _is_finite(value):
                length_valid = False

                _add_issue(
                    issues,
                    code="INVALID_PIPE_LENGTH",
                    message=(
                        f"Pipe '{pipe_id}' has an invalid "
                        f"value for '{column}'."
                    ),
                    entity_type="pipe",
                    entity_id=pipe_id,
                    field_name=column,
                )
                continue

            numeric_value = float(value)
            length_values[column] = numeric_value

            if numeric_value < 0:
                length_valid = False

                _add_issue(
                    issues,
                    code="NEGATIVE_PIPE_LENGTH",
                    message=(
                        f"Pipe '{pipe_id}' has a negative "
                        f"value for '{column}'."
                    ),
                    entity_type="pipe",
                    entity_id=pipe_id,
                    field_name=column,
                )
        
        if length_valid:
            segment_length = (
                length_values["lon1"]
                + length_values["lon2"]
            )

            if segment_length <= 0:
                _add_issue(
                    issues,
                    code="ZERO_PIPE_LENGTH",
                    message=(
                        f"Pipe '{pipe_id}' must have a "
                        "strictly positive segment length."
                    ),
                    entity_type="pipe",
                    entity_id=pipe_id,
                    field_name="lon1",
                )
            
            if not np.isclose(length_values["lon"], segment_length, rtol=1e-6, atol=1e-9):
                _add_issue(
                    issues,
                    code="PIPE_LENGTH_MISMATCH",
                    message=(
                        f"Pipe '{pipe_id}' total length does not "
                        "match lon1 + lon2."
                    ),
                    entity_type="pipe",
                    entity_id=pipe_id,
                    field_name="lon",
                    details={
                        "totalLength": length_values["lon"],
                        "segmentLength": segment_length,
                    },
                )
            
            # A zero-length segment does not require a diameter assignment.
            if length_values["lon1"] > 0:
                diameter_id = row["ID1"]

                if _is_blank(diameter_id):
                    _add_issue(
                        issues,
                        code="MISSING_PIPE_DIAMETER",
                        message=(
                            f"Pipe '{pipe_id}' requires ID1 "
                            "because lon1 is positive."
                        ),
                        entity_type="pipe",
                        entity_id=pipe_id,
                        field_name="ID1",
                    )
                elif str(diameter_id) not in diameter_ids:
                    _add_issue(
                        issues,
                        code="UNKNOWN_PIPE_DIAMETER",
                        message=(
                            f"Pipe '{pipe_id}' references "
                            f"unknown diameter '{diameter_id}'."
                        ),
                        entity_type="pipe",
                        entity_id=pipe_id,
                        field_name="ID1",
                    )
            
            if length_values["lon2"] > 0:
                diameter_id = row["ID2"]

                if _is_blank(diameter_id):
                    _add_issue(
                        issues,
                        code="MISSING_PIPE_DIAMETER",
                        message=(
                            f"Pipe '{pipe_id}' requires ID2 "
                            "because lon2 is positive."
                        ),
                        entity_type="pipe",
                        entity_id=pipe_id,
                        field_name="ID2",
                    )
                elif str(diameter_id) not in diameter_ids:
                    _add_issue(
                        issues,
                        code="UNKNOWN_PIPE_DIAMETER",
                        message=(
                            f"Pipe '{pipe_id}' references "
                            f"unknown diameter '{diameter_id}'."
                        ),
                        entity_type="pipe",
                        entity_id=pipe_id,
                        field_name="ID2",
                    )

def _validate_clients(
        clients: pd.DataFrame,
        terminal_ids: set[str],
        params: dict[str, Any],
        issues: list[ValidationIssue]
) -> None:
    """Validate clients and their association with terminal nodes.

    Client IDs must be unique, ``borne`` must reference a terminal node,
    opening probability must be in ``[0, 1]``, and target flow must be positive.
    Every terminal must have at least one client. Uniform-demand simulations
    additionally require a non-zero probability of opening a tap.
    """
    if not _has_required_columns("clients", clients, issues):
        return
    
    if clients.empty:
        _add_issue(
            issues,
            code="EMPTY_CLIENT_TABLE",
            message="The network must contain at least one client.",
            entity_type="table",
            entity_id="clients",
        )
        return
    
    client_ids = (clients["client"].astype("string").fillna("").str.strip())

    duplicate_mask = (client_ids.duplicated(keep=False) & client_ids.ne(""))

    for client_id in sorted(client_ids[duplicate_mask].unique()):
        _add_issue(
            issues,
            code="DUPLICATE_CLIENT_ID",
            message=f"Client ID '{client_id}' is duplicated.",
            entity_type="client",
            entity_id=str(client_id),
            field_name="client",
        )

    # Convert numeric columns once while retaining the original DataFrame index.
    probabilities = _numeric_series(clients["proba"])
    targets = _numeric_series(clients["cible"])

    for index, row in clients.iterrows():
        client_id = _entity_id(row, "client", f"row-{index}")

        if _is_blank(row["client"]):
            _add_issue(
                issues,
                code="EMPTY_CLIENT_ID",
                message=f"Client at row {index} has an empty ID.",
                entity_type="client",
                entity_id=f"row-{index}",
                field_name="client",
            )
        
        terminal_id = str(row["borne"]).strip()

        if terminal_id not in terminal_ids:
            _add_issue(
                issues,
                code="UNKNOWN_CLIENT_TERMINAL",
                message=(
                    f"Client '{client_id}' references "
                    f"unknown terminal '{terminal_id}'."
                ),
                entity_type="client",
                entity_id=client_id,
                field_name="borne",
            )
        probability = probabilities.loc[index]

        if not np.isfinite(probability):
            _add_issue(
                issues,
                code="INVALID_CLIENT_PROBABILITY",
                message=(
                    f"Client '{client_id}' has an invalid "
                    "opening probability."
                ),
                entity_type="client",
                entity_id=client_id,
                field_name="proba",
            )
        elif probability < 0 or probability > 1:
            _add_issue(
                issues,
                code="CLIENT_PROBABILITY_OUT_OF_RANGE",
                message=(
                    f"Client '{client_id}' probability "
                    "must be between 0 and 1."
                ),
                entity_type="client",
                entity_id=client_id,
                field_name="proba",
                details={"value": float(probability)},
            )
        
        target = targets.loc[index]

        if not np.isfinite(target) or target <= 0:
           _add_issue(
                issues,
                code="INVALID_CLIENT_TARGET_FLOW",
                message=(
                    f"Client '{client_id}' target flow "
                    "must be strictly positive."
                ),
                entity_type="client",
                entity_id=client_id,
                field_name="cible",
            )

    # Detect terminals that could never receive demand because they have no client.
    client_terminal_ids = set(clients["borne"].astype(str))
    missing_terminals = terminal_ids - client_terminal_ids

    for terminal_id in sorted(missing_terminals):
        _add_issue(
            issues,
            code="TERMINAL_WITHOUT_CLIENT",
            message=(
                f"Terminal '{terminal_id}' has no associated client."
            ),
            entity_type="node",
            entity_id=terminal_id,
        )

    uniform_demand = int(params.get("uniform_dem", 1))

    if uniform_demand == 1:
        open_probability = params.get("opentaps")

        if not _is_finite(open_probability):
            return
        
        if float(open_probability) <= 0:
            _add_issue(
                issues,
                code="NO_ACTIVE_CLIENT_PROBABILITY",
                message=(
                    "Parameter 'opentaps' must be greater than zero "
                    "for uniform-demand simulations."
                ),
                entity_type="parameter",
                entity_id="opentaps",
            )
        elif not bool((probabilities > 0).any()):
            _add_issue(
                issues,
                code="NO_ACTIVE_CLIENT_PROBABILITY",
                message=(
                    "At least one client must have a positive "
                    "opening probability."
                ),
                entity_type="table",
                entity_id="clients",
                field_name="proba",
            )

def _validate_bornes(bornes: pd.DataFrame, terminal_ids: set[str], issues: list[ValidationIssue]) -> None:
    """Validate terminal metadata and required pressure values.

    The borne table must have a one-to-one ID coverage with status-2 nodes.
    Pressure values are allowed to be zero but cannot be negative or non-numeric.
    """
    if not _has_required_columns("bornes", bornes, issues):
        return
    
    if bornes.empty:
        _add_issue(
            issues,
            code="EMPTY_BORNE_TABLE",
            message="The borne table cannot be empty.",
            entity_type="table",
            entity_id="bornes",
        )
        return
    
    borne_ids = (bornes["borne"].astype("string").fillna("").str.strip())

    duplicate_mask = (borne_ids.duplicated(keep=False) & borne_ids.ne(""))

    for borne_id in sorted(borne_ids[duplicate_mask].unique()):
        _add_issue(
            issues,
            code="DUPLICATE_BORNE_ID",
            message=f"Borne ID '{borne_id}' is duplicated.",
            entity_type="borne",
            entity_id=str(borne_id),
            field_name="borne",
        )

    # Compare both directions to find missing and unexpected terminal records.
    borne_id_set = set(borne_ids[borne_ids.ne("")])

    for terminal_id in sorted(terminal_ids - borne_id_set):
        _add_issue(
            issues,
            code="MISSING_TERMINAL_BORNE",
            message=(
                f"Terminal '{terminal_id}' is missing "
                "from the borne table."
            ),
            entity_type="node",
            entity_id=terminal_id,
        )
    
    for borne_id in sorted(borne_id_set - terminal_ids):
        _add_issue(
            issues,
            code="UNKNOWN_BORNE_TERMINAL",
            message=(
                f"Borne '{borne_id}' is not a terminal node."
            ),
            entity_type="borne",
            entity_id=borne_id,
        )
    
    pressure_values = _numeric_series(bornes["pressure"])

    for index, pressure in pressure_values.items():
        if np.isfinite(pressure) and pressure >= 0:
            continue

        borne_id = _entity_id(bornes.loc[index], "borne", f"row-{index}")

        _add_issue(
            issues,
            code="INVALID_BORNE_PRESSURE",
            message=(
                f"Borne '{borne_id}' pressure must "
                "be a non-negative number."
            ),
            entity_type="borne",
            entity_id=borne_id,
            field_name="pressure",
        )

def _validate_diameters(diameters: pd.DataFrame, issues: list[ValidationIssue]) -> None:
    """Validate the pipe-diameter catalogue.

    IDs must be unique. Physical diameter must be positive; cost, maximum
    pressure, and roughness must be non-negative. ``Type`` only needs to be a
    finite number because its permitted categories are defined elsewhere.
    """
    if not _has_required_columns("diameters", diameters, issues):
        return
    
    if diameters.empty:
        _add_issue(
            issues,
            code="EMPTY_DIAMETER_TABLE",
            message="The diameter table cannot be empty.",
            entity_type="table",
            entity_id="diameters",
        )
        return
    
    diameter_ids = (diameters["ID"].astype("string").fillna("").str.strip())

    duplicate_mask = (diameter_ids.duplicated(keep=False) & diameter_ids.ne(""))

    for diameter_id in sorted(diameter_ids[duplicate_mask].unique()):
        _add_issue(
            issues,
            code="DUPLICATE_DIAMETER_ID",
            message=(f"Diameter ID '{diameter_id}' is duplicated."),
            entity_type="diameter",
            entity_id=str(diameter_id),
            field_name="ID",
        )

    # Each numeric column has its own physical/domain constraint.
    numeric_rules = {
        "diam": lambda value: value > 0,
        "cost": lambda value: value >= 0,
        "Pmax": lambda value: value >= 0,
        "Type": lambda value: True,
        "Roughness": lambda value: value >= 0,
    }

    for column, rule in numeric_rules.items():
        numeric_values = _numeric_series(diameters[column])

        for index, value in numeric_values.items():
            diameter_id = _entity_id(diameters.loc[index], "ID", f"row-{index}")

            if not np.isfinite(value):
                _add_issue(
                    issues,
                    code="INVALID_DIAMETER_VALUE",
                    message=(
                        f"Diameter '{diameter_id}' has an "
                        f"invalid value for '{column}'."
                    ),
                    entity_type="diameter",
                    entity_id=diameter_id,
                    field_name=column,
                )
                continue

            if not rule(float(value)):
                _add_issue(
                    issues,
                    code="INVALID_DIAMETER_VALUE",
                    message=(
                        f"Diameter '{diameter_id}' has an "
                        f"unsupported value for '{column}'."
                    ),
                    entity_type="diameter",
                    entity_id=diameter_id,
                    field_name=column,
                    details={"value": float(value)},
                )

def _validate_parameters(params: dict[str, Any], nodes: pd.DataFrame, issues: list[ValidationIssue]) -> None:
    """Validate required simulation parameters and mode-dependent rules.

    Checks cover presence, binary flags, numeric domains, scenario count, and
    supported combinations of ``bounded_flow`` and ``uniform_dem``. In
    unbounded-flow mode, positive target flow and terminal orifice values are
    also mandatory.
    """
    # These values are needed by preprocessing or the hydraulic solver in every
    # supported simulation mode.
    required_parameters = {
        "bescal",
        "bounded_flow",
        "faucetcoef",
        "lambda",
        "nbsim",
        "nu",
        "opentaps",
        "orif_com",
        "orifcoef",
        "pressure",
        "servicequal",
        "targetflow",
        "uniform_dem",
        "vmax",
        "vmax_secu",
    }

    # Boolean configuration values may be written as integers or bools.
    for parameter_name in sorted(required_parameters):
        if parameter_name in params:
            continue

        _add_issue(
            issues,
            code="MISSING_SIMULATION_PARAMETER",
            message=(
                f"Required simulation parameter "
                f"'{parameter_name}' is missing."
            ),
            entity_type="parameter",
            entity_id=parameter_name,
        )
    
    for parameter_name in BINARY_PARAMETERS:
        if parameter_name not in params:
            continue

        value = params[parameter_name]

        if value not in (0, 1, False, True):
            _add_issue(
                issues,
                code="INVALID_BINARY_PARAMETER",
                message=(
                    f"Parameter '{parameter_name}' "
                    "must be 0 or 1."
                ),
                entity_type="parameter",
                entity_id=parameter_name,
                details={"value": value},
            )
    
    nbsim = params.get("nbsim")

    if not _is_finite(nbsim):
        _add_issue(
            issues,
            code="INVALID_SCENARIO_COUNT",
            message="Parameter 'nbsim' must be a positive integer.",
            entity_type="parameter",
            entity_id="nbsim",
        )
    else:
        scenario_count = float(nbsim)

        if(scenario_count <= 0 or not scenario_count.is_integer()):
            _add_issue(
                issues,
                code="INVALID_SCENARIO_COUNT",
                message=(
                    "Parameter 'nbsim' must be a positive integer."
                ),
                entity_type="parameter",
                entity_id="nbsim",
                details={"value": nbsim},
            )

    # Coefficients and physical limits used as divisors/exponents must be > 0.
    positive_parameters = {
        "bescal",
        "lambda",
        "nu",
        "orifcoef",
        "vmax",
        "vmax_secu",
    }

    for parameter_name in positive_parameters:
        value = params.get(parameter_name)

        if not _is_finite(value) or float(value) <= 0:
            _add_issue(
                issues,
                code="INVALID_POSITIVE_PARAMETER",
                message=(
                    f"Parameter '{parameter_name}' "
                    "must be strictly positive."
                ),
                entity_type="parameter",
                entity_id=parameter_name,
                details={"value": value},
            )

    # These quantities may legitimately be zero, but never negative.
    non_negative_parameters = {
        "faucetcoef",
        "pressure",
    }

    for parameter_name in non_negative_parameters:
        value = params.get(parameter_name)

        if not _is_finite(value) or float(value) < 0:
            _add_issue(
                issues,
                code="INVALID_NON_NEGATIVE_PARAMETER",
                message=(
                    f"Parameter '{parameter_name}' "
                    "must be non-negative."
                ),
                entity_type="parameter",
                entity_id=parameter_name,
                details={"value": value},
            )
    
    for parameter_name in ["opentaps", "servicequal"]:
        value = params.get(parameter_name)

        if (
            not _is_finite(value)
            or float(value) < 0
            or float(value) > 1
        ):
            _add_issue(
                issues,
                code="PARAMETER_OUT_OF_RANGE",
                message=(
                    f"Parameter '{parameter_name}' "
                    "must be between 0 and 1."
                ),
                entity_type="parameter",
                entity_id=parameter_name,
                details={"value": value},
            )
    
    # Validate combinations whose requirements depend on the selected solver mode.
    bounded_flow = int(params.get("bounded_flow", 0))
    uniform_demand = int(params.get("uniform_dem", 1))

    if bounded_flow == 0 and uniform_demand == 0:
        _add_issue(
            issues,
            code="UNSUPPORTED_SIMULATION_CASE",
            message=(
                "The combination uniform_dem=0 and "
                "bounded_flow=0 is not supported."
            ),
            entity_type="parameter",
            entity_id="uniform_dem",
            details={
                "uniform_dem": uniform_demand,
                "bounded_flow": bounded_flow,
            },
        )
    
    if bounded_flow == 0:
        target_flow = params.get("targetflow")

        if not _is_finite(target_flow) or float(target_flow) <= 0:
            _add_issue(
                issues,
                code="INVALID_TARGET_FLOW_PARAMETER",
                message=(
                    "Parameter 'targetflow' must be strictly "
                    "positive for unbounded-flow simulation."
                ),
                entity_type="parameter",
                entity_id="targetflow",
                details={"value": target_flow},
            )
        
        if {"statut", "noeud", "orif_ideal", "orif_com"}.issubset(nodes.columns):
            terminal_nodes = nodes[nodes["statut"] == 2]
            orifice_parameter = int(params.get("orif_com", 1))
            required_column = ("orif_com" if orifice_parameter == 1 else "orif_ideal")

            for _, row in terminal_nodes.iterrows():
                node_id = str(row["noeud"])
                value = row[required_column]

                if _is_finite(value) and float(value) > 0:
                    continue

                _add_issue(
                    issues,
                    code="INVALID_TERMINAL_ORIFICE",
                    message=(
                        f"Terminal '{node_id}' must have a positive "
                        f"'{required_column}' value for the selected "
                        "orifice model (unbounded-flow simulation)."
                    ),
                    entity_type="node",
                    entity_id=node_id,
                    field_name=required_column,
                    details={
                        "orificeMode": "COMMERCIAL" if orifice_parameter == 1 else "IDEAL"
                    }
                )

def _validate_topology(nodes: pd.DataFrame, pipes: pd.DataFrame, issues: list[ValidationIssue]) -> None:
    """Validate structural rules and source reachability of the directed graph.

    Sources may only have outgoing pipes. Terminals must have exactly one
    incoming pipe and no outgoing pipe. A breadth-first search then verifies
    that every node is reachable from at least one source.
    """
    required_node_columns = {"noeud", "statut"}
    required_pipe_columns = {"external_edge_id", "pred", "succ"}

    if not required_node_columns.issubset(nodes.columns):
        return

    if not required_pipe_columns.issubset(pipes.columns):
        return

    node_ids = set(nodes["noeud"].astype(str))

    source_ids = set(
        nodes.loc[nodes["statut"] == 0, "noeud"].astype(str)
    )

    terminal_ids = set(
        nodes.loc[nodes["statut"] == 2, "noeud"].astype(str)
    )

    # Build degree counters and a directed adjacency list in one pass.
    incoming_count: dict[str, int] = defaultdict(int)
    outgoing_count: dict[str, int] = defaultdict(int)
    adjacency: dict[str, list[str]] = defaultdict(list)

    for _, row in pipes.iterrows():
        pipe_id = str(row["external_edge_id"])
        pred = str(row["pred"])
        succ = str(row["succ"])

        if pred in source_ids and succ in terminal_ids:
            _add_issue(
                issues,
                code="DIRECT_SOURCE_TO_TERMINAL_PIPE",
                message=(
                    f"Pipe '{pipe_id}' connects a source directly "
                    "to a terminal. Add at least one transit node."
                ),
                entity_type="pipe",
                entity_id=pipe_id,
            )

        if pred not in node_ids or succ not in node_ids:
            continue

        outgoing_count[pred] += 1
        incoming_count[succ] += 1
        adjacency[pred].append(succ)

    for source_id in sorted(source_ids):
        if incoming_count[source_id] > 0:
            _add_issue(
                issues,
                code="SOURCE_HAS_INCOMING_PIPE",
                message=(
                    f"Source node '{source_id}' cannot have "
                    "an incoming pipe."
                ),
                entity_type="node",
                entity_id=source_id,
            )

        if outgoing_count[source_id] == 0:
            _add_issue(
                issues,
                code="SOURCE_WITHOUT_OUTGOING_PIPE",
                message=(
                    f"Source node '{source_id}' has no "
                    "outgoing pipe."
                ),
                entity_type="node",
                entity_id=source_id,
            )
    
    for terminal_id in sorted(terminal_ids):
        if outgoing_count[terminal_id] > 0:
            _add_issue(
                issues,
                code="TERMINAL_HAS_OUTGOING_PIPE",
                message=(
                    f"Terminal node '{terminal_id}' cannot "
                    "have an outgoing pipe."
                ),
                entity_type="node",
                entity_id=terminal_id,
            )

        if incoming_count[terminal_id] != 1:
            _add_issue(
                issues,
                code="INVALID_TERMINAL_IN_DEGREE",
                message=(
                    f"Terminal node '{terminal_id}' must have "
                    "exactly one incoming pipe."
                ),
                entity_type="node",
                entity_id=terminal_id,
                details={
                    "incomingPipeCount": incoming_count[terminal_id],
                },
            )

    if not source_ids:
        return
    
    # Breadth-first traversal supports networks with one or multiple sources.
    reached = set(source_ids)
    queue = deque(source_ids)

    while queue:
        current_node = queue.popleft()

        for successor in adjacency[current_node]:
            if successor in reached:
                continue

            reached.add(successor)
            queue.append(successor)
    
    unreachable_nodes = node_ids - reached

    for node_id in sorted(unreachable_nodes):
        _add_issue(
            issues,
            code="NODE_UNREACHABLE_FROM_SOURCE",
            message=(
                f"Node '{node_id}' is not reachable "
                "from any source node."
            ),
            entity_type="node",
            entity_id=node_id,
        )

def validate_simulation_data(instance: Any) -> None:
    """
    Validate all raw simulation data before hydraulic preprocessing.

    Parameters
    ----------
    instance:
        Simulation object exposing ``nodes``, ``pipes``, ``clients``, ``bornes``,
        ``diameters``, and ``param`` attributes.

    Notes
    -----
    Independent validators continue after finding an error. This allows the
    final exception to contain every issue that can be safely detected.

    Raises
    ------
    SimulationValidationError
        If one or more validation issues are found.
    """
    # All validators share one accumulator to produce a comprehensive report.
    issues: list[ValidationIssue] = []

    _validate_tables(instance, issues)

    # Safe defaults keep diagnostics useful even for partially constructed objects.
    nodes = getattr(instance, "nodes", pd.DataFrame())
    pipes = getattr(instance, "pipes", pd.DataFrame())
    clients = getattr(instance, "clients", pd.DataFrame())
    bornes = getattr(instance, "bornes", pd.DataFrame())
    diameters = getattr(instance, "diameters", pd.DataFrame())
    params = getattr(instance, "param", {})

    if isinstance(nodes, pd.DataFrame):
        _validate_nodes(nodes, issues)

    if (
        isinstance(pipes, pd.DataFrame)
        and isinstance(nodes, pd.DataFrame)
        and isinstance(diameters, pd.DataFrame)
        ):
        _validate_pipes(pipes, nodes, diameters, issues)

    # Terminal IDs are shared by client and borne foreign-key validation.
    terminal_ids: set[str] = set()

    if (isinstance(nodes, pd.DataFrame) and {"noeud", "statut"}.issubset(nodes.columns)):
        terminal_ids = set(nodes.loc[nodes["statut"] == 2, "noeud"].astype(str))

    if isinstance(clients, pd.DataFrame):
        _validate_clients(clients, terminal_ids, params, issues)

    if isinstance(bornes, pd.DataFrame):
        _validate_bornes(bornes, terminal_ids, issues)

    if isinstance(diameters, pd.DataFrame):
        _validate_diameters(diameters, issues)

    if isinstance(params, dict):
        _validate_parameters(params, nodes, issues)

    else:
        _add_issue(
            issues,
            code="INVALID_PARAMETER_CONTAINER",
            message="Simulation parameters must be a dictionary.",
            entity_type="parameter",
        )
    
    if(isinstance(nodes, pd.DataFrame) and isinstance(pipes, pd.DataFrame)):
        _validate_topology(nodes, pipes, issues)
    
    # Raise once, after all safe validation stages have contributed their issues.
    if issues:
        raise SimulationValidationError(issues)
