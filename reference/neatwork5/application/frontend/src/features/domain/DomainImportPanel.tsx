import { FormEvent, useMemo, useState, useEffect } from "react";
import type { ProjectArtifact } from "../artifacts/types";
import type { DomainSnapshot, NeatworkFileFormat } from "./types";

type Props = {
    projectId: string | null;
    artifacts: ProjectArtifact[];
    latestSnapshot: DomainSnapshot | null;
    onImport: (params: {
        name: string;
        format: NeatworkFileFormat;
        primaryArtifactId: string;
        diametersDbArtifactId?: string | null;
        orificesDbArtifactId?: string | null;
    }) => Promise<void>;
};

function artifactLabel(artifact: ProjectArtifact): string {
    return `${artifact.originalFilename} · ${artifact.type} · v${artifact.version}`;
}

export function DomainImportPanel({
    projectId,
    artifacts,
    latestSnapshot,
    onImport
}: Props) {
    const [name, setName] = useState("Network");
    const [format, setFormat] = useState<NeatworkFileFormat>("TPO");
    const [primaryArtifactId, setPrimaryArtifactId] = useState("");
    const [diametersDbArtifactId, setDiametersDbArtifactId] = useState("");
    const [orificesDbArtifactId, setOrificesDbArtifactId] = useState("");
    const [importing, setImporting] = useState(false);

    const primaryArtifacts = useMemo(() => {
        return artifacts.filter((artifact) => {
            if (format === "TPO") return artifact.type === "TPO_INPUT";
            if (format === "DSG") return artifact.type === "DSG_INPUT";
            return artifact.type === "JSON_INPUT";
        });
    }, [artifacts, format]);


    const diameterArtifacts = useMemo(
        () => artifacts.filter((artifact) => artifact.type === "DB_DIAMETERS_INPUT"),
        [artifacts]
    );
    const orificeArtifacts = useMemo(
        () => artifacts.filter((artifact) => artifact.type === "DB_ORIFICES_INPUT"),
        [artifacts]
    );

    useEffect(() => {
        setPrimaryArtifactId("");

        if (format !== "TPO") {
            setDiametersDbArtifactId("");
            setOrificesDbArtifactId("");
        }
    }, [format]);

    useEffect(() => {
        if (format !== "TPO") return;

        if (!diametersDbArtifactId && diameterArtifacts.length > 0) {
            setDiametersDbArtifactId(diameterArtifacts[0].id);
        }

        if (!orificesDbArtifactId && orificeArtifacts.length > 0) {
            setOrificesDbArtifactId(orificeArtifacts[0].id);
        }
    }, [format, diameterArtifacts, orificeArtifacts, diametersDbArtifactId, orificesDbArtifactId]);

    async function handleSubmit(event: FormEvent) {
        event.preventDefault();
        if (!projectId || !primaryArtifactId) return;

        setImporting(true);
        try {
            await onImport({
                name,
                format,
                primaryArtifactId,
                diametersDbArtifactId: diametersDbArtifactId || null,
                orificesDbArtifactId: orificesDbArtifactId || null
            });
        }
        finally {
            setImporting(false);

        }
    }

    return (
        <section className="domain-import-panel">
            <div className="domain-import-header">


                <button
                    type="submit"
                    form="domain-import-form"
                    className="domain-import-submit"
                    disabled={!projectId || !primaryArtifactId || importing}
                >
                    {importing ? "Importing..." : "Import domain"}
                </button>
            </div>

            <form id="domain-import-form" className="domain-import-form" onSubmit={handleSubmit}>


                <div className="domain-import-row">
                    <label className="form-field">
                        <span>Source format</span>
                        <select
                            value={format}
                            onChange={(event) => { setFormat(event.target.value as NeatworkFileFormat); }}
                        >
                            <option value="TPO">TPO</option>
                            <option value="DSG">DSG</option>
                            <option value="JSON">JSON</option>
                        </select>
                    </label>

                    <label className="form-field">
                        <span>Primary file</span>
                        <select
                            value={primaryArtifactId}
                            onChange={(event) => setPrimaryArtifactId(event.target.value)}
                        >
                            <option value="">Select primary file</option>
                            {primaryArtifacts.map((artifact) => (
                                <option key={artifact.id} value={artifact.id}>
                                    {artifact.storedFilename}
                                </option>
                            ))}
                        </select>
                    </label>
                </div>

                {format === "TPO" && (
                    <section className="domain-db-section">
                        <div className="domain-db-section-title">
                            <strong>Complementary database files</strong>
                            <span>
                                Select the DB files already uploaded in Project files. They are sent with the TPO file during import.
                            </span>
                        </div>

                        <div className="domain-import-row">
                            <label className="form-field">
                                <span>Diameters DB</span>
                                <select
                                    value={diametersDbArtifactId}
                                    onChange={(event) => setDiametersDbArtifactId(event.target.value)}
                                    disabled={diameterArtifacts.length === 0}
                                >
                                    <option value="">{diameterArtifacts.length === 0
                                        ? "No DB_DIAMETERS_INPUT uploaded"
                                        : "Select diameters.db"}
                                    </option>
                                    {diameterArtifacts.map((artifact) => (
                                        <option key={artifact.id} value={artifact.id}>
                                            {artifactLabel(artifact)}
                                        </option>
                                    ))}
                                </select>
                                <small>
                                    Required when the TPO file does not contain a !Diameters section.
                                </small>
                            </label>

                            <label className="form-field">
                                <span>Orifices DB</span>
                                <select
                                    value={orificesDbArtifactId}
                                    onChange={(event) => setOrificesDbArtifactId(event.target.value)}
                                    disabled={orificeArtifacts.length === 0}
                                >
                                    <option value="">
                                        {orificeArtifacts.length === 0
                                            ? "No DB_ORIFICES_INPUT uploaded"
                                            : "Select orifices.db"}
                                    </option>
                                    {orificeArtifacts.map((artifact) => (
                                        <option key={artifact.id} value={artifact.id}>
                                            {artifactLabel(artifact)}
                                        </option>
                                    ))}
                                </select>
                                <small>
                                    Optional. Use it when the import needs an external orifices database.
                                </small>
                            </label>
                        </div>
                    </section>
                )}
            </form>


        </section>
    );
}