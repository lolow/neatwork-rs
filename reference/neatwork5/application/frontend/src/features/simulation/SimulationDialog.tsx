import { useEffect, useId, useRef, useState } from "react";
import type { FormEvent, MouseEvent, Ref } from "react";
import { createPortal } from "react-dom";

import type {
    SimulationOrificeMode,
    SimulationRunOptions,
    SimulationRunRequestOptions,
} from "./types";
import type { SimulationDialogDefaults } from "./simulationDefaults";

type NumberFieldProps = {
    label: string;
    name: string;
    value: number;
    min?: number;
    max?: number;
    step?: number | "any";
    disabled?: boolean;
    hint?: string;
    inputRef?: Ref<HTMLInputElement>;
    onChange: (value: number) => void;
};

type Props = {
    defaults: SimulationDialogDefaults;
    onClose: () => void;
    // Starts a run using only the active domain configuration and no runtime overrides.
    onRunDefault: () => Promise<boolean>;
    // Starts a run with only explicitly changed runtime values
    onRun: (options: SimulationRunRequestOptions) => Promise<boolean>;
};

function NumberField({
    label,
    name,
    value,
    min,
    max,
    step,
    disabled,
    hint,
    inputRef,
    onChange,
}: NumberFieldProps) {
    const inputId = useId();
    const hintId = `${inputId}-hint`;

    return (
        <label className="simulation-dialog-field" htmlFor={inputId}>
            <span>{label}</span>
            <input
                ref={inputRef}
                id={inputId}
                name={name}
                type="number"
                value={value}
                min={min}
                max={max}
                step={step}
                disabled={disabled}
                required={!disabled}
                aria-describedby={hint ? hintId : undefined}
                onChange={(event) => {
                    const nextValue = event.target.valueAsNumber;

                    if (Number.isFinite(nextValue)) {
                        onChange(nextValue);
                    }
                }}
            />
            {hint && <small id={hintId}>{hint}</small>}
        </label>
    );
}

function validateSimulationOptions(
    values: SimulationRunOptions,
    uniformDemand: boolean,
    boundedFlow: boolean,
): string | null {
    if (!Number.isInteger(values.scenarioCount) || values.scenarioCount < 1) {
        return "Number of scenarios must be a positive whole number.";
    }

    // The current solver still requires bounded flow for heterogeneous client demand.
    if (!uniformDemand && !boundedFlow) {
        return "The current solver requires bounded flow for heterogeneous client demand.";
    }

    if (values.maxGenerationBatches === undefined || !Number.isInteger(values.maxGenerationBatches) || values.maxGenerationBatches < 1) {
        return "Maximum generation batches must be a positive whole number.";
    }

    if (values.oversamplingFactor === undefined || !Number.isFinite(values.oversamplingFactor) || values.oversamplingFactor <= 1) {
        return "Oversampling factor must be greater than 1.";
    }

    if (values.maxDrawsPerBatch === undefined || !Number.isInteger(values.maxDrawsPerBatch) || values.maxDrawsPerBatch < 1) {
        return "Maximum draws per batch must be a positive whole number.";
    }

    return null;
}

// Resolves the initial bounded-flow state from the imported project configuration.
// Heterogeneous client demand currently requires bounded flow in the existing hydraulic solver.
function initialBoundedFlow(defaults: SimulationDialogDefaults): boolean {

    if (!defaults.uniformDemand) {
        return true;
    }
    return defaults.options.boundedFlow ?? false;
}

// Resolves the independently selected terminal orifice model.
function initialOrificeMode(defaults: SimulationDialogDefaults): SimulationOrificeMode {
    return defaults.options.orificeMode ?? "COMMERCIAL";
}

export function SimulationDialog({ defaults, onClose, onRunDefault, onRun }: Props) {
    const [boundedFlow, setBoundedFlow] = useState<boolean>(() => initialBoundedFlow(defaults));

    const [orificeMode, setOrificeMode] = useState<SimulationOrificeMode>(() => initialOrificeMode(defaults));

    const [values, setValues] = useState<SimulationRunOptions>(() => buildInitialSimulationValues(defaults));
    const [useSpecificSeed, setUseSpecificSeed] = useState(false);
    const [specificSeed, setSpecificSeed] = useState(42);
    const [submitting, setSubmitting] = useState(false);
    const [submitError, setSubmitError] = useState<string | null>(null);
    const firstInputRef = useRef<HTMLInputElement>(null);
    const dialogRef = useRef<HTMLFormElement>(null);
    const submittingRef = useRef(false);
    const onCloseRef = useRef(onClose);
    const titleId = useId();
    const descriptionId = useId();

    useEffect(() => {
        onCloseRef.current = onClose;
    }, [onClose]);

    useEffect(() => {
        const previouslyFocused = document.activeElement as HTMLElement | null;
        const previousBodyOverflow = document.body.style.overflow;
        const focusFrame = window.requestAnimationFrame(() => {
            firstInputRef.current?.focus();
            firstInputRef.current?.select();
        });

        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "Escape" && !submittingRef.current) {
                event.preventDefault();
                onCloseRef.current();
                return;
            }

            if (event.key === "Tab") {
                const focusableElements = Array.from(
                    dialogRef.current?.querySelectorAll<HTMLElement>(
                        "button:not(:disabled), input:not(:disabled), select:not(:disabled), summary, [tabindex]:not([tabindex='-1'])",
                    ) ?? [],
                );

                if (focusableElements.length === 0) {
                    return;
                }

                const firstElement = focusableElements[0];
                const lastElement = focusableElements[focusableElements.length - 1];

                if (event.shiftKey && document.activeElement === firstElement) {
                    event.preventDefault();
                    lastElement.focus();
                } else if (!event.shiftKey && document.activeElement === lastElement) {
                    event.preventDefault();
                    firstElement.focus();
                }
            }
        };

        document.body.style.overflow = "hidden";
        document.addEventListener("keydown", handleKeyDown);

        return () => {
            window.cancelAnimationFrame(focusFrame);
            document.body.style.overflow = previousBodyOverflow;
            document.removeEventListener("keydown", handleKeyDown);
            previouslyFocused?.focus();
        };
    }, []);

    const handleChange = <K extends keyof SimulationRunOptions>(
        key: K,
        value: SimulationRunOptions[K],
    ) => {
        setValues((previous) => ({ ...previous, [key]: value }));
    };

    // Builds the effective values displayed when the simulation configuration dialog first opens.
    // Project-level compatibility parameters and seed are not runtime overrides unless explicitly changed.
    function buildInitialSimulationValues(defaults: SimulationDialogDefaults): SimulationRunOptions {
        const values: SimulationRunOptions = {
            ...defaults.options,
        };
        delete values.seed;
        delete values.boundedFlow;
        delete values.orificeMode;
        delete values.serviceQuality;
        delete values.requiredPressureM;
        delete values.orificeCoefficient;
        delete values.faucetCoefficient;
        delete values.waterTemperatureC;
        delete values.commercialPipeLengthM;

        return values;
    }

    // Restores project-derived manual simulation parameters from the imported project.
    function handleReset() {
        setValues(buildInitialSimulationValues(defaults));
        setBoundedFlow(initialBoundedFlow(defaults));
        setOrificeMode(initialOrificeMode(defaults));
        setUseSpecificSeed(false);
        setSpecificSeed(42);
        setSubmitError(null);
    }

    // Adds one runtime option only when its current value differs from the value initially resolved from the imported project.
    function addChangedOption<K extends keyof SimulationRunOptions>(
        target: SimulationRunRequestOptions,
        key: K,
        current: SimulationRunOptions[K],
        baseline: SimulationRunOptions[K],
    ) {
        if (Object.is(current, baseline)) {
            return;
        }
        Object.assign(target, { [key]: current });
    }

    /**
     * Produces the smallest runtime override request.
     * Unchanged values are deliberately omitted so the backend/Python resolver remains the single source of truth for project defaults and provenance.
     */
    function buildRuntimeOverrides(
        defaults: SimulationDialogDefaults,
        values: SimulationRunOptions,
        boundedFlow: boolean,
        orificeMode: SimulationOrificeMode,
        useSpecificSeed: boolean,
        specificSeed: number,
    ): SimulationRunRequestOptions {
        const baselineValues = buildInitialSimulationValues(defaults);
        const overrides: SimulationRunRequestOptions = {};

        addChangedOption(overrides, "scenarioCount", values.scenarioCount, baselineValues.scenarioCount);

        addChangedOption(overrides, "maxGenerationBatches", values.maxGenerationBatches, baselineValues.maxGenerationBatches);

        addChangedOption(overrides, "oversamplingFactor", values.oversamplingFactor, baselineValues.oversamplingFactor);

        addChangedOption(overrides, "maxDrawsPerBatch", values.maxDrawsPerBatch, baselineValues.maxDrawsPerBatch);

        const baselineBoundedFlow = initialBoundedFlow(defaults);
        const baselineOrificeMode = initialOrificeMode(defaults);
        // Bounded flow and orifice model are independent runtime parameters.
        if (boundedFlow !== baselineBoundedFlow) {
            overrides.boundedFlow = boundedFlow;
        }
        if (orificeMode !== baselineOrificeMode) {
            overrides.orificeMode = orificeMode;
        }
        if (useSpecificSeed) {
            overrides.seed = specificSeed;
        }

        return overrides;
    }

    const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault();
        if (submitting) {
            return;
        }

        const validationError = validateSimulationOptions(
            values,
            defaults.uniformDemand,
            boundedFlow,
        );

        if (validationError) {
            setSubmitError(validationError);
            return;
        }

        // Validate the optional execution seed separately from the project-derived simulation parameters.
        if (useSpecificSeed && (!Number.isInteger(specificSeed) || specificSeed < 0)) {
            setSubmitError("Random seed must be a non-negative whole number.");
            return;
        }

        const runtimeOverrides = buildRuntimeOverrides(
            defaults, values, boundedFlow,
            orificeMode, useSpecificSeed, specificSeed
        );

        submittingRef.current = true;
        setSubmitting(true);
        setSubmitError(null);

        try {
            const hasOverrides = Object.keys(runtimeOverrides).length > 0;

            const started = hasOverrides
                ? await onRun(runtimeOverrides)
                : await onRunDefault();

            if (started) {
                onClose();
                return;
            }

            setSubmitError(
                "The simulation could not be started. Review the message above and try again.",
            );
        } catch (error) {
            console.error("Could not submit simulation settings:", error);
            setSubmitError("The simulation could not be started. Please try again.");
        }

        submittingRef.current = false;
        setSubmitting(false);
    };

    const handleBackdropMouseDown = (event: MouseEvent<HTMLDivElement>) => {
        if (event.target === event.currentTarget && !submitting) {
            onClose();
        }
    };

    const runtimeOverrides = buildRuntimeOverrides(
        defaults, values, boundedFlow,
        orificeMode, useSpecificSeed, specificSeed
    );

    const hasRuntimeOverrides = Object.keys(runtimeOverrides).length > 0;
    const uniformDemandDisabled = !defaults.uniformDemand;

    const clientDemandComplete =
        defaults.clientDemand.clientCount > 0
        &&
        defaults.clientDemand.completeClientCount === defaults.clientDemand.clientCount;

    return createPortal(
        <div className="simulation-dialog-backdrop" onMouseDown={handleBackdropMouseDown}>
            <form
                ref={dialogRef}
                className="simulation-dialog"
                noValidate
                role="dialog"
                aria-modal="true"
                aria-labelledby={titleId}
                aria-describedby={descriptionId}
                aria-busy={submitting}
                onSubmit={handleSubmit}
            >
                <header className="simulation-dialog-header">
                    <div>
                        <span className="simulation-dialog-kicker">Simulation configuration</span>
                        <h2 id={titleId}>Configure simulation</h2>
                        <p id={descriptionId}>
                            Values from the active{" "} {defaults.sourceFormat ?? "network"}{" "} input are prefilled. Run immediately to use
                            them unchanged, or edit only the settings you want to override for this execution.
                        </p>
                        <div className="simulation-configuration-state">
                            <span className={hasRuntimeOverrides
                                ? "simulation-configuration-state__badge custom"
                                : "simulation-configuration-state__badge default"
                            }>
                                {hasRuntimeOverrides
                                    ? "Custom overrides"
                                    : "Imported defaults"}
                            </span>

                            <small>
                                {hasRuntimeOverrides
                                    ? (`${Object.keys(runtimeOverrides).length} runtime override`
                                        +
                                        (Object.keys(runtimeOverrides).length === 1 ? "" : "s"))
                                    : "No runtime overrides"}
                            </small>
                        </div>
                    </div>
                    <button
                        type="button"
                        className="simulation-dialog-close"
                        aria-label="Close simulation settings"
                        disabled={submitting}
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>

                <div className="simulation-dialog-body">
                    <section className="simulation-dialog-section">
                        <header>
                            <h3>Hydraulic configuration</h3>
                            <p>Configure flow limitation and the terminal-orifice model independently.</p>
                        </header>
                        <div className="simulation-hydraulic-settings">
                            <label className="simulation-hydraulic-setting">
                                <span className="simulation-hydraulic-setting__content">
                                    <strong>Bounded flow</strong>
                                    <small>Constrain terminal flow according to the current bounded-flow model.</small>
                                </span>
                                <input type="checkbox"
                                    checked={boundedFlow}
                                    disabled={!defaults.uniformDemand}
                                    onChange={event =>
                                        setBoundedFlow(event.target.checked)}
                                />
                            </label>
                            <fieldset className="simulation-orifice-model">
                                <legend>Orifice model</legend>
                                <p>Select which stored terminal-orifice model is associated with this execution.</p>
                                <div className="simulation-orifice-choice">
                                    <label>
                                        <input type="radio"
                                            name="orificeMode"
                                            checked={orificeMode === "IDEAL"}
                                            onChange={() => setOrificeMode("IDEAL")}
                                        />
                                        <span>Ideal</span>
                                    </label>
                                    <label>
                                        <input type="radio"
                                            name="orificeMode"
                                            checked={orificeMode === "COMMERCIAL"}
                                            onChange={() => setOrificeMode("COMMERCIAL")}
                                        />
                                        <span>Commercial</span>
                                    </label>
                                </div>
                            </fieldset>
                        </div>

                        {!defaults.uniformDemand && (
                            <div className="simulation-dialog-note">
                                This network contains heterogeneous client
                                demand. Bounded flow is required by the
                                current solver, while the orifice model is
                                still stored independently.
                            </div>
                        )}
                    </section>
                    <section className="simulation-dialog-section">
                        <div className="simulation-dialog-section-heading">
                            <h3>Run setup</h3>
                            <p>Configure the scenario count and optional reproducibility settings for this execution.</p>
                        </div>
                        <div className="simulation-dialog-grid">
                            <NumberField
                                inputRef={firstInputRef}
                                label="Number of scenarios"
                                name="scenarioCount"
                                value={values.scenarioCount}
                                min={1}
                                step={1}
                                onChange={(value) =>
                                    handleChange("scenarioCount", Math.max(1, Math.trunc(value)))
                                }
                            />
                        </div>
                        <div className="simulation-seed-control">
                            <label className="simulation-seed-toggle">
                                <span>
                                    <strong>Use a specific seed</strong>
                                    <small>Enable this option when you want the simulation to be exactly reproducible.</small>
                                </span>
                                <input
                                    type="checkbox"
                                    checked={useSpecificSeed}
                                    disabled={submitting}
                                    onChange={event => {
                                        const enabled = event.target.checked;
                                        setUseSpecificSeed(enabled);

                                        // Start every newly enabled explicit seed configuration from 42.
                                        if (enabled) {
                                            setSpecificSeed(42);
                                        }
                                    }}
                                />
                            </label>
                            {!useSpecificSeed && (
                                <div className="simulation-seed-random-note">
                                    A fresh random seed will be generated
                                    automatically when the simulation starts.
                                </div>
                            )}
                            {useSpecificSeed && (
                                <NumberField
                                    label="Random seed"
                                    name="seed"
                                    value={specificSeed}
                                    min={0}
                                    step={1}
                                    disabled={submitting}
                                    hint={"Default explicit seed: 42"}
                                    onChange={value =>
                                        setSpecificSeed(Math.max(0, Math.trunc(value)))}
                                />
                            )}
                        </div>
                    </section>
                    <section className="simulation-dialog-section">
                        <header>
                            <h3>Client demand</h3>
                            <p>
                                Demand values are read from the active Client demand table and cannot be overridden per simulation.
                            </p>
                        </header>
                        <div className="simulation-demand-source">
                            {!clientDemandComplete && (
                                <div className="project-modal__error">
                                    Some clients are missing an opening
                                    probability or target flow. Complete the
                                    Client demand table before simulation.
                                </div>

                            )}
                            <div>
                                <span>Source</span>
                                <strong>Client demand table</strong>
                            </div>
                            <div>
                                <span>Clients</span>
                                <strong>{defaults.clientDemand.clientCount}</strong>
                            </div>
                            <div>
                                <span>Demand profile</span>
                                <strong>
                                    {defaults.clientDemand.uniform ? "Uniform" : "Per client"}
                                </strong>
                            </div>

                            {defaults.clientDemand.uniform && (
                                <>
                                    <div>
                                        <span>Open faucets</span>
                                        <strong>
                                            {defaults.clientDemand.openProbability !== null
                                                ? (
                                                    `${defaults.clientDemand.openProbability.toFixed(2)} `
                                                    +
                                                    `(${(defaults.clientDemand.openProbability * 100).toFixed(0)}%)`
                                                )
                                                : "—"
                                            }
                                        </strong>
                                    </div>
                                    <div>
                                        <span>Target flow</span>
                                        <strong>
                                            {defaults.clientDemand.targetFlowLps !== null ? (
                                                `${defaults.clientDemand.targetFlowLps.toFixed(2)} L/s`
                                            )
                                                : "—"
                                            }
                                        </strong>
                                    </div>
                                </>
                            )}
                        </div>
                        <p className="simulation-dialog-note">
                            To change demand values, edit the Client demand table and save a new input revision before running the simulation.
                        </p>

                    </section>

                    <details className="simulation-dialog-advanced">
                        <summary>Advanced scenario generation</summary>
                        <p>Safety limits for random scenario generation. Defaults suit most networks.</p>
                        <div className="simulation-dialog-grid">
                            <NumberField
                                label="Maximum generation batches"
                                name="maxGenerationBatches"
                                value={values.maxGenerationBatches ?? 20}
                                min={1}
                                step={1}
                                onChange={(value) =>
                                    handleChange("maxGenerationBatches", Math.max(1, Math.trunc(value)))
                                }
                            />
                            <NumberField
                                label="Oversampling factor"
                                name="oversamplingFactor"
                                value={values.oversamplingFactor ?? 1.1}
                                min={1}
                                step="any"
                                onChange={(value) => handleChange("oversamplingFactor", value)}
                            />
                            <NumberField
                                label="Maximum draws per batch"
                                name="maxDrawsPerBatch"
                                value={values.maxDrawsPerBatch ?? 100000}
                                min={1}
                                step={1}
                                onChange={(value) =>
                                    handleChange("maxDrawsPerBatch", Math.max(1, Math.trunc(value)))
                                }
                            />
                        </div>
                    </details>

                    {submitError && (
                        <p className="simulation-dialog-error" role="alert">
                            {submitError}
                        </p>
                    )}
                </div>

                <footer className="simulation-dialog-footer">
                    <button
                        type="button"
                        className="simulation-dialog-button secondary"
                        disabled={submitting || !hasRuntimeOverrides}
                        onClick={handleReset}
                    >
                        Reset to imported defaults
                    </button>
                    <div>
                        <button
                            type="button"
                            className="simulation-dialog-button secondary"
                            disabled={submitting}
                            onClick={onClose}
                        >
                            Cancel
                        </button>
                        <button
                            type="submit"
                            className="simulation-dialog-button primary"
                            disabled={submitting || !clientDemandComplete}
                        >
                            {submitting ? "Starting…" : hasRuntimeOverrides ? "Run with overrides" : "Run with imported defaults"}
                        </button>
                    </div>
                </footer>
            </form>
        </div>,
        document.body,
    );
}