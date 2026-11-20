import type { ComparisonMetricResult } from "./types";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import {
    getDisplayUnitSymbol,
    getVelocityDisplayUnitSymbol,
    metersToDisplayUnit,
} from "../domain/unitSystem";

import {
    formatAdaptiveNumber,
    formatRatioAsPercent,
    formatSignedNumber,
} from "../formatting/numericDisplay";

type Props = {
    result:
    ComparisonMetricResult;
    displayUnit: DisplayLengthUnit;
};


/**
 * Render aligned numeric comparison rows.
 */
export function ComparisonResultTable({ result, displayUnit }: Props) {
    if (result.rows.length === 0) {
        return (
            <div className="comparison-metric-empty">No matching numeric rows were found.</div>
        );
    }


    return (
        <div className="comparison-result-scroll">
            <table className="comparison-result-table">
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Left{formatUnitLabel(result.leftUnit, displayUnit)}</th>
                        <th>Right{formatUnitLabel(result.rightUnit, displayUnit)}</th>
                        <th>Difference Δ{result.unitsCompatible ? formatUnitLabel(result.leftUnit, displayUnit) : ""}</th>
                        <th>Ratio</th>
                        <th>Change %</th>
                    </tr>
                </thead>
                <tbody>

                    {result.rows.map(row => (
                        <tr key={row.identity} >

                            <td>{row.identity}</td>

                            <td>{formatAdaptiveNumber(displayMetricValue(row.leftValue, result.leftUnit, displayUnit))}</td>

                            <td>{formatAdaptiveNumber(displayMetricValue(row.rightValue, result.rightUnit, displayUnit))}</td>

                            <td className={valueClass(row.difference)}>{formatSignedNumber(result.unitsCompatible ? displayMetricValue(row.difference, result.leftUnit, displayUnit) : row.difference)} </td>

                            <td>{row.ratio === null ? "—" : formatAdaptiveNumber(row.ratio)}</td>

                            <td className={valueClass(row.percentageChange ?? 0)}>
                                {row.percentageChange === null ? "—" :
                                    (
                                        formatRatioAsPercent(row.percentageChange) + "%"
                                    )
                                }
                            </td>
                        </tr>
                    )
                    )
                    }

                </tbody>
            </table>

        </div>
    );
}

function valueClass(value: number): string {
    if (value > 0) {
        return "positive";
    }
    if (value < 0) {
        return "negative";
    }

    return "neutral";
}

//  convert display unit to canonical unit
function displayMetricUnit(canonicalUnit: string | null, displayUnit: DisplayLengthUnit): string | null {
    if (canonicalUnit === "m") {
        return getDisplayUnitSymbol(displayUnit);
    }
    if (canonicalUnit === "m/s") {
        return getVelocityDisplayUnitSymbol(displayUnit);
    }

    return canonicalUnit;
}

// convert canonical unit to display unit
function displayMetricValue(value: number, canonicalUnit: string | null, displayUnit: DisplayLengthUnit): number {
    if (canonicalUnit === "m" || canonicalUnit === "m/s") {
        return metersToDisplayUnit(value, displayUnit);
    }

    return value;
}

// Build the visible unit suffix for comparison values.
function formatUnitLabel(canonicalUnit: string | null, displayUnit: DisplayLengthUnit): string {
    if (!canonicalUnit || canonicalUnit === "ratio") {
        return "";
    }
    const unit = displayMetricUnit(canonicalUnit, displayUnit);

    return unit ? ` (${unit})` : "";
}
