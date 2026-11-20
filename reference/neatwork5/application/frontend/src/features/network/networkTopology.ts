import type { NetworkEdge, NetworkGraph, NetworkSelection } from "./types";

export type LinearPoint = {
    x: number;
    y: number;
};

export type LinearEdgeGeometry = {
    edgeId: string;

    points: LinearPoint[];

    /**
     * True when the edge belongs to the spanning tree used for the
     * left-to-right hierarchy. False means that the edge closes a loop.
     */
    treeEdge: boolean;
};

export type LinearTopologyLayout = {
    nodePositions: Map<string, LinearPoint>;
    edgeGeometryById: Map<string, LinearEdgeGeometry>;
    depthByNodeId: Map<string, number>;
    parentNodeById: Map<string, string | null>;
    parentEdgeById: Map<string, string | null>;
    sourceNodeIds: Set<string>;
    rootNodeIds: string[];
};


type AdjacencyEntry = {
    nodeId: string;
    edgeId: string;
};

const HORIZONTAL_SPACING = 190;
const VERTICAL_SPACING = 74;

const COMPONENT_GAP = 110;

const LEFT_PADDING = 90;
const TOP_PADDING = 80;

/**
 * Build an undirected adjacency map used for topology traversal.
 */
function buildAdjacency(graph: NetworkGraph): Map<string, AdjacencyEntry[]> {
    const adjacency = new Map<string, AdjacencyEntry[]>();

    for (const node of graph.nodes) {
        adjacency.set(node.id, []);
    }

    for (const edge of graph.edges) {
        adjacency.get(edge.sourceId)?.push({
            nodeId: edge.targetId,
            edgeId: edge.id
        });

        adjacency.get(edge.targetId)?.push({
            nodeId: edge.sourceId,
            edgeId: edge.id
        });
    }
    /*
     * Stable ordering prevents the layout from moving randomly after unrelated graph updates.
     */
    for (const links of adjacency.values()) {
        links.sort(
            (left, right) => left.edgeId.localeCompare(right.edgeId)
        );
    }
    return adjacency;
}

/**
 * Find hydraulic roots used by the topology view.
 */
function findSourceNodeIds(graph: NetworkGraph): string[] {
    /*
     * First choice: explicit hydraulic source nodes.
     */
    const hydraulicSources = graph.nodes
        .filter(node => node.type === "RESERVOIR" || node.type === "TANK")
        .map(node => node.id);

    if (hydraulicSources.length > 0) {
        return hydraulicSources;
    }


    /*
     * Fallback: nodes without incoming edges.
     */
    const incomingCount = new Map<string, number>(
        graph.nodes.map(node => [node.id, 0])
    );

    for (const edge of graph.edges) {
        incomingCount.set(edge.targetId,
            (incomingCount.get(edge.targetId) ?? 0) + 1
        );
    }

    const directedRoots = graph.nodes
        .filter(node => (incomingCount.get(node.id) ?? 0) === 0)
        .map(node => node.id);

    if (directedRoots.length > 0) {
        return directedRoots;
    }

    /*
     * Last fallback for malformed/imported data.
     */
    return graph.nodes.length > 0 ? [graph.nodes[0].id] : [];
}

/**
 * Build a stable left-to-right topology layout.
 *
 * A spanning forest creates the hierarchy. Additional graph edges are
 * preserved separately so meshed networks and loops remain visible.
 */
export function buildLinearTopologyLayout(graph: NetworkGraph): LinearTopologyLayout {

    const adjacency = buildAdjacency(graph);
    const sourceNodeIds = new Set(findSourceNodeIds(graph));
    const depthByNodeId = new Map<string, number>();
    const parentNodeById = new Map<string, string | null>();
    const parentEdgeById = new Map<string, string | null>();
    const discoveryOrder = new Map<string, number>();
    const rootNodeIds: string[] = [];

    let discoveryCounter = 0;
    function traverseRoots(rootIds: string[]) {
        const queue: string[] = [];

        for (const rootId of rootIds) {

            if (depthByNodeId.has(rootId)) {
                continue;
            }

            depthByNodeId.set(rootId, 0);
            parentNodeById.set(rootId, null);
            parentEdgeById.set(rootId, null);
            discoveryOrder.set(rootId, discoveryCounter++);
            rootNodeIds.push(rootId);
            queue.push(rootId);
        }

        let head = 0;
        while (head < queue.length) {
            const currentId = queue[head++];
            const currentDepth = depthByNodeId.get(currentId) ?? 0;
            const links = adjacency.get(currentId) ?? [];

            for (const link of links) {
                /*
                 * Already visited means this connection belongs to a loop/cross edge.
                 */
                if (depthByNodeId.has(link.nodeId)) {
                    continue;
                }

                depthByNodeId.set(link.nodeId, currentDepth + 1);
                parentNodeById.set(link.nodeId, currentId);
                parentEdgeById.set(link.nodeId, link.edgeId);
                discoveryOrder.set(link.nodeId, discoveryCounter++);
                queue.push(link.nodeId);
            }
        }
    }

    /*
     * Multi-source BFS.
     */
    traverseRoots(Array.from(sourceNodeIds));
    /*
     * Disconnected components must not disappear.
     */
    for (const node of graph.nodes) {
        if (!depthByNodeId.has(node.id)) {
            traverseRoots([node.id]);
        }
    }

    const childrenByNodeId = new Map<string, string[]>(
        graph.nodes.map(node => [node.id, []])
    );
    for (const node of graph.nodes) {
        const parentId = parentNodeById.get(node.id);
        if (parentId) {
            childrenByNodeId.get(parentId)?.push(node.id);
        }
    }

    for (const children of childrenByNodeId.values()) {
        children.sort(
            (leftId, rightId) =>
                (discoveryOrder.get(leftId) ?? 0)
                -
                (discoveryOrder.get(rightId) ?? 0)
        );
    }

    const yByNodeId = new Map<string, number>();
    let nextLeafY = TOP_PADDING;
    function assignSubtreeY(nodeId: string): number {

        const existing = yByNodeId.get(nodeId);
        if (existing !== undefined) {
            return existing;
        }
        const children = childrenByNodeId.get(nodeId) ?? [];
        /*
         * Leaf:
         * give it a new vertical row.
         */
        if (children.length === 0) {
            const y = nextLeafY;
            nextLeafY += VERTICAL_SPACING;
            yByNodeId.set(nodeId, y);
            return y;
        }

        /*
         * Parent:
         * position it at the center of its children.
         */
        const childYs = children.map(childId => assignSubtreeY(childId));
        const y = (childYs[0] + childYs[childYs.length - 1]) / 2;
        yByNodeId.set(nodeId, y);
        return y;
    }

    for (const rootId of rootNodeIds) {
        assignSubtreeY(rootId);
        nextLeafY += COMPONENT_GAP;
    }

    const nodePositions = new Map<string, LinearPoint>();


    for (const node of graph.nodes) {

        nodePositions.set(node.id, {
            x: LEFT_PADDING + (depthByNodeId.get(node.id) ?? 0)
                * HORIZONTAL_SPACING,
            y: yByNodeId.get(node.id) ?? nextLeafY,
        }
        );
    }

    const edgeGeometryById = new Map<string, LinearEdgeGeometry>();
    let crossEdgeIndex = 0;
    for (const edge of graph.edges) {

        const sourcePoint = nodePositions.get(edge.sourceId);
        const targetPoint = nodePositions.get(edge.targetId);
        if (!sourcePoint || !targetPoint) {
            continue;
        }
        const treeEdge = parentEdgeById.get(edge.sourceId) === edge.id ||
            parentEdgeById.get(edge.targetId) === edge.id;

        /*
         * Normal topology edge.
         *
         * Use an orthogonal elbow similar to
         * the legacy NeatWork Tree View.
         */
        if (treeEdge) {
            const middleX = (sourcePoint.x + targetPoint.x) / 2;
            edgeGeometryById.set(edge.id,
                {
                    edgeId: edge.id,
                    treeEdge: true,
                    points: [sourcePoint,
                        { x: middleX, y: sourcePoint.y },
                        { x: middleX, y: targetPoint.y },
                        targetPoint,
                    ],
                }
            );
            continue;
        }

        /*
         * Cross edge / loop.
         *
         * Alternate its route above/below the main tree
         * so a meshed network remains visible.
         */
        const laneDistance = 44 + (crossEdgeIndex % 5) * 18;

        const above = crossEdgeIndex % 2 === 0;
        const laneY = above ? Math.min(sourcePoint.y, targetPoint.y)
            -
            laneDistance : Math.max(sourcePoint.y, targetPoint.y) + laneDistance;


        const direction = targetPoint.x >= sourcePoint.x ? 1 : -1;


        edgeGeometryById.set(
            edge.id,
            {
                edgeId: edge.id,
                treeEdge: false,
                points: [sourcePoint,
                    {
                        x: sourcePoint.x + 34 * direction,
                        y: sourcePoint.y,
                    },
                    {
                        x: sourcePoint.x + 34 * direction,
                        y: laneY,
                    },
                    {
                        x: targetPoint.x - 34 * direction,
                        y: laneY,
                    },
                    {
                        x: targetPoint.x - 34 * direction,
                        y: targetPoint.y,
                    },
                    targetPoint,
                ],
            }
        );

        crossEdgeIndex += 1;
    }


    return {
        nodePositions,
        edgeGeometryById,

        depthByNodeId,
        parentNodeById,
        parentEdgeById,

        sourceNodeIds,
        rootNodeIds,
    };
}

export type TopologyPathHighlight = { nodeIds: Set<string>; edgeIds: Set<string>; };

/**
 * Build the complete topology path from a root/source to the selection.
 */
export function buildSelectionPathHighlight(layout: LinearTopologyLayout, selection: NetworkSelection): TopologyPathHighlight {
    const nodeIds = new Set<string>();
    const edgeIds = new Set<string>();
    function collectPathToRoot(nodeId: string) {
        let currentId: string | null = nodeId;
        const visited = new Set<string>();
        while (currentId !== null && !visited.has(currentId)) {
            visited.add(currentId);
            nodeIds.add(currentId);
            const parentEdgeId = layout.parentEdgeById.get(currentId) ?? null;
            if (parentEdgeId) {
                edgeIds.add(parentEdgeId);
            }
            currentId = layout.parentNodeById.get(currentId) ?? null;
        }
    }
    if (!selection) {
        return { nodeIds, edgeIds };
    }

    /*
     * Node selection: directly walk toward the source.
     */
    if (selection.kind === "node") {
        collectPathToRoot(selection.item.id);
        return { nodeIds, edgeIds };
    }

    const edge: NetworkEdge = selection.item;

    /*
     * If this is a tree edge, find its downstream child.
     */
    let downstreamNodeId: string | null = null;
    if (layout.parentEdgeById.get(edge.sourceId) === edge.id) {
        downstreamNodeId = edge.sourceId;
    }

    if (layout.parentEdgeById.get(edge.targetId) === edge.id) {
        downstreamNodeId = edge.targetId;
    }

    if (downstreamNodeId) {
        collectPathToRoot(downstreamNodeId);
        return { nodeIds, edgeIds };
    }

    /*
     * Cross/loop edge: use the endpoint closer to a source as the upstream side.
     */
    const sourceDepth = layout.depthByNodeId.get(edge.sourceId) ?? Number.MAX_SAFE_INTEGER;

    const targetDepth = layout.depthByNodeId.get(edge.targetId) ?? Number.MAX_SAFE_INTEGER;

    const upstreamNodeId = sourceDepth <= targetDepth ? edge.sourceId : edge.targetId;

    const otherNodeId = upstreamNodeId === edge.sourceId ? edge.targetId : edge.sourceId;

    collectPathToRoot(upstreamNodeId);
    nodeIds.add(otherNodeId);
    edgeIds.add(edge.id);

    return { nodeIds, edgeIds };
}