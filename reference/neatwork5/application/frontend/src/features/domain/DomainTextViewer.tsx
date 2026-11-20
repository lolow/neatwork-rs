import { useEffect, useMemo, useState } from "react";
import { fetchArtifactText } from "../../api/artifactApi";
import type { ProjectArtifact } from "../artifacts/types";


type TextViewMode = "SOURCE" | "JSON";

type Props = {
    projectId: string;
    sourceArtifact: ProjectArtifact | null;
    domain: unknown;
};

// Source artifacts are immutable revisions, so their textual content can safely be cached by artifact ID.
const sourceTextCache = new Map<string, string>();

function sourceCacheKey(projectId: string, artifactId: string): string {
    return `${projectId}:${artifactId}`;
}

export function DomainTextViewer({ projectId, sourceArtifact, domain }: Props) {

    const [viewMode, setViewMode] = useState<TextViewMode>(sourceArtifact ? "SOURCE" : "JSON");
    const [sourceText, setSourceText] = useState<string | null>(null);
    const [loading, setLoading] = useState(false);
    const [sourceError, setSourceError] = useState<string | null>(null);
    const jsonText = useMemo(() => JSON.stringify(domain, null, 2), [domain]);

    useEffect(() => {
        setSourceText(null);
        setSourceError(null);


        // Prefer the real source file whenever the active domain has an accessible artifact.
        setViewMode(sourceArtifact ? "SOURCE" : "JSON");
    },
        [projectId, sourceArtifact?.id]
    );
    useEffect(
        () => {
            if (viewMode !== "SOURCE" || !sourceArtifact) {
                return;
            }
            const cacheKey = sourceCacheKey(projectId, sourceArtifact.id);
            const cached = sourceTextCache.get(cacheKey);
            if (cached !== undefined) {
                setSourceText(cached);
                setSourceError(null);
                return;
            }
            let disposed = false;
            async function loadSourceText() {
                setLoading(true);
                setSourceError(null);
                try {
                    const content = await fetchArtifactText(projectId, sourceArtifact!.id);
                    if (disposed) {
                        return;
                    }
                    sourceTextCache.set(cacheKey, content);
                    setSourceText(content);
                } catch (error) {
                    if (disposed) {
                        return;
                    }
                    console.error("Could not load source artifact text:", error);
                    setSourceError("The original source file could not be read.");
                } finally {
                    if (!disposed) {
                        setLoading(false);
                    }
                }
            }
            void loadSourceText();
            return () => {
                disposed = true;
            };
        },
        [projectId, sourceArtifact, viewMode]
    );

    return (
        <section className="domain-text-viewer">
            <header className="domain-text-viewer__header">
                <div className="domain-text-viewer__heading">
                    <div className="domain-text-viewer__tabs" role="tablist" aria-label="Text representation">
                        <button
                            type="button"
                            role="tab"
                            aria-selected={viewMode === "SOURCE"}
                            className={viewMode === "SOURCE" ? "active" : ""}
                            disabled={!sourceArtifact}
                            onClick={() => setViewMode("SOURCE")}
                        >
                            Source File
                        </button>
                        <button
                            type="button"
                            role="tab"
                            aria-selected={viewMode === "JSON"}
                            className={viewMode === "JSON" ? "active" : ""}
                            onClick={() => setViewMode("JSON")}
                        >
                            JSON
                        </button>
                    </div>
                    <div className="domain-text-viewer__description">
                        {viewMode === "SOURCE" ? (
                            <>Original network file stored in this project.</>
                        )
                            : (
                                <>Parsed domain representation used by the application.</>
                            )
                        }
                    </div>
                </div>
                <div className="domain-text-viewer__metadata">
                    {viewMode === "SOURCE" && sourceArtifact ? (
                        <>
                            <strong>{sourceArtifact.originalFilename}</strong>
                            <span>
                                {artifactFormatLabel(sourceArtifact)}{" · "}
                                v
                                {sourceArtifact.version}
                            </span>
                        </>
                    )
                        : (
                            <>
                                <strong>Parsed JSON</strong>
                                <span>Domain snapshot</span>
                            </>
                        )
                    }
                </div>
            </header>
            <div className="domain-text-viewer__content">
                {viewMode === "SOURCE" ? (
                    <SourceTextContent
                        sourceArtifact={sourceArtifact}
                        loading={loading}
                        error={sourceError}
                        text={sourceText}
                    />
                )
                    : (
                        <pre className="domain-text-viewer__code">{jsonText}</pre>
                    )
                }
            </div>
        </section>
    );
}

function artifactFormatLabel(artifact: ProjectArtifact): string {
    switch (artifact.type) {
        case "DSG_INPUT":
            return "DSG";

        case "TPO_INPUT":
            return "TPO";

        case "JSON_INPUT":
            return "JSON";

        default:
            return artifact.type;
    }
}

function SourceTextContent({ sourceArtifact, loading, error, text }: {
    sourceArtifact: ProjectArtifact | null;
    loading: boolean;
    error: string | null;
    text: string | null;
}) {

    if (!sourceArtifact) {
        return (
            <div className="domain-text-viewer__state">
                <strong>Source file unavailable</strong>
                <span>This domain snapshot does not have an accessible source artifact. The parsed JSON representation is still available.</span>
            </div>
        );
    }
    if (loading) {
        return (
            <div className="domain-text-viewer__state">Reading source file...</div>
        );
    }
    if (error) {
        return (
            <div className="domain-text-viewer__state error" role="alert">
                <strong>Could not read source file</strong>
                <span>{error}</span>
            </div>
        );
    }
    return (
        <pre className="domain-text-viewer__code">{text ?? ""}</pre>
    );
}