import type { CSSProperties } from "react";
import { NetworkInspector } from "./NetworkInpector";
import type { EdgeStatus, NetworkSelection } from "./types";
import type { DisplayLengthUnit } from "../domain/unitSystem";


export type FloatingInspectorAnchor = {
    x: number;
    y: number;
    horizontal: "left" | "right";
    vertical: "above" | "below";
};


type Props = {
    selection: NetworkSelection;
    anchor: FloatingInspectorAnchor | null;
    onClose: () => void;
    onToggleEdgeStatus: (edgeId: string, nextStatus: EdgeStatus) => Promise<void>;
    onMoveNode: (nodeId: string, x: number, y: number) => Promise<void>;
    displayUnit: DisplayLengthUnit;
};


/**
 * Display the existing network inspector near the selected canvas element.
 */
export function FloatingNetworkInspector({ selection, anchor, onClose, onToggleEdgeStatus, onMoveNode, displayUnit }: Props) {

    if (!selection || !anchor) {
        return null;
    }


    const translateX = anchor.horizontal === "left" ? "calc(-100% - 14px)" : "14px";
    const translateY = anchor.vertical === "above" ? "calc(-100% - 14px)" : "14px";


    const positionStyle: CSSProperties = {
        left: anchor.x,
        top: anchor.y,
        transform: `translate(${translateX}, ${translateY})`,
    };


    return (
        <div
            className="floating-network-inspector"
            style={positionStyle}
        >
            <div className="floating-network-inspector-surface">

                <button
                    type="button"
                    className="floating-inspector-close"
                    aria-label="Close inspector"
                    title="Close inspector"
                    onClick={onClose}
                >
                    ×
                </button>


                <NetworkInspector
                    selection={selection}
                    onToggleEdgeStatus={onToggleEdgeStatus}
                    onMoveNode={onMoveNode}
                    displayUnit={displayUnit}
                />

            </div>
        </div>
    );
}