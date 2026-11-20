import type { ComparisonDataset, ComparisonTable } from "./types";

export type SharedComparisonTable = {
    key: string;
    label: string;
    description: string;
};

/**
 * Removes the dataset prefix from comparison keys.
 * Example: 
 * simulation: terminalPressure
 * becomes: terminalPressure
 */
export function semanticTableKey(table: ComparisonTable): string {
    const separator = table.key.indexOf(":");

    if (separator < 0) {
        return table.key;
    }
    return table.key.substring(separator + 1);
}

/**
 * Returns only table types available on both sides.
 * One shared table selector therefore always controls two compatible table views.
 */
export function getSharedComparisonTables(
    left: ComparisonDataset | null,
    right: ComparisonDataset | null
): SharedComparisonTable[] {
    if (!left || !right) {
        return [];
    }

    const rightKeys = new Map(
        right.tables.map(
            table => [semanticTableKey(table), table]
        )
    );

    return left.tables
        .map(leftTable => {
            const key = semanticTableKey(leftTable);
            const rightTable = rightKeys.get(key);
            if (!rightTable) {
                return null;
            }
            return {
                key,
                label: leftTable.label,
                description: leftTable.description || rightTable.description,
            };
        })
        .filter((item): item is SharedComparisonTable => item !== null);
}

// Resolves one dataset table from its shared semantic key.
export function findSharedComparisonTable(dataset: ComparisonDataset | null, sharedKey: string): ComparisonTable | null {
    if (!dataset || !sharedKey) {
        return null;
    }
    return (
        dataset.tables.find(table => semanticTableKey(table) === sharedKey) ?? null
    );
}