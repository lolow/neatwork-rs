from __future__ import annotations

from dataclasses import dataclass
from .models import HydraulicReferenceState

@dataclass(frozen=True,slots=True)
class NumericDifference:
    actual: float
    reference: float
    absolute_error: float
    relative_error: float


def numeric_difference(actual: float, reference: float) -> NumericDifference:
    absolute_error = abs(actual - reference)
    scale = max(abs(reference), 1e-12)
    return NumericDifference(
        actual=actual,
        reference=reference,
        absolute_error=absolute_error,
        relative_error=absolute_error / scale,
    )


FLOW_REL_TOLERANCE = 1e-5
FLOW_ABS_TOLERANCE = 1e-8
PRESSURE_REL_TOLERANCE = 1e-5
PRESSURE_ABS_TOLERANCE = 1e-4
VELOCITY_REL_TOLERANCE = 1e-5
VELOCITY_ABS_TOLERANCE = 1e-6
SATISFACTION_REL_TOLERANCE = 1e-5
SATISFACTION_ABS_TOLERANCE = 1e-6