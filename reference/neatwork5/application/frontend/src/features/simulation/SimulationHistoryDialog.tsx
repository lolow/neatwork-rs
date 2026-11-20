import { useMemo, useState } from "react";
import { createPortal } from "react-dom";
import type { SimulationHistoryItem } from "./types";


type Props = {
    runs: SimulationHistoryItem[];
    loading: boolean;
    cancellingId: string | null;
    error: string | null;
    onViewDetails: (run: SimulationHistoryItem) => void;
    onViewResults: (run: SimulationHistoryItem) => void;
    onCancelRun: (run: SimulationHistoryItem) => Promise<void>;
    onRequestDelete: (run: SimulationHistoryItem) => void;
    onClose: () => void;
};

function isActive(status: SimulationHistoryItem["status"]) {
    return (
        status === "PENDING"
        ||
        status === "RUNNING"
        ||
        status === "CANCEL_REQUESTED"
    );
}

function canCancel(status: SimulationHistoryItem["status"]) {
    return (
        status === "PENDING"
        ||
        status === "RUNNING"
    );
}

function canDelete(status: SimulationHistoryItem["status"]) {

    return (
        status === "COMPLETED"
        ||
        status ===
        "COMPLETED_WITH_ERRORS"
        ||
        status === "FAILED"
        ||
        status === "CANCELLED"
    );
}

function formatDate(value: string) {
    return new Date(value).toLocaleString();
}

export function SimulationHistoryDialog({
    runs,
    loading,
    cancellingId,
    error,
    onViewDetails,
    onViewResults,
    onCancelRun,
    onRequestDelete,
    onClose,
}: Props) {
    const [search, setSearch] = useState("");

    const filteredRuns = useMemo(() => {
        const query = search.trim().toLowerCase();
        if (!query) {
            return [...runs].reverse();
        }
        return runs.filter(run => {
            return (
                String(run.runNumber).includes(query) ||
                run.status.toLowerCase().includes(query) ||
                (run.seed != null && String(run.seed).includes(query))
            );
        })
            .reverse();
    },
        [runs, search]
    );

    function hasResults(status: SimulationHistoryItem["status"]): boolean {
        return status === "COMPLETED" || status === "COMPLETED_WITH_ERRORS";
    }

    return createPortal(
        <div className="project-modal-backdrop">
            <section className="project-modal project-modal--wide simulation-history-dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="simulation-history-title"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Project simulations</span>
                        <h2 id="simulation-history-title">Simulation History</h2>
                        <p>Running, completed,failed and cancelled executions.</p>
                    </div>
                    <button type="button"
                        className="project-modal__close"
                        aria-label="Close"
                        onClick={onClose}>
                        ×
                    </button>
                </header>
                <div className="project-modal__body">
                    <div className="simulation-history-toolbar">
                        <input
                            type="search"
                            placeholder="Search simulation number, status or seed..."
                            value={search}
                            onChange={event => setSearch(event.target.value)}
                        />
                        <span>{runs.length} executions</span>
                    </div>
                    {error && (
                        <div className="project-modal__error" role="alert">{error}</div>
                    )}

                    {loading ? (
                        <div className="input-library-status">Loading simulation history...</div>
                    ) : filteredRuns.length === 0 ? (
                        <div className="open-project-empty">No simulations have been recorded for this project.</div>
                    ) : (
                        <div className="simulation-history-list">
                            {filteredRuns.map(
                                run => {
                                    const active = isActive(run.status);
                                    const cancelling = cancellingId === run.id;
                                    return (
                                        <article key={run.id}
                                            className={active
                                                ? "simulation-history-item active"
                                                : "simulation-history-item"
                                            }>
                                            <div className="simulation-history-item__number">
                                                <strong>#{run.runNumber}</strong>
                                                <span>Simulation</span>
                                            </div>
                                            <div className="simulation-history-item__main">
                                                <div className="simulation-history-item__top">
                                                    <span className={`simulation-status-badge status-${run.status.toLowerCase()}`}>{run.status}</span>
                                                    <time>{formatDate(run.createdAt)}</time>
                                                </div>
                                                <div className="simulation-history-input">
                                                    <strong>
                                                        {run.inputName ?? run.inputFilename ?? "Unknown input"}
                                                    </strong>
                                                    {run.inputFilename && (
                                                        <span>
                                                            {run.inputFilename}
                                                            {run.inputVersion !== null ? ` · v${run.inputVersion}` : ""}
                                                            {run.inputSourceFormat ? ` · ${run.inputSourceFormat}` : ""}
                                                        </span>
                                                    )}

                                                    <span
                                                        className={
                                                            run.configurationMode === "DEFAULT"
                                                                ? "simulation-config-badge default"
                                                                : "simulation-config-badge manual"
                                                        }
                                                    >
                                                        {run.configurationMode === "DEFAULT"
                                                            ? "Default"
                                                            : "Manual"
                                                        }
                                                    </span>

                                                </div>
                                                <div className="simulation-history-meta">
                                                    <span>Scenarios <strong>{run.scenarioCount ?? "—"}</strong></span>
                                                    <span>Seed <strong>{run.seed ?? "—"}</strong></span>
                                                </div>
                                                {run.errorMessage && (
                                                    <div className="simulation-history-error">{run.errorMessage}</div>
                                                )}
                                            </div>
                                            {hasResults(run.status) && (
                                                <button
                                                    type="button"
                                                    className="project-modal__primary"
                                                    onClick={() => onViewResults(run)}
                                                >
                                                    Results
                                                </button>
                                            )}
                                            <button
                                                type="button"
                                                className="project-modal__secondary"
                                                onClick={() => onViewDetails(run)}
                                            >
                                                Details
                                            </button>
                                            <div className="simulation-history-item__actions">
                                                {canCancel(run.status) && (
                                                    <button type="button" className="project-modal__secondary"
                                                        disabled={cancellingId !== null}
                                                        onClick={() => void onCancelRun(run)}>
                                                        {cancelling ? "Cancelling..." : "Cancel"}
                                                    </button>
                                                )}

                                                {run.status === "CANCEL_REQUESTED" && (
                                                    <span className="simulation-history-pending-action">Cancelling…</span>
                                                )}
                                                {canDelete(run.status) && (
                                                    <button type="button"
                                                        className="project-modal__danger-outline"
                                                        onClick={() => onRequestDelete(run)}>
                                                        Delete
                                                    </button>
                                                )}
                                            </div>
                                        </article>
                                    );
                                }
                            )}

                        </div>
                    )}
                </div>

                <footer className="project-modal__footer">
                    <button
                        type="button"
                        className="project-modal__secondary"
                        onClick={onClose}
                    >
                        Close
                    </button>
                </footer>
            </section>
        </div>,
        document.body
    );
}