"""Strict JSON export of processed hydraulic simulation statistics."""

from __future__ import annotations

import json
from enum import Enum
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd

from .statistics import SimulationStatistics


def _json_compatible(value: Any) -> Any:
    """Recursively convert scientific Python values to JSON-safe values.

    NumPy scalar values are converted to native Python scalars. Arrays,
    mappings and sequences are converted recursively. Non-finite floating
    values and pandas missing values become ``None``.

    Args:
        value: Arbitrary value produced by the statistics layer.

    Returns:
        A value accepted by the standard-library JSON encoder.
    """

    if value is None:
        return None

    if isinstance(value, Enum):
        return _json_compatible(value.value)

    if isinstance(value, np.ndarray):
        return [
            _json_compatible(item)
            for item in value.tolist()
        ]

    if isinstance(value, np.generic):
        return _json_compatible(value.item())

    if isinstance(value, dict):
        return {
            str(key): _json_compatible(item)
            for key, item in value.items()
        }

    if isinstance(value, (list, tuple)):
        return [_json_compatible(item) for item in value]

    if isinstance(value, float):
        if not np.isfinite(value):
            return None

        return value

    try:
        if pd.isna(value):
            return None
    except (TypeError, ValueError):
        pass

    return value


def _dataframe_records(frame: pd.DataFrame, *, index_name: str | None = None) -> list[dict[str, Any]]:
    """Convert a DataFrame into JSON-safe record objects.

    Args:
        frame: Statistical table to convert.
        index_name: Optional field name used to expose the DataFrame index.

    Returns:
        JSON-compatible records in the original row ordering.
    """

    converted = frame.copy()

    if index_name is not None:
        converted.index.name = index_name
        converted = converted.reset_index()

    records = converted.to_dict(orient="records")

    return _json_compatible(records)


class SimulationJsonExporter:
    """Serialize simulation statistics to a stable strict-JSON contract.

    Attributes:
        schema_version: Version of the exported JSON document contract.
    """

    schema_version = "1.1"

    def build_payload(self, statistics: SimulationStatistics, *, network_name: str, network_version: str = "") -> dict[str, Any]:
        """Build an API-friendly JSON-compatible statistics payload.

        Args:
            statistics: Processed hydraulic simulation statistics.
            network_name: Human-readable simulated network name.
            network_version: Optional network data version.

        Returns:
            Strictly JSON-compatible nested dictionary.
        """

        summary_records = _dataframe_records(statistics.run_summary)

        if len(summary_records) != 1:
            raise ValueError("Simulation run summary must contain exactly one row.")

        successful_mask = np.asarray(statistics.successful_scenario_mask, dtype=bool)

        successful_indices = np.flatnonzero(successful_mask).tolist()

        excluded_indices = np.flatnonzero(~successful_mask).tolist()

        payload = {
            "schemaVersion": self.schema_version,
            "network": {
                "name": str(network_name),
                "version": str(network_version),
            },
            "demand": {"terminals": []},
            "units": {
                "sourceFlow": "L/s",
                "pipeVelocity": "m/s",
                "terminalPressure": "m",
                "transitPressure": "m",
                "duration": "s",
                "satisfaction": "ratio",
            },
            "summary": summary_records[0],
            "scenarioSelection": {
                "successfulMask": successful_mask.tolist(),
                "includedScenarioIndices": successful_indices,
                "excludedScenarioIndices": excluded_indices,
            },
            "statistics": {
                "terminalSatisfaction": _dataframe_records(statistics.terminal_satisfaction, index_name="terminalId"),
                "sourceFlow": _dataframe_records(statistics.source_flow, index_name="sourceId"),
                "pipeVelocity": _dataframe_records(statistics.pipe_velocity),
                "terminalPressure": _dataframe_records(statistics.terminal_pressure, index_name="terminalId"),
                "transitPressure": _dataframe_records(statistics.transit_pressure, index_name="transitNodeId"),
            },
            "failures": _dataframe_records(statistics.failures),
        }

        return _json_compatible(payload)

    def export_text(self,statistics: SimulationStatistics,*,network_name: str,network_version: str = "",indent: int | None = 2) -> str:
        """Serialize statistics to a UTF-8 JSON string.

        Args:
            statistics: Processed simulation statistics.
            network_name: Human-readable simulated network name.
            network_version: Optional network data version.
            indent: JSON indentation level, or ``None`` for compact output.

        Returns:
            Standards-compliant JSON text.
        """

        payload = self.build_payload(statistics,network_name=network_name,network_version=network_version)

        return json.dumps(payload,ensure_ascii=False,allow_nan=False,indent=indent,sort_keys=False)

    def export_bytes(self,statistics: SimulationStatistics,*,network_name: str,network_version: str = "",indent: int | None = None) -> bytes:
        """Serialize statistics to UTF-8 JSON bytes for an HTTP response."""

        return self.export_text(statistics,network_name=network_name,network_version=network_version,indent=indent).encode("utf-8")

    def export(self,statistics: SimulationStatistics,destination: str | Path,*,network_name: str,network_version: str = "",indent: int | None = 2) -> Path:
        """Write simulation statistics to a UTF-8 JSON file.
        Args:
            statistics: Processed simulation statistics.
            destination: Output JSON file path.
            network_name: Human-readable simulated network name.
            network_version: Optional network data version.
            indent: JSON indentation level, or ``None`` for compact output.

        Returns:
            Resolved path of the generated JSON file.
        """

        path = Path(destination).expanduser()

        path.parent.mkdir(parents=True,exist_ok=True)

        content = self.export_text(statistics,network_name=network_name,network_version=network_version,indent=indent)

        path.write_text(content,encoding="utf-8")

        return path.resolve()

    def export_payload_text(self,payload:dict[str, Any],*,indent:int | None = 2) -> str:
        """Serialize an already composed statistics payload."""
        compatible = _json_compatible(payload)
        return json.dumps(compatible,ensure_ascii=False,
                        allow_nan=False,indent=indent,sort_keys=False)

    def export_payload_bytes(self,payload:dict[str, Any],*,indent:int | None = None,) -> bytes:
        """Serialize one composed payload as UTF-8 bytes."""
        return (
            self.export_payload_text(payload,indent=indent)
        ).encode("utf-8")