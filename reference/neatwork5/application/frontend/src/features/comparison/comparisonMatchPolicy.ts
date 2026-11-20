import type { DomainSnapshotCompatibility, ComparisonTable } from "./types";
import type { ComparisonEntityMatchPolicy } from "./comparisonRowAlignment";

function normalizedTableKey(table: ComparisonTable): string {
    return table.key.trim().toLowerCase();
}

export function buildComparisonEntityMatchPolicy(
    table: ComparisonTable,
    compatibility: DomainSnapshotCompatibility | null
): ComparisonEntityMatchPolicy | undefined {
    if (!compatibility) {
        return undefined;
    }
    const key = normalizedTableKey(table);
    let summary: { matched: string[]; contextChanged: string[]; } | null = null;

    if (key.includes("pressure")) {
        return {
            allowedMatchedIds: new Set([
                ...compatibility.terminals.matched,
                ...compatibility.transitNodes.matched,
            ]),

            forceUnmatchedIds: new Set([
                ...compatibility.terminals.contextChanged,
                ...compatibility.transitNodes.contextChanged,
            ]),
        };
    }
    if (key.includes("terminalsatisfaction")) {
        summary = compatibility.terminals;
    }

    else if (key.includes("sourceflow")) {
        summary = compatibility.sources;
    }

    else if (key.includes("pipevelocity")) {
        summary = compatibility.pipes;
    }
    if (!summary) {
        return undefined;
    }

    return {
        allowedMatchedIds: new Set(summary.matched),
        forceUnmatchedIds: new Set(summary.contextChanged),
    };
}