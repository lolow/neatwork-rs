"""Canonical hydraulic unit conversions used by NeatWork."""

from __future__ import annotations

LITRES_PER_CUBIC_METRE = 1000.0

def lps_to_m3s(value):
    """Convert litres per second to cubic metres per second."""
    return value / LITRES_PER_CUBIC_METRE

def m3s_to_lps(value):
    """Convert cubic metres per second to litres per second."""
    return value * LITRES_PER_CUBIC_METRE