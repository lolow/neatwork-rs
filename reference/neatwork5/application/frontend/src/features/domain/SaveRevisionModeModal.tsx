import { createPortal } from "react-dom";
import type { ProjectArtifact } from "../artifacts/types";


type Props = {
    sourceArtifact: ProjectArtifact;
    saving: boolean;
    error?: string | null;
    onAutomaticSave: () => void;
    onCustomSave: () => void;
    onCancel: () => void;
};

export function SaveRevisionModeModal({
    sourceArtifact,
    saving,
    error,
    onAutomaticSave,
    onCustomSave,
    onCancel,
}: Props) {

    return createPortal(
        <div className="project-modal-backdrop">
            <section className="project-modal save-revision-modal" role="dialog" aria-modal="true">
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Save revision</span>
                        <h2>Save edited input</h2>
                        <p>The original file will not be modified.</p>
                    </div>

                    <button type="button" className="project-modal__close"
                        disabled={saving}
                        onClick={onCancel}
                    >
                        ×
                    </button>
                </header>

                <div className="project-modal__body">
                    <div className="revision-source-file">
                        <span>Original file</span>
                        <strong>{sourceArtifact.originalFilename}</strong>
                        <small>Revision {sourceArtifact.version}</small>
                    </div>

                    {error && (
                        <div className="project-modal__error">
                            {error}
                        </div>
                    )}
                    <div className="revision-save-options">
                        <button type="button" className="revision-save-option"
                            disabled={saving}
                            onClick={onAutomaticSave}
                        >
                            <strong>Save automatically</strong>
                            <span>NeatWork creates the next revision filename automatically.</span>
                            <small>Recommended</small>
                        </button>

                        <button type="button" className="revision-save-option"
                            disabled={saving}
                            onClick={onCustomSave}
                        >
                            <strong>Choose file name</strong>
                            <span>Enter the visible filename for the new revision.</span>
                        </button>

                    </div>

                </div>

                <footer className="project-modal__footer">
                    <button type="button" className="project-modal__secondary"
                        disabled={saving}
                        onClick={onCancel}
                    >
                        Cancel
                    </button>
                </footer>
            </section>

        </div>,
        document.body
    );
}