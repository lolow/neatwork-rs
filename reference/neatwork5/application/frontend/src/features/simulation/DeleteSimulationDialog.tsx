import { useEffect, useId } from "react";
import { createPortal } from "react-dom";
import type { SimulationHistoryItem } from "./types";


type Props = {
    run: SimulationHistoryItem;
    deleting: boolean;
    error: string | null;
    onConfirm: () => Promise<void>;
    onCancel: () => void;
};


export function DeleteSimulationDialog({
    run,
    deleting,
    error,
    onConfirm,
    onCancel,
}: Props) {

    const titleId = useId();

    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !deleting) {
                onCancel();
            }
        }
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [deleting, onCancel]);

    return createPortal(
        <div className="project-modal-backdrop project-modal-backdrop--danger">

            <section className="project-modal simulation-delete-dialog"
                role="alertdialog" aria-modal="true" aria-labelledby={titleId}>

                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow project-modal__eyebrow--danger">Permanent action</span>
                        <h2 id={titleId}>Delete Simulation #{run.runNumber}?</h2>
                    </div>

                    <button type="button" className="project-modal__close"
                        disabled={deleting}
                        onClick={onCancel}>
                        ×
                    </button>
                </header>

                <div className="project-modal__body">
                    <div className="project-delete-warning">
                        <div className="project-delete-warning__icon" aria-hidden="true">!</div>
                        <div>
                            <strong>The simulation result will be permanently deleted.</strong>
                            <p>Its source project, DSG/TPO file and domain snapshot will not be deleted.</p>
                        </div>
                    </div>

                    <div className="simulation-delete-summary">
                        <span>Status<strong>{run.status}</strong></span>
                        {run.seed !== null && (<span>Seed<strong>{run.seed}</strong></span>)}
                        {run.scenarioCount !== null && (
                            <span>Scenarios<strong>{run.scenarioCount}</strong></span>
                        )}
                    </div>

                    {error && (
                        <div className="project-modal__error" role="alert">{error}</div>
                    )}
                </div>
                <footer className="project-modal__footer">
                    <button type="button" className="project-modal__secondary"
                        disabled={deleting} onClick={onCancel}>Cancel</button>

                    <button type="button" className="project-modal__danger"
                        disabled={deleting}
                        onClick={() => void onConfirm()}
                    >
                        {deleting ? "Deleting..." : "Delete Simulation"}
                    </button>
                </footer>
            </section>
        </div>,
        document.body
    );
}