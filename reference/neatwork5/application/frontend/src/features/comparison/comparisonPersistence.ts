import type { DisplayLengthUnit } from "../domain/unitSystem";
import type { ComparisonWorkspaceState } from "./types";


const STORAGE_PREFIX = "neatwork-comparison:";


// Return the project-specific local-storage key.
function storageKey(projectId: string, domainSnapshotId: string): string {

    return STORAGE_PREFIX + projectId + ":" + domainSnapshotId;
}


// Restore the latest comparison workspace. Legacy two-pane settings are migrated in memory to the unified table/unit model.
export function loadComparisonWorkspace(projectId: string, domainSnapshotId: string): ComparisonWorkspaceState | null {
    try {

        const raw = window.localStorage.getItem(storageKey(projectId, domainSnapshotId));
        if (!raw) {
            return null;
        }
        const parsed = JSON.parse(raw) as ComparisonWorkspaceState | LegacyComparisonWorkspaceState;

        // New unified workspace format.
        if ("leftSourceId" in parsed && "rightSourceId" in parsed) {
            return parsed as ComparisonWorkspaceState;
        }
        // Convert the old independent-pane format.
        // The right-side table is preferred when the previous panes used different table selections.
        const legacy = parsed as LegacyComparisonWorkspaceState;
        const leftSourceId = legacy.left?.sourceId ?? "";
        const rightSourceId = legacy.right?.sourceId ?? "";
        const tableKey = legacy.right?.tableKey ?? legacy.left?.tableKey ?? "";

        return {
            leftSourceId,
            rightSourceId,
            tableKey,
        };
    } catch {
        return null;
    }
}


/**
 * Persist comparison selections for one project.
 */
export function saveComparisonWorkspace(
    projectId: string,
    domainSnapshotId: string,
    state: ComparisonWorkspaceState,
): void {
    window.localStorage.setItem(storageKey(projectId, domainSnapshotId), JSON.stringify(state));

}

/**
 * Remove comparison preferences when a project is deleted.
 */
export function clearComparisonWorkspace(projectId: string): void {

    const prefix = `${STORAGE_PREFIX}${projectId}:`;
    const keysToRemove: string[] = [];

    for (let index = 0; index < window.localStorage.length; index += 1) {
        const key = window.localStorage.key(index);
        if (key && key.startsWith(prefix)) {
            keysToRemove.push(key);
        }
    }
    keysToRemove.forEach(key => window.localStorage.removeItem(key));
}

type LegacyComparisonWorkspaceState = {
    left?: {
        sourceId?: string;
        tableKey?: string;
        displayUnit?: DisplayLengthUnit;
    };
    right?: {
        sourceId?: string;
        tableKey?: string;
        displayUnit?: DisplayLengthUnit;
    };
};