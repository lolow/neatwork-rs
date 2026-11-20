import { useState, type ReactNode } from "react";

import type {
    SimulationRunRequestOptions,
    SimulationStatisticsPayload,
    SimulationSummary,
} from "./types";
import type { SimulationDialogDefaults } from "./simulationDefaults";
import { SimulationDialog } from "./SimulationDialog";


type Props = {
    historyCount: number;
    onOpenHistory: () => void;
    disabled: boolean;
    activeRunCount: number;
    foregroundRunning: boolean;
    cancelling: boolean;
    defaults: SimulationDialogDefaults;
    summary: SimulationSummary | null;
    statistics: SimulationStatisticsPayload | null;
    canRunAgain: boolean;
    canReview: boolean;
    exportControl?: ReactNode;
    // Starts one simulation with only the runtime overrides explicitly changed by the user.
    onRun: (options: SimulationRunRequestOptions) => Promise<boolean>;
    // Runs the active domain without any frontend runtime overrides.
    onRunDefault: () => Promise<boolean>;
    onRunAgain: () => Promise<void>;
    onCancel: () => Promise<void>;
    onOpenReview: () => void;
};

export function SimulationPanel({
    historyCount,
    onOpenHistory,
    disabled,
    activeRunCount,
    foregroundRunning,
    cancelling,
    defaults,
    summary,
    statistics,
    canRunAgain,
    canReview,
    exportControl,
    onRun,
    onRunDefault,
    onRunAgain,
    onCancel,
    onOpenReview,
}: Props) {

    const [configurationDialogOpen, setConfigurationDialogOpen] = useState(false);

    // Closes every simulation configuration dialog.
    function closeSimulationSetup() {
        setConfigurationDialogOpen(false);
    }

    return (
        <section className="simulation-bar">
            <div className="simulation-controls">
                <button
                    disabled={disabled}
                    onClick={() => setConfigurationDialogOpen(true)}
                >
                    Simulation
                </button>

                {canRunAgain && (
                    <button type="button" onClick={onRunAgain}>
                        Run Again
                    </button>
                )}
                {/* <button type="button"
                    disabled={disabled}
                    onClick={onOpenHistory}
                >
                    History
                    {historyCount > 0 && (
                        <span className="simulation-history-count">{historyCount}</span>
                    )}
                </button> */}
                {canReview && (
                    <button
                        type="button"
                        disabled={disabled}
                        onClick={onOpenReview}
                    >
                        Review
                    </button>
                )}


                {canReview && exportControl}

                {foregroundRunning && (
                    <button
                        type="button"
                        className="simulation-cancel-button"
                        disabled={cancelling}
                        onClick={onCancel}
                    >
                        {cancelling ? "Cancelling..." : "Cancel"}
                    </button>
                )}
            </div>

            {activeRunCount > 0 && (
                <div className="simulation-running-status" role="status" aria-live="polite">
                    <span aria-hidden="true" />
                    {activeRunCount === 1
                        ? "1 hydraulic simulation is running."
                        : `${activeRunCount} hydraulic simulations are running.`
                    }
                </div>
            )}

            {summary && (
                <div className="simulation-summary">
                    <span>
                        <strong>{summary.status}</strong>
                    </span>
                    <span>
                        Success{" "}
                        <strong>
                            {summary.successfulScenarios}/{summary.scenarioCount}
                        </strong>
                    </span>
                    <span>
                        Execution failures {" "} <strong>{summary.failedScenarios}</strong>
                    </span>

                    <span>
                        Low P <strong>{summary.lowPressureNodes}</strong>
                    </span>
                    <span>
                        Seed <strong>{summary.seed}</strong>
                    </span>
                    {statistics && statistics.failures.length > 0 && (
                        <span>
                            Failure records <strong>{statistics.failures.length}</strong>
                        </span>
                    )}
                </div>
            )}

            {configurationDialogOpen && (
                <SimulationDialog
                    defaults={defaults}
                    onRunDefault={onRunDefault}
                    onRun={onRun}
                    onClose={closeSimulationSetup}
                />
            )}

        </section>
    );
}