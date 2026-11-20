import { useEffect, useMemo, useState } from "react";

import type {
    ComparisonDataset,
    ComparisonDatasetLoader,
    ComparisonSourceDescriptor,
    ComparisonWorkspaceState,
    DomainSnapshotCompatibility
} from "./types";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import { DISPLAY_LENGTH_UNITS } from "../domain/unitSystem";
import { UnifiedComparisonTable } from "./UnifiedComparisonTable";
import { ComparisonMetricPanel } from "./ComparisonMetricPanel";
import { findSharedComparisonTable, getSharedComparisonTables } from "./comparisonTableCompatibility";
import { fetchDomainSnapshotCompatibility } from "../../api/projectApi";

type Props = {
    projectId: string;
    sources: ComparisonSourceDescriptor[];
    loadDataset: ComparisonDatasetLoader;
    initialWorkspace: ComparisonWorkspaceState | null;
    onWorkspaceChange: (state: ComparisonWorkspaceState) => void;
    onClose: () => void;
    displayUnit: DisplayLengthUnit;
    embedded?: boolean;

};

// Prefer the two latest persisted simulations for an immediately useful comparison.
function findInitialLeftSource(sources: ComparisonSourceDescriptor[]): string {
    const simulations = sources.filter(source => source.sourceType === "SIMULATION");
    if (simulations.length >= 2) {
        return simulations[simulations.length - 2].id;
    }
    return (sources.find(source => source.sourceType === "DEFAULT")?.id ?? sources[0]?.id ?? "");
}

// Prefer the two latest persisted simulations for an immediately useful comparison.
function findInitialRightSource(sources: ComparisonSourceDescriptor[]): string {

    const simulations = sources.filter(source => source.sourceType === "SIMULATION");

    // The latest persisted simulation is the most useful initial right-hand comparison source.
    return (simulations[simulations.length - 1]?.id ?? sources[1]?.id ?? sources[0]?.id ?? "");
}

// Resolve initial source from requested or fallback
function resolveInitialSource(requested: string | undefined, fallback: string, sources: ComparisonSourceDescriptor[]): string {
    if (requested && sources.some(source => source.id === requested)) {
        return requested;
    }

    return fallback;
}

function getComparisonApiErrorMessage(error: unknown, fallback: string): string {

    if (error instanceof Error && error.message.trim()) {
        return error.message;
    }

    return fallback;
}

//simulation comparison dialog component
export function SimulationComparisonDialog({ projectId, sources, loadDataset, initialWorkspace, onWorkspaceChange, onClose, displayUnit, embedded = false }: Props) {

    // initialize state from initialWorkspace if available
    const initialLeft = useMemo(() => findInitialLeftSource(sources), [sources]);
    const initialRight = useMemo(() => findInitialRightSource(sources), [sources]);
    const [leftSourceId, setLeftSourceId] = useState(() => resolveInitialSource(initialWorkspace?.leftSourceId, initialLeft, sources));
    const [rightSourceId, setRightSourceId] = useState(() => resolveInitialSource(initialWorkspace?.rightSourceId, initialRight, sources));

    const [sharedTableKey, setSharedTableKey] = useState(initialWorkspace?.tableKey ?? "");

    const [leftDataset, setLeftDataset] = useState<ComparisonDataset | null>(null);
    const [rightDataset, setRightDataset] = useState<ComparisonDataset | null>(null);
    const [leftLoading, setLeftLoading] = useState(false);
    const [rightLoading, setRightLoading] = useState(false);
    const [leftError, setLeftError] = useState<string | null>(null);
    const [rightError, setRightError] = useState<string | null>(null);
    const [compatibility, setCompatibility] = useState<DomainSnapshotCompatibility | null>(null);
    const [compatibilityLoading, setCompatibilityLoading] = useState(false);
    const [compatibilityError, setCompatibilityError] = useState<string | null>(null);

    const activeInputLabel = sources[0]?.inputLabel ?? "Current input";

    const sharedTables = useMemo(() => getSharedComparisonTables(
        leftDataset, rightDataset
    ),
        [leftDataset, rightDataset]
    );
    // Fetch the left dataset when the left source ID changes
    useEffect(() => {
        if (!leftSourceId) {
            setLeftDataset(null);
            return;
        }
        let disposed = false;
        setLeftLoading(true);
        setLeftError(null);

        loadDataset(leftSourceId)
            .then(dataset => {
                if (disposed) {
                    return;
                }
                setLeftDataset(dataset);
            }
            )
            .catch(error => {
                if (disposed) {
                    return;
                }
                console.error("Could not load left comparison source:", error);
                setLeftDataset(null);
                setLeftError("Could not load this comparison source.");
            }
            )
            .finally(() => {
                if (!disposed) {
                    setLeftLoading(false);
                }
            }
            );
        return () => {
            disposed = true;
        };

    }, [leftSourceId, loadDataset]);

    // Fetch the right dataset when the right source ID changes
    useEffect(() => {

        if (!rightSourceId) {
            setRightDataset(null);
            return;
        }
        let disposed = false;
        setRightLoading(true);
        setRightError(null);
        loadDataset(rightSourceId)
            .then(dataset => {
                if (disposed) {
                    return;
                }
                setRightDataset(dataset);
            }
            )
            .catch(error => {
                if (disposed) {
                    return;
                }
                console.error("Could not load right comparison source:", error);
                setRightDataset(null);
                setRightError("Could not load this comparison source.");
            }
            )
            .finally(() => {
                if (!disposed) {
                    setRightLoading(false);
                }
            }
            );
        return () => {
            disposed = true;
        };

    }, [rightSourceId, loadDataset]);

    // Close the dialog when the Escape key is pressed
    useEffect(() => {
        // The parent Hydraulic Review owns keyboard handling when comparison is embedded.
        if (embedded) {
            return;
        }

        function handleKeyDown(event: KeyboardEvent) {

            if (event.key === "Escape") {
                onClose();
            }
        }
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [embedded, onClose]);

    // Update the comparison workspace when the left or right source ID or table key changes
    useEffect(() => {
        if (!leftSourceId || !rightSourceId) {
            return;
        }
        onWorkspaceChange({
            leftSourceId,
            rightSourceId,
            tableKey: sharedTableKey,
        });


    }, [leftSourceId, rightSourceId, sharedTableKey, onWorkspaceChange]);

    // keep left and right source IDs in sync with the available sources.
    useEffect(() => {
        const leftStillExists = sources.some(source => source.id === leftSourceId);
        if (!leftSourceId || !leftStillExists) {
            setLeftSourceId(initialLeft);
        }
        const rightStillExists = sources.some(source => source.id === rightSourceId);
        if (!rightSourceId || !rightStillExists) {
            setRightSourceId(initialRight);
        }
    }, [sources, leftSourceId, rightSourceId, initialLeft, initialRight]);

    // Keep the shared table selection valid whenever either comparison source changes.
    useEffect(() => {
        const stillAvailable = sharedTables.some(table => table.key === sharedTableKey);
        if (stillAvailable) {
            return;
        }
        setSharedTableKey(sharedTables[0]?.key ?? "");
    }, [sharedTables, sharedTableKey]);

    useEffect(() => {
        if (!leftDataset || !rightDataset || !leftDataset.domainSnapshotId || !rightDataset.domainSnapshotId) {
            setCompatibility(null);
            return;
        }
        let disposed = false;
        setCompatibilityLoading(true);
        setCompatibilityError(null);
        fetchDomainSnapshotCompatibility(projectId, leftDataset.domainSnapshotId, rightDataset.domainSnapshotId)
            .then(result => {
                if (!disposed) {
                    setCompatibility(result);
                }
            }
            ).catch(error => {
                if (!disposed) {
                    setCompatibility(null);
                    setCompatibilityError(getComparisonApiErrorMessage(error, "Could not determine revision compatibility."));
                }
            }
            ).finally(() => {
                if (!disposed) {
                    setCompatibilityLoading(false);
                }
            });
        return () => {
            disposed = true;
        };
    }, [projectId, leftDataset?.domainSnapshotId, rightDataset?.domainSnapshotId]);

    const selectedLeftTable = findSharedComparisonTable(leftDataset, sharedTableKey);
    const selectedRightTable = findSharedComparisonTable(rightDataset, sharedTableKey);

    return (
        <div className={embedded ? ("comparison-backdrop comparison-backdrop--embedded") : "comparison-backdrop"} role={embedded ? undefined : "presentation"}>
            <section
                className={embedded ? "comparison-dialog comparison-dialog--embedded" : "comparison-dialog"}
                role={embedded ? undefined : "dialog"}
                aria-modal={embedded ? undefined : "true"}
                aria-label={embedded ? undefined : "Hydraulic comparison"}
            >
                {!embedded && (
                    <header className="comparison-dialog-header">
                        <div>
                            <h2> Hydraulic comparison </h2>
                            <p> Compare simulation results across network revisions.</p>
                            <div className="comparison-input-scope">
                                <span> Input data </span>
                                <strong> {activeInputLabel} </strong>
                                <small>Simulations from earlier revisions can be compared according to their network compatibility.</small>
                            </div>
                        </div>
                        <button
                            type="button"
                            className="comparison-close-button"
                            aria-label="Close comparison"
                            onClick={onClose}> × </button>

                    </header>
                )}
                <div className="comparison-dialog-content">
                    <div className="comparison-unified-controls">
                        <label className="comparison-select-field">
                            <span>Left simulation</span>
                            <select
                                value={leftSourceId}
                                onChange={event => setLeftSourceId(event.target.value)}>
                                {sources.map(
                                    source => (
                                        <option key={source.id} value={source.id} disabled={source.id === rightSourceId}>
                                            {source.label}
                                        </option>
                                    )
                                )}
                            </select>
                        </label>

                        <label className="comparison-select-field">
                            <span>Right simulation</span>
                            <select
                                value={rightSourceId}
                                onChange={event => setRightSourceId(event.target.value)}>
                                {sources.map(
                                    source => (
                                        <option key={source.id} value={source.id} disabled={source.id === leftSourceId}>
                                            {source.label}
                                        </option>
                                    )
                                )}
                            </select>
                        </label>

                        <label className="comparison-select-field">
                            <span>Table</span>
                            <select value={sharedTableKey} disabled={sharedTables.length === 0} onChange={event => setSharedTableKey(event.target.value)}>
                                {sharedTables.map(table => (
                                    <option key={table.key} value={table.key}>
                                        {table.label}
                                    </option>
                                ))}

                            </select>
                        </label>

                    </div>

                    {(leftLoading || rightLoading) && (
                        <div className="comparison-unified-state">
                            Loading comparison data...
                        </div>
                    )}
                    {!leftLoading && !rightLoading && (leftError || rightError) && (
                        <div className="comparison-unified-state error">
                            {leftError ?? rightError}
                        </div>
                    )}
                    {!leftLoading && !rightLoading && !leftError &&
                        !rightError && leftDataset && rightDataset &&
                        sharedTables.length === 0 && (
                            <div className="comparison-no-common-table">
                                <strong>No common table</strong>
                                <p>
                                    These sources do not expose a compatible table structure. Choose two simulation results that can be compared directly.
                                </p>
                            </div>
                        )}

                    {!leftLoading && !rightLoading && !leftError &&
                        !rightError && leftDataset && rightDataset &&
                        selectedLeftTable && selectedRightTable && (
                            <UnifiedComparisonTable
                                leftDataset={leftDataset}
                                leftTable={selectedLeftTable}
                                rightDataset={rightDataset}
                                rightTable={selectedRightTable}
                                displayUnit={displayUnit}
                                compatibility={compatibility}
                            />
                        )}

                    {compatibilityLoading && (
                        <div className="comparison-snapshot-banner">
                            Checking revision compatibility...
                        </div>
                    )}

                    {compatibilityError && (
                        <div className="comparison-snapshot-banner warning">
                            {compatibilityError}
                        </div>
                    )}

                    {compatibility
                        &&
                        compatibility.level ===
                        "SAME_SNAPSHOT"
                        && (
                            <div
                                className=
                                "comparison-snapshot-banner safe"
                            >
                                <strong>
                                    Exact same input snapshot
                                </strong>

                                <span>
                                    Both simulations were generated from
                                    exactly the same immutable network input.
                                </span>
                            </div>
                        )
                    }

                    {compatibility
                        &&
                        compatibility.level ===
                        "EQUIVALENT_INPUT"
                        && (
                            <div
                                className=
                                "comparison-snapshot-banner safe"
                            >
                                <strong>
                                    Equivalent network input
                                </strong>

                                <span>
                                    The simulations belong to different
                                    revisions, but their effective hydraulic
                                    input is identical.
                                </span>
                            </div>
                        )
                    }

                    {compatibility
                        &&
                        compatibility.level ===
                        "SAME_TOPOLOGY"
                        && (
                            <div
                                className=
                                "comparison-snapshot-banner revised"
                            >
                                <strong>
                                    Same topology — revised input
                                </strong>

                                <span>
                                    Node and pipe structure is unchanged,
                                    but hydraulic input values differ.
                                </span>
                            </div>
                        )
                    }

                    {compatibility
                        &&
                        compatibility.level ===
                        "TOPOLOGY_CHANGED"
                        && (
                            <div
                                className=
                                "comparison-snapshot-banner topology-changed"
                            >
                                <strong>
                                    Topology changed
                                </strong>

                                <span>
                                    Only entities that exist with the same
                                    identity in both revisions are compared
                                    directly.
                                </span>
                            </div>
                        )
                    }

                    {/* Backend warnings (input revision differs, demand redistribution, topology change, ...) */}
                    {compatibility
                        && compatibility.warnings.length > 0
                        && (
                            <div className="comparison-compatibility-warnings">
                                {compatibility.warnings.map(warning => (
                                    <p key={warning}>{warning}</p>
                                ))}
                            </div>
                        )
                    }

                    {compatibility
                        &&
                        compatibility.level ===
                        "TOPOLOGY_CHANGED"
                        && (
                            <>
                                {/* Revision summary: entity counts + matched/added/removed */}
                                <div
                                    className=
                                    "comparison-revision-summary"
                                >
                                    <div>
                                        <strong>Nodes</strong>

                                        <span>
                                            {
                                                compatibility
                                                    .changes
                                                    .leftNodeCount
                                            }
                                            {" → "}
                                            {
                                                compatibility
                                                    .changes
                                                    .rightNodeCount
                                            }
                                        </span>

                                        <small>
                                            Matched{" "}
                                            {
                                                compatibility
                                                    .changes
                                                    .matchedNodeCount
                                            }
                                            {" · +"}
                                            {
                                                compatibility
                                                    .changes
                                                    .addedNodes.length
                                            }
                                            {" · -"}
                                            {
                                                compatibility
                                                    .changes
                                                    .removedNodes.length
                                            }
                                        </small>
                                    </div>

                                    <div>
                                        <strong>Pipes</strong>

                                        <span>
                                            {
                                                compatibility
                                                    .changes
                                                    .leftPipeCount
                                            }
                                            {" → "}
                                            {
                                                compatibility
                                                    .changes
                                                    .rightPipeCount
                                            }
                                        </span>

                                        <small>
                                            Matched{" "}
                                            {
                                                compatibility
                                                    .changes
                                                    .matchedPipeCount
                                            }
                                            {" · +"}
                                            {
                                                compatibility
                                                    .changes
                                                    .addedPipes.length
                                            }
                                            {" · -"}
                                            {
                                                compatibility
                                                    .changes
                                                    .removedPipes.length
                                            }
                                        </small>
                                    </div>

                                    <div>
                                        <strong>Terminals</strong>

                                        <span>
                                            {
                                                compatibility
                                                    .changes
                                                    .leftTerminalCount
                                            }
                                            {" → "}
                                            {
                                                compatibility
                                                    .changes
                                                    .rightTerminalCount
                                            }
                                        </span>

                                        <small>
                                            Matched{" "}
                                            {
                                                compatibility
                                                    .changes
                                                    .matchedTerminalCount
                                            }
                                            {" · +"}
                                            {
                                                compatibility
                                                    .changes
                                                    .addedTerminals.length
                                            }
                                            {" · -"}
                                            {
                                                compatibility
                                                    .changes
                                                    .removedTerminals.length
                                            }
                                        </small>
                                    </div>
                                </div>

                                {/* Expandable structural changes */}
                                <details
                                    className=
                                    "comparison-revision-details"
                                >
                                    <summary>
                                        View structural changes
                                    </summary>

                                    {compatibility.changes.addedNodes.length > 0 && (
                                        <p>
                                            <strong>Added nodes: </strong>
                                            {
                                                compatibility
                                                    .changes
                                                    .addedNodes
                                                    .join(", ")
                                            }
                                        </p>
                                    )}

                                    {compatibility.changes.removedNodes.length > 0 && (
                                        <p>
                                            <strong>Removed nodes: </strong>
                                            {
                                                compatibility
                                                    .changes
                                                    .removedNodes
                                                    .join(", ")
                                            }
                                        </p>
                                    )}

                                    {compatibility.changes.changedNodeRoles.length > 0 && (
                                        <p>
                                            <strong>Changed roles: </strong>
                                            {
                                                compatibility
                                                    .changes
                                                    .changedNodeRoles
                                                    .join(", ")
                                            }
                                        </p>
                                    )}

                                    {compatibility.changes.addedPipes.length > 0 && (
                                        <p>
                                            <strong>Added pipes: </strong>
                                            {
                                                compatibility
                                                    .changes
                                                    .addedPipes
                                                    .join(", ")
                                            }
                                        </p>
                                    )}

                                    {compatibility.changes.removedPipes.length > 0 && (
                                        <p>
                                            <strong>Removed pipes: </strong>
                                            {
                                                compatibility
                                                    .changes
                                                    .removedPipes
                                                    .join(", ")
                                            }
                                        </p>
                                    )}

                                    {compatibility.changes.changedPipeEndpoints.length > 0 && (
                                        <p>
                                            <strong>Changed endpoints: </strong>
                                            {
                                                compatibility
                                                    .changes
                                                    .changedPipeEndpoints
                                                    .join(", ")
                                            }
                                        </p>
                                    )}
                                </details>
                            </>
                        )
                    }

                    {leftDataset &&
                        rightDataset &&
                        selectedLeftTable &&
                        selectedRightTable && compatibility && (
                            compatibility.directMetricComparisonAllowed
                            ||
                            compatibility.partialMetricComparisonAllowed
                        ) && (
                            <ComparisonMetricPanel
                                key={[leftDataset.id,
                                selectedLeftTable.key,
                                rightDataset.id,
                                selectedRightTable.key,
                                ].join("::")
                                }
                                leftDataset={leftDataset}
                                leftTable={selectedLeftTable}
                                rightDataset={rightDataset}
                                rightTable={selectedRightTable}
                                displayUnit={displayUnit}
                            />
                        )
                    }
                </div>

            </section>

        </div>
    );
}