import type { ComparisonRow, ComparisonTable } from "./types";

export type ComparisonRowMatchStatus = "MATCHED" | "LEFT_ONLY" | "RIGHT_ONLY";
export type AlignedComparisonRow = {
    key: string;
    identity: string;
    left: ComparisonRow | null;
    right: ComparisonRow | null;
    status: ComparisonRowMatchStatus;
};

// Finds an entity column available on both sides.
function findSharedIdentityColumns(left: ComparisonTable, right: ComparisonTable): string[] {
    return left.identityColumns
        .filter(column => right.identityColumns.includes(column));
}

// Creates one stable identity for a hydraulic row.
function rowIdentity(row: ComparisonRow, columns: string[], fallbackIndex: number): string {
    if (columns.length === 0) {
        return `row:${fallbackIndex}`;
    }

    const value =
        columns.map(column => String(row[column] ?? "")).join("::");

    return value || `row:${fallbackIndex}`;
}

// Groups rows by hydraulic entity identity.
// Arrays are used because malformed/legacy inputs may contain duplicate identifiers.
function groupRows(table: ComparisonTable, identityColumns: string[]): Map<string, ComparisonRow[]> {
    const groups = new Map<string, ComparisonRow[]>();
    table.rows.forEach(
        (row, index) => {
            const identity = rowIdentity(row, identityColumns, index);
            const bucket = groups.get(identity) ?? [];
            bucket.push(row);
            groups.set(identity, bucket);
        }
    );

    return groups;
}

// Aligns the complete left and right table into one row stream consumed by a single comparison grid.
export function alignComparisonRows(left: ComparisonTable, right: ComparisonTable, policy?: ComparisonEntityMatchPolicy): AlignedComparisonRow[] {

    const identityColumns = findSharedIdentityColumns(left, right);

    // Without one shared safe identity, rows must not be paired by position.
    if (identityColumns.length === 0) {
        return [...left.rows.map((row, index) => ({
            key: `left:${index}`,
            identity: `Left row ${index + 1}`,
            status: "LEFT_ONLY" as const,
            left: row,
            right: null,
        })),

        ...right.rows.map((row, index) => ({
            key: `right:${index}`,
            identity: `Right row ${index + 1}`,
            status: "RIGHT_ONLY" as const,
            left: null,
            right: row,
        })
        ),
        ];
    }

    const leftGroups = groupRows(left, identityColumns);
    const rightGroups = groupRows(right, identityColumns);
    const identities = Array.from(new Set([
        ...leftGroups.keys(),
        ...rightGroups.keys(),
    ]));

    const rows: AlignedComparisonRow[] = [];

    identities.forEach(identity => {
        const leftRows = leftGroups.get(identity) ?? [];
        const rightRows = rightGroups.get(identity) ?? [];
        const count = Math.max(leftRows.length, rightRows.length);
        const representativeRow = leftRows[0] ?? rightRows[0] ?? null;
        const policyId = representativeRow ? String(representativeRow[identityColumns[0]] ?? identity) : identity;
        const forcedUnmatched = policy?.forceUnmatchedIds?.has(policyId) ?? false;
        const explicitlyAllowed = !policy?.allowedMatchedIds || policy.allowedMatchedIds.has(policyId);

        if (forcedUnmatched || !explicitlyAllowed) {
            leftRows.forEach((row, index) => {
                rows.push({
                    key: `left:${identity}:${index}`,
                    identity,
                    status: "LEFT_ONLY",
                    left: row,
                    right: null,
                });
            }
            );


            rightRows.forEach((row, index) => {
                rows.push({
                    key: `right:${identity}:${index}`,
                    identity,
                    status: "RIGHT_ONLY",
                    left: null,
                    right: row,
                });
            }
            );

            return;
        }
        for (let index = 0; index < count; index += 1) {
            const leftRow = leftRows[index] ?? null;
            const rightRow = rightRows[index] ?? null;
            const status: ComparisonRowMatchStatus = leftRow && rightRow ? "MATCHED"
                : leftRow ? "LEFT_ONLY"
                    : "RIGHT_ONLY";
            rows.push({
                key: `${identity}::${index}`,
                identity,
                status,
                left: leftRow,
                right: rightRow,
            });
        }
    });

    return rows;
}

export type ComparisonEntityMatchPolicy = {
    allowedMatchedIds?: ReadonlySet<string>;
    forceUnmatchedIds?: ReadonlySet<string>;
};
