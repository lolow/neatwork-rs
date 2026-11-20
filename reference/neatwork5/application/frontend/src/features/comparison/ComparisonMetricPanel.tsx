import { useEffect, useMemo, useState } from "react";

import {
    areMetricColumnsCompatible,
    compareMetrics,
    findMatchStrategies,
    findRecommendedMetricPair,
    getComparableColumns,
} from "./comparisonEngine";

import { ComparisonResultTable } from "./ComparisonResultTable";

import type { ComparisonDataset, ComparisonTable } from "./types";
import type { DisplayLengthUnit } from "../domain/unitSystem";

type Props = {
    leftDataset: ComparisonDataset;
    leftTable: ComparisonTable;
    rightDataset: ComparisonDataset;
    rightTable: ComparisonTable;
    displayUnit: DisplayLengthUnit;
};

export function ComparisonMetricPanel({
    leftDataset,
    leftTable,
    rightDataset,
    rightTable,
    displayUnit
}: Props) {

    const strategies = useMemo(() => findMatchStrategies(leftTable, rightTable), [leftTable, rightTable]);
    const leftColumns = useMemo(() => getComparableColumns(leftTable), [leftTable]);
    const rightColumns = useMemo(() => getComparableColumns(rightTable), [rightTable]);
    const recommended = useMemo(() => findRecommendedMetricPair(leftTable, rightTable), [leftTable, rightTable]);
    const [matchStrategyId, setMatchStrategyId] = useState("");
    const [leftColumnKey, setLeftColumnKey] = useState("");
    const [rightColumnKey, setRightColumnKey] = useState("");

    useEffect(() => {
        setMatchStrategyId(strategies[0]?.id ?? "");
        setLeftColumnKey(recommended?.leftColumnKey ?? leftColumns[0]?.key ?? "");
        setRightColumnKey(recommended?.rightColumnKey ?? rightColumns[0]?.key ?? "");

    }, [strategies, recommended, leftColumns, rightColumns]);

    const selectedStrategy = strategies.find(strategy => strategy.id === matchStrategyId) ?? null;
    const selectedLeftColumn = leftColumns.find(column => column.key === leftColumnKey) ?? null;
    const selectedRightColumn = rightColumns.find(column => column.key === rightColumnKey) ?? null;

    const result = useMemo(() => {
        if (!selectedStrategy || !selectedLeftColumn || !selectedRightColumn) {
            return null;
        }

        return compareMetrics(
            leftDataset, leftTable, selectedLeftColumn,
            rightDataset, rightTable, selectedRightColumn,
            selectedStrategy
        );
    },
        [leftDataset, leftTable, selectedLeftColumn,
            rightDataset, rightTable, selectedRightColumn,
            selectedStrategy,
        ]
    );

    return (
        <section className="comparison-metric-panel">

            <header className="comparison-metric-header">
                <div>
                    <h3>Numeric comparison</h3>
                    <p>Match hydraulic entities and compare selected numerical metrics.</p>
                </div>
            </header>

            {strategies.length === 0
                ? (
                    <div className="comparison-metric-message">
                        These tables do not expose a compatible entity identifier.
                        Side-by-side inspection remains available above.
                    </div>
                )
                : (
                    <>
                        <div className="comparison-metric-controls">
                            <label>
                                <span>Match rows by</span>

                                <select value={matchStrategyId} onChange={event => setMatchStrategyId(event.target.value)}>
                                    {strategies.map(
                                        strategy => (
                                            <option key={strategy.id} value={strategy.id}>
                                                {strategy.label}
                                            </option>
                                        )
                                    )
                                    }
                                </select>
                            </label>
                            <label>
                                <span> Left metric</span>

                                <select value={leftColumnKey} onChange={event => setLeftColumnKey(event.target.value)}>
                                    {leftColumns.map(
                                        column => (
                                            <option key={column.key} value={column.key}>
                                                {column.label}
                                            </option>
                                        )
                                    )
                                    }
                                </select>
                            </label>
                            <label>
                                <span>Right metric</span>

                                <select value={rightColumnKey} onChange={event => setRightColumnKey(event.target.value)}>
                                    {rightColumns.map(
                                        column => (
                                            <option key={column.key} value={column.key}>
                                                {column.label}
                                            </option>
                                        )
                                    )
                                    }
                                </select>
                            </label>
                        </div>
                        {selectedLeftColumn
                            && selectedRightColumn
                            && !areMetricColumnsCompatible(selectedLeftColumn, selectedRightColumn)
                            && (
                                <div className="comparison-metric-message warning">
                                    The selected metrics use different physical units.
                                    Difference, ratio and percentage change may not represent
                                    a meaningful hydraulic comparison.
                                </div>
                            )
                        }

                        {result && (
                            <div className="comparison-match-summary">
                                <span>
                                    Matched: {" "}
                                    <strong>{result.matchedRowCount}</strong>
                                </span>
                                <span>
                                    Left unmatched:{" "}
                                    <strong>{result.unmatchedLeftCount}</strong>
                                </span>
                                <span>
                                    Right unmatched: {" "}
                                    <strong>{result.unmatchedRightCount}</strong>
                                </span>

                                {(result.duplicateLeftCount + result.duplicateRightCount) > 0 && (
                                    <span className="warning">
                                        Duplicate IDs skipped: {" "}
                                        {result.duplicateLeftCount + result.duplicateRightCount}
                                    </span>
                                )
                                }

                            </div>
                        )
                        }
                        <div className="comparison-calculation-note">
                            Numeric calculations use canonical hydraulic units.
                            Values are converted to the project
                            display unit only after the comparison is
                            calculated.
                        </div>
                        {result && (
                            <ComparisonResultTable result={result} displayUnit={displayUnit} />
                        )
                        }
                    </>
                )
            }
        </section>
    );
}