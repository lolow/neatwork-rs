import {
    useEffect,
    useState,
} from "react";

import { createPortal } from "react-dom";

import type { ProjectSummary } from "./types";

type DeleteProjectConfirmModalProps = {
    project: ProjectSummary;
    deleting: boolean;

    onCancel: () => void;
    onConfirm: () => Promise<void>;
};

export function DeleteProjectConfirmModal({ project, deleting, onCancel, onConfirm }: DeleteProjectConfirmModalProps) {
    const [confirmed, setConfirmed] = useState(false);

    useEffect(() => {
        const previousOverflow = document.body.style.overflow;

        document.body.style.overflow = "hidden";
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !deleting) {
                onCancel();
            }
        }
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            document.body.style.overflow = previousOverflow;
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [deleting, onCancel]);

    async function handleConfirm() {
        if (!confirmed || deleting) {
            return;
        }

        await onConfirm();
    }

    return createPortal(
        <div className="project-modal-backdrop project-modal-backdrop--danger" onMouseDown={(event) => {
            if (event.target === event.currentTarget && !deleting) {
                onCancel();
            }
        }}
        >
            <section
                className="project-modal project-delete-modal"
                role="alertdialog"
                aria-modal="true"
                aria-labelledby="delete-project-title"
                aria-describedby="delete-project-description"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow project-modal__eyebrow--danger">Permanent action</span>
                        <h2 id="delete-project-title">Delete Project?</h2>
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
                        <div className="project-delete-warning__icon" aria-hidden="true">
                            !
                        </div>

                        <div>
                            <strong>You are about to permanently delete “{project.name}”.</strong>
                            <p id="delete-project-description">
                                Deleting this project also removes all data associated with it.
                            </p>
                        </div>
                    </div>

                    <div className="project-delete-content">
                        <p>
                            This includes the project's network data, imported files, artifacts,
                            domain snapshots, simulations and generated results.
                        </p>

                        <p>This action cannot be undone.</p>
                    </div>

                    <label className="project-delete-confirmation">
                        <input
                            type="checkbox"
                            checked={confirmed}
                            disabled={deleting}
                            onChange={(event) =>
                                setConfirmed(
                                    event.target.checked
                                )
                            }
                        />

                        <span>
                            I understand that this project and all its associated files, artifacts,
                            imported data, simulations and results will be permanently deleted.
                        </span>
                    </label>
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
                        onClick={handleConfirm}
                    >
                        {deleting ? "Deleting..." : "Delete Project"}
                    </button>
                </footer>
            </section>
        </div>,
        document.body
    );
}