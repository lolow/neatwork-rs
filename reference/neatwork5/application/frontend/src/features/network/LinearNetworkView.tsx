import { useEffect, useMemo, useRef, useState } from "react";

import type {
    EdgeStatus,
    NetworkEdge,
    NetworkGraph,
    NetworkNode,
    NetworkSelection,
    NodeFocusRequest
} from "./types";

import { FloatingNetworkInspector, type FloatingInspectorAnchor } from "./FloatingNetworkInspector";
import { buildLinearTopologyLayout, buildSelectionPathHighlight } from "./networkTopology";
import type { DisplayLengthUnit } from "../domain/unitSystem";

type Props = {
    graph: NetworkGraph;
    selection: NetworkSelection;
    focusRequest?: NodeFocusRequest | null;
    onSelectionChange: (selection: NetworkSelection) => void;
    onToggleEdgeStatus: (edgeId: string, nextStatus: EdgeStatus) => Promise<void>;
    onMoveNode: (nodeId: string, x: number, y: number) => Promise<void>;
    displayUnit: DisplayLengthUnit;
};

type ViewTransform = {
    scale: number;
    offsetX: number;
    offsetY: number;
};

const MIN_SCALE = 0.01;
const MAX_SCALE = 5000;

export function LinearNetworkView({
    graph,
    selection,
    focusRequest,
    onSelectionChange,
    onToggleEdgeStatus,
    onMoveNode,
    displayUnit
}: Props) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const dragRef = useRef({ x: 0, y: 0, dragging: false, moved: false });

    const [transform, setTransform] = useState<ViewTransform>({
        scale: 1,
        offsetX: 40,
        offsetY: 40
    });

    const [inspectorAnchor, setInspectorAnchor] = useState<FloatingInspectorAnchor | null>(null);
    const [flashedNodeId, setFlashedNodeId] = useState<string | null>(null);
    const flashTimerRef = useRef<number | null>(null);

    // Layout and path highlight
    const layout = useMemo(() => buildLinearTopologyLayout(graph), [graph]);
    const pathHighlight = useMemo(() => buildSelectionPathHighlight(layout, selection), [layout, selection]);

    const nodeById = useMemo(() => { return new Map(graph.nodes.map((node) => [node.id, node])); }, [graph.nodes]);

    // Fit graph to view
    function fitGraphToView() {
        if (!graph.nodes.length) return;

        const canvas = canvasRef.current;
        if (!canvas || !canvas.parentElement) return;

        const width = canvas.parentElement.clientWidth;
        const height = canvas.parentElement.clientHeight;

        const positions = layout.nodePositions;
        if (positions.size === 0) return;

        let minX = Infinity;
        let maxX = -Infinity;
        let minY = Infinity;
        let maxY = -Infinity;

        for (const [, point] of positions) {
            minX = Math.min(minX, point.x);
            maxX = Math.max(maxX, point.x);
            minY = Math.min(minY, point.y);
            maxY = Math.max(maxY, point.y);
        }

        const padding = 90;
        const safeWidth = Math.max(width - padding * 2, 1);
        const safeHeight = Math.max(height - padding * 2, 1);
        const dx = Math.max(maxX - minX, 1);
        const dy = Math.max(maxY - minY, 1);

        const scaleX = safeWidth / dx;
        const scaleY = safeHeight / dy;
        const scale = Math.min(scaleX, scaleY, 2.5);

        const cx = (minX + maxX) / 2;
        const cy = (minY + maxY) / 2;

        setTransform({
            scale,
            offsetX: width / 2 - cx * scale,
            offsetY: height / 2 - cy * scale
        });
    }

    // Focus request handler
    useEffect(() => {
        if (!focusRequest) return;

        const point = layout.nodePositions.get(focusRequest.nodeId);
        const canvas = canvasRef.current;
        const parent = canvas?.parentElement;
        if (!point || !parent) return;

        const width = parent.clientWidth;
        const height = parent.clientHeight;

        setTransform(current => {
            const nextScale = Math.max(current.scale, 0.9);
            return {
                scale: nextScale,
                offsetX: width / 2 - point.x * nextScale,
                offsetY: height / 2 - point.y * nextScale
            };
        });

        setFlashedNodeId(focusRequest.nodeId);

        if (flashTimerRef.current !== null) {
            window.clearTimeout(flashTimerRef.current);
        }

        flashTimerRef.current = window.setTimeout(() => {
            setFlashedNodeId(null);
            flashTimerRef.current = null;
        }, 2500);

    }, [focusRequest, layout.nodePositions]);

    // Cleanup timer
    useEffect(() => {
        return () => {
            if (flashTimerRef.current !== null) {
                window.clearTimeout(flashTimerRef.current);
            }
        };
    }, []);

    // Draw effect
    useEffect(() => {
        draw();
        const parent = canvasRef.current?.parentElement;
        if (!parent) {
            return;
        }
        let animationFrame = 0;
        const resizeObserver = new ResizeObserver(() => {
            window.cancelAnimationFrame(animationFrame);
            animationFrame = window.requestAnimationFrame(draw);
        });
        resizeObserver.observe(parent);
        return () => {
            resizeObserver.disconnect();
            window.cancelAnimationFrame(animationFrame);
        };
    }, [graph, transform, selection, layout, pathHighlight, flashedNodeId, displayUnit]);

    // Close inspector when selection is cleared
    useEffect(() => {
        if (!selection) {
            setInspectorAnchor(null);
        }
    }, [selection]);

    /**
     * Fit the generated topology when the graph or its linear layout changes.
     */
    useEffect(() => {
        fitGraphToView();
    }, [graph.id, layout]);

    // World to screen conversion
    function worldToScreen(x: number, y: number) {
        return {
            x: x * transform.scale + transform.offsetX,
            y: y * transform.scale + transform.offsetY
        };
    }

    function screenToWorld(x: number, y: number) {
        return {
            x: (x - transform.offsetX) / transform.scale,
            y: (y - transform.offsetY) / transform.scale
        };
    }

    function zoomAtCenter(factor: number) {
        const canvas = canvasRef.current;
        if (!canvas || !canvas.parentElement) return;

        const width = canvas.parentElement.clientWidth;
        const height = canvas.parentElement.clientHeight;

        const centerX = width / 2;
        const centerY = height / 2;

        const pointer = screenToWorld(centerX, centerY);
        const nextScale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, transform.scale * factor));

        setTransform({
            scale: nextScale,
            offsetX: centerX - pointer.x * nextScale,
            offsetY: centerY - pointer.y * nextScale
        });
    }

    /**
     * Draw one topology edge using its generated linear geometry.
     *
     * Tree edges are rendered as solid orthogonal lines.
     * Cross or loop edges are rendered as dashed lines.
     *
     * @param ctx Canvas drawing context
     * @param edge Network edge to render
     */
    function drawEdge(ctx: CanvasRenderingContext2D, edge: NetworkEdge) {
        const geometry = layout.edgeGeometryById.get(edge.id);
        if (!geometry || geometry.points.length < 2) {
            return;
        }

        const screenPoints = geometry.points.map(point => worldToScreen(point.x, point.y));
        const highlighted = pathHighlight.edgeIds.has(edge.id);
        const selected = selection?.kind === "edge" && selection.item.id === edge.id;

        ctx.save();

        if (highlighted) {
            ctx.strokeStyle = "#facc15";
            ctx.lineWidth = selected ? 5.5 : 4.2;
            ctx.shadowColor = "rgba(250, 204, 21, 0.42)";
            ctx.shadowBlur = 8;

        } else if (edge.status === "CLOSED") {
            ctx.strokeStyle = "rgba(100, 116, 139, 0.72)";
            ctx.lineWidth = selected ? 4.5 : 1.5;

        } else {
            ctx.strokeStyle = selected ? "#7dd3fc" : "rgba(56, 189, 248, 0.56)";
            ctx.lineWidth = selected ? 4.5 : 1.5;
        }
        /*
         * A non-tree edge closes a loop and is displayed
         * with a dashed line.
         */
        if (!geometry.treeEdge) {
            ctx.setLineDash([7, 5]);
        }
        ctx.beginPath();
        ctx.moveTo(screenPoints[0].x, screenPoints[0].y);
        for (let index = 1; index < screenPoints.length; index += 1) {
            ctx.lineTo(screenPoints[index].x, screenPoints[index].y);
        }
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.shadowBlur = 0;

        /*
         * Display the valve marker near the middle
         * point of the generated geometry.
         */
        if (edge.type === "VALVE") {
            const middleIndex = Math.floor(screenPoints.length / 2);
            const middlePoint = screenPoints[middleIndex];
            ctx.fillStyle = edge.status === "OPEN" ? "#22c55e" : "#ef4444";
            ctx.strokeStyle = "#e5f6ff";
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            ctx.rect(middlePoint.x - 6, middlePoint.y - 6, 12, 12);
            ctx.fill();
            ctx.stroke();
        }
        ctx.restore();
    }

    function drawNode(ctx: CanvasRenderingContext2D, node: NetworkNode) {
        const pos = layout.nodePositions.get(node.id);
        if (!pos) return;

        const p = worldToScreen(pos.x, pos.y);
        const selected = selection?.kind === "node" && selection.item.id === node.id;
        const flashed = flashedNodeId === node.id;
        const highlighted = pathHighlight.nodeIds.has(node.id);

        const baseRadius = selected ? 9
            : node.type === "RESERVOIR" ? 8
                : node.type === "FAUCET" ? 5
                    : 4;

        const radius = flashed ? 13 : baseRadius;

        ctx.save();

        // Flash style
        if (flashed) {
            ctx.fillStyle = "#f472b6";
            ctx.strokeStyle = "#ffffff";
            ctx.lineWidth = 3.5;
            ctx.shadowColor = "rgba(244, 114, 182, 0.78)";
            ctx.shadowBlur = 18;
        } else {
            // Normal node color
            if (node.lowPressure) {
                ctx.fillStyle = "#ef4444";
            } else if (node.type === "RESERVOIR") {
                ctx.fillStyle = "#38bdf8";
            } else if (node.type === "FAUCET") {
                ctx.fillStyle = "#fb923c";
            } else if (node.pressureM >= 30) {
                ctx.fillStyle = "#22c55e";
            } else if (node.pressureM >= 18) {
                ctx.fillStyle = "#facc15";
            } else {
                ctx.fillStyle = "#fb923c";
            }

            ctx.strokeStyle = highlighted ? "#facc15"
                : selected ? "#ffffff"
                    : "rgba(255,255,255,.6)";

            ctx.lineWidth = highlighted ? 3
                : selected ? 2.8
                    : 1.2;
        }

        ctx.beginPath();
        ctx.arc(p.x, p.y, radius, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.shadowBlur = 0;

        // Node label
        const showLabel = selected || flashed || transform.scale > 0.036;
        if (showLabel) {
            const labelWidth = Math.max(34, Math.min(90, node.label.length * 7 + 14));
            ctx.fillStyle = selected ? "rgba(8, 18, 35, 0.88)" : "rgba(8, 18, 35, 0.62)";
            ctx.fillRect(p.x + radius + 5, p.y - 15, labelWidth, 18);

            ctx.fillStyle = selected ? "#ffffff" : "#b5d2f9ff";
            ctx.font = selected ? "12px Inter, sans-serif" : "10px Inter, sans-serif";
            ctx.fillText(node.label, p.x + radius + 10, p.y - 2);
        }

        ctx.restore();
    }

    function draw() {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const parent = canvas.parentElement;
        if (!parent) return;

        const dpr = window.devicePixelRatio || 1;
        const width = parent.clientWidth;
        const height = parent.clientHeight;

        canvas.width = width * dpr;
        canvas.height = height * dpr;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, width, height);

        ctx.fillStyle = "#08182d";
        ctx.fillRect(0, 0, width, height);

        // Draw edges
        for (const edge of graph.edges) {
            drawEdge(ctx, edge);
        }

        // Draw nodes
        for (const node of graph.nodes) {
            drawNode(ctx, node);
        }
    }

    function distanceToSegment(
        px: number,
        py: number,
        ax: number,
        ay: number,
        bx: number,
        by: number
    ) {
        const dx = bx - ax;
        const dy = by - ay;

        if (dx === 0 && dy === 0) {
            return Math.hypot(px - ax, py - ay);
        }

        const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)));
        const projectionX = ax + t * dx;
        const projectionY = ay + t * dy;

        return Math.hypot(px - projectionX, py - projectionY);
    }

    function detectSelection(screenX: number, screenY: number): NetworkSelection {
        // Check nodes first
        for (const node of graph.nodes) {
            const pos = layout.nodePositions.get(node.id);
            if (!pos) continue;
            const p = worldToScreen(pos.x, pos.y);
            const radius = node.type === "RESERVOIR" ? 12 : node.type === "FAUCET" ? 10 : 9;
            if (Math.hypot(p.x - screenX, p.y - screenY) < radius) {
                return { kind: "node", item: node };
            }
        }

        /*
        * Check every segment of every generated edge geometry.
        */
        for (const edge of graph.edges) {
            const geometry = layout.edgeGeometryById.get(edge.id);

            if (!geometry || geometry.points.length < 2) {
                continue;
            }

            const screenPoints = geometry.points.map(point => worldToScreen(point.x, point.y));

            for (let index = 1; index < screenPoints.length; index += 1) {
                const start = screenPoints[index - 1];
                const end = screenPoints[index];
                const distance = distanceToSegment(screenX, screenY, start.x, start.y, end.x, end.y);

                if (distance < 7) {
                    return {
                        kind: "edge",
                        item: edge
                    };
                }
            }
        }

        return null;
    }

    function handleWheel(event: React.WheelEvent<HTMLCanvasElement>) {
        event.preventDefault();

        const rect = event.currentTarget.getBoundingClientRect();
        const pointer = screenToWorld(event.clientX - rect.left, event.clientY - rect.top);
        const zoomFactor = Math.pow(0.995, event.deltaY);
        const nextScale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, transform.scale * zoomFactor));

        setTransform({
            scale: nextScale,
            offsetX: event.clientX - rect.left - pointer.x * nextScale,
            offsetY: event.clientY - rect.top - pointer.y * nextScale
        });
    }

    function handlePointerDown(event: React.PointerEvent<HTMLCanvasElement>) {
        dragRef.current = {
            x: event.clientX,
            y: event.clientY,
            dragging: true,
            moved: false
        };
    }

    function handlePointerMove(event: React.PointerEvent<HTMLCanvasElement>) {
        if (!dragRef.current.dragging) return;

        const dx = event.clientX - dragRef.current.x;
        const dy = event.clientY - dragRef.current.y;

        if (Math.abs(dx) + Math.abs(dy) > 2) {
            dragRef.current.moved = true;
        }

        dragRef.current.x = event.clientX;
        dragRef.current.y = event.clientY;

        setTransform((current) => ({
            ...current,
            offsetX: current.offsetX + dx,
            offsetY: current.offsetY + dy
        }));
    }

    function handlePointerUp(event: React.PointerEvent<HTMLCanvasElement>) {
        const wasMoved = dragRef.current.moved;
        dragRef.current.dragging = false;

        if (wasMoved) return;

        const rect = event.currentTarget.getBoundingClientRect();
        const screenX = event.clientX - rect.left;
        const screenY = event.clientY - rect.top;
        const nextSelection = detectSelection(screenX, screenY);

        onSelectionChange(nextSelection);

        if (!nextSelection) {
            setInspectorAnchor(null);
            return;
        }

        const inspectorWidth = 360;
        const inspectorHeight = 420;

        setInspectorAnchor({
            x: screenX,
            y: screenY,
            horizontal: screenX > rect.width - inspectorWidth ? "left" : "right",
            vertical: screenY > rect.height - inspectorHeight ? "above" : "below",
        });
    }

    return (
        <section className="network-workspace linear-network-workspace">
            <div className="network-toolbar">
                <div>
                    <h2>Linear topology</h2>
                    <p>
                        Source-to-terminal hierarchy with loop edges preserved.
                    </p>
                </div>

                <div className="linear-network-meta">
                    <span>
                        Sources {layout.sourceNodeIds.size}
                    </span>
                    <span>
                        Nodes {graph.nodes.length}
                    </span>
                    <span>
                        Pipes {graph.edges.length}
                    </span>
                </div>
            </div>

            <div className="canvas-frame">
                <div className="canvas-overlay-controls linear-canvas-controls">
                    <div className="zoom-controls">
                        <button
                            type="button"
                            onClick={() => zoomAtCenter(0.8)}
                            title="Zoom out"
                            aria-label="Zoom out"
                        >
                            −
                        </button>

                        <button
                            type="button"
                            onClick={fitGraphToView}
                            title="Fit to view"
                            aria-label="Fit to view"
                        >
                            Fit
                        </button>

                        <button
                            type="button"
                            onClick={() => zoomAtCenter(1.25)}
                            title="Zoom in"
                            aria-label="Zoom in"
                        >
                            +
                        </button>
                    </div>

                    <div className="linear-network-legend">
                        <span>
                            <i className="linear-legend-line path" />
                            Upstream path
                        </span>

                        <span>
                            <i className="linear-legend-line loop" />
                            Loop edge
                        </span>
                    </div>
                </div>

                <canvas
                    ref={canvasRef}
                    onWheel={handleWheel}
                    onPointerDown={handlePointerDown}
                    onPointerMove={handlePointerMove}
                    onPointerUp={handlePointerUp}
                />

                <FloatingNetworkInspector
                    selection={selection}
                    anchor={inspectorAnchor}
                    onClose={() => {
                        setInspectorAnchor(null);
                        onSelectionChange(null);
                    }}
                    onToggleEdgeStatus={onToggleEdgeStatus}
                    onMoveNode={onMoveNode}
                    displayUnit={displayUnit}
                />
            </div>
        </section>
    );
}