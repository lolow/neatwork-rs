import { ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ProjectArtifact, ProjectArtifactType } from "../artifacts/types";
import type { ProjectSummary } from "../projects/types";
import type { TpoArtifactRequirements } from "./types";


type TpoDatabaseModalProps = {
    tpoArtifact: ProjectArtifact;
    requirements: TpoArtifactRequirements | null;
    loadingRequirements: boolean;
    diametersArtifacts: ProjectArtifact[];
    orificesArtifacts: ProjectArtifact[];
    projects: ProjectSummary[];
    importing: boolean;
    error: string | null;
    onUploadDatabase: (type: "DB_DIAMETERS_INPUT" | "DB_ORIFICES_INPUT", file: File) => Promise<ProjectArtifact>;
    onImport: (params: {
        diametersArtifactId: string | null;
        orificesArtifactId: string | null;
    }) => Promise<void>;
    onBack: () => void;
    onClose: () => void;
};

export function TpoDatabaseModal({
    tpoArtifact,
    requirements,
    loadingRequirements,
    diametersArtifacts,
    orificesArtifacts,
    projects,
    importing,
    error,
    onUploadDatabase,
    onImport,
    onBack,
    onClose,
}: TpoDatabaseModalProps) {

    const [diametersArtifactId, setDiametersArtifactId] = useState("");
    const [orificesArtifactId, setOrificesArtifactId] = useState("");
    const [uploadingType, setUploadingType] = useState<ProjectArtifactType | null>(null);

    const diametersInputRef = useRef<HTMLInputElement>(null);
    const orificesInputRef = useRef<HTMLInputElement>(null);

    const projectNames = useMemo(() =>
        new Map(projects.map(project => [project.id, project.name])),
        [projects]
    );


    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !importing && !uploadingType) {
                onClose();
            }
        }
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, [importing, uploadingType, onClose]);


    async function handleDatabaseUpload(event: ChangeEvent<HTMLInputElement>, type: "DB_DIAMETERS_INPUT" | "DB_ORIFICES_INPUT") {
        const file = event.target.files?.[0];

        /*
         * Allow selecting the same file again later.
         */
        event.target.value = "";
        if (!file) {
            return;
        }

        setUploadingType(type);
        try {
            const artifact = await onUploadDatabase(type, file);
            if (type === "DB_DIAMETERS_INPUT") {
                setDiametersArtifactId(artifact.id);
            } else {
                setOrificesArtifactId(artifact.id);
            }

        } finally {
            setUploadingType(null);
        }
    }

    const diameterRequired = requirements?.diametersDatabaseRequired ?? false;

    const canImport = !loadingRequirements
        && requirements !== null
        && !importing
        && uploadingType === null
        && (!diameterRequired || Boolean(diametersArtifactId));


    async function handleImport() {
        if (!canImport) {
            return;
        }

        await onImport({
            diametersArtifactId: diametersArtifactId || null,
            orificesArtifactId: orificesArtifactId || null,
        });
    }

    return createPortal(
        <div className="project-modal-backdrop">
            <section
                className="project-modal project-modal--wide tpo-database-modal"
                role="dialog" aria-modal="true"
                aria-labelledby="tpo-db-title"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">TPO configuration</span>
                        <h2 id="tpo-db-title">Complementary databases</h2>
                        <p>{tpoArtifact.originalFilename}</p>
                    </div>

                    <button type="button"
                        className="project-modal__close"
                        aria-label="Close"
                        disabled={importing || uploadingType !== null}
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>

                <div className="project-modal__body">
                    {loadingRequirements && (
                        <div className="input-library-status">Analysing TPO file...</div>
                    )}


                    {!loadingRequirements && requirements && (
                        <div className="tpo-analysis">
                            <div>
                                <span>!Diameters</span>
                                <strong>
                                    {requirements.hasDiametersSection
                                        ? "Included in TPO"
                                        : "Not included"}
                                </strong>
                            </div>

                            <div>
                                <span>!Orifices</span>
                                <strong>
                                    {requirements.hasOrificesSection
                                        ? "Included in TPO"
                                        : "Not included"}
                                </strong>
                            </div>
                        </div>
                    )}


                    {error && <div className="project-modal__error">{error}</div>}


                    {!loadingRequirements && requirements && (
                        <div className="tpo-db-grid">

                            {/* DIAMETERS */}
                            <section className={
                                diameterRequired
                                    ? "tpo-db-card required"
                                    : "tpo-db-card"
                            }>
                                <div className="tpo-db-card__header">
                                    <div>
                                        <strong> Diameters database </strong>
                                        <span>
                                            {diameterRequired
                                                ? "Required"
                                                : "Optional"}
                                        </span>
                                    </div>
                                </div>

                                <select
                                    value={diametersArtifactId}
                                    disabled={importing || uploadingType !== null}
                                    onChange={event => setDiametersArtifactId(event.target.value)}>
                                    <option value="">{requirements
                                        .hasDiametersSection ? "Use !Diameters from TPO"
                                        : diameterRequired ? "Select diameters.db"
                                            : "No external diameters DB"}
                                    </option>

                                    {diametersArtifacts.map(
                                        artifact => (
                                            <option key={artifact.id} value={artifact.id}>
                                                {artifact.originalFilename}
                                                {" · "}
                                                {projectNames.get(artifact.projectId) ?? "Unknown project"}
                                                {" · v"}
                                                {artifact.version}
                                            </option>
                                        )
                                    )}
                                </select>

                                {diameterRequired &&
                                    !diametersArtifactId && (
                                        <small className="tpo-db-required-message">
                                            This TPO has no !Diameters section. Select or upload a diameters database.
                                        </small>
                                    )}

                                <input ref={diametersInputRef} type="file" accept=".db" hidden
                                    onChange={event => void handleDatabaseUpload(
                                        event, "DB_DIAMETERS_INPUT")} />

                                <button type="button"
                                    className="project-modal__secondary tpo-db-upload"
                                    disabled={importing || uploadingType !== null}
                                    onClick={() => diametersInputRef.current?.click()}>
                                    {uploadingType === "DB_DIAMETERS_INPUT" ? "Uploading..."
                                        : "Upload diameters.db"}
                                </button>
                            </section>


                            {/* ORIFICES */}
                            <section className="tpo-db-card">
                                <div className="tpo-db-card__header">
                                    <div>
                                        <strong> Orifices database </strong>
                                        <span> Optional </span>
                                    </div>
                                </div>

                                <select
                                    value={orificesArtifactId}
                                    disabled={importing || uploadingType !== null}
                                    onChange={event => setOrificesArtifactId(event.target.value)}
                                >
                                    <option value="">{requirements
                                        .hasOrificesSection
                                        ? "Use !Orifices from TPO"
                                        : "No external orifices DB"}
                                    </option>

                                    {orificesArtifacts.map(
                                        artifact => (
                                            <option key={artifact.id} value={artifact.id}>
                                                {artifact.originalFilename}
                                                {" · "}
                                                {projectNames.get(artifact.projectId) ?? "Unknown project"}
                                                {" · v"}
                                                {artifact.version}
                                            </option>
                                        )
                                    )}
                                </select>

                                <small>Select an external orifices database only when required by the network.</small>

                                <input
                                    ref={orificesInputRef}
                                    type="file"
                                    accept=".db"
                                    hidden
                                    onChange={event => void handleDatabaseUpload(event, "DB_ORIFICES_INPUT")}
                                />

                                <button
                                    type="button"
                                    className="project-modal__secondary tpo-db-upload"
                                    disabled={importing || uploadingType !== null}
                                    onClick={() => orificesInputRef.current?.click()}
                                >
                                    {uploadingType === "DB_ORIFICES_INPUT" ? "Uploading..." : "Upload orifices.db"}
                                </button>
                            </section>

                        </div>
                    )}

                </div>


                <footer className="project-modal__footer">
                    <button
                        type="button"
                        className="project-modal__secondary"
                        disabled={importing || uploadingType !== null}
                        onClick={onBack}
                    >
                        Back
                    </button>

                    <button
                        type="button"
                        className="project-modal__secondary"
                        disabled={importing || uploadingType !== null}
                        onClick={onClose}
                    >
                        Cancel
                    </button>

                    <button
                        type="button"
                        className="project-modal__primary"
                        disabled={!canImport}
                        onClick={() => void handleImport()}
                    >
                        {importing ? "Importing..." : "Import TPO"}
                    </button>
                </footer>

            </section>
        </div>,
        document.body
    );
}