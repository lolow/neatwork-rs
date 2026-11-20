
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { ProjectArtifact } from "./types";

type DeleteInputArtifactModalProps = {
    artifact: ProjectArtifact;
    deleting: boolean;
    error: string | null;

    onConfirm: () => Promise<void>;
    onCancel: () => void;
};

export function DeleteInputArtifactModal({
    artifact,
    deleting,
    error,
    onConfirm,
    onCancel,
}: DeleteInputArtifactModalProps) {

    const [confirmed, setConfirmed] = useState(false);

    useEffect(() => {
        setConfirmed(false);
    }, [artifact.id]);

    useEffect(() => {
        function handleEscape(event: KeyboardEvent) {
            if (event.key === "Escape" && !deleting) {
                onCancel();
            }
        }

        window.addEventListener("keydown", handleEscape);

        return () => {
            window.removeEventListener("keydown", handleEscape);
        };
    }, [deleting, onCancel]);

    async function handleDelete() {
        if (!confirmed || deleting) {
            return;
        }

        await onConfirm();
    }

    return createPortal(
        <div className="project-modal-backdrop project-modal-backdrop--danger" >
            <section
                className="project-modal"
                role="alertdialog"
                aria-modal="true"
                aria-labelledby="delete-input-title"
                aria-describedby="delete-input-description"
            >

                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow project-modal__eyebrow--danger">File deletion</span>
                        <h2 id="delete-input-title">Delete input file?</h2>
                    </div>

                    <button
                        type="button"
                        className="project-modal__close"
                        aria-label="Close"
                        disabled={deleting}
                        onClick={onCancel}
                    >
                        ×
                    </button>
                </header>

                <div className="project-modal__body">

                    <div className="project-delete-warning">
                        <div className="project-delete-warning__icon" aria-hidden="true">!</div>
                        <div>
                            <strong>{artifact.originalFilename}</strong>
                            <p id="delete-input-description">This file will be removed from the active project library.</p>
                        </div>
                    </div>

                    <div className="project-delete-content">
                        <p>Historical file content and previous simulation results will be preserved.</p>

                        <p>This file will no longer be available for new imports.</p>
                    </div>

                    <label className="project-delete-confirmation">
                        <input
                            type="checkbox"
                            checked={confirmed}
                            disabled={deleting}
                            onChange={event => setConfirmed(event.target.checked)}
                        />

                        <span>I understand that this file will be removed from the active project library.</span>
                    </label>

                    {error && (
                        <div className="project-modal__error" role="alert">
                            {error}
                        </div>
                    )}

                </div>

                <footer className="project-modal__footer">
                    <button
                        type="button"
                        className="project-modal__secondary"
                        disabled={deleting}
                        onClick={onCancel}
                    >
                        Cancel
                    </button>

                    <button
                        type="button"
                        className="project-modal__danger"
                        disabled={!confirmed || deleting}
                        onClick={() => void handleDelete()}
                    >
                        {deleting ? "Deleting..." : "Delete file"}
                    </button>
                </footer>

            </section>
        </div>,
        document.body
    );
}