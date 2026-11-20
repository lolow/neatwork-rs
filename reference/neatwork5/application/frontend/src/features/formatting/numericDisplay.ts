/**
 * Shared numeric presentation policy.
 * This module must never modify canonical values.
 * It only converts numbers to user-facing strings.
 */
export const SCIENTIFIC_NOTATION_THRESHOLD = 1.0e-4;
export const EXTRA_DECIMAL_THRESHOLD = 1.0e-2;

/**
 * Formats one continuous numerical value.
 * Important:
 * - Exact zero is displayed as 0.00.
 * - Non-zero values are never visually replaced by zero.
 * - Very small values use scientific notation.
 */
export function formatAdaptiveNumber(value: number): string {
    if (!Number.isFinite(value)) {
        return "—";
    }
    if (value === 0 || Object.is(value, -0)) {
        return "0.00";
    }
    const absolute = Math.abs(value);

    // Ordinary engineering values.
    if (absolute >= EXTRA_DECIMAL_THRESHOLD) {
        return value.toFixed(2);
    }
    // Small values remain in decimal notation while additional precision is meaningful.
    if (absolute >= SCIENTIFIC_NOTATION_THRESHOLD) {
        return value.toFixed(4);
    }

    // Very small non-zero values use scientific notation rather than becoming visual zero.
    return value.toExponential(2).replace("e+", "e");
}

/**
 * Formats a dimensionless ratio as a percentage.
 * Ratios may be greater than one.
 * For example, 1.23 is displayed as 123.00%.
 */
export function formatRatioAsPercent(ratio: number): string {
    if (!Number.isFinite(ratio)) {
        return "—";
    }
    return `${formatAdaptiveNumber(ratio * 100)}%`;
}

export function formatInteger(value: number): string {
    if (!Number.isFinite(value)) {
        return "—";
    }
    return Math.trunc(value).toLocaleString();
}

export function formatSignedNumber(value: number): string {
    const formatted = formatAdaptiveNumber(value);
    if (formatted === "—" || value <= 0) {
        return formatted;
    }
    return `+${formatted}`;
}