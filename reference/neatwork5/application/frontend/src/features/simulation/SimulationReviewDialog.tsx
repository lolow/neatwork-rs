import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import type { SimulationRunDetails, SimulationRunSummary } from "./types";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import type { ComparisonDatasetLoader, ComparisonSourceDescriptor, ComparisonWorkspaceState } from "../comparison/types";
import { SimulationResultsDialog } from "./SimulationResultsDialog";
import { SimulationComparisonDialog } from "../comparison/SimulationComparisonDialog";
import { buildSimulationResultTables } from "./simulationResultTables";


export type SimulationReviewMode = "SINGLE" | "COMPARISON";

type Props = {
    projectId: string;
    mode: SimulationReviewMode;
    onModeChange: (mode: SimulationReviewMode) => void;
    // Every completed simulation in the project can be inspected individually.
    runs: SimulationRunSummary[];
    selectedRunId: string | null;
    onSelectRun: (runId: string) => void;
    details: SimulationRunDetails | null;
    loading: boolean;
    error: string | null;
    // Comparison sources are intentionally restricted to simulations generated from the active input snapshot.
    comparisonSources: ComparisonSourceDescriptor[];
    loadComparisonDataset: ComparisonDatasetLoader;
    comparisonWorkspace: ComparisonWorkspaceState | null;
    onComparisonWorkspaceChange: (state: ComparisonWorkspaceState) => void;
    displayUnit: DisplayLengthUnit;
    onClose: () => void;
};

// Provides one shared workspace for inspecting one persisted simulation or comparing two simulations.
export function SimulationReviewDialog({
    projectId,
    mode,
    onModeChange,
    runs,
    selectedRunId,
    onSelectRun,
    details,
    loading,
    error,
    comparisonSources,
    loadComparisonDataset,
    comparisonWorkspace,
    onComparisonWorkspaceChange,
    displayUnit,
    onClose,
}: Props) {

    const canCompare = comparisonSources.length >= 2;
    useEffect(() => {
        const previousOverflow = document.body.style.overflow;
        document.body.style.overflow = "hidden";
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape") {
                onClose();
            }
        }
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
            document.body.style.overflow = previousOverflow;
        };
    },
        [onClose]
    );

    const singleTables = useMemo(() => details?.statistics
        ? buildSimulationResultTables(details.statistics)
        : [], [details]
    );

    const [selectedSingleTableKey, setSelectedSingleTableKey] = useState<string>("");

    useEffect(() => {
        if (singleTables.length === 0) {
            setSelectedSingleTableKey("");
            return;
        }
        const stillExists = singleTables.some(table => table.key === selectedSingleTableKey);
        if (!stillExists) {
            setSelectedSingleTableKey(singleTables[0].key);
        }
    }, [singleTables, selectedSingleTableKey]);

    return createPortal(
        <div className="simulation-review-backdrop">
            <section className="simulation-review-dialog" role="dialog" aria-modal="true" aria-labelledby="simulation-review-title">
                <header className="simulation-review-header">
                    <div>
                        <span className="simulation-review-eyebrow">Hydraulic review</span>
                        <h2 id="simulation-review-title">Review simulations</h2>
                        <p>Inspect one persisted simulation or compare two simulations side by side.</p>
                    </div>
                    <button type="button"
                        className="simulation-review-close"
                        aria-label="Close review"
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>
                <div className="simulation-review-mode-switch" role="tablist" aria-label="Review mode">
                    <button type="button"
                        role="tab"
                        aria-selected={mode === "SINGLE"}
                        className={mode === "SINGLE" ? "active" : ""}
                        onClick={() => onModeChange("SINGLE")}
                    >
                        Single simulation
                    </button>
                    <button
                        type="button"
                        role="tab"
                        aria-selected={mode === "COMPARISON"}
                        className={mode === "COMPARISON" ? "active" : ""}
                        disabled={!canCompare}
                        title={canCompare
                            ? undefined : ("At least two completed simulations are required.")
                        }
                        onClick={() => onModeChange("COMPARISON")}
                    >
                        Comparison
                    </button>

                    {!canCompare && (
                        <span className="simulation-review-mode-hint">
                            At least two completed simulations are required to enable comparison.
                        </span>
                    )}
                </div>
                <div className="simulation-review-content">
                    {mode === "SINGLE" && (<section className="simulation-review-single">
                        <div className="simulation-review-single-controls">
                            <label className="comparison-select-field">
                                <span>Simulation</span>
                                <select
                                    value={selectedRunId ?? ""}
                                    disabled={runs.length === 0}
                                    onChange={event => onSelectRun(event.target.value)}
                                >
                                    {runs.map(run => (
                                        <option key={run.id} value={run.id}>
                                            {`Simulation #${run.runNumber}`}
                                        </option>
                                    ))}
                                </select>
                            </label>

                            <label className="comparison-select-field">
                                <span>Table</span>
                                <select
                                    value={selectedSingleTableKey}
                                    disabled={singleTables.length === 0}
                                    onChange={event => setSelectedSingleTableKey(event.target.value)}
                                >
                                    {singleTables.map(table => (
                                        <option key={table.key} value={table.key}>
                                            {table.title}
                                        </option>
                                    ))}
                                </select>
                            </label>
                        </div>
                        {runs.length === 0 ? (
                            <div className="simulation-review-empty">
                                No completed simulation is available yet.
                            </div>
                        ) : (
                            <SimulationResultsDialog
                                embedded
                                details={details}
                                loading={loading}
                                error={error}
                                displayUnit={displayUnit}
                                onClose={onClose}
                                selectedTableKey={selectedSingleTableKey}
                            />
                        )}
                    </section>
                    )}
                    {mode === "COMPARISON" && canCompare && (
                        <SimulationComparisonDialog
                            projectId={projectId}
                            embedded
                            sources={comparisonSources}
                            loadDataset={loadComparisonDataset}
                            initialWorkspace={comparisonWorkspace}
                            onWorkspaceChange={onComparisonWorkspaceChange}
                            onClose={onClose}
                            displayUnit={displayUnit}
                        />
                    )}
                </div>
            </section>
        </div>,

        document.body
    );
}