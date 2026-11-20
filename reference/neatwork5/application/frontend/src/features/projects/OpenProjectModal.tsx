import {
    useEffect,
    useMemo,
    useState,
} from "react";

import { createPortal } from "react-dom";

import type {
    ProjectSummary
} from "./types";

type OpenProjectModalProps = {
    projects: ProjectSummary[];
    loading: boolean;
    loadError: string | null;
    selectedProjectId: string | null;
    onSelect: (projectId: string) => void;
    onRequestDelete: (project: ProjectSummary) => void;
    onRetry: () => void;
    onClose: () => void;
};

export function OpenProjectModal({ projects, loading, loadError, selectedProjectId, onSelect, onRequestDelete, onRetry, onClose, }: OpenProjectModalProps) {
    const [search, setSearch] = useState("");

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
            document.body.style.overflow = previousOverflow;
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [onClose]);

    const filteredProjects = useMemo(() => {
        const query = search.trim().toLocaleLowerCase();

        if (!query) {
            return projects;
        }

        return projects.filter(project =>
            project.name.toLocaleLowerCase().includes(query)
        );
    }, [projects, search]);

    function handleOpenProject(projectId: string) {
        onSelect(projectId);
        onClose();
    }

    return createPortal(
        <div className="project-modal-backdrop" onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
                onClose();
            }
        }}>
            <section
                className="project-modal project-modal--wide"
                role="dialog"
                aria-modal="true"
                aria-labelledby="open-project-title"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Project</span>
                        <h2 id="open-project-title">Open Project</h2>
                        <p>Select an existing project to continue working on it.</p>
                    </div>
                    <button
                        type="button"
                        className="project-modal__close"
                        aria-label="Close"
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>

                <div className="project-modal__body">
                    {loadError && (
                        <div className="project-modal__error" role="alert">
                            {loadError}
                            <button type="button" onClick={onRetry} disabled={loading}>Retry</button>
                        </div>
                    )}
                    <label className="project-search">
                        <span className="sr-only">Search projects</span>
                        <input
                            type="search"
                            placeholder="Search projects..."
                            value={search}
                            onChange={(event) => setSearch(event.target.value)}
                        />
                    </label>

                    <div className="open-project-list">
                        {loading && filteredProjects.length === 0 && (
                            <div className="open-project-empty">Loading projects...</div>
                        )}

                        {!loading && !loadError && filteredProjects.length === 0 && (
                            <div className="open-project-empty">
                                {projects.length === 0
                                    ? "No projects found in the connected database."
                                    : "No project matches your search."}
                            </div>
                        )}

                        {filteredProjects.map(project => {
                            const active = project.id === selectedProjectId;

                            return (
                                <article key={project.id} className={
                                    active
                                        ? "open-project-item active"
                                        : "open-project-item"
                                }>
                                    <div className="open-project-item__info">
                                        <strong>{project.name}</strong>
                                        <span> Created {new Date(project.createdAt).toLocaleDateString()}</span>
                                        {active && (
                                            <small>Currently open</small>
                                        )}
                                    </div>

                                    <div className="open-project-item__actions">
                                        <button
                                            type="button"
                                            className="project-modal__secondary"
                                            onClick={() => handleOpenProject(project.id)}
                                        >
                                            Open
                                        </button>

                                        <button
                                            type="button"
                                            className="project-modal__danger-outline"
                                            onClick={() => onRequestDelete(project)}
                                        >
                                            Delete
                                        </button>
                                    </div>
                                </article>
                            );
                        })}
                    </div>
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