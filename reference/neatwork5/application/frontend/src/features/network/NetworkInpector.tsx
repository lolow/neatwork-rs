import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import type { EdgeStatus, NetworkSelection } from "./types";
import {
    getDisplayUnitSymbol,
    getVelocityDisplayUnitSymbol,
    metersToDisplayUnit,
} from "../domain/unitSystem";
import type { DisplayLengthUnit } from "../domain/unitSystem";

type Props = {
    selection: NetworkSelection;
    onToggleEdgeStatus: (edgeId: string, nextStatus: EdgeStatus) => Promise<void>;
    onMoveNode: (nodeId: string, x: number, y: number) => Promise<void>;
    displayUnit: DisplayLengthUnit;
};

export function NetworkInspector({ selection, onToggleEdgeStatus, onMoveNode, displayUnit }: Props) {
    const [x, setX] = useState("");
    const [y, setY] = useState("");
    const [saving, setSaving] = useState(false);

    useEffect(() => {
        if (selection?.kind === "node") {
            setX(String(Math.round(selection.item.x)));
            setY(String(Math.round(selection.item.y)));
        }
    }, [selection]);

    if (!selection) {
        return (
            <aside className="network-inspector">
                <h2>Inspector</h2>
                <p>Select a node, pipe, or valve from the canvas.</p>
            </aside>
        );
    }

    async function handleMoveNode(event: FormEvent) {
        event.preventDefault();

        if (selection?.kind !== "node") return;

        const nextX = Number(x);
        const nextY = Number(y);

        if (!Number.isFinite(nextX) || !Number.isFinite(nextY)) return;

        setSaving(true);
        await onMoveNode(selection.item.id, nextX, nextY);
        setSaving(false);
    }

    async function handleToggleEdge() {
        if (selection?.kind !== "edge") return;

        const nextStatus = selection.item.status === "OPEN" ? "CLOSED" : "OPEN";

        setSaving(true);
        await onToggleEdgeStatus(selection.item.id, nextStatus);
        setSaving(false);
    }



    if (selection.kind === "node") {
        const node = selection.item;
        const unitSymbol = getDisplayUnitSymbol(displayUnit);

        // Handle node coordinate display visual or data
        const dataX = node.sourceX ?? node.x;
        const dataY = node.sourceY ?? node.y;
        const dataXDisplay = metersToDisplayUnit(dataX, displayUnit);
        const dataYDisplay = metersToDisplayUnit(dataY, displayUnit);
        const elevationDisplay = metersToDisplayUnit(node.elevationM ?? 0, displayUnit);
        const pressureDisplay = metersToDisplayUnit(node.pressureM, displayUnit);
        const graphX = node.x;
        const graphY = node.y;
        const showVisualCoords = node.layoutGenerated === true;

        return (
            <aside className="network-inspector">
                <h2>{node.label}</h2>

                <div className="inspector-grid">
                    <span>Type</span>
                    <strong>{node.type}</strong>

                    <span>X {unitSymbol}</span>
                    <strong>{dataXDisplay.toFixed(2)}</strong>

                    <span>Y {unitSymbol}</span>
                    <strong>{dataYDisplay.toFixed(2)}</strong>

                    {showVisualCoords && (
                        <>
                            <span>Visual X</span>
                            <strong>{graphX.toFixed(1)}</strong>

                            <span>Visual Y</span>
                            <strong>{graphY.toFixed(1)}</strong>
                        </>
                    )}

                    <span>Elevation {unitSymbol}</span>
                    <strong>{elevationDisplay.toFixed(2)}{" "}{unitSymbol}</strong>

                    <span>Pressure {unitSymbol}</span>
                    <strong>{pressureDisplay.toFixed(2)}{" "}{unitSymbol}</strong>

                    <span>Status</span>
                    <strong>{node.lowPressure ? "LOW PRESSURE" : "NORMAL"}</strong>
                </div>

                <form className="position-form" onSubmit={handleMoveNode}>
                    <label>
                        Visual graph X
                        <input value={x} onChange={(event) => setX(event.target.value)} />
                    </label>

                    <label>
                        Visual graph Y
                        <input value={y} onChange={(event) => setY(event.target.value)} />
                    </label>

                    <button disabled={saving}>{saving ? "Saving..." : "Save position"}</button>
                </form>
            </aside>
        );
    }

    const edge = selection.item;
    const hasSimulationResult =
        Math.abs(edge.flowRateLps) > 0 || Math.abs(edge.velocityMps) > 0;

    const unitSymbol = getDisplayUnitSymbol(displayUnit);
    const velocityUnit = getVelocityDisplayUnitSymbol(displayUnit);
    const velocityDisplay = metersToDisplayUnit(edge.velocityMps, displayUnit);
    const lengthDisplay = edge.lengthM == null ? null : metersToDisplayUnit(edge.lengthM, displayUnit);
    const innerDiameterDisplay = edge.innerDiameter == null ? null : metersToDisplayUnit(edge.innerDiameter, displayUnit);

    return (
        <aside className="network-inspector">
            <h2>{edge.id}</h2>

            <div className="inspector-grid">
                <span>Type</span>
                <strong>{edge.type}</strong>

                <span>Status</span>
                <strong>{edge.status}</strong>

                <span>Flow</span>
                <strong>{hasSimulationResult ? `${edge.flowRateLps.toFixed(2)} L/s` : "Not simulated"}</strong>

                <span>Velocity</span>
                <strong>{hasSimulationResult ? `${velocityDisplay.toFixed(2)} ${velocityUnit}` : "Not simulated"}</strong>

                <span>Length</span>
                <strong>{lengthDisplay != null ? `${lengthDisplay.toFixed(2)} ${unitSymbol}` : "-"}</strong>

                <span>Diameter ID</span>
                <strong>{edge.diameterId ?? "-"}</strong>

                <span>Nominal</span>
                <strong>{edge.nominalSize ?? "-"}</strong>

                <span>SDR</span>
                <strong>{edge.sdr != null ? edge.sdr.toFixed(2) : "-"}</strong>

                <span>Inner</span>
                <strong>{innerDiameterDisplay != null ? `${innerDiameterDisplay} ${unitSymbol}` : "-"}</strong>

                <span>Cost</span>
                <strong>{edge.cost != null ? edge.cost.toFixed(3) : "-"}</strong>

                <span>pmax</span>
                <strong>{edge.pmax != null ? edge.pmax.toFixed(1) : "-"}</strong>

                <span>Pipe type</span>
                <strong>{edge.pipeTypeCode ?? "-"}</strong>

                <span>Roughness</span>
                <strong>{edge.roughness != null ? edge.roughness.toFixed(2) : "-"}</strong>
            </div>

            {edge.type === "VALVE" && (
                <button className="danger-button" disabled={saving} onClick={handleToggleEdge}>
                    {saving ? "Saving..." : edge.status === "OPEN" ? "Close valve" : "Open valve"}
                </button>
            )}
        </aside>
    );
}