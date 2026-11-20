import { ChangeEvent, useState, useRef, useEffect } from "react";
import { artifactDownloadUrl } from "../../api/artifactApi";
import type { ProjectArtifact, ProjectArtifactType } from "./types";

type Props = {
    projectId: string | null;
    artifacts: ProjectArtifact[];
    onUpload: (type: ProjectArtifactType, file: File) => Promise<void>;
};

const artifactTypes: ProjectArtifactType[] = [
    "TPO_INPUT",
    "DSG_INPUT",
    "DB_DIAMETERS_INPUT",
    "DB_ORIFICES_INPUT",
    "JSON_INPUT"
];

function acceptedExtensions(type: ProjectArtifactType): string {
    if (type === "TPO_INPUT") { return ".tpo"; }

    if (type === "DSG_INPUT") { return ".dsg"; }

    if (type === "DB_DIAMETERS_INPUT" || type === "DB_ORIFICES_INPUT") {
        return ".db,.txt";
    }

    if (type === "JSON_INPUT") {
        return ".json";
    }

    return "";
}

export function ArtifactPanel({ projectId, artifacts, onUpload }: Props) {
    const [type, setType] = useState<ProjectArtifactType>("DSG_INPUT");
    const [selectedFile, setSelectedFile] = useState<File | null>(null);
    const [uploading, setUploading] = useState(false);
    const [uploadError, setUploadError] = useState<string | null>(null);
    const fileInputRef = useRef<HTMLInputElement | null>(null);


    useEffect(() => {
        setSelectedFile(null);

        setUploadError(null);

        if (fileInputRef.current) {
            fileInputRef.current.value = "";
        }
    }, [projectId]);

    function handleTypeChange(event: ChangeEvent<HTMLSelectElement>) {
        setType(event.target.value as ProjectArtifactType);

        setSelectedFile(null);

        setUploadError(null);

        if (fileInputRef.current) {
            fileInputRef.current.value = "";
        }
    }

    async function handleFileChange(event: ChangeEvent<HTMLInputElement>) {
        const file = event.target.files?.[0] ?? null;
        setSelectedFile(file);
        setUploadError(null);
    }

    async function handleUpload() {
        if (!projectId || !selectedFile || uploading) {
            return;
        }

        setUploading(true);

        setUploadError(null);

        try {
            await onUpload(type, selectedFile);

            setSelectedFile(null);

            if (fileInputRef.current) {
                fileInputRef.current.value = "";
            }
        } catch (error) {
            console.error("Artifact upload failed:", error);
            setUploadError("Could not upload the selected file.");
        } finally {
            setUploading(false);
        }
    }

    return (
        <section className="artifact-panel">
            <div>
                <h2>Project files</h2>
                <p>Inputs and generated outputs are versioned per project.</p>
            </div>

            {!projectId && (
                <p className="artifact-project-message" role="status">
                    Select or create a project before uploading a file.
                </p>
            )}

            <div className="artifact-upload">
                <label className="form-field">
                    <span> File type</span>
                    <select value={type}
                        disabled={!projectId || uploading}
                        onChange={handleTypeChange}
                    >
                        {artifactTypes.map(
                            (item) => (
                                <option key={item} value={item}>
                                    {item}
                                </option>
                            )
                        )}
                    </select>
                </label>

                <div className="artifact-file-selection">
                    <span className="artifact-field-label">Local file</span>

                    <div className="artifact-file-controls">
                        <label className={`file-button secondary ${!projectId || uploading ? "disabled" : ""}`}>
                            <span>Choose file</span>
                            <input
                                ref={fileInputRef}
                                type="file"
                                accept={acceptedExtensions(type)}
                                disabled={!projectId || uploading}
                                onChange={handleFileChange}
                            />
                        </label>

                        <button
                            type="button"
                            className="artifact-upload-button"
                            disabled={!projectId || !selectedFile || uploading}
                            onClick={handleUpload}
                        >
                            {uploading ? "Uploading..." : "Upload"}
                        </button>
                    </div>
                </div>

                <div
                    className={selectedFile ? "selected-file has-file" : "selected-file"}
                    title={selectedFile?.name}
                >
                    {selectedFile ? selectedFile.name : "No file selected"}
                </div>
            </div>

            {uploadError && (
                <p className="artifact-upload-error" role="alert">
                    {uploadError}
                </p>
            )}

            <div className="artifact-list">
                {artifacts.length === 0 ? (
                    <p>No files yet.</p>
                ) : (
                    artifacts.map((artifact) => (
                        <div key={artifact.id} className="artifact-row">
                            <div>
                                <strong>{artifact.storedFilename}</strong>
                                <span>{artifact.type} · v{artifact.version} · {(artifact.sizeBytes / 1024).toFixed(1)} KB</span>
                            </div>

                            {projectId && (
                                <a href={artifactDownloadUrl(projectId, artifact.id)}>
                                    Download
                                </a>
                            )}
                        </div>
                    ))
                )}
            </div>
        </section>
    );
}