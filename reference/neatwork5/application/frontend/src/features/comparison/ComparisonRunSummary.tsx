import type { ComparisonDataset } from "./types";


function hasOwnOption(values: Record<string, unknown> | null | undefined, key: string): boolean {
    if (!values) {
        return false;
    }
    return Object.prototype.hasOwnProperty.call(values, key);
}

// Returns the parameter actually used by the solver.
// Resolved values are preferred over user-requested values because they represent execution truth.
function datasetOption(dataset: ComparisonDataset, key: string): unknown {
    if (hasOwnOption(dataset.resolvedOptions, key)) {
        return dataset.resolvedOptions?.[key];
    }
    if (hasOwnOption(dataset.requestedOptions, key)) {
        return dataset.requestedOptions?.[key];
    }
    return undefined;
}

function numericDatasetOption(dataset: ComparisonDataset, key: string): number | null {
    const value = datasetOption(dataset, key);
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

function booleanDatasetOption(dataset: ComparisonDataset, key: string): boolean | null {
    const value = datasetOption(dataset, key);
    if (typeof value === "boolean") {
        return value;
    }
    return null;
}

function orificeModeLabel(dataset: ComparisonDataset): string {
    const value = datasetOption(dataset, "orificeMode");
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

function targetFlowLabel(dataset: ComparisonDataset): string {
    if (datasetOption(dataset, "uniformDemand") !== true) {
        return "Per client";
    }
    const value = numericDatasetOption(dataset, "targetFlowLps");

    return value === null
        ? "—"
        : `${value.toFixed(2)} L/s`;
}

function openFaucetsLabel(dataset: ComparisonDataset): string {
    if (datasetOption(dataset, "uniformDemand") !== true) {
        return "Per client";
    }
    const value = numericDatasetOption(dataset, "openTapsProbability");
    if (value === null) {
        return "—";
    }

    return (`${value.toFixed(2)} (${(value * 100).toFixed(0)}%)`);
}

function ComparisonParameter({ label, value }: {
    label: string;
    value: string;
}) {
    return (
        <div className="comparison-run-parameter">
            <span>{label}</span>
            <strong className="comparison-run-parameter__value">
                {value}
            </strong>
        </div>
    );
}

function boundedFlowLabel(dataset: ComparisonDataset): string {
    const value = booleanDatasetOption(dataset, "boundedFlow");

    if (value === true) {
        return "Enabled";
    }
    if (value === false) {
        return "Disabled";
    }

    return "—";
}

type Props = {
    dataset: ComparisonDataset;
    side: "LEFT" | "RIGHT";
};

// Summarizes the execution parameters that are most useful when visually comparing two simulation runs.
export function ComparisonRunSummary({ dataset, side }: Props) {

    return (
        <section className={`comparison-run-summary ${side.toLowerCase()}`}>
            <header className="comparison-run-summary__header">
                <div className="comparison-run-summary__identity">
                    <span className="comparison-run-summary__side">{side}</span>
                    <div className="comparison-run-summary__title-line">
                        <strong className="comparison-run-summary__title">
                            {dataset.runNumber !== undefined
                                ? (`Simulation #${dataset.runNumber}`)
                                : dataset.label}
                        </strong>
                        {dataset.inputFilename && (
                            <small className="comparison-run-summary__input-file">
                                {dataset.inputFilename}

                                {dataset.inputVersion !== undefined
                                    ? ` · v${dataset.inputVersion}`
                                    : ""}
                            </small>
                        )}
                    </div>
                </div>
                {dataset.configurationMode && (
                    <span className={dataset.configurationMode === "MANUAL"
                        ? ("comparison-config-badge " + "manual")
                        : ("comparison-config-badge " + "default")}>
                        {dataset.configurationMode === "MANUAL"
                            ? "Manual"
                            : "Default"}
                    </span>
                )}
            </header>

            <div className="comparison-run-summary__parameters">
                <ComparisonParameter
                    label="Bounded flow"
                    value={boundedFlowLabel(dataset)}
                />

                <ComparisonParameter
                    label="Orifice model"
                    value={orificeModeLabel(dataset)}
                />

                <ComparisonParameter
                    label="Target flow"
                    value={targetFlowLabel(dataset)} />

                <ComparisonParameter
                    label="Open faucets"
                    value={openFaucetsLabel(dataset)} />
            </div>

        </section >
    );
}