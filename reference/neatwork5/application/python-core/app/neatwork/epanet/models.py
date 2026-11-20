from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, Field

# This file contains the data-layer models for the EPANET parser.
class EpanetMappingKind(str, Enum):
    DIRECT = "DIRECT"
    ADAPTER = "ADAPTER"
    PRESERVE = "PRESERVE"
    UNSUPPORTED = "UNSUPPORTED"

# Error severities used in EPANET converter and utilities.
class EpanetIssueSeverity(str, Enum):
    INFO = "INFO"
    WARNING = "WARNING"
    BLOCKING = "BLOCKING"

# Data-line and section models.
class EpanetDataLine(BaseModel):
    """One parsed data line from an EPANET section."""
    line_number: int
    raw: str
    data: str
    tokens: list[str] = Field(default_factory=list)
    comment: str | None = None

# Section and document models.
class EpanetSection(BaseModel):
    """One EPANET input-file section."""
    name: str
    lines: list[EpanetDataLine] = Field(default_factory=list)

    #Raw lines are preserved so unsupported information is never silently destroyed.
    raw_lines: list[str] = Field(default_factory=list)

# The document as a whole. 
class EpanetInpDocument(BaseModel):
    """Lossless-enough representation of one INP document."""
    sections: dict[str, EpanetSection] = Field(default_factory=dict)
    section_order: list[str] = Field(default_factory=list)
    preamble: list[str] = Field(default_factory=list)
    end_seen: bool = False

    def section(self, name: str) -> EpanetSection | None:
        return self.sections.get(name.strip().upper())

# Node models
class EpanetJunction(BaseModel):
    node_id: str
    elevation: float
    base_demand: float = 0.0
    pattern_id: str | None = None
    source_line: int

class EpanetReservoir(BaseModel):
    node_id: str
    head: float
    pattern_id: str | None = None
    source_line: int

class EpanetPipe(BaseModel):
    pipe_id: str
    start_node_id: str
    end_node_id: str
    length: float
    diameter: float
    roughness: float
    minor_loss: float = 0.0
    status: str = "OPEN"
    source_line: int  

class EpanetDemand(BaseModel):
    node_id: str
    base_demand: float
    pattern_id: str | None = None
    category: str | None = None
    source_line: int

class EpanetEmitter(BaseModel):
    node_id: str
    coefficient: float
    source_line: int

class EpanetCoordinate(BaseModel):
    node_id: str
    x: float
    y: float
    source_line: int

# Semantic model
class EpanetSemanticModel(BaseModel):
    title: str = ""
    junctions: list[EpanetJunction] = Field(default_factory=list)
    reservoirs: list[EpanetReservoir] = Field(default_factory=list)
    pipes: list[EpanetPipe] = Field(default_factory=list)
    demands: list[EpanetDemand] = Field(default_factory=list)
    emitters: list[EpanetEmitter] = Field(default_factory=list)
    coordinates: list[EpanetCoordinate] = Field(default_factory=list)
    patterns: dict[str, list[float]] = Field(default_factory=dict)
    options: dict[str, str] = Field(default_factory=dict)
    preserved_sections: dict[str, list[str]] = Field(default_factory=dict)

# Mapping issues and summaries.
class EpanetMappingIssue(BaseModel):
    code: str
    severity: EpanetIssueSeverity
    message: str
    section: str | None = None
    line_number: int | None = None

class EpanetMappingReport(BaseModel):
    mapped: dict[str, int] = Field(default_factory=dict)
    adapted: dict[str, int] = Field(default_factory=dict)
    preserved: dict[str, int] = Field(default_factory=dict)
    unsupported: dict[str, int] = Field(default_factory=dict)
    issues: list[EpanetMappingIssue] = Field(default_factory=list)
    simulation_ready: bool = False

    def add_issue(self, *,
        code: str,
        severity: EpanetIssueSeverity,
        message: str,
        section: str | None = None,
        line_number: int | None = None,
    ) -> None:
        self.issues.append(
            EpanetMappingIssue(
                code=code,
                severity=severity,
                message=message,
                section=section,
                line_number=line_number,
            )
        )

    def finalize(self) -> None:
        self.simulation_ready = (
            not any(
                issue.severity == EpanetIssueSeverity.BLOCKING
                for issue in self.issues
            )
        )

class EpanetDemandPolicy(str, Enum):
    # Safest default: preserve EPANET demand but do not invent intermittent client probabilities.
    PRESERVE_UNRESOLVED = "PRESERVE_UNRESOLVED"

    # Explicit compatibility mode for later EPANET-reference experiments.
    # A deterministic demand becomes an always-open NeatWork client only when no time pattern is involved.
    DETERMINISTIC_ALWAYS_OPEN = "DETERMINISTIC_ALWAYS_OPEN"

class EpanetAdapterOptions(BaseModel):

    demand_policy: EpanetDemandPolicy = EpanetDemandPolicy.PRESERVE_UNRESOLVED
    default_required_pressure_m: float | None = None