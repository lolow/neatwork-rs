import { useEffect, useMemo, useState, ChangeEvent, useRef } from "react";
import { createPortal } from "react-dom";

import type { ProjectArtifact } from "../artifacts/types";
import type { ProjectSummary } from "../projects/types";

import type { NetworkInputFormat } from "./InputTypeModal";


type InputFileLibraryModalProps = {
    format: NetworkInputFormat;
    artifacts: ProjectArtifact[];
    projects: ProjectSummary[];
    selectedProjectId: string;
    loading: boolean;
    importingArtifactId: string | null;
    error: string | null;
    onImport: (artifact: ProjectArtifact) => Promise<void>;
    onBack: () => void;
    onClose: () => void;
    // file upload (copy from CreateNewModal.tsx)
    uploading: boolean;
    onUpload: (file: File) => Promise<void>;
    onRequestDelete: (artifact: ProjectArtifact) => void;
};

export function InputFileLibraryModal({
    format,
    artifacts,
    projects,
    selectedProjectId,
    loading,
    importingArtifactId,
    error,
    onImport,
    onBack,
    onClose,
    onUpload,
    uploading,
    onRequestDelete,
}: InputFileLibraryModalProps) {
    const [search, setSearch] = useState("");
    // Create a file input for uploading a local file.
    const uploadInputRef = useRef<HTMLInputElement>(null);

    async function handleUpload(event: ChangeEvent<HTMLInputElement>) {
        const file = event.target.files?.[0];

        event.target.value = "";

        if (!file) {
            return;
        }

        await onUpload(file);
    }

    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !importingArtifactId) {
                onClose();
            }
        }
        window.addEventListener("keydown", handleKeyDown);

        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [importingArtifactId, onClose]);

    const projectNames = useMemo(() => {
        return new Map(projects.map(project => [project.id, project.name]));
    }, [projects]);

    const filteredArtifacts = useMemo(() => {
        const query = search.trim().toLocaleLowerCase();

        if (!query) {
            return artifacts;
        }

        return artifacts.filter(
            artifact => {
                const projectName = projectNames.get(artifact.projectId) ?? "";

                return (
                    artifact.originalFilename.toLocaleLowerCase().includes(query)
                    ||
                    projectName.toLocaleLowerCase().includes(query)
                );
            }
        );
    }, [artifacts, projectNames, search]);

    return createPortal(
        <div className="project-modal-backdrop">
            <section
                className="project-modal project-modal--wide input-library-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="input-library-title"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">{format} input</span>
                        <h2 id="input-library-title">Select {format} file</h2>
                        <p>Select an existing file from the artifact library.</p>
                    </div>

                    <button
                        type="button"
                        className="project-modal__close"
                        disabled={importingArtifactId !== null}
                        onClick={onClose}>
                        ×
                    </button>
                </header>

                <div className="project-modal__body">

                    <label className="project-search">
                        <span className="sr-only">Search input files</span>
                        <input
                            type="search"
                            placeholder={`Search ${format} files...`}
                            value={search}
                            disabled={importingArtifactId !== null}
                            onChange={(event) => setSearch(event.target.value)}
                        />
                    </label>

                    <div className="input-library-toolbar">
                        <span>Existing {format} files</span>
                        <input
                            ref={uploadInputRef}
                            type="file"
                            hidden
                            accept={format === "DSG" ? ".dsg" : ".tpo"}
                            onChange={event => void handleUpload(event)}
                        />

                        <button
                            type="button"
                            className="project-modal__secondary"
                            disabled={uploading || importingArtifactId !== null}
                            onClick={() => uploadInputRef.current?.click()}
                        >
                            {uploading ? "Uploading..." : `Upload new ${format}`}
                        </button>
                    </div>

                    {error && (<div className="project-modal__error">{error}</div>)}

                    {loading && (<div className="input-library-status">Loading files...</div>)}

                    {!loading && filteredArtifacts.length === 0 && (
                        <div className="open-project-empty">
                            No {format} files are available.
                        </div>
                    )}

                    {!loading && (
                        <div className="input-library-list">
                            {filteredArtifacts.map(
                                artifact => {
                                    const isCurrentProject = artifact.projectId === selectedProjectId;
                                    const importing = importingArtifactId === artifact.id;

                                    return (
                                        <article key={artifact.id} className="input-library-item">
                                            <div className="input-library-item__file">
                                                <strong>{artifact.originalFilename}</strong>

                                                <span>{projectNames.get(artifact.projectId) ?? "Unknown project"}</span>

                                                <small>
                                                    v{artifact.version}
                                                    {" · "}
                                                    {(artifact.sizeBytes / 1024).toFixed(1)}{" KB"}
                                                    {" · "}
                                                    {new Date(artifact.createdAt).toLocaleDateString()}
                                                </small>

                                                {isCurrentProject && <em>Already in this project</em>}
                                            </div>

                                            <div className="input-library-item__actions">

                                                <button
                                                    type="button"
                                                    className="project-modal__primary"
                                                    disabled={loading || uploading || importingArtifactId !== null}
                                                    onClick={() => void onImport(artifact)}
                                                >
                                                    {importing ? "Importing..." : "Import"}
                                                </button>
                                                {artifact.projectId === selectedProjectId && (
                                                    <button
                                                        type="button"
                                                        className="project-modal__danger-outline"
                                                        disabled={
                                                            loading ||
                                                            uploading ||
                                                            importingArtifactId !== null
                                                        }
                                                        onClick={() =>
                                                            onRequestDelete(artifact)
                                                        }
                                                    >
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
                        disabled={importingArtifactId !== null}
                        onClick={onBack}
                    >
                        Back
                    </button>

                    <button
                        type="button"
                        className="project-modal__secondary"
                        disabled={importingArtifactId !== null}
                        onClick={onClose}
                    >
                        Close
                    </button>
                </footer>
            </section >
        </div >,
        document.body
    );
}