import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ProjectArtifact } from "../artifacts/types";
import type { ProjectSummary } from "../projects/types";
import type { DatabaseType } from "../menu/AppMenuBar";

import { artifactDownloadUrl } from "../../api/artifactApi";


type DatabaseManagerModalProps = {
    type: DatabaseType;
    artifacts: ProjectArtifact[];
    projects: ProjectSummary[];
    loading: boolean;
    uploading: boolean;
    error: string | null;

    onUpload: (file: File) => Promise<void>;
    canUpload: boolean;
    onArchive: (artifact: ProjectArtifact) => Promise<void>;
    onClose: () => void;
};


export function DatabaseManagerModal({
    type,
    artifacts,
    projects,
    loading,
    uploading,
    error,
    onUpload,
    canUpload,
    onArchive,
    onClose,
}: DatabaseManagerModalProps) {

    const [search, setSearch] = useState("");
    const [pendingArchive, setPendingArchive] = useState<ProjectArtifact | null>(null);
    const [archiveConfirmed, setArchiveConfirmed] = useState(false);
    const [archivingId, setArchivingId] = useState<string | null>(null);
    const inputRef = useRef<HTMLInputElement>(null);

    const title = type === "DB_DIAMETERS_INPUT" ? "Diameters Database" : "Orifices Database";

    const projectNames = useMemo(() =>
        new Map(projects.map(project => [project.id, project.name,])),
        [projects]
    );

    const filteredArtifacts = useMemo(() => {
        const query = search.trim().toLowerCase();
        return artifacts.filter(artifact =>
            artifact.originalFilename.toLowerCase().includes(query)
            ||
            (projectNames.get(artifact.projectId) ?? "")
                .toLowerCase()
                .includes(query)
        );
    },
        [artifacts, projectNames, search]
    );


    useEffect(() => {
        function handleEscape(event: KeyboardEvent) {
            if (event.key === "Escape" && !uploading && !archivingId) {
                if (pendingArchive) {
                    setPendingArchive(null);
                    setArchiveConfirmed(false);
                } else {
                    onClose();
                }
            }
        }

        window.addEventListener("keydown", handleEscape);

        return () => {
            window.removeEventListener("keydown", handleEscape);
        };
    }, [onClose, uploading, archivingId, pendingArchive]);


    async function handleUpload(event: React.ChangeEvent<HTMLInputElement>) {
        const file = event.target.files?.[0];

        event.target.value = "";

        if (!file) {
            return;
        }

        await onUpload(file);
    }

    async function confirmArchive() {
        if (!pendingArchive || !archiveConfirmed || archivingId) {
            return;
        }
        setArchivingId(pendingArchive.id);

        try {
            await onArchive(pendingArchive);
            setPendingArchive(null);
            setArchiveConfirmed(false);
        } catch {
            // The parent displays the API error.
        } finally {
            setArchivingId(null);
        }
    }

    return createPortal(
        <div className="project-modal-backdrop">

            <section
                className="project-modal project-modal--wide database-manager"
                role="dialog"
                aria-modal="true"
                aria-labelledby="database-manager-title"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Database management</span>
                        <h2 id="database-manager-title"> {title} </h2>
                        <p>Browse and manage your database files.</p>
                    </div>

                    <button
                        type="button"
                        className="project-modal__close"
                        aria-label="Close"
                        disabled={uploading || archivingId !== null}
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>


                <div className="project-modal__body">

                    <div className="database-manager__toolbar">
                        <input
                            type="search"
                            placeholder="Search databases..."
                            value={search}
                            onChange={event => setSearch(event.target.value)}
                        />

                        <input
                            ref={inputRef}
                            type="file"
                            accept=".db"
                            hidden
                            onChange={event => void handleUpload(event)}
                        />

                        <button
                            type="button"
                            className="project-modal__primary"
                            disabled={!canUpload || uploading}
                            onClick={() => inputRef.current?.click()}
                        >
                            {uploading ? "Uploading..." : "Upload .db"}
                        </button>
                        {!canUpload && (<small> Open a project before uploading a database.</small>)}
                    </div>

                    {error && (<div className="project-modal__error"> {error} </div>)}

                    {loading ? (
                        <div className="input-library-status">Loading databases...</div>
                    ) : (
                        <div className="database-manager__list">

                            {filteredArtifacts.length === 0 && (
                                <div className="open-project-empty"> No database files found. </div>
                            )}

                            {filteredArtifacts.map(
                                artifact => (
                                    <article key={artifact.id} className="database-manager__item">
                                        <div>
                                            <strong> {artifact.originalFilename} </strong>
                                            <small>
                                                {projectNames.get(artifact.projectId) ?? "Unknown project"}
                                                {" · v"}
                                                {artifact.version}
                                                {" · "}
                                                {(artifact.sizeBytes / 1024).toFixed(1)}
                                                {" KB"}
                                            </small>
                                        </div>

                                        <div className="database-manager__actions">

                                            <a className="project-modal__secondary"
                                                href={artifactDownloadUrl(artifact.projectId, artifact.id)}
                                                download={artifact.originalFilename}
                                            >
                                                Download
                                            </a>

                                            <button
                                                type="button"
                                                className="project-modal__danger-outline"
                                                onClick={() => {
                                                    setPendingArchive(artifact);
                                                    setArchiveConfirmed(false);
                                                }}
                                            >
                                                Remove
                                            </button>
                                        </div>
                                    </article>
                                )
                            )}

                        </div>
                    )}

                </div>


                <footer className="project-modal__footer">
                    <button
                        type="button"
                        className="project-modal__secondary"
                        onClick={onClose}
                        disabled={uploading || archivingId !== null}
                    >
                        Close
                    </button>
                </footer>
            </section>


            {pendingArchive && (
                <div className="database-manager__confirm-backdrop">
                    <section
                        className="database-manager__confirm"
                        role="alertdialog"
                        aria-modal="true"
                        aria-labelledby="database-remove-title"
                    >
                        <h3 id="database-remove-title">Remove from Library?</h3>

                        <p>
                            {pendingArchive.originalFilename}
                            {" "}
                            will no longer appear in the database library. Existing project files and results will be preserved.
                        </p>

                        <label>
                            <input
                                type="checkbox"
                                checked={archiveConfirmed}
                                onChange={event => setArchiveConfirmed(event.target.checked)}
                            />

                            I understand that this file will be removed from the Library.
                        </label>

                        <div className="database-manager__actions">
                            <button
                                type="button"
                                className="project-modal__secondary"
                                disabled={archivingId !== null}
                                onClick={() => {
                                    setPendingArchive(null);
                                    setArchiveConfirmed(false);
                                }}
                            >
                                Cancel
                            </button>

                            <button
                                type="button"
                                className="project-modal__danger"
                                disabled={!archiveConfirmed || archivingId !== null}
                                onClick={() => void confirmArchive()}
                            >
                                {archivingId ? "Removing..." : "Remove"}
                            </button>
                        </div>
                    </section>
                </div>
            )}

        </div>,
        document.body
    );
}