from enum import Enum
from typing import Any, Optional

from pydantic import BaseModel, Field

class NeatworkFileFormat(str, Enum):
    TPO = "TPO"
    DSG = "DSG"
    JSON = "JSON"

class VisualNode(BaseModel):
    id: str
    label: str
    type: str
    x: float
    y: float
    elevationM: float = 0.0
    pressureM: float = 0.0
    lowPressure: bool = False
    sourceX: float | None = None
    sourceY: float | None = None
    layoutGenerated: bool = False

class VisualEdge(BaseModel):
    id: str
    sourceId: str
    targetId: str
    type: str = "PIPE"
    status: str = "OPEN"
    flowRateLps: float = 0.0
    velocityMps: float = 0.0

    lengthM: float | None = None
    diameterMm: float | None = None
    nominalSize: str | None = None
    sdr: float | None = None
    innerDiameter: float | None = None
    cost: float | None = None
    pmax: float | None = None
    pipeTypeCode: str | None = None
    roughness: float | None = None

class VisualGraph(BaseModel):
    id: str
    name: str
    nodes: list[VisualNode]
    edges: list[VisualEdge]

class NeatworkParseRequest(BaseModel):
    name: str
    format: NeatworkFileFormat
    primaryContent: str
    diametersDbContent: str | None = None
    orificesDbContent: str | None = None

class NeatworkTables(BaseModel):
    nodes: list[dict[str, Any]] = Field(default_factory=list)
    pipes: list[dict[str, Any]] = Field(default_factory=list)
    edges: list[dict[str, Any]] = Field(default_factory=list)
    clients: list[dict[str, Any]] = Field(default_factory=list)
    clientBehaviors: list[dict[str, Any]] = Field(default_factory=list)
    bornes: list[dict[str, Any]] = Field(default_factory=list)
    diameters: list[dict[str, Any]] = Field(default_factory=list)
    orifices: list[dict[str, Any]] = Field(default_factory=list)

class NeatworkDomainPayload(BaseModel):
    name: str
    sourceFormat: NeatworkFileFormat
    comments: list[str] | None = None
    params: dict[str, Any] = Field(default_factory=dict)
    technicalParams: dict[str, Any] = Field(default_factory=dict)
    designParams: dict[str, Any] = Field(default_factory=dict)
    tables: NeatworkTables
    warnings: list[str] = Field(default_factory=list)

class NeatworkParseResponse(BaseModel):
    domain: NeatworkDomainPayload
    graph: VisualGraph

class NeatworkRevisionSerializeRequest(BaseModel):
    name: str
    format: NeatworkFileFormat
    sourceContent: str
    domain: NeatworkDomainPayload

class NeatworkRevisionSerializeResponse(BaseModel):
    content: str
    domain: NeatworkDomainPayload
    graph: VisualGraph
