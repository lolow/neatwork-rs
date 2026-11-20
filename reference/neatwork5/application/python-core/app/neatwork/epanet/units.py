from __future__ import annotations

from dataclasses import dataclass
from app.neatwork.epanet.models import (EpanetSemanticModel)

FLOW_TO_LPS = {
    "CFS": 28.316846592,
    "GPM": 0.0630901964,
    "MGD": 43.81263638888889,
    "IMGD": 52.61678240740741,
    "AFD": 14.2764101568,
    "LPS": 1.0,
    "LPM": 1.0 / 60.0,
    "MLD": 11.574074074074074,
    "CMH": 1.0 / 3.6,
    "CMD": 1.0 / 86.4,
}

US_FLOW_UNITS = {
    "CFS",
    "GPM",
    "MGD",
    "IMGD",
    "AFD",
}

@dataclass(frozen=True, slots=True)
class EpanetUnitContext:
    flow_units: str
    headloss: str

    @classmethod
    def from_model(cls, model: EpanetSemanticModel) -> "EpanetUnitContext":
        flow_units = (model.options.get("UNITS", "GPM").strip().upper())

        if flow_units not in FLOW_TO_LPS:
            raise ValueError("Unsupported EPANET flow units: " f"{flow_units}")

        headloss = (model.options.get("HEADLOSS", "H-W").strip().upper())
        return cls(flow_units=flow_units, headloss=headloss)

    @property
    def uses_us_units(self) -> bool:
        return self.flow_units in US_FLOW_UNITS

    def length_to_m(self, value: float) -> float:
        return value * 0.3048 if self.uses_us_units else value

    def head_to_m(self, value: float) -> float:
        return self.length_to_m(value)

    def pipe_diameter_to_m(self, value: float) -> float:
        # US pipe diameters are inches. Metric pipe diameters are millimetres.
        if self.uses_us_units:
            return value * 0.0254
        return value / 1000.0

    def flow_to_lps(self, value: float) -> float:
        return value * FLOW_TO_LPS[self.flow_units]

    def pressure_to_head_m(self, value: float) -> float:
        if not self.uses_us_units:
            # Metric EPANET pressure is expressed as metres of water head.
            return value

        # 1 psi ≈ 0.703249615 metres of water head.
        return value * 0.703249614902
    
    def darcy_roughness_to_m(self, value: float) -> float | None:

        if self.headloss != "D-W":
            return None

        if self.uses_us_units:
            # EPANET D-W roughness in US projects is expressed in millifeet.
            return value * 0.0003048

        # Metric D-W roughness is millimetres.
        return value / 1000.0