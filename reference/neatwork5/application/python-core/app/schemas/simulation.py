from enum import Enum
from pydantic import BaseModel

class NodeType(str, Enum):
    RESERVOIR = "RESERVOIR"
    JUNCTION = "JUNCTION"
    TANK = "TANK"
    FAUCET = "FAUCET"

class EdgeType(str, Enum):
    PIPE = "PIPE"
    PUMP = "PUMP"
    VALVE = "VALVE"

class EdgeStatus(str, Enum):
    OPEN = "OPEN"
    CLOSED = "CLOSED"

class SimulationNodeRequest(BaseModel):
    id: str
    type: NodeType
    x: float
    y: float
    elevationM: float
    pressureM: float

class SimulationEdgeRequest(BaseModel):
    id: str
    sourceId: str
    targetId: str
    type: EdgeType
    status : EdgeStatus
    flowRateLps: float
    velocityMps: float

class SimulationRunRequest(BaseModel):
    projectId: str
    nodes: list[SimulationNodeRequest]
    edges: list[SimulationEdgeRequest]

class SimulationNodeResult(BaseModel):
    id: str
    pressureM: float
    lowPressure: bool

class SimulationEdgeResult(BaseModel):
    id: str
    flowRateLps: float
    velocityMps: float

class SimulationSummary(BaseModel):
    status: str
    openEdges: int
    closedEdges: int
    lowPressureNodes: int

class SimulationRunResponse(BaseModel):
    nodes: list[SimulationNodeResult]
    edges: list[SimulationEdgeResult]
    summary: SimulationSummary