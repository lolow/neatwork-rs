import React, { useEffect, useMemo, useRef, useState } from "react";
import type {
    EdgeStatus,
    NetworkEdge,
    NetworkGraph,
    NetworkNode,
    NetworkSelection,
    NodeFocusRequest
} from "./types";
import { FloatingNetworkInspector, type FloatingInspectorAnchor } from "./FloatingNetworkInspector";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import { getDisplayUnitSymbol, metersToDisplayUnit } from "../domain/unitSystem";


type Props = {
    graph: NetworkGraph;
    selection: NetworkSelection;
    displayUnit: DisplayLengthUnit;
    focusRequest?: NodeFocusRequest | null;
    onSelectionChange: (selection: NetworkSelection) => void;
    onToggleEdgeStatus: (edgeId: string, nextStatus: EdgeStatus) => Promise<void>;
    onMoveNode: (nodeId: string, x: number, y: number) => Promise<void>;
    coordinateMode?: "visual" | "raw";
};

type ViewTransform = {
    scale: number;
    offsetX: number;
    offsetY: number;
};

type ViewRotation = {
    azimuthDeg: number;
    tiltDeg: number;
};

type ViewMode = "iso" | "x" | "y" | "z";

const MIN_SCALE = 0.01;
const MAX_SCALE = 5000;
const ROTATION_STEP_DEG = 15;
const TILT_STEP_DEG = 8;
const DEFAULT_ROTATION: ViewRotation = {
    azimuthDeg: 0,
    tiltDeg: 63
}


/**
 * Render an interactive network graph on a canvas.
 *
 * The component supports node search focus, graph navigation,
 * node and edge selection, edge status changes and node movement.
 */
export function NetworkCanvas({ graph, selection, focusRequest, onSelectionChange, onToggleEdgeStatus, onMoveNode, coordinateMode = "visual", displayUnit }: Props) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const dragRef = useRef({ x: 0, y: 0, dragging: false, moved: false });

    const [transform, setTransform] = useState<ViewTransform>({
        scale: 1,
        offsetX: 40,
        offsetY: 40
    });

    const [rotation, setRotation] = useState<ViewRotation>(DEFAULT_ROTATION);
    const [viewMode, setViewMode] = useState<ViewMode>("iso");
    const [showCanvasDetails, setShowCanvasDetails] = useState(false);
    const [inspectorAnchor, setInspectorAnchor] = useState<FloatingInspectorAnchor | null>(null);
    const [flashedNodeId, setFlashedNodeId] = useState<string | null>(null);
    const flashTimerRef = useRef<number | null>(null);

    const rawCoordinatesCollapsed =
        coordinateMode === "raw" &&
        graph.nodes.length > 1 &&
        graph.nodes.every((node) => node.x === graph.nodes[0].x && node.y === graph.nodes[0].y);

    const nodeById = useMemo(() => {
        return new Map(graph.nodes.map((node) => [node.id, node]));
    }, [graph.nodes]);

    const graphBounds = useMemo(() => {
        if (!graph.nodes.length) {
            return {
                centerX: 0,
                centerY: 0,
                centerZ: 0,
                xRange: 1,
                yRange: 1,
                zRange: 1,
                dominantAxis: "x" as "x" | "y"
            };
        }

        let minX = Infinity;
        let maxX = -Infinity;
        let minY = Infinity;
        let maxY = -Infinity;
        let minZ = Infinity;
        let maxZ = -Infinity;

        for (const node of graph.nodes) {
            const z = node.elevationM ?? 0;

            minX = Math.min(minX, node.x);
            maxX = Math.max(maxX, node.x);
            minY = Math.min(minY, node.y);
            maxY = Math.max(maxY, node.y);
            minZ = Math.min(minZ, z);
            maxZ = Math.max(maxZ, z);
        }

        const xRange = Math.max(maxX - minX, 1);
        const yRange = Math.max(maxY - minY, 1);
        const zRange = Math.max(maxZ - minZ, 1);

        return {
            centerX: (minX + maxX) / 2,
            centerY: (minY + maxY) / 2,
            centerZ: (minZ + maxZ) / 2,
            xRange,
            yRange,
            zRange,
            dominantAxis: yRange >= xRange ? "y" as const : "x" as const
        };
    }, [graph.nodes]);

    const boundsKey = useMemo(() => {
        return graph.nodes
            .map((node) => `${node.id}:${node.x}:${node.y}:${node.elevationM}:${node.pressureM}`)
            .join("|");
    }, [graph.nodes]);

    useEffect(() => {
        fitGraphToView();
    }, [graph.id, boundsKey, rotation.azimuthDeg, rotation.tiltDeg, viewMode, coordinateMode]);

    function fitGraphToView() {
        if (!graph.nodes.length) return;

        const canvas = canvasRef.current;
        if (!canvas || !canvas.parentElement) return;

        const width = canvas.parentElement.clientWidth;
        const height = canvas.parentElement.clientHeight;

        let minX = Infinity;
        let maxX = -Infinity;
        let minY = Infinity;
        let maxY = -Infinity;

        for (const node of graph.nodes) {
            const point = projectNode(node);
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
        const scale = Math.min(scaleX, scaleY);

        const cx = (minX + maxX) / 2;
        const cy = (minY + maxY) / 2;

        setTransform({
            scale,
            offsetX: width / 2 - cx * scale,
            offsetY: height / 2 - cy * scale
        });
    }
    useEffect(() => {

        if (!focusRequest) { return; }
        const node = nodeById.get(focusRequest.nodeId);
        const canvas = canvasRef.current;
        const parent = canvas?.parentElement;
        if (!node || !parent) { return; }
        const projected = projectNode(node);
        const width = parent.clientWidth;
        const height = parent.clientHeight;

        /*
         * Keep the current zoom level and simply bring the searched node to the center.
         */
        setTransform(
            current => ({
                ...current, offsetX: width / 2 -
                    projected.x * current.scale, offsetY: height / 2 -
                        projected.y * current.scale
            })
        );
        setFlashedNodeId(node.id);

        if (flashTimerRef.current !== null) {
            window.clearTimeout(flashTimerRef.current);
        }

        flashTimerRef.current = window.setTimeout(() => {

            setFlashedNodeId(current => current === node.id ? null : current);
            flashTimerRef.current = null;
        },
            2500
        );

    }, [focusRequest, nodeById]);

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

        return () => { resizeObserver.disconnect(); window.cancelAnimationFrame(animationFrame); };
    }, [graph, transform, selection, rotation, viewMode, showCanvasDetails, coordinateMode, flashedNodeId]);

    useEffect(() => {
        if (!selection) {
            setInspectorAnchor(null);
        }
    }, [selection]);

    useEffect(() => {
        return () => {
            if (flashTimerRef.current !== null) {
                window.clearTimeout(flashTimerRef.current);
            }
        };
    }, []);

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

    const MIN_SCALE = 0.01;
    const MAX_SCALE = 5000;

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

    function zoomIn() {
        zoomAtCenter(1.25);
    }

    function zoomOut() {
        zoomAtCenter(0.8);
    }

    function rotateView(deltaAzimuthDeg: number, deltaTiltDeg = 0) {
        setViewMode("iso");

        setRotation((current) => ({
            azimuthDeg: current.azimuthDeg + deltaAzimuthDeg,
            tiltDeg: Math.min(82, Math.max(20, current.tiltDeg + deltaTiltDeg))
        }));
    }

    function resetViewRotation() {
        setViewMode("iso");
        setRotation(DEFAULT_ROTATION);
    }

    function setViewPreset(preset: ViewMode) {
        setViewMode(preset);

        if (preset === "iso") {
            setRotation(DEFAULT_ROTATION);
            return;
        }

        if (preset === "x") {
            setRotation({ azimuthDeg: 0, tiltDeg: 90 });
            return;
        }

        if (preset === "y") {
            setRotation({ azimuthDeg: 90, tiltDeg: 90 });
            return;
        }

        if (preset === "z") {
            setRotation({ azimuthDeg: 0, tiltDeg: 0 });
        }
    }

    function pressureColor(node: NetworkNode) {
        if (node.lowPressure) return "#ef4444";
        if (node.type === "RESERVOIR") return "#38bdf8";
        if (node.type === "FAUCET") return "#fb923c";
        if (node.pressureM >= 30) return "#22c55e";
        if (node.pressureM >= 18) return "#facc15";
        return "#fb923c";
    }

    function shouldShowNodeLabel(node: NetworkNode) {
        const selected = selection?.kind === "node" && selection.item.id === node.id;
        const flashed = flashedNodeId === node.id;
        return selected || flashed || transform.scale > 0.036;
    }

    function shouldShowNodeDetails(node: NetworkNode) {
        //const selected = selection?.kind === "node" && selection.item.id === node.id;
        //return selected || transform.scale > 0.10;
        return showCanvasDetails;
    }

    function shouldShowEdgeLabel(edge: NetworkEdge) {
        const selected = selection?.kind === "edge" && selection.item.id === edge.id;
        return showCanvasDetails || selected;
    }

    function shouldShowArrow(edge: NetworkEdge) {
        const selected = selection?.kind === "edge" && selection.item.id === edge.id;
        return selected || transform.scale > 0.07;
    }

    function drawEdge(ctx: CanvasRenderingContext2D, from: NetworkNode, to: NetworkNode, edge: NetworkEdge) {
        const a = nodeToScreen ? nodeToScreen(from) : worldToScreen(from.x, from.y);
        const b = nodeToScreen ? nodeToScreen(to) : worldToScreen(to.x, to.y);
        const selected = selection?.kind === "edge" && selection.item.id === edge.id;

        ctx.save();

        ctx.strokeStyle = edge.status === "OPEN"
            ? selected ? "#7dd3fc" : "rgba(56, 189, 248, 0.45)"
            : "rgba(100, 116, 139, 0.55)";

        ctx.lineWidth = selected ? 5 : 1.4;
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.stroke();

        if (edge.type === "VALVE") {
            const midX = (a.x + b.x) / 2;
            const midY = (a.y + b.y) / 2;

            ctx.fillStyle = edge.status === "OPEN" ? "#22c55e" : "#ef4444";
            ctx.strokeStyle = "#e5f6ff";
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            ctx.rect(midX - 6, midY - 6, 12, 12);
            ctx.fill();
            ctx.stroke();
        }

        if (shouldShowArrow(edge)) {
            drawDirectionArrow(ctx, a.x, a.y, b.x, b.y, edge);
        }

        if (shouldShowEdgeLabel(edge)) {
            const midX = (a.x + b.x) / 2;
            const midY = (a.y + b.y) / 2;

            ctx.fillStyle = "rgba(8, 18, 35, 0.82)";
            ctx.fillRect(midX + 6, midY - 20, 86, 18);

            ctx.fillStyle = "#dff3ff";
            ctx.font = "11px Inter, sans-serif";
            ctx.fillText(`${edge.id} · ${edge.flowRateLps.toFixed(2)} L/s`, midX + 10, midY - 7);
        }

        ctx.restore();
    }

    function drawDirectionArrow(
        ctx: CanvasRenderingContext2D,
        ax: number,
        ay: number,
        bx: number,
        by: number,
        edge: NetworkEdge
    ) {
        const angle = Math.atan2(by - ay, bx - ax);
        const midX = (ax + bx) / 2;
        const midY = (ay + by) / 2;
        const size = 6;

        ctx.fillStyle = edge.status === "OPEN" ? "#e0f2fe" : "#94a3b8";
        ctx.beginPath();
        ctx.moveTo(midX + Math.cos(angle) * size, midY + Math.sin(angle) * size);
        ctx.lineTo(midX + Math.cos(angle + 2.5) * size, midY + Math.sin(angle + 2.5) * size);
        ctx.lineTo(midX + Math.cos(angle - 2.5) * size, midY + Math.sin(angle - 2.5) * size);
        ctx.closePath();
        ctx.fill();
    }

    function drawNode(ctx: CanvasRenderingContext2D, node: NetworkNode) {
        const p = nodeToScreen ? nodeToScreen(node) : worldToScreen(node.x, node.y);
        const selected = selection?.kind === "node" && selection.item.id === node.id;
        const flashed = flashedNodeId === node.id;

        const baseRadius = selected ? 10 : node.type === "RESERVOIR" ? 8
            : node.type === "FAUCET" ? 5 : 4;
        const radius = flashed ? Math.max(14, baseRadius * 1.8) : baseRadius;

        ctx.save();

        ctx.fillStyle = flashed ? "#f472b6" : pressureColor(node);
        ctx.strokeStyle = flashed ? "#ffffff" : selected ? "#ffffff" : "rgba(255, 255, 255, 0.55)";
        ctx.lineWidth = flashed ? 4 : selected ? 3 : 1.3;
        ctx.beginPath();
        if (flashed) {
            ctx.shadowColor = "rgba(244, 114, 182, 0.8)";
            ctx.shadowBlur = 18;
        }
        ctx.arc(p.x, p.y, radius, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.shadowBlur = 0;

        if (shouldShowNodeLabel(node)) {
            const labelWidth = Math.max(34, Math.min(90, node.label.length * 7 + 14));
            ctx.fillStyle = selected ? "rgba(8, 18, 35, 0.88)" : "rgba(8, 18, 35, 0.62)";
            ctx.fillRect(p.x + radius + 5, p.y - 15, labelWidth, 18);

            ctx.fillStyle = selected ? "#ffffff" : "#b5d2f9ff";
            ctx.font = selected ? "12px Inter, sans-serif" : "10px Inter, sans-serif";
            ctx.fillText(node.label, p.x + radius + 10, p.y - 2);
        }

        if (shouldShowNodeDetails(node)) {
            const elevation = node.elevationM ?? 0;

            ctx.fillStyle = "#94a3b8";
            ctx.font = "10px Inter, sans-serif";

            const unitSymbol = getDisplayUnitSymbol(displayUnit);
            const elevationDisplay = metersToDisplayUnit(elevation, displayUnit);
            const pressureDisplay = metersToDisplayUnit(node.pressureM, displayUnit);

            ctx.fillText(
                `z=${elevationDisplay.toFixed(2)} ${unitSymbol}` + ` · p=${pressureDisplay.toFixed(2)} ${unitSymbol}`,
                p.x + radius + 8,
                p.y + 12
            );
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

        for (const edge of graph.edges) {
            const from = nodeById.get(edge.sourceId);
            const to = nodeById.get(edge.targetId);
            if (from && to) drawEdge(ctx, from, to, edge);
        }

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
        const clickedNode = graph.nodes.find((node) => {
            const point = nodeToScreen(node);
            const radius =
                node.type === "RESERVOIR" ? 12 :
                    node.type === "FAUCET" ? 10 :
                        9;
            return Math.hypot(point.x - screenX, point.y - screenY) < radius;
        });

        if (clickedNode) {
            return { kind: "node", item: clickedNode };
        }

        const clickedEdge = graph.edges.find((edge) => {
            const from = nodeById.get(edge.sourceId);
            const to = nodeById.get(edge.targetId);
            if (!from || !to) return false;

            const a = nodeToScreen(from);
            const b = nodeToScreen(to);
            return distanceToSegment(screenX, screenY, a.x, a.y, b.x, b.y) < 7;
        });

        return clickedEdge ? { kind: "edge", item: clickedEdge } : null;
    }

    function handleWheel(event: React.WheelEvent<HTMLCanvasElement>) {
        //event.preventDefault();

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

        /*
         * A pan gesture must not create
         * a new selection or inspector.
         */
        if (wasMoved) return;
        const rect = event.currentTarget.getBoundingClientRect();
        const screenX = event.clientX - rect.left;
        const screenY = event.clientY - rect.top;
        const nextSelection = detectSelection(screenX, screenY);

        onSelectionChange(nextSelection);

        /*
         * Clicking on empty canvas closes
         * the floating inspector.
         */
        if (!nextSelection) {
            setInspectorAnchor(null);
            return;
        }

        /*
         * Open the popup on the side with enough free space.
         * Approximate popup dimensions are used only for choosing its direction;
         * CSS still constrains the actual size.
         */

        const inspectorWidth = 360;

        const inspectorHeight = 420;

        setInspectorAnchor({
            x: screenX,
            y: screenY,
            horizontal: screenX > rect.width - inspectorWidth
                ? "left"
                : "right",
            vertical: screenY > rect.height - inspectorHeight
                ? "above"
                : "below",
        });
    }

    function projectNode(node: NetworkNode) {
        if (coordinateMode === "raw") {
            return {
                x: node.x,
                y: node.y,
            };
        }

        const localX = node.x - graphBounds.centerX;
        const localY = node.y - graphBounds.centerY;
        const localZ = (node.elevationM ?? 0) - graphBounds.centerZ;

        function zScaleFor(horizontalRange: number) {
            // elevation is scaled to be visible
            return (horizontalRange * 0.35) / graphBounds.zRange;
        }

        // X view mode: horizontal = Y, vertical = height
        if (viewMode === "x") {
            const zScale = zScaleFor(graphBounds.yRange);

            return {
                x: localY,
                y: -localZ * zScale
            };
        }

        // Y profile: horizontal = X, vertical = height
        // seeing the XZ plane from Y direction
        if (viewMode === "y") {
            const zScale = zScaleFor(graphBounds.xRange);

            return {
                x: localX,
                y: -localZ * zScale
            };
        }

        // Z / Height profile:
        // to see the network horizontally
        if (viewMode === "z") {
            const horizontal =
                graphBounds.dominantAxis === "y"
                    ? localY
                    : localX;

            const horizontalRange =
                graphBounds.dominantAxis === "y"
                    ? graphBounds.yRange
                    : graphBounds.xRange;

            const zScale = zScaleFor(horizontalRange);

            return {
                x: horizontal,
                y: -localZ * zScale
            };
        }

        // Default rotatable isometric view
        const azimuth = (rotation.azimuthDeg * Math.PI) / 180;
        const tilt = (rotation.tiltDeg * Math.PI) / 180;

        const cosA = Math.cos(azimuth);
        const sinA = Math.sin(azimuth);

        const rotatedX = localX * cosA - localY * sinA;
        const rotatedY = localX * sinA + localY * cosA;

        const zIsoScale = Math.max(graphBounds.xRange, graphBounds.yRange) * 0.12 / graphBounds.zRange;

        return {
            x: rotatedX,
            y: rotatedY * Math.cos(tilt) - localZ * zIsoScale
        };
    }

    function worldPointToScreen(point: { x: number; y: number }) {
        return {
            x: point.x * transform.scale + transform.offsetX,
            y: point.y * transform.scale + transform.offsetY
        };
    }

    function nodeToScreen(node: NetworkNode) {
        return worldPointToScreen(projectNode(node));
    }

    return (
        <section className="network-workspace">
            <div className="network-toolbar">
                <div>
                    <h2>
                        {coordinateMode === "raw"
                            ? "Raw 2D network graph"
                            : graph.name}
                    </h2>

                    <p>
                        {coordinateMode === "raw"
                            ? "This view uses the original X/Y coordinates from the imported file."
                            : "This view uses generated visual coordinates when the source file has missing or zero coordinates."}
                    </p>
                </div>
            </div>

            <div className="canvas-frame">
                <div className="canvas-overlay-controls">
                    <div className="canvas-control-group">
                        <label className="canvas-detail-toggle">
                            <input
                                type="checkbox"
                                checked={showCanvasDetails}
                                onChange={(e) => setShowCanvasDetails(e.target.checked)}
                            />
                            <span>Show Details</span>
                        </label>

                        <div className="zoom-controls" aria-label="Network zoom controls">
                            <button type="button" onClick={zoomOut} title="Zoom out" aria-label="Zoom out">−</button>
                            <button type="button" onClick={fitGraphToView} title="Fit to view" aria-label="Fit to view">Fit</button>
                            <button type="button" onClick={zoomIn} title="Zoom in" aria-label="Zoom in">+</button>
                        </div>

                        {coordinateMode === "visual" && (
                            <div className="orientation-overlay-control" aria-label="3D view orientation controls">
                                <div className="orientation-sphere" role="group" aria-label="View axis presets">
                                    <span className="axis-line axis-line-x" />
                                    <span className="axis-line axis-line-y" />
                                    <span className="axis-line axis-line-z" />

                                    <button
                                        type="button"
                                        className="axis-pill axis-pill-x"
                                        onClick={() => setViewPreset("x")}
                                        title="View along X axis"
                                    >
                                        X
                                    </button>

                                    <button
                                        type="button"
                                        className="axis-pill axis-pill-y"
                                        onClick={() => setViewPreset("y")}
                                        title="View along Y axis"
                                    >
                                        Y
                                    </button>

                                    <button
                                        type="button"
                                        className="axis-pill axis-pill-z"
                                        onClick={() => setViewPreset("z")}
                                        title="Top / height view"
                                    >
                                        Z
                                    </button>
                                </div>
                                <div className="rotation-buttons">
                                    <button type="button" onClick={() => rotateView(-ROTATION_STEP_DEG)} title="Rotate left">
                                        ↺
                                    </button>
                                    <button type="button" onClick={() => rotateView(ROTATION_STEP_DEG)} title="Rotate right">
                                        ↻
                                    </button>
                                    <button type="button" onClick={() => rotateView(0, TILT_STEP_DEG)} title="Tilt up">
                                        ↑
                                    </button>
                                    <button type="button" onClick={() => rotateView(0, -TILT_STEP_DEG)} title="Tilt down">
                                        ↓
                                    </button>
                                    <button type="button" onClick={() => setViewPreset("iso")} title="Isometric view">
                                        Iso
                                    </button>
                                </div>
                            </div>
                        )}

                        {rawCoordinatesCollapsed && (
                            <p className="canvas-warning">
                                All raw coordinates are identical. Nodes may overlap because the source file does not provide usable X/Y coordinates.
                            </p>
                        )}
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

        </section >
    )


}
