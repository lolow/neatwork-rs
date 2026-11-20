import { useEffect, useMemo, useState, Fragment } from "react";
import type { ReactNode } from "react";
import type { SimulationResultColumn, SimulationResultTable as ResultTable } from "./simulationResultTables";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import {
    getDisplayUnitSymbol,
    getVelocityDisplayUnitSymbol,
    metersToDisplayUnit,
} from "../domain/unitSystem";
import { formatAdaptiveNumber, formatRatioAsPercent, formatInteger } from "../formatting/numericDisplay";
import { NodeIdBadge, NodeTypeBadge } from "../network/nodeSemantics";
import type { NodeSemanticType } from "../network/nodeSemantics";


type Props = {
    table: ResultTable;
    displayUnit: DisplayLengthUnit;
    // Controls visibility of columns marked as detailsOnly.
    showDetails?: boolean;
    // Optional extra row content displayed below each result row while details are expanded.
    renderRowDetails?: (row: Record<string, unknown>) => ReactNode;
    headerAction?: ReactNode;
};

type SortDirection = "ASC" | "DESC";

type SortState = {
    columnKey: string;
    direction: SortDirection;
};

// Formats one result cell according to its explicit column metadata.
function formatResultCell(value: unknown, column: SimulationResultColumn, displayUnit: DisplayLengthUnit): string {

    if (value === null || value === undefined) {
        return "—";
    }

    if (column.numeric) {
        const numericValue = typeof value === "number" ? value : Number(value);
        if (!Number.isFinite(numericValue)) {
            return "—";
        }
        const converted = convertResultNumber(numericValue, column.unit, displayUnit);
        if (column.format === "INTEGER") {
            return formatInteger(converted);
        }
        if (column.format === "PERCENT") {
            return formatRatioAsPercent(converted);
        }
        return formatAdaptiveNumber(converted);
    }
    if (typeof value === "boolean") {
        return value ? "Yes" : "No";
    }

    if (typeof value === "object") {
        return JSON.stringify(value);
    }

    return String(value);
}

// Converts canonical result units to the active project display unit.
function resultDisplayUnit(canonicalUnit: string | null | undefined,
    displayUnit: DisplayLengthUnit): string | null {

    if (canonicalUnit === "m") {
        return getDisplayUnitSymbol(displayUnit);
    }
    if (canonicalUnit === "m/s") {
        return getVelocityDisplayUnitSymbol(displayUnit);
    }
    return canonicalUnit ?? null;
}

// Convert canonical hydraulic results only at presentation time.
function convertResultNumber(value: number, canonicalUnit: string | null | undefined,
    displayUnit: DisplayLengthUnit): number {
    if (canonicalUnit === "m" || canonicalUnit === "m/s") {
        return metersToDisplayUnit(value, displayUnit);
    }
    return value;
}

// Identifies entity and topology columns that should receive a subtle visual background for easier scanning.
function isResultIdentityColumn(columnKey: string): boolean {
    return [
        "terminalId",
        "sourceId",
        "edgeId",
        "startNodeId",
        "endNodeId",
        "transitNodeId",
    ].includes(columnKey);
}

const resultTextCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

// Returns true if a result cell should be treated as empty.
function isEmptyResultValue(value: unknown): boolean {
    return (value === null || value === undefined || value === "");
}

// Comparison logic used when sorting results.
function compareResultValues(left: unknown,
    right: unknown,
    column: SimulationResultColumn,
): number {

    const leftEmpty = isEmptyResultValue(left);
    const rightEmpty = isEmptyResultValue(right);
    if (leftEmpty && rightEmpty) {
        return 0;
    }
    if (leftEmpty) {
        return 1;
    }
    if (rightEmpty) {
        return -1;
    }
    if (column.numeric) {
        const leftNumber = Number(left);
        const rightNumber = Number(right);
        if (Number.isFinite(leftNumber) && Number.isFinite(rightNumber)) {
            return leftNumber - rightNumber;
        }
    }
    return resultTextCollator.compare(String(left), String(right));
}

function getResultColumnGroup(columnKey: string): "identity" | "stats" | "demand" | "quality" {

    if (
        [
            "terminalId",
            "sourceId",
            "edgeId",
            "startNodeId",
            "endNodeId",
            "transitNodeId",
        ].includes(columnKey)
    ) {
        return "identity";
    }


    if (
        [
            "minSat",
            "meanSat",
            "maxSat",
            "cvar5Sat",
            "cvar20Sat",
        ].includes(columnKey)
    ) {
        return "stats";
    }

    if (
        [
            "clientCount",
            "targetFlowSummary",
            "probabilitySummary",
        ].includes(columnKey)
    ) {
        return "demand";
    }

    return "quality";
}

export function SimulationResultTable({ table, displayUnit, showDetails = false, renderRowDetails, headerAction }: Props) {
    const [sortState, setSortState] = useState<SortState | null>(null);
    const visibleColumns = useMemo(() => table.columns.filter(
        column => !column.detailsOnly || showDetails)
        , [table.columns, showDetails]
    );
    const displayedRows = useMemo(() => {
        const indexedRows = table.rows.map((row, originalIndex) =>
            ({ row, originalIndex }))
        if (!sortState) {
            return indexedRows;
        }
        const column = visibleColumns.find(
            candidate => candidate.key === sortState.columnKey);

        if (!column) {
            return indexedRows;
        }

        return [...indexedRows].sort((
            left,
            right) => {
            const leftValue = left.row[column.key];
            const rightValue = right.row[column.key];
            const leftEmpty = isEmptyResultValue(leftValue);
            const rightEmpty = isEmptyResultValue(rightValue);

            if (leftEmpty && !rightEmpty) {
                return 1;
            }
            if (!leftEmpty && rightEmpty) {
                return -1;
            }
            const compared = compareResultValues(leftValue, rightValue, column);

            if (compared === 0) {
                //Stable deterministic fallback.
                return left.originalIndex - right.originalIndex;
            }

            return (
                sortState.direction === "ASC" ? compared : -compared
            );
        });
    }, [table.rows, sortState, visibleColumns]);

    function handleSort(columnKey: string) {
        setSortState(current => {
            if (!current || current.columnKey !== columnKey) {
                return {
                    columnKey,
                    direction: "ASC",
                };
            }
            if (current.direction === "ASC") {
                return {
                    columnKey,
                    direction: "DESC",
                };
            }
            // Third click restores the original simulation order.
            return null;
        }
        );
    }

    function resolveColumnNodeType(row: Record<string, unknown>, column: SimulationResultColumn): NodeSemanticType | null {
        if (column.nodeType) {
            return column.nodeType;
        }

        if (column.nodeTypeKey) {
            const value = row[column.nodeTypeKey];
            if (value === "SOURCE" || value === "JUNCTION" || value === "TERMINAL") {
                return value;
            }
        }
        return null;
    }

    function renderResultCell(row: Record<string, unknown>, column: SimulationResultColumn) {
        const value = row[column.key];
        const nodeType = resolveColumnNodeType(row, column);
        if (column.nodePresentation === "ID") {
            return (
                <NodeIdBadge
                    nodeId={value}
                    type={nodeType}
                />
            );
        }
        if (column.nodePresentation === "TYPE") {
            return <NodeTypeBadge type={nodeType} />;
        }
        return formatResultCell(value, column, displayUnit);
    }

    useEffect(() => {
        setSortState(null);
    }, [table.key]);

    useEffect(() => {
        if (!sortState) { return; }
        const stillVisible = visibleColumns.some(
            column => column.key === sortState.columnKey);
        if (!stillVisible) {
            setSortState(null);
        }
    }, [sortState, visibleColumns]
    );

    return (
        <section className="simulation-result-table-section">
            <header className="simulation-result-table-header">
                <div>
                    <h3>{table.title}</h3>
                    <p>{table.description}</p>
                </div>
                <div className="simulation-result-table-header__actions">
                    <span>
                        {table.rows.length}{" "}
                        {table.rows.length === 1 ? "row" : "rows"}
                    </span>
                    {headerAction}
                </div>
            </header>
            <div className={showDetails ? ("simulation-result-table-scroll "
                + "simulation-result-table-scroll--expanded")
                : "simulation-result-table-scroll"
            }>
                <table className="simulation-result-table">
                    <thead>
                        <tr>
                            <th className="simulation-result-row-index">#</th>
                            {visibleColumns.map(column => {
                                const unit = resultDisplayUnit(column.unit, displayUnit);
                                const identityColumn = isResultIdentityColumn(column.key);
                                const columnGroup = getResultColumnGroup(column.key);
                                return (
                                    <th key={column.key} title={column.title}
                                        className={[identityColumn
                                            ? "simulation-result-identity-column"
                                            : "",
                                        `simulation-result-column-group--${columnGroup}`]
                                            .filter(Boolean).join(" ")
                                        }
                                        aria-sort={sortState?.columnKey === column.key
                                            ? (sortState.direction === "ASC"
                                                ? "ascending"
                                                : "descending"
                                            ) : "none"
                                        }
                                    >
                                        <button
                                            type="button"
                                            className="simulation-result-sort-button"
                                            onClick={() => handleSort(column.key)}
                                        >
                                            <span>
                                                {column.label}
                                                {unit && (
                                                    <small>
                                                        {" "}
                                                        ({unit})
                                                    </small>
                                                )}
                                            </span>

                                            <span className="simulation-result-sort-icon"
                                                aria-hidden="true">

                                                {sortState?.columnKey !== column.key
                                                    ? "↕"
                                                    : sortState.direction === "ASC"
                                                        ? "↑"
                                                        : "↓"
                                                }
                                            </span>
                                        </button>
                                    </th>
                                );
                            })}
                        </tr>
                    </thead>

                    <tbody>
                        {displayedRows.map((item, rowIndex) => {
                            const row = item.row;
                            const rowKey = String(row.terminalId
                                ?? row.edgeId ?? row.sourceId
                                ?? row.transitNodeId
                                ?? item.originalIndex);

                            const detailContent = showDetails
                                && renderRowDetails
                                ? renderRowDetails(row)
                                : null;
                            return (
                                <Fragment key={`${table.key}-${rowKey}`}>
                                    <tr className={[
                                        "simulation-result-data-row",
                                        item.originalIndex % 2 === 0
                                            ? "simulation-result-data-row--even"
                                            : "simulation-result-data-row--odd",
                                        detailContent ? "simulation-result-data-row--expanded" : "",
                                    ].filter(Boolean).join(" ")}>
                                        <td className="simulation-result-row-index">
                                            {rowIndex + 1}
                                        </td>
                                        {visibleColumns.map(
                                            column => {
                                                const identityColumn = isResultIdentityColumn(column.key);
                                                const columnGroup = getResultColumnGroup(column.key);
                                                return (
                                                    <td
                                                        key={column.key}
                                                        className={[identityColumn
                                                            ? "simulation-result-identity-column"
                                                            : "",
                                                        `simulation-result-column-group--${columnGroup}`]
                                                            .filter(Boolean).join(" ")}>
                                                        {renderResultCell(row, column)}
                                                    </td>
                                                );
                                            })}
                                    </tr>
                                    {detailContent && (
                                        <tr className="simulation-result-detail-row">
                                            <td colSpan={visibleColumns.length + 1} >
                                                {detailContent}
                                            </td>
                                        </tr>
                                    )}
                                </Fragment>
                            );
                        })}
                    </tbody>
                </table>
            </div>
        </section >
    );
}