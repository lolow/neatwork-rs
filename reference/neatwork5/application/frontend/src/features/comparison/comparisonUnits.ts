import {
    metersToDisplayUnit,
} from "../domain/unitSystem";

import type {
    DisplayLengthUnit,
} from "../domain/unitSystem";

import type {
    ComparisonColumn,
    ComparisonSourceType,
} from "./types";

export function comparisonDisplayUnitLabel(
    sourceType: ComparisonSourceType,
    column: ComparisonColumn,
    displayUnit: DisplayLengthUnit,
): string | null {

    // Simulation pressure is stored as metres of head.
    if (sourceType === "SIMULATION" && column.unit === "m") {
        return displayUnit;
    }

    // Velocity is stored as metres per second.
    if (sourceType === "SIMULATION" && column.unit === "m/s") {
        return `${displayUnit}/s`;
    }

    if (sourceType === "SIMULATION" && column.unit === "ratio") {
        return "%";
    }

    // Flow and ratio do not depend on length-display units.
    return column.unit;
}

export function convertSimulationDisplayValue(
    value:
        number,

    column:
        ComparisonColumn,

    displayUnit:
        DisplayLengthUnit,
): number {

    if (
        column.unit === "m"
    ) {

        return metersToDisplayUnit(
            value,
            displayUnit
        );
    }


    /*
     * 1 m/s → ft/s or in/s uses the same
     * length conversion factor.
     */
    if (
        column.unit === "m/s"
    ) {

        return metersToDisplayUnit(
            value,
            displayUnit
        );
    }


    return value;
}

export function tableSupportsDisplayUnits(
    columns:
        ComparisonColumn[],
): boolean {

    return columns.some(
        column =>
            column.unit === "m"
            ||
            column.unit === "m/s"
    );
}

