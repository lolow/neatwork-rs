import { useMemo, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import type { SimulationRunDetails } from "./types";
import { buildSimulationResultTables } from "./simulationResultTables";
import { SimulationResultTable } from "./SimulationResultTable";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import { SimulationReviewRunSummary } from "./SimulationReviewRunSummary";
import { formatAdaptiveNumber, formatRatioAsPercent } from "./resultNumberFormatting";

type Props = {
    details: SimulationRunDetails | null;
    loading: boolean;
    error: string | null;
    onClose: () => void;
    displayUnit: DisplayLengthUnit;
    embedded?: boolean;
    selectedTableKey?: string;
};

function formatMetric(value: number | null | undefined): string {
    if (value === null || value === undefined) {
        return "—";
    }

    return formatAdaptiveNumber(value);
}

function formatPercentage(value: number | null | undefined): string {
    if (value === null || value === undefined) {
        return "—";
    }

    return formatRatioAsPercent(value);
}

export function SimulationResultsDialog({ details, loading, error, onClose, displayUnit, embedded = false, selectedTableKey }: Props) {
    const statistics = details?.statistics ?? null;
    const resultTables = useMemo(() => statistics ? buildSimulationResultTables(statistics) : [], [statistics]);

    const [terminalDetailsExpanded, setTerminalDetailsExpanded] = useState(false);

    useEffect(() => {
        setTerminalDetailsExpanded(false);
    }, [details?.id]);

    const visibleTables = useMemo(() => {
        if (!selectedTableKey) {
            return resultTables;
        }

        return resultTables.filter(
            table => table.key === selectedTableKey
        );
    }, [resultTables, selectedTableKey]);

    const dialogContent = (
        <section className={embedded ? ("simulation-results-dialog " +
            "simulation-results-dialog--embedded")
            : "simulation-results-dialog"
        }
            role={embedded ? undefined : "dialog"}
            aria-modal={embedded ? undefined : true}
            aria-labelledby={embedded ? undefined : "simulation-results-title"}
        >
            {!embedded && (
                <header className="simulation-results-header">
                    <div>
                        <span className="simulation-results-eyebrow">Hydraulic results</span>

                        <h2 id="simulation-results-title">
                            {details ? `Simulation #${details.runNumber}` : "Simulation results"}
                        </h2>
                        {details && (
                            <p>Persisted hydraulic simulation result.</p>
                        )}
                    </div>
                    <button type="button" className="simulation-results-close"
                        aria-label="Close results"
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>
            )}
            <div className="simulation-results-body">{loading && (
                <div className="simulation-results-state">Loading simulation results...</div>
            )}
                {error && (
                    <div className="project-modal__error" role="alert">{error}</div>
                )}
                {!loading && details && statistics && (
                    <>
                        <SimulationReviewRunSummary details={details} />
                        <section className="simulation-results-summary">
                            <div>
                                <span>Status</span>
                                <strong>{details.status}</strong>
                            </div>
                            <div>
                                <span>Scenarios</span>
                                <strong>{statistics.summary.scenarioCount}</strong>
                            </div>
                            <div>
                                <span>Successful</span>
                                <strong>{statistics.summary.successfulScenarios}</strong>
                            </div>
                            <div>
                                <span>Execution failures</span>
                                <strong>{statistics.summary.failedScenarios}</strong>
                            </div>
                            <div>
                                <span>Success rate</span>
                                <strong>{formatPercentage(statistics.summary.successRate)}</strong>
                            </div>
                            <div>
                                <span>Duration</span>
                                <strong>{formatMetric(statistics.summary.durationSeconds)}
                                    {" "}
                                    {statistics.units.duration ?? "s"}
                                </strong>
                            </div>
                        </section>

                        <div className="simulation-results-tables">
                            {visibleTables.map(table => {
                                const terminalTable = table.key === "terminalSatisfaction";
                                return (
                                    <SimulationResultTable
                                        key={table.key}
                                        table={table}
                                        displayUnit={displayUnit}
                                        showDetails={terminalTable && terminalDetailsExpanded}
                                        headerAction={
                                            terminalTable ? (
                                                <button
                                                    type="button"
                                                    className="simulation-result-details-toggle"
                                                    aria-expanded={terminalDetailsExpanded}
                                                    onClick={() => setTerminalDetailsExpanded(current => !current)}
                                                >
                                                    {terminalDetailsExpanded ? "Hide details" : "Show details"}
                                                </button>
                                            ) : undefined
                                        }

                                    />
                                );
                            })}
                        </div>

                        {statistics.failures.length > 0 && (
                            <section className={"simulation-result-table-section " +
                                "simulation-failure-table"
                            }
                            >
                                <header className="simulation-result-table-header">
                                    <div>
                                        <h3>Execution failure records</h3>
                                        <p>Scenarios that failed during preparation solving or convergence.</p>
                                    </div>
                                    <span>{statistics.failures.length}{" records"}</span>
                                </header>

                                <div className="simulation-result-table-scroll">
                                    <table className="simulation-result-table">
                                        <thead>
                                            <tr>
                                                <th>Scenario</th>
                                                <th>Stage</th>
                                                <th>Code</th>
                                                <th>Message</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {statistics.failures.map(
                                                failure => (
                                                    <tr key={`${failure.scenarioIndex}-${failure.code}`}>
                                                        <td>{failure.scenarioIndex}</td>
                                                        <td>{failure.stage}</td>
                                                        <td>{failure.code}</td>
                                                        <td>{failure.message}</td>
                                                    </tr>
                                                ))}
                                        </tbody>
                                    </table>
                                </div>
                            </section>
                        )}
                    </>
                )}

                {!loading && details && !statistics && (
                    <div className="simulation-results-state error">
                        This execution does not contain hydraulic result tables.
                    </div>
                )}
            </div>
            {!embedded && (
                <footer className="simulation-results-footer">
                    <span>Result values use adaptive display precision. Stored simulation values retain full numerical precision.</span>
                    <button type="button" className="project-modal__secondary" onClick={onClose}>
                        Close
                    </button>
                </footer>
            )}
        </section>
    );

    if (embedded) {
        return dialogContent;
    }

    return createPortal(
        <div className="simulation-results-backdrop">{dialogContent}</div>,
        document.body
    );

}