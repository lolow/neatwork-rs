import { formatTableCell, labelTableColumn } from "../domain/tableUtils";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import { comparisonDisplayUnitLabel, convertSimulationDisplayValue } from "./comparisonUnits";
import type { ComparisonColumn, ComparisonSourceType } from "./types";
import { formatAdaptiveNumber, formatRatioAsPercent } from "../formatting/numericDisplay";

// Builds one comparison-column label including its current display unit.
export function buildComparisonColumnLabel(
    sourceType: ComparisonSourceType,
    column: ComparisonColumn,
    displayUnit: DisplayLengthUnit
): string {
    if (sourceType === "DEFAULT") {
        return labelTableColumn(column.key, displayUnit);
    }

    const unit = comparisonDisplayUnitLabel(sourceType, column, displayUnit);

    if (!unit) {
        return column.label;
    }
    return (`${column.label} (${unit})`);
}

// Formats comparison values only at presentation time.
// Hydraulic calculations and persisted values retain their full numerical precision.
export function formatComparisonValue(
    sourceType: ComparisonSourceType,
    value: unknown,
    column: ComparisonColumn,
    displayUnit: DisplayLengthUnit
): string {
    if (value === null || value === undefined) {
        return "—";
    }

    if (sourceType === "DEFAULT") {
        return formatTableCell(value, column.key, displayUnit);
    }

    if (typeof value === "number") {
        if (!Number.isFinite(value)) {
            return "—";
        }
        const converted = convertSimulationDisplayValue(value, column, displayUnit);

        if (column.unit === "ratio") {
            return formatRatioAsPercent(converted);
        }
        return formatAdaptiveNumber(converted);
    }

    if (typeof value === "boolean") {
        return value ? "Yes" : "No";
    }
    return String(value);
}