type WorkspaceEmptyStateProps = {
    projectName: string | null;

    onNewProject: () => void;
    onOpenProject: () => void;
    onImportNetwork: () => void;
};

export function WorkspaceEmptyState({
    projectName,
    onNewProject,
    onOpenProject,
    onImportNetwork,
}: WorkspaceEmptyStateProps) {

    const hasProject = projectName !== null;

    return (
        <section className="workspace-empty" aria-live="polite">
            <div className="workspace-empty__background">
                <div className="workspace-empty__orb workspace-empty__orb--one" />
                <div className="workspace-empty__orb workspace-empty__orb--two" />
            </div>


            <div className="workspace-empty__content">
                <div className="workspace-empty__icon" aria-hidden="true">
                    <svg viewBox="0 0 64 64" focusable="false">
                        <circle cx="12" cy="39" r="5" />
                        <circle cx="31" cy="25" r="5" />
                        <circle cx="52" cy="38" r="5" />
                        <path d="M16 36L27 28" />
                        <path d="M35 28L48 35" />
                        <path d="M31 5C31 5 22 15 22 22" />
                    </svg>
                </div>

                {hasProject ? (
                    <>
                        <span className="workspace-empty__eyebrow">Current project</span>
                        <h1>{projectName}</h1>
                        <h2>No network imported</h2>
                        <p>Import a DSG or TPO network to start working with this project.</p>
                        <button type="button" className="workspace-empty__primary" onClick={onImportNetwork}>
                            <span>Import Network</span>
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M12 3v12M7 10l5 5 5-5M5 20h14" />
                            </svg>
                        </button>
                    </>
                ) : (
                    <>
                        <span className="workspace-empty__eyebrow">NeatWork Workspace</span>
                        <h1>No project is open</h1>
                        <p>Create a new project or open an existing one to start working with a water distribution network.</p>

                        <div className="workspace-empty__actions">
                            <button type="button"
                                className="workspace-empty__primary"
                                onClick={onNewProject}
                            >
                                New Project
                            </button>

                            <button type="button"
                                className="workspace-empty__secondary"
                                onClick={onOpenProject}
                            >
                                Open Project
                            </button>
                        </div>
                    </>
                )}

            </div>
        </section>
    );
}