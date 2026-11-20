import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { DISPLAY_LENGTH_UNITS } from "../domain/unitSystem";
import type { DisplayLengthUnit } from "../domain/unitSystem";

type Props = {
    projectName: string;
    currentUnit: DisplayLengthUnit;
    saving: boolean;
    error: string | null;
    onApply: (unit: DisplayLengthUnit) => Promise<void>;
    onClose: () => void;
};

export function UnitSettingsModal({
    projectName,
    currentUnit,
    saving,
    error,
    onApply,
    onClose,
}: Props) {

    const [pendingUnit, setPendingUnit] = useState<DisplayLengthUnit>(currentUnit);

    useEffect(() => {
        setPendingUnit(currentUnit);

    }, [currentUnit]);

    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (event.key === "Escape" && !saving) {
                onClose();
            }
        }
        window.addEventListener("keydown", handleKeyDown);

        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };

    }, [saving, onClose]);

    return createPortal(
        <div className="project-modal-backdrop">
            <section
                className="project-modal unit-settings-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="unit-settings-title"
            >
                <header className="project-modal__header">
                    <div>
                        <span className="project-modal__eyebrow">Project preferences</span>
                        <h2 id="unit-settings-title">Unit Settings</h2>
                        <p>{projectName}</p>
                    </div>

                    <button
                        type="button"
                        className="project-modal__close"
                        disabled={saving}
                        onClick={onClose}
                    >
                        ×
                    </button>
                </header>
                <div className="project-modal__body">
                    <p className="unit-settings-description">
                        Choose the unit used to display hydraulic lengths, pressure head, coordinates and velocities throughout this project.
                    </p>
                    <div className="unit-settings-options">
                        {DISPLAY_LENGTH_UNITS.map(
                            unit => (
                                <label
                                    key={unit.value}
                                    className={pendingUnit === unit.value
                                        ? "unit-settings-option selected"
                                        : "unit-settings-option"
                                    }
                                >
                                    <input
                                        type="radio"
                                        name="project-display-unit"
                                        value={unit.value}
                                        checked={pendingUnit === unit.value}
                                        disabled={saving}
                                        onChange={() => setPendingUnit(unit.value)}
                                    />
                                    <span>
                                        <strong>{unit.label}</strong>
                                        <small>{unit.symbol}</small>
                                    </span>
                                </label>
                            )
                        )}

                    </div>

                    <div className="unit-settings-note">
                        Internal hydraulic calculations remain in canonical SI units. This setting changes display and user input only.
                    </div>

                    {error && (
                        <div className="project-modal__error" role="alert">{error}</div>
                    )}

                </div>

                <footer className="project-modal__footer">

                    <button type="button" className="project-modal__secondary" disabled={saving} onClick={onClose}>
                        Cancel
                    </button>

                    <button
                        type="button"
                        className="project-modal__primary"
                        disabled={saving || pendingUnit === currentUnit}
                        onClick={() => void onApply(pendingUnit)}
                    >
                        {saving ? "Applying..." : "Apply"}
                    </button>
                </footer>
            </section>

        </div>,
        document.body
    );
}