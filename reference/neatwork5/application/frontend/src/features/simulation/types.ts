import type { NetworkGraph } from "../network/types";


export type SimulationRunOptions = {
    scenarioCount: number;
    seed?: number;

    orificeMode?: SimulationOrificeMode;
    boundedFlow?: boolean;
    serviceQuality?: number;
    requiredPressureM?: number;
    orificeCoefficient?: number;
    faucetCoefficient?: number;
    waterTemperatureC?: number;
    commercialPipeLengthM?: number;

    maxGenerationBatches?: number;
    oversamplingFactor?: number;
    maxDrawsPerBatch?: number;
};


export type SimulationSummary = {
    status: string;

    scenarioCount: number;
    successfulScenarios: number;
    // Execution/preparation/solver failures only.
    // Terminal water shortfalls are reported per terminal.
    failedScenarios: number;
    successRate: number;

    seed: number;

    durationSeconds: number;

    minIterations: number;
    meanIterations: number;
    maxIterations: number;

    openEdges: number;
    closedEdges: number;
    lowPressureNodes: number;
};


export type SimulationFailure = {
    scenarioIndex: number;
    stage: string;
    code: string;
    message: string;
    details?: Record<string, unknown>;
};


export type TerminalSatisfactionStatistics = {
    terminalId: string;

    // New simulations may return null when a terminal had no successfully evaluated active scenario.
    minSat: number | null;
    moySat: number | null;
    maxSat: number | null;

    cVarSat5: number | null;
    cVarSat20: number | null;

    /*
     * Added in statistics schema 1.1.
     *
     * Optional keeps historical persisted simulations
     * from schema 1.0 readable.
     */
    stdSat?: number | null;
    cvSat?: number | null;
    activeScenarioCount?: number;
    evaluatedActiveScenarioCount?: number;
    waterShortfallCount?: number;
};


export type SourceFlowStatistics = {
    sourceId: string;

    minSt: number;
    moySt: number;
    maxSt: number;

    cVarSt5: number;
    cVarSt95: number;
};


export type PipeVelocityStatistics = {
    edgeId: string;

    startNodeId: string;
    endNodeId: string;

    minVelocityMps: number;
    meanVelocityMps: number;
    maxVelocityMps: number;
};


export type TerminalPressureStatistics = {
    terminalId: string;

    minPressureM: number;
    meanPressureM: number;
    maxPressureM: number;
};


export type TransitPressureStatistics = {
    transitNodeId: string;

    minPressureM: number;
    meanPressureM: number;
    maxPressureM: number;
};

export type TerminalDemandBehaviorGroup = {
    groupId: string;

    /*
     * Normally one canonical behavior is used.
     * More than one ID can appear when multiple behavior definitions are hydraulically equivalent.
     */
    behaviorIds: string[];
    behaviorNames: string[];
    clientCount: number;
    // Probability per client.
    openProbability: number;
    // Target flow of one client, not aggregate terminal flow.
    targetFlowLps: number;
    profileId: string | null;
};

export type TerminalDemandProfile = {
    terminalId: string;
    // Total number of consumers physically/logically attached to this terminal.
    clientCount: number;
    // Consumers grouped by identical effective demand behavior.
    groups: TerminalDemandBehaviorGroup[];
};

export type SimulationStatisticsPayload = {
    schemaVersion: string;

    network: {
        name: string;
        version: string;
    };

    /*
     * Added in statistics schema 1.2.
     * Optional keeps persisted schema 1.0/1.1 simulations readable.
     */
    demand?: {
        terminals: TerminalDemandProfile[];
    };

    units: {
        sourceFlow: string;
        pipeVelocity: string;
        terminalPressure: string;
        transitPressure: string;
        duration: string;
        satisfaction: string;
    };

    summary: {
        scenarioCount: number;
        successfulScenarios: number;
        // Execution/preparation/solver failures only.
        // Terminal water shortfalls are reported per terminal.
        failedScenarios: number;

        successRate: number;

        seed: number;

        durationSeconds: number;

        minIterations: number;
        meanIterations: number;
        maxIterations: number;

        failureCountByCode:
        Record<string, number>;
    };

    scenarioSelection: {
        successfulMask: boolean[];
        includedScenarioIndices: number[];
        excludedScenarioIndices: number[];
    };

    statistics: {
        terminalSatisfaction: TerminalSatisfactionStatistics[];
        sourceFlow: SourceFlowStatistics[];
        pipeVelocity: PipeVelocityStatistics[];
        terminalPressure: TerminalPressureStatistics[];
        transitPressure: TransitPressureStatistics[];
    };

    failures: SimulationFailure[];
};


export type SimulatedGraphResponse = {
    graph: NetworkGraph;
    summary: SimulationSummary;
    statistics: SimulationStatisticsPayload;
    executionParameters: SimulationExecutionParameters;
};

export type SimulationOrificeMode = "COMMERCIAL" | "IDEAL";

export type SimulationJobStatus =
    | "PENDING"
    | "RUNNING"
    | "CANCEL_REQUESTED"
    | "CANCELLED"
    | "COMPLETED"
    | "COMPLETED_WITH_ERRORS"
    | "FAILED";


export type SimulationJob = {
    jobId: string;
    projectId: string;
    status: SimulationJobStatus;
    cancelRequested: boolean;
    createdAt: string;
    completedAt: string | null;
    result: SimulatedGraphResponse | null;
    errorMessage: string | null;
};

/**
 * Lightweight metadata describing one persisted simulation.
 */
export type SimulationRunSummary = {

    id: string;
    runNumber: number;
    projectId: string;
    domainSnapshotId: string;
    seed: number;
    scenarioCount: number;
    status: string;
    createdAt: string;
};

/**
 * Complete historical simulation result used by the comparison interface.
 */
export type SimulationRunDetails = {
    id: string;
    runNumber: number;
    projectId: string;
    seed: number | null;
    scenarioCount: number | null;
    status: string;
    resultStatus: string | null;
    startedAt: string | null;
    createdAt: string;
    completedAt: string | null;

    configurationMode: SimulationConfigurationMode;
    input: SimulationInputDescriptor;
    requestedOptions: Record<string, unknown>;
    resolvedOptions: Record<string, unknown> | null;

    parameterSources: Record<string, SimulationParameterSource> | null;
    edgeOverrides: Array<{ edgeId: string; enabled: boolean; }>;
    statistics: SimulationStatisticsPayload | null;
    errorMessage: string | null;
};

// Runtime overrides submitted for one simulation.
// An empty object means: "Use the values defined by the active domain."
export type SimulationRunRequestOptions = Partial<SimulationRunOptions>;

// Persistent lifecycle row displayed in simulation history.
export type SimulationHistoryItem = {
    id: string;
    runNumber: number;
    projectId: string;
    domainSnapshotId: string;
    status: SimulationJobStatus;
    cancelRequested: boolean;
    configurationMode: SimulationConfigurationMode;
    inputName: string | null;
    inputFilename: string | null;
    inputVersion: number | null;
    inputSourceFormat: string | null;
    seed: number | null;
    scenarioCount: number | null;
    resultStatus: string | null;
    createdAt: string;
    startedAt: string | null;
    completedAt: string | null;
    errorMessage: string | null;
};

export type SimulationConfigurationMode = "DEFAULT" | "MANUAL";

export type SimulationParameterSource = "USER_OVERRIDE"
    | "PROJECT_DEFAULT"
    | "CLIENT_DEMAND_TABLE"
    | "GENERATED"
    | "NOT_APPLICABLE";

export type SimulationInputDescriptor = {
    networkName: string | null;
    artifactId: string | null;
    filename: string | null;
    version: number | null;
    sourceFormat: string | null;
    domainSnapshotId: string;
};

export type SimulationExecutionParameters = {
    values: Record<string, unknown>;
    sources: Record<string, SimulationParameterSource>;
};
