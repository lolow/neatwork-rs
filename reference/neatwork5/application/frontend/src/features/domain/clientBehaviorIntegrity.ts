
export function findBehaviorReferences(behaviorId: string, clients: Record<string, unknown>[]): string[] {
    return clients
        .filter(
            row => String(row.behaviorId ?? row.behavior_id ?? "") === behaviorId
        )
        .map(
            row => String(row.client ?? row.clientId ?? "?")
        );
}

export function findDuplicateBehaviorIds(rows: Record<string, unknown>[]): string[] {
    const counts = new Map<string, number>();
    for (const row of rows) {
        const id = String(row.behaviorId ?? row.behavior_id ?? "").trim();
        if (!id) {
            continue;
        }
        counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    return [...counts.entries()].filter(([, count]) => count > 1)
        .map(([id]) => id);
}