"""Pydantic input models for the hydraulic simulation API boundary.

Each model accepts both the modern API field names and aliases used by legacy
DSG tables. This allows callers to submit camelCase JSON, Python snake_case, or
historical French column names while exposing one consistent internal model.
"""

from typing import Any

from pydantic import (
    AliasChoices,
    BaseModel,
    ConfigDict,
    Field,
    field_validator,
)

def _normalize_required_identifier(value: Any, field_name: str) -> str:
    if value is None:
        raise ValueError(f"{field_name} cannot be None.")

    normalized = str(value).strip()

    if not normalized:
        raise ValueError(f"{field_name} cannot be empty.")

    return normalized

class SimulationInputModel(BaseModel):
    """Provide shared parsing behavior for every simulation input model.

    Unknown fields are preserved for forward compatibility, aliases may be used
    alongside Python field names, and surrounding whitespace is removed from
    string values automatically.
    """
    model_config = ConfigDict(
        extra="allow",
        populate_by_name=True,
        str_strip_whitespace=True,
    )

class SimulationNodeInput(SimulationInputModel):
    """Represent one hydraulic-network node.

    Attributes
    ----------
    node_id:
        Unique node identifier.
    elevation_m:
        Node elevation or hydraulic head datum in metres.
    x, y:
        Two-dimensional coordinates used for layout and visualization.
    ideal_orifice_m, commercial_orifice_m:
        Ideal and selected commercial orifice diameters in metres.
    status:
        Node role: ``0`` source, ``1`` junction, or ``2`` terminal.
    """
    node_id: str = Field(
       validation_alias=AliasChoices("node_id", "nodeId", "noeud", "id"), 
    )

    elevation_m: float = Field(
        validation_alias=AliasChoices("elevation_m", "elevationM", "haut"),
    )

    x: float = Field(
        default=0.0,
        validation_alias=AliasChoices("x", "xdata"),
    )

    y: float = Field(
        default=0.0,
        validation_alias=AliasChoices("y", "ydata"),
    )

    ideal_orifice_m: float = Field(
        default=0.0,
        validation_alias=AliasChoices("ideal_orifice_m", "idealOrificeM", "orif_ideal"),
    )

    commercial_orifice_m: float = Field(
        default=0.0,
        validation_alias=AliasChoices("commercial_orifice_m", "commercialOrificeM", "orif_com"),
    )

    status: int = Field(
        validation_alias=AliasChoices("status", "statut"),
    )

    @field_validator("node_id", mode="before")
    @classmethod
    def normalize_node_reference(cls, value: Any) -> str:
        return _normalize_required_identifier(value, "Node identifier")

class SimulationPipeInput(SimulationInputModel):
    """Represent a directed pipe, optionally composed of two diameter segments.

    Attributes
    ----------
    edge_id:
        Optional external ID used to map numerical results back to the UI edge.
    predecessor_id, successor_id:
        IDs of the upstream and downstream nodes.
    length_m:
        Optional total pipe length in metres.
    first_length_m, second_length_m:
        Length of each diameter segment in metres.
    first_diameter_id, second_diameter_id:
        References into the diameter catalogue for the two segments.
    metadata_n:
        Optional legacy ``N`` value preserved during adaptation.
    """
    edge_id: str | None = Field(
        default=None,
        validation_alias=AliasChoices("edge_id", "edgeId", "external_edge_id", "externalEdgeId", "id"),
    )

    predecessor_id: str = Field(
        validation_alias=AliasChoices("predecessor_id", "predecessorId", "pred", "sourceId"),
    )

    successor_id: str = Field(
        validation_alias=AliasChoices("successor_id", "successorId", "succ", "targetId"),
    )

    length_m: float | None = Field(
        default=None,
        validation_alias=AliasChoices("length_m", "lengthM", "length", "lon"),
    )

    first_length_m: float | None = Field(
        default=None,
        validation_alias=AliasChoices("first_length_m", "firstLengthM", "lon1"),
    )

    first_diameter_id: str | None = Field(
        default=None,
        validation_alias=AliasChoices("first_diameter_id", "firstDiameterId", "ID1"),
    )

    second_length_m: float = Field(
        default=0.0,
        validation_alias=AliasChoices("second_length_m", "secondLengthM", "lon2"),
    )

    second_diameter_id: str | None = Field(
        default=None,
        validation_alias=AliasChoices("second_diameter_id", "secondDiameterId", "ID2"),
    )

    metadata_n: Any | None = Field(
        default=None,
        validation_alias=AliasChoices("metadata_n", "metadataN", "N"),
    )

    enabled: bool = Field(
        default=True,
        validation_alias=AliasChoices("enabled", "is_open", "isOpen"),
    )

    operational_status: str | None = Field(
        default=None,
        validation_alias=AliasChoices("operational_status", "operationalStatus", "status", "edgeStatus"),
    )

    @property
    def is_hydraulically_open(self) -> bool:
        """
        Return whether the pipe should participate in hydraulic flow.

        The explicit enabled flag has priority. Status values such as
        CLOSED, DISABLED and OFF also close the pipe.
        """

        if not self.enabled:
            return False

        if self.operational_status is None:
            return True

        normalized_status = (
            self.operational_status.strip().upper().replace("-", "_").replace(" ", "_")
        )

        closed_statuses = {"CLOSED", "DISABLED", "INACTIVE", "OFF", "OUT_OF_SERVICE"}

        return normalized_status not in closed_statuses

    @field_validator("first_diameter_id", "second_diameter_id", mode="before")
    @classmethod
    def normalize_diameter_id(cls, value: Any) -> str | None:
        if value in (None, "", 0, "0"):
            return None

        return str(value)
    
    @field_validator("predecessor_id", "successor_id", mode="before")
    @classmethod
    def normalize_pipe_identifier(cls, value: Any) -> str | None:
        return _normalize_required_identifier(value, "Pipe endpoint identifier")
    
    
    @field_validator("edge_id", mode="before")
    @classmethod
    def normalize_edge_id(cls, value: Any) -> str | None:
        if value in (None, ""):
            return None

        return str(value).strip() or None

class SimulationClientInput(SimulationInputModel):
    """Represent one consumer connected to a terminal node.

    Attributes
    ----------
    client_id:
        Unique consumer identifier.
    terminal_id:
        ID of the terminal/borne serving this consumer.
    open_probability:
        Probability that the consumer tap is open during a scenario.
    target_flow_lps:
        Desired discharge in litres per second.
    """
    client_id: str = Field(
        validation_alias=AliasChoices("client_id", "clientId", "client"),
    )

    terminal_id: str = Field(
        validation_alias=AliasChoices("terminal_id", "terminalId", "borne"),
    )

    behavior_id: str | None = Field(
        default=None,
        validation_alias=AliasChoices("behavior_id", "behaviorId"),
    )

    # Legacy compatibility only.
    # New canonical domains should use behaviorId.
    open_probability: float | None = Field(
        default=None,
        validation_alias=AliasChoices("open_probability", "openProbability", "proba"),
    )

    # Legacy compatibility only.
    # New canonical domains should use behaviorId.
    target_flow_lps: float | None = Field(
        default=None,
        validation_alias=AliasChoices("target_flow_lps", "targetFlowLps", "cible"),
    )

    @field_validator("client_id", "terminal_id", mode="before")
    @classmethod
    def normalize_client_identifier(cls, value: Any) -> str:
        return _normalize_required_identifier(value, "Client identifier")


class SimulationBorneInput(SimulationInputModel):
    """Define the pressure requirement of one terminal node.

    ``terminal_id`` references a status-2 node, ``required_pressure_m`` stores
    the minimum required pressure head in metres, and ``metadata_n`` preserves
    the optional legacy ``N`` value.
    """
    terminal_id: str = Field(
        validation_alias=AliasChoices("terminal_id", "terminalId", "borne"),
    )

    required_pressure_m: float = Field(
        validation_alias=AliasChoices("required_pressure_m", "requiredPressureM", "pressure"),
    )

    metadata_n: Any | None = Field(
        default=None,
        validation_alias=AliasChoices("metadata_n", "metadataN", "N"),
    )

    @field_validator("terminal_id", mode="before")
    @classmethod
    def normalize_terminal_id(cls, value: Any) -> str:
        return _normalize_required_identifier(value, "Borne endpoint identifier")

class SimulationDiameterInput(SimulationInputModel):
    """Describe one commercially available pipe diameter.

    Attributes
    ----------
    diameter_id:
        Unique catalogue identifier referenced by pipes.
    nominal_size:
        Human-readable nominal size or commercial designation.
    sdr:
        Standard dimension ratio.
    inner_diameter:
        Hydraulic inner diameter in metres.
    cost:
        Unit cost used by design/optimization routines.
    maximum_pressure:
        Maximum supported pressure.
    type_code:
        Catalogue-specific material or diameter category code.
    roughness:
        Pipe roughness used in hydraulic calculations.
    metadata_n:
        Optional legacy ``N`` value.
    """
    diameter_id: str = Field(
        validation_alias=AliasChoices("diameter_id", "diameterId", "ID", "id"),
    )

    nominal_size: str = Field(
        validation_alias=AliasChoices("nominal_size", "nominalSize", "Nominal", "size"),
    )

    sdr: float = Field(
        default=0.0,
        validation_alias=AliasChoices("sdr", "SDR"),
    )

    inner_diameter: float = Field(
        validation_alias=AliasChoices("inner_diameter", "innerDiameter", "diam"),
    )

    cost: float = 0.0

    maximum_pressure: float = Field(
        default=0.0,
        validation_alias=AliasChoices("maximum_pressure", "maximumPressure", "Pmax", "pressure"),
    )

    type_code: str = Field(
        default="0",
        validation_alias=AliasChoices("type_code", "typeCode", "type", "type"),
    )

    @field_validator("type_code", mode="before")
    @classmethod
    def normalize_type_code(cls, value: Any) -> str:
        if value is None:
            return "0"

        if isinstance(value, float) and value.is_integer():
            return str(int(value))

        normalized = str(value).strip()

        return normalized or "0"

    roughness: float = Field(
        default=0.0,
        validation_alias=AliasChoices("roughness", "Roughness", "rugosité", "rugo"),
    )

    metadata_n: Any | None = Field(
        default=None,
        validation_alias=AliasChoices("metadata_n", "metadataN", "N"),
    )

class SimulationClientBehaviorInput(SimulationInputModel):
    """Reusable stochastic demand behaviour."""
    behavior_id: str = Field(
        validation_alias=AliasChoices("behavior_id", "behaviorId"),
    )

    name: str | None = None
    open_probability: float = Field(
        validation_alias=AliasChoices("open_probability","openProbability"),
    )

    target_flow_lps: float = Field(
        validation_alias=AliasChoices("target_flow_lps","targetFlowLps"),
    )

    profile_id: str | None = Field(
        default=None,
        validation_alias=AliasChoices("profile_id", "profileId"),
    )

    @field_validator("open_probability")
    @classmethod
    def validate_probability(cls, value: float) -> float:
        if not (0.0 <= value <= 1.0):
            raise ValueError("Open probability must be between 0 and 1.")
        return value
    
    @field_validator("target_flow_lps")
    @classmethod
    def validate_target_flow(cls, value: float):
        if value < 0:
            raise ValueError("Target flow must not be negative.")
        return value

class SimulationTablesInput(SimulationInputModel):
    """Group all tabular collections required by a simulation.

    ``nodes``, ``pipes``, ``clients``, ``bornes``, and ``diameters`` are adapted
    into pandas tables. ``orifices`` is retained for payload compatibility even
    though the current numerical adapter does not build a separate orifice table.
    """
    nodes: list[SimulationNodeInput]
    pipes: list[SimulationPipeInput]
    clients: list[SimulationClientInput]
    client_behaviors: list[SimulationClientBehaviorInput] = Field(
        default_factory=list,
        validation_alias=AliasChoices("client_behaviors","clientBehaviors"),
    )
    bornes: list[SimulationBorneInput]
    diameters: list[SimulationDiameterInput]

    orifices: list[dict[str, Any]] = Field(default_factory=list)

class SimulationDomainInput(SimulationInputModel):
    """Top-level request model for an in-memory hydraulic simulation.

    Attributes
    ----------
    name, version:
        Network identity and optional data-format version.
    source_format:
        Origin format label, normalized to uppercase.
    params:
        General overrides with the highest parameter precedence.
    technical_params:
        Technical/hydraulic parameter overrides.
    design_params:
        Network-design parameter overrides.
    tables:
        Typed collections containing the complete network definition.
    """
    name: str
    version: str = ""

    source_format: str = Field(
        default="DSG",
        validation_alias=AliasChoices("source_format", "sourceFormat"),
    )

    params: dict[str, Any] = Field(default_factory=dict)

    technical_params: dict[str, Any] = Field(
        default_factory=dict,
        validation_alias=AliasChoices("technical_params", "technicalParams")
    )

    design_params: dict[str, Any] = Field(
        default_factory=dict,
        validation_alias=AliasChoices("design_params", "designParams")
    )
    comments: list[str] = Field(default_factory=list)

    tables : SimulationTablesInput

    @field_validator("source_format")
    @classmethod
    def normalize_source_format(cls, value: str) -> str:
        """Strip surrounding whitespace and normalize the format to uppercase."""
        return value.strip().upper()


