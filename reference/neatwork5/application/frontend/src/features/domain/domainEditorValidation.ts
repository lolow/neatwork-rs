import { convertDisplayMeasurementToTableValue, getPhysicalColumnUnit } from "./unitSystem";
import type { DisplayLengthUnit } from "./unitSystem";

export type EditableCellParseResult =
    | { ok: true; value: unknown; }
    | { ok: false; message: string; };

type NumericRule = {
    integer?: boolean;
    min?: number;
    max?: number;
    exclusiveMin?: number;
    allowedValues?: number[];
};

function normalizeColumn(value: string): string {
    return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

function numericRuleForColumn(column: string, previousValue: unknown): NumericRule | null {
    const normalized = normalizeColumn(column);

    if (["statut", "status", "statuscode", "nodestatus", "nodestatuscode"]
        .includes(normalized)) {
        return {
            integer: true,
            allowedValues: [0, 1, 2],
        };
    }

    if (["proba", "probability", "openprobability", "opentaps"]
        .includes(normalized)) {
        return { min: 0, max: 1, };
    }

    if (["cible", "targetflow", "targetflowlps"]
        .includes(normalized)) {
        return { exclusiveMin: 0 };
    }

    if (["lon", "lon1", "lon2", "sdr", "cost", "pmax", "roughness",
    ].includes(normalized)) {
        return { min: 0 };
    }

    // Every physical measurement registered in unitSystem.ts is numeric.
    if (getPhysicalColumnUnit(column)) {
        return {};
    }

    // Existing numeric values remain numeric even when their legacy column name is unknown.
    if (typeof previousValue === "number") {
        return {};
    }

    return null;
}

export function validateStoredNumericValue(column: string, value: unknown): string | null {
    const rule = numericRuleForColumn(column, value);
    if (!rule) {
        return null;
    }
    if (typeof value !== "number" || !Number.isFinite(value)) {
        return "Expected a finite numeric value.";
    }
    if (rule.integer && !Number.isInteger(value)) {
        return "Expected an integer.";
    }
    if (rule.allowedValues && !rule.allowedValues.includes(value)) {
        return "Allowed values are 0, 1 or 2.";
    }
    if (rule.min !== undefined && value < rule.min) {
        return `Value must be at least ${rule.min}.`;
    }
    if (rule.max !== undefined && value > rule.max) {
        return `Value must not exceed ${rule.max}.`;
    }
    if (rule.exclusiveMin !== undefined && value <= rule.exclusiveMin) {
        return `Value must be greater than ${rule.exclusiveMin}.`;
    }
    return null;
}

export function parseDomainEditorCell(
    rawValue: string,
    column: string,
    displayUnit: DisplayLengthUnit,
    previousValue: unknown
): EditableCellParseResult {
    const rule = numericRuleForColumn(column, previousValue);

    // Identifier/text columns keep their string form.
    // For example "001" must remain "001".
    if (!rule) {
        return { ok: true, value: rawValue };
    }

    const trimmed = rawValue.trim();
    if (!trimmed) {
        return { ok: false, message: "A numeric value is required." };
    }

    const numeric = Number(trimmed);

    if (!Number.isFinite(numeric)) {
        return { ok: false, message: `"${rawValue}" is not a valid number.` };
    }

    if (rule.integer && !Number.isInteger(numeric)) {
        return { ok: false, message: "This value must be an integer." };
    }

    if (rule.allowedValues && !rule.allowedValues.includes(numeric)) {
        return { ok: false, message: "Allowed values are 0, 1 or 2." };
    }

    if (rule.min !== undefined && numeric < rule.min) {
        return { ok: false, message: `Value must be at least ${rule.min}.` };
    }

    if (rule.max !== undefined && numeric > rule.max) {
        return { ok: false, message: `Value must not exceed ${rule.max}.` };
    }

    if (rule.exclusiveMin !== undefined && numeric <= rule.exclusiveMin) {
        return { ok: false, message: `Value must be greater than ${rule.exclusiveMin}.` };
    }

    const canonical = convertDisplayMeasurementToTableValue(column, numeric, displayUnit);

    return {
        ok: true,
        value: canonical ?? numeric,
    };
}