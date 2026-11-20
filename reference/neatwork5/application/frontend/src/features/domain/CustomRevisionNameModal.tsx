import { useMemo, useState } from "react";
import type { FormEvent } from "react";
import { createPortal } from "react-dom";
import type { ProjectArtifact } from "../artifacts/types";


type Props = {
    sourceArtifact: ProjectArtifact;
    saving: boolean;
    onBack: () => void;
    onSave: (filename: string) => void;
    onCancel: () => void;
};

function buildSuggestedName(filename: string): string {
    const dot = filename.lastIndexOf(".");

    const extension = dot >= 0 ? filename.substring(dot) : "";
    const base = dot >= 0 ? filename.substring(0, dot) : filename;

    return base.replace(/_v\d+$/i, "") + "_edited" + extension;
}


export function CustomRevisionNameModal({
    sourceArtifact,
    saving,
    onBack,
    onSave,
    onCancel,
}: Props) {

    const suggestion = useMemo(() => buildSuggestedName(sourceArtifact.originalFilename), [sourceArtifact.originalFilename]);
    const [filename, setFilename] = useState(suggestion);

    function handleSubmit(event: FormEvent) {
        event.preventDefault();

        const value = filename.trim();

        if (!value || saving) {
            return;
        }

        onSave(value);
    }

    return createPortal(
        <div className="project-modal-backdrop">
            <form className="project-modal" role="dialog" aria-modal="true" onSubmit={handleSubmit}>
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Custom revision</span>
                        <h2>Choose file name</h2>
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
                        <small>This file will remain unchanged.</small>
                    </div>
                    <label className="project-modal__field">
                        <span>New file name</span>
                        <input
                            type="text"
                            value={filename}
                            disabled={saving}
                            autoFocus
                            onChange={event => setFilename(event.target.value)}
                        />
                    </label>
                    <p className="project-modal__hint">If the extension is omitted, NeatWork adds the correct one automatically.
                    </p>
                </div>

                <footer className="project-modal__footer">
                    <button type="button" className="project-modal__secondary"
                        disabled={saving}
                        onClick={onBack}
                    >
                        Back
                    </button>

                    <button type="submit" className="project-modal__primary"
                        disabled={saving || !filename.trim()}
                    >
                        {saving ? "Saving..." : "Save new version"}
                    </button>
                </footer>
            </form>
        </div>,
        document.body
    );
}