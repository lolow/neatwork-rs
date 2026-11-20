import { useEffect, useRef, useState } from "react";


export type DatabaseType = "DB_DIAMETERS_INPUT" | "DB_ORIFICES_INPUT"
type OpenMenu = "file" | "database" | "help" | null;

type AppMenuBarProps = {
    activeProjectName?: string | null;
    canImportNetwork: boolean;
    onNewProject: () => void;
    onOpenProject: () => void;
    onImportNetwork: () => void;
    onOpenDatabase: (type: DatabaseType) => void;
    canConfigureUnits: boolean;
    onOpenUnitSettings: () => void;
};

export function AppMenuBar({ activeProjectName, canImportNetwork, onNewProject, onOpenProject, onImportNetwork, onOpenDatabase, canConfigureUnits, onOpenUnitSettings }: AppMenuBarProps) {
    const [fileMenuOpen, setFileMenuOpen] = useState(false);
    const fileMenuRef = useRef<HTMLDivElement>(null);
    const databaseMenuRef = useRef<HTMLDivElement>(null);
    const [databaseMenuOpen, setDatabaseMenuOpen] = useState(false);
    const [openMenu, setOpenMenu] = useState<OpenMenu>(null);
    const helpMenuRef = useRef<HTMLDivElement>(null);

    function handleToggleDatabaseMenu() {
        setFileMenuOpen(false);
        setDatabaseMenuOpen(current => !current);
    }

    useEffect(() => {
        function handlePointerDown(event: PointerEvent) {
            const target = event.target as Node;

            if (helpMenuRef.current && !helpMenuRef.current.contains(target)) {
                setOpenMenu(null);
            }
            if (fileMenuRef.current && !fileMenuRef.current.contains(target)) {
                setFileMenuOpen(false);
            }
            if (databaseMenuRef.current && !databaseMenuRef.current.contains(target)) {
                setDatabaseMenuOpen(false);
            }
        }

        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape") {
                setFileMenuOpen(false);
                setDatabaseMenuOpen(false);
                setOpenMenu(null);
            }
        }
        document.addEventListener("pointerdown", handlePointerDown);
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            document.removeEventListener("pointerdown", handlePointerDown);
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, []);
    function handleNewProject() {
        setFileMenuOpen(false);
        onNewProject();
    }
    function handleOpenProject() {
        setFileMenuOpen(false);
        onOpenProject();
    }

    function handleImportNetwork() {
        setFileMenuOpen(false);
        if (!canImportNetwork) {
            return;
        }
        onImportNetwork();
    }
    return (
        <header className="app-menu-bar">
            <div className="app-menu-bar__menus">
                <div ref={fileMenuRef} className="app-menu">
                    <button type="button" className={
                        fileMenuOpen
                            ? "app-menu__trigger active"
                            : "app-menu__trigger"
                    }
                        aria-haspopup="menu"
                        aria-expanded={fileMenuOpen}
                        onClick={() => setFileMenuOpen(current => !current)}
                    >
                        File
                    </button>

                    {fileMenuOpen && (
                        <div className="app-menu__dropdown" role="menu">
                            <button type="button" role="menuitem"
                                className="app-menu__item"
                                onClick={handleNewProject}
                            >
                                <span className="app-menu__item-icon"> + </span>

                                <span className="app-menu__item-content">
                                    <strong>New Project</strong>
                                </span>
                            </button>

                            <button
                                type="button"
                                role="menuitem"
                                className="app-menu__item"
                                onClick={handleOpenProject}
                            >
                                <span className="app-menu__item-icon"> ↗ </span>

                                <span className="app-menu__item-content">
                                    <strong>Open an Existing Project</strong>
                                </span>
                            </button>
                            <div className="app-menu__separator" />
                            <button type="button" role="menuitem"
                                className="app-menu__item"
                                disabled={!canImportNetwork}
                                onClick={handleImportNetwork}
                            >
                                <span className="app-menu__item-icon">↓</span>
                                <span className="app-menu__item-content">
                                    <strong>Import Network...</strong>
                                    <small>Choose a DSG or TPO input file</small>
                                </span>
                            </button>
                        </div>
                    )}
                </div>

                <div ref={databaseMenuRef} className="app-menu">
                    <button type="button" className={
                        databaseMenuOpen
                            ? "app-menu__trigger active"
                            : "app-menu__trigger"
                    }
                        aria-haspopup="menu"
                        aria-expanded={databaseMenuOpen}
                        onClick={handleToggleDatabaseMenu}
                    >
                        Database
                    </button>

                    {databaseMenuOpen && (
                        <div className="app-menu__dropdown" role="menu">
                            <button type="button" role="menuitem"
                                className="app-menu__item"
                                onClick={() => {
                                    setDatabaseMenuOpen(false)
                                    onOpenDatabase("DB_DIAMETERS_INPUT")
                                }}
                            >
                                <span className="app-menu__item-icon"> D </span>

                                <span className="app-menu__item-content">
                                    <strong>Diameters Database</strong>
                                    <small>Manage commercial pipe diameters</small>
                                </span>
                            </button>

                            <button
                                type="button"
                                role="menuitem"
                                className="app-menu__item"
                                onClick={() => {
                                    setDatabaseMenuOpen(false);
                                    onOpenDatabase("DB_ORIFICES_INPUT");
                                }}
                            >
                                <span className="app-menu__item-icon"> O </span>

                                <span className="app-menu__item-content">
                                    <strong>Orifices Database</strong>
                                    <small>Manage commercial orifices </small>
                                </span>
                            </button>

                        </div>
                    )}
                </div>

                <div ref={helpMenuRef} className="app-menu">
                    <button type="button" className={
                        openMenu === "help"
                            ? "app-menu__trigger active"
                            : "app-menu__trigger"
                    }
                        onClick={() => {
                            setFileMenuOpen(false);
                            setDatabaseMenuOpen(false);
                            setOpenMenu(current => current === "help" ? null : "help");
                        }}
                    >
                        Help
                    </button>

                    {openMenu === "help" && (
                        <div className="app-menu__dropdown app-menu__dropdown--help" role="menu">
                            <button
                                type="button"
                                role="menuitem"
                                className="app-menu__item app-menu__item--text-only"
                                disabled={!canConfigureUnits}
                                onClick={() => { setOpenMenu(null); onOpenUnitSettings(); }}
                            >
                                <span className="app-menu__item-content">
                                    <strong>Set global units</strong>
                                </span>
                            </button>
                        </div>
                    )}
                </div>

            </div>

            <div className="app-menu-bar__project">
                {activeProjectName ? (
                    <>
                        <span>Project</span>
                        <strong>{activeProjectName}</strong>
                    </>
                ) : (
                    <span>No project selected</span>
                )}
            </div>
        </header>
    );
}