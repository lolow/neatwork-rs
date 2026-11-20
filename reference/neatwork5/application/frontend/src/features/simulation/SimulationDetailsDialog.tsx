import { createPortal } from "react-dom";
import type { SimulationRunDetails, SimulationParameterSource } from "./types";
import { metersToDisplayUnit, getDisplayUnitSymbol } from "../domain/unitSystem";
import type { DisplayLengthUnit } from "../domain/unitSystem";


type Props = {
    details: SimulationRunDetails | null;
    loading: boolean;
    error: string | null;
    onClose: () => void;
    displayUnit: DisplayLengthUnit
};

function formatValue(value: unknown): string {
    if (value === null || value === undefined) {
        return "—";
    }
    if (typeof value === "boolean") {
        return value ? "Yes" : "No";
    }
    if (typeof value === "object") {
        return JSON.stringify(value, null, 2);
    }
    return String(value);
}



function sourceLabel(source: SimulationParameterSource | undefined): string {
    switch (source) {
        case "USER_OVERRIDE":
            return "Modified by user";
        case "PROJECT_DEFAULT":
            return "Project default";
        case "GENERATED":
            return "Generated";
        case "NOT_APPLICABLE":
            return "Not applicable";
        default:
            return "—";
    }
}

const LENGTH_PARAMETER_KEYS = new Set(["requiredPressureM", "commercialPipeLengthM"]);
const LENGTH_MAP_PARAMETER_KEYS = new Set(["terminalRequiredPressureM"]);

function formatSimulationParameterValue(key: string, value: unknown, displayUnit: DisplayLengthUnit): string {
    if (value === null || value === undefined) {
        return "—";
    }

    if (LENGTH_PARAMETER_KEYS.has(key) && typeof value === "number") {

        return (metersToDisplayUnit(value, displayUnit).toFixed(2) + " " + getDisplayUnitSymbol(displayUnit));
    }

    if (LENGTH_MAP_PARAMETER_KEYS.has(key) && value && typeof value === "object" && !Array.isArray(value)) {

        const converted = Object.fromEntries(Object.entries(value).map(
            ([id, raw]) => [
                id,
                typeof raw === "number" ? Number(
                    metersToDisplayUnit(raw, displayUnit).toFixed(2)) : raw])
        );

        return (JSON.stringify(converted, null, 2) + ` ${getDisplayUnitSymbol(displayUnit)}`);
    }

    return formatValue(value);
}


export function SimulationDetailsDialog({ details, loading, error, onClose, displayUnit }: Props) {
    const resolved = details?.resolvedOptions ?? {};
    const requested = details?.requestedOptions ?? {};
    const parameterSources = details?.parameterSources ?? {};
    const edgeOverrides = details?.edgeOverrides ?? [];
    const boundedFlow = resolved["boundedFlow"];
    const orificeMode = resolved["orificeMode"];

    return createPortal(
        <div className="project-modal-backdrop">
            <section className="project-modal project-modal--wide simulation-details-dialog" role="dialog" aria-modal="true">

                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Execution details</span>
                        <h2>{details ? `Simulation #${details.runNumber}` : "Simulation details"}</h2>

                        {details && (
                            <p>Exact input and reproducibility parameters used by this execution.</p>
                        )}
                    </div>
                    <button
                        type="button"
                        className="project-modal__close"
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>
                <div className="project-modal__body simulation-details-body">
                    {loading && (
                        <div className="input-library-status">
                            Loading simulation details...
                        </div>
                    )}

                    {error && (
                        <div className="project-modal__error">{error}</div>
                    )}

                    {!loading &&
                        details && (
                            <>
                                <section className="simulation-details-section">
                                    <h3>Input data</h3>
                                    <div className="simulation-details-grid">
                                        <Detail
                                            label="Network"
                                            value={details.input.networkName}
                                        />
                                        <Detail
                                            label="File"
                                            value={details.input.filename}
                                        />
                                        <Detail
                                            label="File version"
                                            value={details.input.version}
                                        />
                                        <Detail
                                            label="Format"
                                            value={details.input.sourceFormat}
                                        />
                                        <Detail
                                            label="Domain snapshot"
                                            value={details.input.domainSnapshotId}
                                        />
                                    </div>
                                </section>

                                <section className="simulation-details-section">
                                    <h3>Execution</h3>
                                    <div className="simulation-details-grid">
                                        <Detail
                                            label="Configuration"
                                            value={
                                                details.configurationMode === "DEFAULT"
                                                    ? "Default"
                                                    : "Manual"
                                            }
                                        />
                                        <Detail
                                            label="Status"
                                            value={details.status}
                                        />
                                        <Detail
                                            label="Scenarios"
                                            value={details.scenarioCount}
                                        />
                                        <Detail
                                            label="Effective seed"
                                            value={details.seed}
                                        />
                                        <Detail
                                            label="Bounded flow"
                                            value={boundedFlow}
                                        />
                                        <Detail
                                            label="Orifice model"
                                            value={orificeMode}
                                        />
                                    </div>
                                </section>
                                <section className="simulation-details-section">
                                    <h3>Requested parameters</h3>
                                    {Object.keys(requested).length === 0 ? (
                                        <div className="simulation-details-default-message">No runtime overrides were submitted. Project defaults were used.</div>
                                    ) : (
                                        <ParameterTable values={requested} displayUnit={displayUnit} />
                                    )}
                                </section>
                                <section
                                    className="simulation-details-section">
                                    <h3>Resolved parameters</h3>

                                    {details.resolvedOptions ? (
                                        <ParameterTable
                                            values={resolved}
                                            sources={Object.keys(parameterSources).length > 0
                                                ? parameterSources
                                                : undefined}
                                            displayUnit={displayUnit}
                                        />
                                    ) : (
                                        <div className="simulation-details-default-message">No resolved parameter set is available because this execution did not complete successfully.</div>
                                    )}
                                </section>

                                <details className="simulation-details-section simulation-edge-details">
                                    <summary>
                                        Pipe state used by this simulation {" · "} {edgeOverrides.length}{" pipes"}
                                    </summary>
                                    <div className="simulation-edge-state-list">
                                        {edgeOverrides.map(edge => (
                                            <div key={edge.edgeId}>
                                                <span>{edge.edgeId}</span>
                                                <strong>{edge.enabled ? "OPEN" : "CLOSED"}</strong>
                                            </div>
                                        ))}
                                    </div>
                                </details>
                                {details.errorMessage && (
                                    <section className="simulation-details-section">
                                        <h3>Error</h3>
                                        <div className="project-modal__error">{details.errorMessage}</div>
                                    </section>
                                )}
                            </>
                        )}
                </div>
                <footer className="project-modal__footer">
                    <button type="button"
                        className="project-modal__secondary"
                        onClick={onClose}>
                        Close
                    </button>
                </footer>
            </section>
        </div>,
        document.body
    );
}

function Detail({ label, value }: {
    label: string;
    value: unknown;
}) {
    return (
        <div className="simulation-detail-value">
            <span>{label}</span>
            <strong>{formatValue(value)}</strong>
        </div>
    );
}

function ParameterTable({ values, sources, displayUnit }: {
    values: Record<string, unknown>;
    sources?: Record<string, SimulationParameterSource>;
    displayUnit: DisplayLengthUnit;
}) {
    return (
        <div className="simulation-parameter-table-wrapper">
            <table className="simulation-parameter-table">
                <thead>
                    <tr>
                        <th>Parameter</th>
                        <th>Value</th>
                        {sources && <th>Source</th>}
                    </tr>
                </thead>

                <tbody>
                    {Object.entries(values).map(
                        ([key, value]) => (
                            <tr key={key}>
                                <td>{key}</td>
                                <td><pre>{formatSimulationParameterValue(key, value, displayUnit)}</pre></td>
                                {sources && (
                                    <td>
                                        <span className={`parameter-source source-${(
                                            sources[key] ?? "").toLowerCase()}`}>
                                            {sourceLabel(sources[key])}
                                        </span>
                                    </td>
                                )}
                            </tr>

                        )
                    )}
                </tbody>
            </table>
        </div>
    );
}

