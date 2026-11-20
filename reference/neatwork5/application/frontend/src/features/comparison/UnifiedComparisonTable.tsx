import { useMemo, useEffect, useState } from "react";
import type { DisplayLengthUnit } from "../domain/unitSystem";
import type { ComparisonDataset, ComparisonTable, ComparisonColumn, DomainSnapshotCompatibility } from "./types";
import { alignComparisonRows } from "./comparisonRowAlignment";
import type { AlignedComparisonRow } from "./comparisonRowAlignment";
import { buildComparisonColumnLabel, formatComparisonValue } from "./comparisonFormatting";
import { ComparisonRunSummary } from "./ComparisonRunSummary";
import { buildComparisonEntityMatchPolicy } from "./comparisonMatchPolicy";
import { NodeIdBadge, NodeTypeBadge } from "../network/nodeSemantics";
import type { NodeSemanticType } from "../network/nodeSemantics";


type Props = {
    leftDataset: ComparisonDataset;
    leftTable: ComparisonTable;
    rightDataset: ComparisonDataset;
    rightTable: ComparisonTable;
    displayUnit: DisplayLengthUnit;
    compatibility: DomainSnapshotCompatibility | null;
};

type SharedContextColumn = {
    key: string;
    label: string;
    leftColumn: ComparisonColumn | null;
    rightColumn: ComparisonColumn | null;
};

type SortDirection =
    | "ASC"
    | "DESC";


type ComparisonSortTarget =
    | { kind: "MATCH"; }
    | { kind: "SHARED"; columnKey: string; }
    | { kind: "LEFT"; columnKey: string; }
    | { kind: "RIGHT"; columnKey: string; };

type ComparisonSortState = {
    target: ComparisonSortTarget;
    direction: SortDirection;
};

//Returns true when a column participates in hydraulic row identity and is therefore already represented by the shared Match column.
function isIdentityColumn(table: ComparisonTable, column: ComparisonColumn): boolean {
    return (column.identity || table.identityColumns.includes(column.key));
}

//Returns only columns that truly need independent Left and Right values.
//Identity and shared topology columns are excluded because they are rendered once.
function comparisonValueColumns(table: ComparisonTable): ComparisonColumn[] {
    return table.columns.filter(column => {
        return (!isIdentityColumn(table, column) && !column.sharedContext);
    }
    );
}

// Builds the union of contextual columns shared by both sides, for example From and To in Pipe Velocity.
function buildSharedContextColumns(leftTable: ComparisonTable, rightTable: ComparisonTable): SharedContextColumn[] {
    const keys = new Set<string>();
    for (const column of leftTable.columns) {
        if (column.sharedContext) {
            keys.add(column.key);
        }
    }
    for (const column of rightTable.columns) {
        if (column.sharedContext) {
            keys.add(column.key);
        }
    }

    return Array.from(keys).map(key => {
        const leftColumn = leftTable.columns.find(column => column.key === key) ?? null;
        const rightColumn = rightTable.columns.find(column => column.key === key) ?? null;
        return {
            key,
            label: leftColumn?.label ?? rightColumn?.label ?? key,
            leftColumn,
            rightColumn,
        };
    }
    );
}

function comparisonIdentityNodeType(table: ComparisonTable, row: AlignedComparisonRow): NodeSemanticType | null {
    if (table.identityNodeType) {
        return table.identityNodeType;
    }
    if (table.identityNodeTypeKey) {
        const value = row.left?.[table.identityNodeTypeKey] ?? row.right?.[table.identityNodeTypeKey]
        if (value === "SOURCE" || value === "JUNCTION" || value === "TERMINAL") {
            return value;
        }
    }
    return null;
}

type SharedContextValue = {
    text: string;
    mismatch: boolean;
};

function sharedContextNodeType(row: AlignedComparisonRow, column: SharedContextColumn): NodeSemanticType | null {
    const typeKey = column.leftColumn?.nodeTypeKey ?? column.rightColumn?.nodeTypeKey;
    if (!typeKey) {
        return null;
    }
    const value = row.left?.[typeKey] ?? row.right?.[typeKey];
    if (value === "SOURCE" || value === "JUNCTION" || value === "TERMINAL") {
        return value;
    }
    return null;
}

// Displays one shared contextual value.
// Normally both simulations contain the same topology.
// If legacy or inconsistent data disagree, both values are shown instead of silently hiding the mismatch.
function formatSharedContextValue(
    row: AlignedComparisonRow,
    sharedColumn: SharedContextColumn,
    leftDataset: ComparisonDataset,
    rightDataset: ComparisonDataset,
    displayUnit: DisplayLengthUnit
): SharedContextValue {
    const leftRaw = row.left && sharedColumn.leftColumn ? row.left[sharedColumn.leftColumn.key] : undefined;
    const rightRaw = row.right && sharedColumn.rightColumn ? row.right[sharedColumn.rightColumn.key] : undefined;

    const leftExists = leftRaw !== null && leftRaw !== undefined;
    const rightExists = rightRaw !== null && rightRaw !== undefined;
    if (!leftExists && !rightExists) {
        return { text: "—", mismatch: false, };
    }
    const leftText = leftExists && sharedColumn.leftColumn
        ? formatComparisonValue(leftDataset.sourceType,
            leftRaw, sharedColumn.leftColumn, displayUnit
        ) : null;

    const rightText = rightExists && sharedColumn.rightColumn
        ? formatComparisonValue(rightDataset.sourceType,
            rightRaw, sharedColumn.rightColumn, displayUnit
        ) : null;
    if (leftText === null) {
        return {
            text: rightText ?? "—",
            mismatch: false,
        };
    }
    if (rightText === null) {
        return {
            text: leftText,
            mismatch: false,
        };
    }
    // Same-snapshot simulations normally reach this branch because their topology is identical.
    if (String(leftRaw) === String(rightRaw)) {
        return {
            text: leftText,
            mismatch: false,
        };
    }

    // Do not hide unexpected topology differences.
    return {
        text: `${leftText} ↔ ${rightText}`,
        mismatch: true,
    };
}

function matchStatusLabel(status: AlignedComparisonRow["status"]): string {
    switch (status) {
        case "MATCHED":
            return "Matched";
        case "LEFT_ONLY":
            return "Left only";
        case "RIGHT_ONLY":
            return "Right only";
    }
}


function sortTargetKey(target: ComparisonSortTarget): string {
    if (target.kind === "MATCH") {
        return "MATCH";
    }
    return `${target.kind}:${target.columnKey}`;
}

function compareRawValues(left: unknown, right: unknown): number {
    const leftMissing = left === null || left === undefined;
    const rightMissing = right === null || right === undefined;
    if (leftMissing && rightMissing) {
        return 0;
    }

    if (leftMissing) {
        return 1;
    }

    if (rightMissing) {
        return -1;
    }

    if (typeof left === "number" && typeof right === "number") {
        return left - right;
    }

    return naturalCollator.compare(String(left), String(right));
}

function matchStatusRank(status: AlignedComparisonRow["status"]): number {
    switch (status) {
        case "MATCHED":
            return 0;
        case "LEFT_ONLY":
            return 1;
        case "RIGHT_ONLY":
            return 2;
    }
}

function rawSortValue(row: AlignedComparisonRow, target: Exclude<ComparisonSortTarget, { kind: "MATCH" }>): unknown {
    switch (target.kind) {
        case "LEFT":
            return (row.left ? row.left[target.columnKey] : null);
        case "RIGHT":
            return (row.right ? row.right[target.columnKey] : null);
        case "SHARED":
            return (row.left?.[target.columnKey] ?? row.right?.[target.columnKey] ?? null);
    }
}

function compareAlignedRows(
    left: AlignedComparisonRow,
    right: AlignedComparisonRow,
    target: ComparisonSortTarget
): number {
    if (target.kind !== "MATCH") {
        return 0;
    }
    const statusDifference = matchStatusRank(left.status) - matchStatusRank(right.status);
    if (statusDifference !== 0) {
        return statusDifference;
    }
    return naturalCollator.compare(left.identity, right.identity);
}

function sortIndicator(target: ComparisonSortTarget, state: ComparisonSortState | null): string {
    if (!state || sortTargetKey(state.target) !== sortTargetKey(target)) {
        return "↕";
    }
    return state.direction === "ASC" ? "↑" : "↓";
}

function compareSortValues(left: unknown, right: unknown, direction: SortDirection): number {
    const leftMissing = left === null || left === undefined;
    const rightMissing = right === null || right === undefined;

    if (leftMissing || rightMissing) {
        if (leftMissing && rightMissing) {
            return 0;
        }
        return leftMissing ? 1 : -1;
    }
    const compared = compareRawValues(left, right);
    return direction === "ASC" ? compared : -compared;
}

const naturalCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

// Renders both comparison sources inside one physical table and one scroll container.
// Vertical and horizontal scrolling are therefore always synchronized by construction.
export function UnifiedComparisonTable({ leftDataset, leftTable, rightDataset, rightTable, displayUnit, compatibility }: Props) {
    const matchPolicy = useMemo(() => buildComparisonEntityMatchPolicy(leftTable, compatibility), [leftTable, compatibility]);
    const rows = useMemo(() => alignComparisonRows(leftTable, rightTable, matchPolicy), [leftTable, rightTable, matchPolicy]);
    const leftValueColumns = useMemo(() => comparisonValueColumns(leftTable), [leftTable]);
    const rightValueColumns = useMemo(() => comparisonValueColumns(rightTable), [rightTable]);
    const sharedContextColumns = useMemo(() => buildSharedContextColumns(leftTable, rightTable), [leftTable, rightTable]);
    const [sortState, setSortState] = useState<ComparisonSortState | null>(null);
    const displayedRows = useMemo(() => {
        if (!sortState) {
            return rows;
        }
        return [...rows].sort((left, right) => {
            if (sortState.target.kind === "MATCH") {
                const compared = compareAlignedRows(left, right, sortState.target);
                return (sortState.direction === "ASC" ? compared : -compared);
            }
            return compareSortValues(rawSortValue(left, sortState.target),
                rawSortValue(right, sortState.target), sortState.direction);
        });
    }, [rows, sortState]);

    const alignmentSummary = useMemo(() => {
        let matched = 0;
        let leftOnly = 0;
        let rightOnly = 0;
        for (const row of rows) {
            switch (row.status) {
                case "MATCHED":
                    matched += 1;
                    break;
                case "LEFT_ONLY":
                    leftOnly += 1;
                    break;
                case "RIGHT_ONLY":
                    rightOnly += 1;
                    break;
            }
        }
        return {
            matched,
            leftOnly,
            rightOnly,
        };
    }, [rows]);

    function matchStatusClassName(status: AlignedComparisonRow["status"]): string {
        return status.toLowerCase().replace(/_/g, "-");
    }

    function handleSort(target: ComparisonSortTarget) {
        setSortState(current => {
            const nextKey = sortTargetKey(target);
            if (!current || sortTargetKey(current.target) !== nextKey) {
                return { target, direction: "ASC" };
            }
            if (current.direction === "ASC") {
                return { target, direction: "DESC" };
            }
            return null;
        });
    }

    useEffect(() => {
        setSortState(null);
    }, [leftDataset.id, rightDataset.id, leftTable.key, rightTable.key]);

    return (
        <section className="comparison-unified-table-shell">
            <div className="comparison-run-summary-grid">
                <ComparisonRunSummary dataset={leftDataset} side="LEFT" />
                <ComparisonRunSummary dataset={rightDataset} side="RIGHT" />
            </div>

            <div className="comparison-alignment-summary">
                <span>
                    Matched{" "}
                    <strong>{alignmentSummary.matched}</strong>
                </span>
                <span>
                    Left only{" "}
                    <strong>{alignmentSummary.leftOnly}</strong>
                </span>
                <span>
                    Right only{" "}
                    <strong>{alignmentSummary.rightOnly}</strong>
                </span>
            </div>
            {rows.length === 0 ? (
                <div className="comparison-table-empty">No rows are available for this table.</div>
            ) : (
                // This is deliberately the only scroll container for both comparison sides.
                <div className="comparison-unified-scroll">
                    <table className="comparison-unified-table">
                        <thead>
                            <tr className="comparison-unified-source-header">
                                <th rowSpan={2} className="comparison-unified-identity-header" aria-sort={
                                    sortState && sortTargetKey(sortState.target) === "MATCH"
                                        ? (sortState.direction === "ASC" ? "ascending" : "descending")
                                        : "none"
                                }>
                                    <button type="button" className="comparison-sort-button"
                                        onClick={() => handleSort({ kind: "MATCH" })}
                                    >
                                        <span>Match</span>
                                        <span aria-hidden="true">{sortIndicator({ kind: "MATCH" }, sortState)}</span>
                                    </button>
                                </th>
                                {sharedContextColumns.map(column => {
                                    const target: ComparisonSortTarget = {
                                        kind: "SHARED",
                                        columnKey: column.key,
                                    };
                                    return (
                                        <th key={`shared:${column.key}`} rowSpan={2} className="comparison-unified-shared-header">
                                            <button type="button" className="comparison-sort-button" onClick={() => handleSort(target)}>
                                                <span>{column.label}</span>
                                                <span aria-hidden="true">{sortIndicator(target, sortState)}</span>
                                            </button>
                                        </th>
                                    );
                                })}
                                {leftValueColumns.length > 0 && (
                                    <th colSpan={leftValueColumns.length}
                                        className="comparison-unified-side-header comparison-unified-side-header--left">
                                        LEFT
                                    </th>
                                )}
                                {rightValueColumns.length > 0 && (
                                    <th colSpan={rightValueColumns.length}
                                        className="comparison-unified-side-header comparison-unified-side-header--right comparison-right-divider">
                                        RIGHT
                                    </th>
                                )}
                            </tr>
                            <tr className="comparison-unified-column-header">
                                {leftValueColumns.map(
                                    column => {
                                        const target: ComparisonSortTarget = {
                                            kind: "LEFT",
                                            columnKey: column.key,
                                        };
                                        return (
                                            <th key={`left:${column.key}`}
                                                aria-sort={sortState && sortTargetKey(sortState.target) === sortTargetKey(target)
                                                    ? (sortState.direction === "ASC" ? "ascending" : "descending") : "none"
                                                }
                                            >
                                                <button
                                                    type="button"
                                                    className="comparison-sort-button"
                                                    onClick={() => handleSort(target)}>
                                                    <span>
                                                        {buildComparisonColumnLabel(leftDataset.sourceType, column, displayUnit)}
                                                    </span>
                                                    <span aria-hidden="true">{sortIndicator(target, sortState)}</span>
                                                </button>

                                            </th>
                                        );
                                    }
                                )}
                                {rightValueColumns.map((column, index) => {
                                    const target: ComparisonSortTarget = {
                                        kind: "RIGHT",
                                        columnKey: column.key,
                                    };
                                    return (
                                        <th key={`right:${column.key}`}
                                            className={index === 0 ? "comparison-right-divider" : undefined}
                                            aria-sort={sortState && sortTargetKey(sortState.target) ===
                                                sortTargetKey(target) ? (sortState.direction ===
                                                    "ASC" ? "ascending" : "descending") : "none"}>
                                            <button type="button" className="comparison-sort-button" onClick={() => handleSort(target)}>
                                                <span>
                                                    {buildComparisonColumnLabel(rightDataset.sourceType, column, displayUnit)}
                                                </span>
                                                <span aria-hidden="true">
                                                    {sortIndicator(target, sortState)}
                                                </span>
                                            </button>

                                        </th>
                                    );
                                })}
                            </tr>

                        </thead>
                        <tbody>
                            {displayedRows.map(row => (
                                <tr key={row.key}
                                    className={row.status === "LEFT_ONLY"
                                        ? "comparison-unified-row--left-only"
                                        : row.status === "RIGHT_ONLY"
                                            ? "comparison-unified-row--right-only"
                                            : undefined
                                    }>
                                    <td className="comparison-unified-match-cell">
                                        <NodeIdBadge nodeId={row.identity} type={comparisonIdentityNodeType(leftTable, row)} />
                                        <span className={`comparison-match-status comparison-match-status--${matchStatusClassName(row.status)}`}>
                                            {matchStatusLabel(row.status)}
                                        </span>
                                    </td>
                                    {sharedContextColumns.map(column => {
                                        const nodePresentation = column.leftColumn?.nodePresentation ?? column.rightColumn?.nodePresentation;
                                        const semanticType = sharedContextNodeType(row, column);
                                        const value = formatSharedContextValue(row, column, leftDataset, rightDataset, displayUnit);

                                        return (
                                            <td key={`shared:${column.key}`} className={value.mismatch
                                                ? "comparison-shared-context-mismatch"
                                                : undefined}>
                                                {nodePresentation === "TYPE" ? (<NodeTypeBadge type={semanticType} />)
                                                    : nodePresentation === "ID" && !value.mismatch ? (<NodeIdBadge
                                                        nodeId={value.text}
                                                        type={semanticType} />
                                                    ) : value.text
                                                }
                                            </td>
                                        );
                                    })}

                                    {leftValueColumns.map(column => (

                                        <td key={`left:${column.key}`} className={row.left ? "" : "missing"}>
                                            {row.left ? formatComparisonValue(
                                                leftDataset.sourceType,
                                                row.left[column.key],
                                                column,
                                                displayUnit
                                            ) : "—"}
                                        </td>
                                    ))}

                                    {rightValueColumns.map((column, index) => (
                                        <td key={`right:${column.key}`}
                                            className={[index === 0 ? "comparison-right-divider" : "",
                                            row.right ? "" : "missing"].filter(Boolean).join(" ")}
                                        >
                                            {row.right ? formatComparisonValue(
                                                rightDataset.sourceType,
                                                row.right[column.key],
                                                column,
                                                displayUnit
                                            ) : "—"}
                                        </td>
                                    ))}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </section>
    );
}
