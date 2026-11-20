export type NodeType = "RESERVOIR" | "JUNCTION" | "TANK" | "FAUCET";
export type EdgeType = "PIPE" | "PUMP" | "VALVE";
export type EdgeStatus = "OPEN" | "CLOSED";

export type NetworkNode = {
    id: string;
    label: string;
    type: NodeType;
    x: number; // coordinate visual (Visual graph)
    y: number; // coordinate visual (Visual graph)
    sourceX?: number | null; // coordinate source (database)
    sourceY?: number | null; // coordinate source (database)
    elevationM: number;
    pressureM: number;
    lowPressure: boolean;
    layoutGenerated?: boolean | null; // flag to indicate if the layout was generated automatically
};

export type NetworkEdge = {
    id: string;
    sourceId: string;
    targetId: string;
    type: EdgeType;
    status: EdgeStatus;
    flowRateLps: number;
    velocityMps: number;

    lengthM?: number | null;
    diameterId?: string | null;
    nominalSize?: string | null;
    sdr?: number | null;
    innerDiameter?: number | null;
    cost?: number | null;
    pmax?: number | null;
    pipeTypeCode?: string | null;
    roughness?: number | null;
};

export type NetworkGraph = {
    id: string;
    name: string;
    nodes: NetworkNode[];
    edges: NetworkEdge[];
};

export type NetworkSelection =
    | { kind: "node"; item: NetworkNode }
    | { kind: "edge"; item: NetworkEdge }
    | null;

/**
 * Request used to focus and temporarily highlight one network node.
 * requestId allows the same node to be searched repeatedly.
 */
export type NodeFocusRequest = {
    nodeId: string;
    requestId: number;
};