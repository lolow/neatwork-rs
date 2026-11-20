import { FormEvent, useState } from "react";
import type { ProjectSummary } from "./types";

type Props = {
    projects: ProjectSummary[];
    selectedProjectId: string | null;
    onSelectProject: (projectId: string) => void;
    onCreateProject: (name: string) => Promise<void>;
    onDeleteProject: (project: ProjectSummary) => Promise<void>;
};

/**
 * Display project creation, selection and deletion controls.
 */
export function ProjectSidebar({
    projects,
    selectedProjectId,
    onSelectProject,
    onCreateProject,
    onDeleteProject
}: Props) {
    const [name, setName] = useState("");
    const [saving, setSaving] = useState(false);
    const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent) {
        event.preventDefault();

        const trimmedName = name.trim();
        if (!trimmedName) return;

        setSaving(true);
        try {
            await onCreateProject(trimmedName);
            setName("");
        } finally {
            setSaving(false);
        }
    }

    async function handleDelete(project: ProjectSummary) {
        setDeletingProjectId(project.id);
        try {
            await onDeleteProject(project);
        } finally {
            setDeletingProjectId(null);
        }
    }

    return (
        <aside className="project-sidebar">
            <div>
                <h2>Projects</h2>
                <p>Stored in SQL</p>
            </div>

            <form className="project-form" onSubmit={handleSubmit}>
                <input
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                    placeholder="New project name.."
                />
                <button disabled={saving || !name.trim()}>
                    {saving ? "Creating..." : "Create"}
                </button>
            </form>

            <div className="project-list">
                {projects.map(project => {
                    const active = project.id === selectedProjectId;
                    const deleting = project.id === deletingProjectId;

                    return (
                        <div key={project.id} className={active ? "project-item-row active" : "project-item-row"}>

                            <button type="button" className="project-item-main" onClick={() => onSelectProject(project.id)}>
                                <strong> {project.name} </strong>
                                <span>{new Date(project.createdAt).toLocaleDateString()}</span>
                            </button>


                            <button type="button" className="project-delete-button" aria-label={`Delete project ${project.name}`} title="Delete project" disabled={deleting} onClick={() => handleDelete(project)}>
                                {deleting ? "…" : "×"}
                            </button>

                        </div>
                    );
                })}
            </div>
        </aside>
    );
}