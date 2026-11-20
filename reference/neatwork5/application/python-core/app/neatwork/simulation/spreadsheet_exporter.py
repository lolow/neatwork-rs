"""Spreadsheet representation of canonical simulation statistics."""
from __future__ import annotations

from io import BytesIO
from typing import Any
from openpyxl import Workbook
from openpyxl.styles import (Alignment, Font, PatternFill)
from openpyxl.utils import (get_column_letter)


class SimulationSpreadsheetExporter:
    """Convert the canonical JSON payload into an XLSX workbook.
    No hydraulic or statistical calculations are performed here.
    """
    def export_bytes(self, payload: dict[str, Any],) -> bytes:
        """Serialize one statistics payload to XLSX bytes."""
        workbook = Workbook()

        default_sheet = workbook.active
        workbook.remove(default_sheet)
        self._write_summary(workbook, payload)

        statistics = payload.get("statistics", {})

        self._write_records(workbook, "Terminal Satisfaction", statistics.get("terminalSatisfaction", []))

        self._write_records(workbook, "Source Flow", statistics.get("sourceFlow", []))

        self._write_records(workbook, "Pipe Velocity", statistics.get("pipeVelocity", []))

        self._write_records(workbook, "Terminal Pressure", statistics.get("terminalPressure", [],))

        self._write_records(workbook, "Transit Pressure", statistics.get("transitPressure", [],))

        self._write_records(workbook, "Failures", payload.get("failures", [],))

        buffer = BytesIO()

        workbook.save(buffer)

        return buffer.getvalue()

    def _write_summary(self, workbook: Workbook, payload: dict[str, Any]) -> None:
        """Write global execution metadata."""
        sheet = workbook.create_sheet("Summary")
        sheet.append(["Property", "Value"])
        network = payload.get("network", {})
        summary = payload.get("summary", {})

        rows = [
            ("Schema version", payload.get("schemaVersion")),
            ("Network", network.get("name")),
            ("Network version", network.get("version")),
            ("Scenario count", summary.get("scenarioCount")),
            ("Successful scenarios", summary.get("successfulScenarios")),
            ("Execution failures", summary.get("failedScenarios")),
            ("Success rate", summary.get("successRate")),
            ("Seed", summary.get("seed")),
            ("Duration (s)", summary.get("durationSeconds")),
            ("Mean iterations", summary.get("meanIterations")),
        ]

        for row in rows:
            sheet.append(list(row))

        self._style_sheet(sheet)
    
    def _write_records(self, workbook: Workbook, sheet_name: str, records: list[dict[str, Any]],) -> None:
        """Write JSON records without recomputing any values."""
        sheet = workbook.create_sheet(sheet_name)
        if not records:
            sheet.append(["No data"])
            return
        columns = list(records[0].keys())
        sheet.append(columns)
        for record in records:
            sheet.append([
                self._cell_value(record.get(column))
                for column in columns
            ])

        self._style_sheet(sheet)

    def _cell_value(self,value: Any) -> Any:
        """Convert nested values to readable spreadsheet text."""

        if isinstance(value(dict,list,tuple)):            
            import json
            return json.dumps(value,ensure_ascii=False)

        return value


    def _style_sheet(self, sheet,) -> None:
        """Apply lightweight readable formatting."""
        header_fill = PatternFill(fill_type="solid", fgColor="1E3A5F")

        for cell in sheet[1]:
            cell.font = Font(bold=True, color="FFFFFF")
            cell.fill = header_fill
            cell.alignment = Alignment(vertical="center")

        sheet.freeze_panes = "A2"

        for column_index, column in enumerate(sheet.columns,start=1):

            width = 10

            for cell in column:

                value = cell.value

                if value is None:
                    continue

                width = max(width, min(len(str(value)) + 2, 38))


            sheet.column_dimensions[get_column_letter(column_index)].width = width