import type { SimulationConfigurationMode, SimulationParameterSource } from "../simulation/types";
import type { NodeSemanticType } from "../network/nodeSemantics";


export type ComparisonSourceType = "DEFAULT" | "SIMULATION";

export type ComparisonValueKind = "text" | "number" | "boolean" | "unknown";

export type ComparisonUnit = "m" | "m/s" | "L/s" | "ratio" | null;

export type ComparisonSnapshotRelation = "SAME" | "DIFFERENT" | "UNKNOWN";

export type ComparisonColumn = {
    key: string;
    label: string;
    valueKind: ComparisonValueKind;
    unit: ComparisonUnit;
    /**
     * Identifies the hydraulic entity used to align left and right rows, for example terminalId, sourceId or edgeId.
     * Identity columns are displayed once through the shared Match column.
     */
    identity: boolean;

    /**
     * Describes topology or contextual information shared by both comparison sides. 
     * Shared-context columns are displayed once instead of being duplicated under Left/Right.
     * Example: Pipe Velocity -> From / To.
     */
    sharedContext?: boolean;

    // True when numeric comparison is meaningful
    comparable: boolean;
    nodePresentation?: "ID" | "TYPE";
    nodeType?: NodeSemanticType;
    nodeTypeKey?: string;
};

export type ComparisonRow = Record<string, unknown>;

export type ComparisonTable = {
    key: string;
    label: string;
    description: string;
    rows: ComparisonRow[];
    columns: ComparisonColumn[];

    // Preferred entity keys used to align left/right table rows.
    identityColumns: string[];
    identityNodeType?: NodeSemanticType;
    identityNodeTypeKey?: string;
};

export type ComparisonDataset = {
    id: string;
    sourceType: ComparisonSourceType;
    label: string;
    subtitle: string;
    projectId: string;
    domainSnapshotId: string;
    createdAt: string;
    runId?: string;
    runNumber?: number;
    seed?: number | null;
    scenarioCount?: number | null;
    status?: string;
    tables: ComparisonTable[];
    // Human-readable name of the imported network used by this source.
    inputLabel: string;
    // Simulation number relative to this exact input snapshot.
    // This is intentionally different from the project-wide historical runNumber stored by the backend.
    displayRunNumber?: number;

    // Exact network-file revision associated with a persisted simulation.
    inputFilename?: string;
    inputVersion?: number;
    configurationMode?: SimulationConfigurationMode;

    // Parameters explicitly requested by the user.
    requestedOptions?: Record<string, unknown>;

    // Effective parameters actually used by the solver.
    resolvedOptions?: Record<string, unknown> | null;

    // Provenance of each effective simulation parameter.
    parameterSources?: Record<string, SimulationParameterSource> | null;
};

export type ComparisonSourceDescriptor = {
    id: string;
    sourceType: ComparisonSourceType;
    label: string;
    subtitle: string;
    inputLabel: string;
    domainSnapshotId: string;
    runId?: string;
    displayRunNumber?: number;
};

/**
 * Load one normalized comparison dataset by its source identifier.
 */
export type ComparisonDatasetLoader = (sourceId: string) => Promise<ComparisonDataset>;

/**
 * Describe how rows from two different tables can be aligned.
 */
export type ComparisonMatchStrategy = {
    id: string;
    label: string;
    leftKeys: string[];
    rightKeys: string[];
};

export type ComparisonMetricRow = {
    identity: string;
    leftValue: number;
    rightValue: number;
    difference: number;
    ratio: number | null;
    percentageChange: number | null;
};

export type ComparisonMetricResult = {
    rows: ComparisonMetricRow[];
    matchedRowCount: number;
    unmatchedLeftCount: number;
    unmatchedRightCount: number;
    duplicateLeftCount: number;
    duplicateRightCount: number;
    leftUnit: ComparisonUnit;
    rightUnit: ComparisonUnit;
    unitsCompatible: boolean;
};

export type ComparisonMetricSelection = {
    matchStrategyId: string;
    leftColumnKey: string;
    rightColumnKey: string;
};

/**
 * Persisted state of the unified comparison workspace.
 * Sources remain independently selectable, while table are shared by both sides.
 */
export type ComparisonWorkspaceState = {
    leftSourceId: string;
    rightSourceId: string;
    tableKey: string;
};

/**
 * Describes the degree of structural compatibility between the two networks used by the comparison.
 * This is used to determine whether the comparison is meaningful or not.
 */
export type ComparisonCompatibilityLevel =
    | "SAME_SNAPSHOT"
    | "EQUIVALENT_INPUT"
    | "SAME_TOPOLOGY"
    | "TOPOLOGY_CHANGED"
    | "UNKNOWN";

/**
 * Counts of topological elements and lists of entity identifiers for each side of the comparison.
 * This is used to determine the degree of structural compatibility between the two networks.
 */
export type ComparisonChangeSummary = {
    leftNodeCount: number;
    rightNodeCount: number;
    matchedNodeCount: number;
    addedNodes: string[];
    removedNodes: string[];
    changedNodeRoles: string[];

    leftPipeCount: number;
    rightPipeCount: number;
    matchedPipeCount: number;
    addedPipes: string[];
    removedPipes: string[];
    changedPipeEndpoints: string[];

    leftTerminalCount: number;
    rightTerminalCount: number;
    matchedTerminalCount: number;
    addedTerminals: string[];
    removedTerminals: string[];
};

/**
 * Describes the structural compatibility between two domain snapshots.
 * This is used to determine whether the comparison is meaningful or not.
 */
export type DomainSnapshotCompatibility = {
    leftSnapshotId: string;
    rightSnapshotId: string;
    level: ComparisonCompatibilityLevel;
    sameTopology: boolean;
    sameDemandStructure: boolean;
    sameEffectiveInput: boolean;

    directMetricComparisonAllowed: boolean;
    partialMetricComparisonAllowed: boolean;

    changes: ComparisonChangeSummary;
    warnings: string[];
    terminals: ComparableEntitySummary;
    sources: ComparableEntitySummary;
    transitNodes: ComparableEntitySummary;
    pipes: ComparableEntitySummary;
};

export type ComparableEntitySummary = {
    matched: string[];
    leftOnly: string[];
    rightOnly: string[];
    contextChanged: string[];
};