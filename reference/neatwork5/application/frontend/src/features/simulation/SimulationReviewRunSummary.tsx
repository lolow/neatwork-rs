import type { SimulationRunDetails } from "./types";

// Checks whether one runtime parameter was explicitly stored in a simulation option object.
function hasOwnOption(values: Record<string, unknown>, key: string): boolean {
    return Object.prototype.hasOwnProperty.call(values, key);
}

// Returns the effective value used by the simulation. Resolved options are preferred because they contain the final parameters actually sent to the solver.
function simulationOption(details: SimulationRunDetails, key: string): unknown {
    const resolved = details.resolvedOptions ?? {};
    if (hasOwnOption(resolved, key)) {
        return resolved[key];
    }
    if (hasOwnOption(details.requestedOptions, key)) {
        return details.requestedOptions[key];
    }

    return undefined;
}

// Helper to extract numeric values from options, supporting both number and string forms.
function numericOption(details: SimulationRunDetails, key: string): number | null {
    const value = simulationOption(details, key);
    if (typeof value === "number" && Number.isFinite(value)) {
        return value;
    }

    if (typeof value === "string") {
        const parsed = Number(value);
        if (Number.isFinite(parsed)) {
            return parsed;
        }
    }

    return null;
}

// Helper to extract boolean values from options.
function booleanOption(details: SimulationRunDetails, key: string): boolean | null {
    const value = simulationOption(details, key);
    if (typeof value === "boolean") {
        return value;
    }

    return null;
}

// Helper to format the target flow value with units.
function formatTargetFlow(details: SimulationRunDetails): string {
    if (!hasUniformDemand(details)) {
        return "Per client";
    }
    const value = numericOption(details, "targetFlowLps");

    return value === null
        ? "—"
        : `${value.toFixed(2)} L/s`;
}

// Helper to format the open taps fraction with percentage.
function formatOpenFaucetsFraction(details: SimulationRunDetails): string {
    if (!hasUniformDemand(details)) {
        return "Per client";
    }
    const value = numericOption(details, "openTapsProbability");
    if (value === null) {
        return "—";
    }

    return (`${value.toFixed(2)} ` + `(${(value * 100).toFixed(0)}%)`);
}

// Returns the bounded flow used for the simulation.
function boundedFlowLabel(details: SimulationRunDetails): string {
    const value = booleanOption(details, "boundedFlow");
    if (value === true) {
        return "Enabled";
    }
    if (value === false) {
        return "Disabled";
    }

    return "—";
}

// Returns the orifice model used for the simulation.
function orificeModeLabel(details: SimulationRunDetails): string {
    const value = simulationOption(details, "orificeMode");

    if (typeof value !== "string") {
        return "—";
    }

    switch (value.toUpperCase()) {
        case "IDEAL":
            return "Ideal";

        case "COMMERCIAL":
            return "Commercial";

        default:
            return value;
    }
}

function hasUniformDemand(details: SimulationRunDetails): boolean {

    return (simulationOption(details, "uniformDemand") === true);
}

function ReviewParameter({ label, value, prominent = false }: { label: string; value: string; prominent?: boolean; }) {
    return (
        <div className={prominent ? "simulation-review-parameter prominent" : "simulation-review-parameter"}>
            <span className="simulation-review-parameter__label">
                {label}
            </span>

            <strong className="simulation-review-parameter__value">
                {value}
            </strong>
        </div>
    );
}

export function SimulationReviewRunSummary({ details }: { details: SimulationRunDetails }) {
    const boundedFlow = booleanOption(details, "boundedFlow");
    return (
        <section className="simulation-review-run-summary">
            <header className="simulation-review-run-summary__header">
                <div>
                    <span>Selected execution</span>
                    <strong>Simulation #{details.runNumber}</strong>
                </div>

                <span
                    className={
                        details.configurationMode === "MANUAL"
                            ? ("simulation-review-config-badge " + "manual")
                            : ("simulation-review-config-badge " + "default")
                    }
                >
                    {details.configurationMode === "MANUAL" ? "Manual" : "Default"}
                </span>
            </header>
            <div className="simulation-review-parameters">
                <ReviewParameter
                    label="Bounded flow"
                    value={boundedFlowLabel(details)}
                    prominent
                />
                <ReviewParameter
                    label="Orifice model"
                    value={orificeModeLabel(details)}
                    prominent
                />
                <ReviewParameter
                    label="Target flow"
                    value={formatTargetFlow(details)}
                    prominent
                />
                <ReviewParameter
                    label="Open faucets"
                    value={formatOpenFaucetsFraction(details)}
                    prominent
                />
            </div>
            <details className="simulation-review-traceability">
                <summary>Run traceability</summary>
                <div className="simulation-review-traceability__grid">
                    <div>
                        <span>Input network</span>
                        <strong>{details.input.networkName ?? "—"}</strong>
                    </div>
                    <div>
                        <span>Source file</span>
                        <strong>{details.input.filename ?? "—"}</strong>
                    </div>
                    <div>
                        <span>Version</span>
                        <strong>
                            {details.input.version !== null
                                ? (`v${details.input.version}`) : "—"
                            }
                        </strong>
                    </div>
                    <div>
                        <span>Format</span>
                        <strong>
                            {details.input.sourceFormat ?? "—"
                            }
                        </strong>
                    </div>
                    <div>
                        <span>Scenarios</span>
                        <strong>{details.scenarioCount ?? "—"}</strong>
                    </div>
                    <div>
                        <span>Effective seed</span>
                        <strong>{details.seed ?? "—"}</strong>
                    </div>
                </div>
            </details>
        </section>
    );
}