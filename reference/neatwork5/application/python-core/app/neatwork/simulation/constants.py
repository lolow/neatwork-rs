"""Central configuration constants for the hydraulic simulation core.

Keeping defaults, integer-valued parameter names, and accepted aliases in one
module ensures that file parsing and API adaptation use the same vocabulary.
The mappings are annotated with :class:`typing.Final` to indicate that callers
should treat them as configuration constants and must not mutate them.
"""

from typing import Final

"""
bounded_flow = 1
    maximum flow = total scenario demand

bounded_flow = 0
    maximum flow = 20 × total scenario demand
"""
UNBOUNDED_FLOW_MULTIPLIER: Final[float] = 20.0

# Complete baseline configuration used before file- or API-provided overrides.
# Parameter names intentionally follow the numerical core's legacy vocabulary.
DEFAULT_SIMULATION_PARAMS: Final[dict[str, float | int]] = {
    "bescal": 8.9396e-04,          # Head loss coefficient scaling factor
    "bounded_flow": 0,              # Flow constraint flag (0=unbounded, 1=bounded)
    "faucetcoef": 1.0,              # Faucet opening coefficient
    "fusion": 0,                    # Pipe fusion flag
    "lambda": 1.7810,               # Hazen-Williams coefficient
    "limitbudget": float('inf'),    # Maximum budget constraint
    "nbsim": 100,                   # Maximum number of simulation iterations
    "nu": 4.7810,                   # Exponent for head loss calculation
    "opentaps": 1,                  # Open taps flag
    "orif_com": 1,                  # Orifice coefficient mode
    "orifcoef": 1.0,                # Orifice coefficient value
    "pipelength": 6.0,              # Default pipe length
    "pressure": 2,                  # Default pressure requirement
    "secu": 1.0,                    # Safety factor
    "servicequal": 0.65,            # Service quality requirement
    "seuil": 0.0,                   # Threshold value
    "split": 0,                     # Flow splitting mode
    "targetflow": 0.0,              # Target flow rate
    "uniform_dem": 1,               # Uniform demand flag
    "use_efficient_pipes": 0,       # Efficient pipe selection flag
    "vmax": 1.5,                    # Maximum velocity (m/s)
    "vmax_secu": 0.95,              # Maximum velocity safety factor
    "watertemp": 20.0,              # Water temperature in Celsius
}

# Parameters whose normalized values must be integers rather than floats.
# This includes boolean feature flags as well as the scenario count.
INTEGER_SIMULATION_PARAMS: Final[set[str]] = {
    "bounded_flow",
    "fusion",
    "nbsim",
    "orif_com",
    "split",
    "uniform_dem",
    "use_efficient_pipes",
}

# Map accepted legacy/API spellings to the canonical keys above. Input names are
# converted to snake_case before this mapping is consulted.
PARAMETER_ALIASES: Final[dict[str, str]] = {
    # Existing DSG parser names
    "bounded_flow": "bounded_flow",
    "serviceequal": "servicequal",

    # Camel-case API names after snake-case conversion
    "bounded_flow_enabled": "bounded_flow",
    "open_taps": "opentaps",
    "orifice_commercial": "orif_com",
    "orifice_coefficient": "orifcoef",
    "faucet_coefficient": "faucetcoef",
    "pipe_length": "pipelength",
    "service_quality": "servicequal",
    "target_flow": "targetflow",
    "uniform_demand": "uniform_dem",
    "maximum_velocity": "vmax",
    "maximum_safe_velocity": "vmax_secu",
    "water_temperature": "watertemp",
}