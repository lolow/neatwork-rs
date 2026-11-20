"""Models used by simulation export adapters."""
from typing import Any
from pydantic import (BaseModel, Field)


class SimulationSpreadsheetRequest(BaseModel):
    """Statistics to convert into XLSX."""

    statistics: dict[str, Any] = Field(default_factory=dict)