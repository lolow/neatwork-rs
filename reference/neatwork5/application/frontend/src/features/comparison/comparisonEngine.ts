import { convertTableMeasurement, getPhysicalColumnUnit } from "../domain/unitSystem";
import type {
    ComparisonColumn,
    ComparisonDataset,
    ComparisonMatchStrategy,
    ComparisonMetricResult,
    ComparisonMetricRow,
    ComparisonRow,
    ComparisonTable,
} from "./types";

type IdentityAlias = {

    label:
    string;

    leftKeys:
    string[];

    rightKeys:
    string[];
};


/**
 * Known equivalences between imported-domain identifiers
 * and canonical simulation-output identifiers.
 */
const IDENTITY_ALIASES:
    IdentityAlias[] = [

        {
            label:
                "Terminal ID",

            leftKeys: [
                "borne"
            ],

            rightKeys: [
                "terminalId"
            ],
        },

        {
            label:
                "Terminal ID",

            leftKeys: [
                "outlet"
            ],

            rightKeys: [
                "terminalId"
            ],
        },

        {
            label:
                "Node ID",

            leftKeys: [
                "noeud"
            ],

            rightKeys: [
                "terminalId"
            ],
        },

        {
            label:
                "Node ID",

            leftKeys: [
                "noeud"
            ],

            rightKeys: [
                "transitNodeId"
            ],
        },

        {
            label:
                "Source ID",

            leftKeys: [
                "noeud"
            ],

            rightKeys: [
                "sourceId"
            ],
        },

        {
            label:
                "Pipe endpoints",

            leftKeys: [
                "pred",
                "succ",
            ],

            rightKeys: [
                "startNodeId",
                "endNodeId",
            ],
        },

        {
            label:
                "Pipe endpoints",

            leftKeys: [
                "sourceId",
                "targetId",
            ],

            rightKeys: [
                "startNodeId",
                "endNodeId",
            ],
        },
    ];

function reverseAlias(
    alias:
        IdentityAlias,
): IdentityAlias {

    return {

        label:
            alias.label,

        leftKeys:
            alias.rightKeys,

        rightKeys:
            alias.leftKeys,
    };
}

function tableHasColumns(
    table:
        ComparisonTable,

    columns:
        string[],
): boolean {

    const available =
        new Set(
            table.columns.map(
                column =>
                    column.key
            )
        );


    return columns.every(
        column =>
            available.has(
                column
            )
    );
}

/**
 * Return all safe row-alignment strategies available for two tables.
 */
export function findMatchStrategies(
    left:
        ComparisonTable,

    right:
        ComparisonTable,
): ComparisonMatchStrategy[] {

    const strategies:
        ComparisonMatchStrategy[] = [];


    const seen =
        new Set<string>();


    function addStrategy(
        label:
            string,

        leftKeys:
            string[],

        rightKeys:
            string[],
    ) {

        if (
            !tableHasColumns(
                left,
                leftKeys
            )
            ||
            !tableHasColumns(
                right,
                rightKeys
            )
        ) {
            return;
        }


        const id =
            `${leftKeys.join("+")}`
            +
            `::`
            +
            `${rightKeys.join("+")}`;


        if (
            seen.has(
                id
            )
        ) {
            return;
        }


        seen.add(
            id
        );


        strategies.push({
            id,
            label,
            leftKeys,
            rightKeys,
        });
    }

    /*
 * Highest-confidence case:
 * both tables expose the same identity key.
 */
    for (
        const leftKey
        of left.identityColumns
    ) {

        if (
            right.identityColumns.includes(
                leftKey
            )
        ) {

            addStrategy(
                leftKey,

                [
                    leftKey
                ],

                [
                    leftKey
                ]
            );
        }
    }

    const aliases = [

        ...IDENTITY_ALIASES,

        ...IDENTITY_ALIASES.map(
            reverseAlias
        ),
    ];


    for (
        const alias
        of aliases
    ) {

        addStrategy(
            alias.label,
            alias.leftKeys,
            alias.rightKeys
        );
    }


    return strategies;
}

/**
 * Return numeric columns suitable for user-selected comparison.
 */
export function getComparableColumns(
    table:
        ComparisonTable,
): ComparisonColumn[] {

    return table.columns.filter(
        column =>
            column.valueKind
            === "number"

            &&

            column.comparable
    );
}

export function areMetricColumnsCompatible(
    left:
        ComparisonColumn,

    right:
        ComparisonColumn,
): boolean {

    /*
     * Explicit known units must agree.
     */
    if (
        left.unit !== null
        ||
        right.unit !== null
    ) {

        return (
            left.unit
            === right.unit
        );
    }


    /*
     * Unit-less values are only assumed comparable
     * automatically when they represent the same key.
     */
    return (
        left.key
        === right.key
    );
}

function readNumericValue(
    dataset:
        ComparisonDataset,

    row:
        ComparisonRow,

    column:
        ComparisonColumn,
): number | null {

    const rawValue =
        row[
        column.key
        ];


    const numericValue =
        typeof rawValue
            === "number"

            ? rawValue

            : typeof rawValue
                === "string"

                ? Number(
                    rawValue
                )

                : Number.NaN;


    if (
        !Number.isFinite(
            numericValue
        )
    ) {

        return null;
    }


    /*
     * Imported-domain measurements may be stored in metres or
     * explicit millimetres. Normalize them to metres before
     * any calculation.
     */
    if (
        dataset.sourceType
        === "DEFAULT"

        &&

        getPhysicalColumnUnit(
            column.key
        )
    ) {

        const converted =
            convertTableMeasurement(
                column.key,
                numericValue,
                "m"
            );


        return (
            converted
            ?? numericValue
        );
    }


    return numericValue;
}

function buildIdentityKey(
    row:
        ComparisonRow,

    keys:
        string[],
): string | null {

    const parts =
        keys.map(
            key =>
                row[key]
        );


    if (
        parts.some(
            part =>
                part === null
                ||
                part === undefined
                ||
                String(part).trim()
                === ""
        )
    ) {

        return null;
    }


    return parts
        .map(
            part =>
                String(part)
                    .trim()
        )
        .join(
            " → "
        );
}

type UniqueRowMap = {

    rows:
    Map<
        string,
        ComparisonRow
    >;

    duplicateCount:
    number;
};


function buildUniqueRowMap(
    rows:
        ComparisonRow[],

    keys:
        string[],
): UniqueRowMap {

    const candidates =
        new Map<
            string,
            ComparisonRow[]
        >();


    for (
        const row
        of rows
    ) {

        const identity =
            buildIdentityKey(
                row,
                keys
            );


        if (!identity) {
            continue;
        }


        const existing =
            candidates.get(
                identity
            );


        if (existing) {

            existing.push(
                row
            );

        } else {

            candidates.set(
                identity,
                [
                    row
                ]
            );
        }
    }


    const unique =
        new Map<
            string,
            ComparisonRow
        >();


    let duplicateCount =
        0;


    for (
        const [
            key,
            values,
        ]
        of candidates
    ) {

        if (
            values.length === 1
        ) {

            unique.set(
                key,
                values[0]
            );

        } else {

            duplicateCount +=
                values.length;
        }
    }


    return {
        rows:
            unique,

        duplicateCount,
    };
}

export function compareMetrics(
    leftDataset:
        ComparisonDataset,

    leftTable:
        ComparisonTable,

    leftColumn:
        ComparisonColumn,

    rightDataset:
        ComparisonDataset,

    rightTable:
        ComparisonTable,

    rightColumn:
        ComparisonColumn,

    strategy:
        ComparisonMatchStrategy,
): ComparisonMetricResult {

    const leftMap =
        buildUniqueRowMap(
            leftTable.rows,
            strategy.leftKeys
        );


    const rightMap =
        buildUniqueRowMap(
            rightTable.rows,
            strategy.rightKeys
        );


    const unitsCompatible =
        areMetricColumnsCompatible(
            leftColumn,
            rightColumn
        );


    const rows:
        ComparisonMetricRow[] =
        [];


    let unmatchedLeftCount =
        0;


    const matchedRightKeys =
        new Set<string>();

    for (
        const [
            identity,
            leftRow,
        ]
        of leftMap.rows
    ) {

        const rightRow =
            rightMap.rows.get(
                identity
            );


        if (!rightRow) {

            unmatchedLeftCount += 1;

            continue;
        }


        const leftValue =
            readNumericValue(
                leftDataset,
                leftRow,
                leftColumn
            );


        const rightValue =
            readNumericValue(
                rightDataset,
                rightRow,
                rightColumn
            );


        if (
            leftValue === null
            ||
            rightValue === null
        ) {

            unmatchedLeftCount += 1;

            continue;
        }


        matchedRightKeys.add(
            identity
        );


        const difference =
            rightValue
            -
            leftValue;


        const ratio =
            leftValue === 0

                ? null

                : rightValue
                / leftValue;


        const percentageChange =
            leftValue === 0

                ? null

                : (
                    difference
                    /
                    leftValue
                )
                * 100;


        rows.push({

            identity,

            leftValue,

            rightValue,

            difference,

            ratio,

            percentageChange,
        });
    }

    let unmatchedRightCount =
        0;


    for (
        const key
        of rightMap.rows.keys()
    ) {

        if (
            !matchedRightKeys.has(
                key
            )
        ) {

            unmatchedRightCount += 1;
        }
    }


    return {

        rows,

        matchedRowCount:
            rows.length,

        unmatchedLeftCount,

        unmatchedRightCount,

        duplicateLeftCount:
            leftMap.duplicateCount,

        duplicateRightCount:
            rightMap.duplicateCount,

        leftUnit:
            leftColumn.unit,

        rightUnit:
            rightColumn.unit,

        unitsCompatible,
    };
}

/**
 * Return a safe automatic pair when both tables expose
 * the same comparable numerical statistic.
 */
export function findRecommendedMetricPair(
    left:
        ComparisonTable,

    right:
        ComparisonTable,
): {
    leftColumnKey:
    string;

    rightColumnKey:
    string;
} | null {

    const leftColumns =
        getComparableColumns(
            left
        );


    const rightColumns =
        getComparableColumns(
            right
        );


    /*
     * First choice:
     * identical metric key.
     */
    for (
        const leftColumn
        of leftColumns
    ) {

        const rightColumn =
            rightColumns.find(
                candidate =>
                    candidate.key
                    === leftColumn.key

                    &&

                    areMetricColumnsCompatible(
                        leftColumn,
                        candidate
                    )
            );


        if (rightColumn) {

            return {
                leftColumnKey:
                    leftColumn.key,

                rightColumnKey:
                    rightColumn.key,
            };
        }
    }

    for (
        const alias
        of METRIC_ALIASES
    ) {

        const leftColumn =
            leftColumns.find(
                column =>
                    column.key
                    === alias.left
            );


        const rightColumn =
            rightColumns.find(
                column =>
                    column.key
                    === alias.right
            );


        if (
            leftColumn
            &&
            rightColumn
            &&
            areMetricColumnsCompatible(
                leftColumn,
                rightColumn
            )
        ) {

            return {
                leftColumnKey:
                    leftColumn.key,

                rightColumnKey:
                    rightColumn.key,
            };
        }


        /*
         * Also support reversed pane order.
         */
        const reversedLeft =
            leftColumns.find(
                column =>
                    column.key
                    === alias.right
            );


        const reversedRight =
            rightColumns.find(
                column =>
                    column.key
                    === alias.left
            );


        if (
            reversedLeft
            &&
            reversedRight
            &&
            areMetricColumnsCompatible(
                reversedLeft,
                reversedRight
            )
        ) {

            return {
                leftColumnKey:
                    reversedLeft.key,

                rightColumnKey:
                    reversedRight.key,
            };
        }
    }


    return null;
}

const METRIC_ALIASES: Array<{
    left:
    string;

    right:
    string;
}> = [

        {
            left:
                "pressure",

            right:
                "meanPressureM",
        },

        {
            left:
                "required_pressure_m",

            right:
                "meanPressureM",
        },

        {
            left:
                "cible",

            right:
                "moySt",
        },
    ];