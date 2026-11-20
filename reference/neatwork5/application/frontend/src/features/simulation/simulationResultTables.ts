import type { SimulationStatisticsPayload, TerminalDemandProfile } from "./types";
import type { NodeSemanticType } from "../network/nodeSemantics";

export type SimulationResultValueFormat = "NUMBER" | "PERCENT" | "INTEGER";

export type SimulationResultColumn = {
    key: string;
    label: string;

    // Unit displayed next to the column label
    unit?: string | null;

    //Only quantitative result values receive two-decimal numeric formatting.
    numeric?: boolean;

    //Controls presentation only. Raw statistics always remain unchanged.
    format?: SimulationResultValueFormat;

    // The column is hidden in the compact result view and becomes visible only when terminal details are expanded.
    detailsOnly?: boolean;

    // Optional explanation shown as a native tooltip on the column header.
    title?: string;
    nodePresentation?: "ID" | "TYPE";
    nodeType?: NodeSemanticType;
    nodeTypeKey?: string;
};

export type SimulationResultTable = {
    key: string;
    title: string;
    description: string;
    columns: SimulationResultColumn[];
    rows: Record<string, unknown>[];
};

//Returns the unit provided by Python Core while keeping one canonical fallback for older results.
function unit(value: string | null | undefined, fallback: string): string {
    const trimmed = value?.trim();
    return trimmed ? trimmed : fallback;
}

//Returns a map of terminalId → profile for quick lookups.
function terminalDemandById(payload: SimulationStatisticsPayload): Map<string, TerminalDemandProfile> {
    const profiles = payload.demand?.terminals ?? [];
    return new Map(profiles.map(profile => [profile.terminalId, profile]));
}

//For the terminal-satisfaction table: renders a compact target-flow summary.
function targetFlowSummary(profile: TerminalDemandProfile | undefined): number | string | null {
    if (!profile || profile.groups.length === 0) {
        return null;
    }
    if (profile.groups.length === 1) {
        return profile.groups[0].targetFlowLps;
    }
    return `${profile.groups.length} groups`;
}

//For the terminal-satisfaction table: renders a compact probability summary.
function probabilitySummary(profile: TerminalDemandProfile | undefined): number | string | null {
    if (!profile || profile.groups.length === 0) {
        return null;
    }
    if (profile.groups.length === 1) {
        return profile.groups[0].openProbability;
    }
    return `${profile.groups.length} groups`;
}

function buildSimulationNodeTypeMap(payload: SimulationStatisticsPayload): Map<string, NodeSemanticType> {
    const result = new Map<string, NodeSemanticType>();
    for (const row of payload.statistics.sourceFlow) {
        result.set(row.sourceId, "SOURCE");
    }
    for (const row of payload.statistics.transitPressure) {
        result.set(row.transitNodeId, "JUNCTION");
    }
    for (const row of payload.statistics.terminalPressure) {
        result.set(row.terminalId, "TERMINAL");
    }
    return result;
}

function buildPressureRows(payload: SimulationStatisticsPayload) {
    const terminalRows = payload.statistics.terminalPressure.map(
        row => ({
            nodeId: row.terminalId,
            nodeType: "TERMINAL" as const,
            minPressureM: row.minPressureM,
            meanPressureM: row.meanPressureM,
            maxPressureM: row.maxPressureM,
        })
    );
    const transitRows = payload.statistics.transitPressure.map(
        row => ({
            nodeId: row.transitNodeId,
            nodeType: "JUNCTION" as const,
            minPressureM: row.minPressureM,
            meanPressureM: row.meanPressureM,
            maxPressureM: row.maxPressureM,
        })
    );
    return [...terminalRows, ...transitRows];
}

//Builds the complete result-table collection for one persisted simulation.
//Keeping this transformation outside the dialog allows the same tables to be reused by comparison and export interfaces later.
export function buildSimulationResultTables(payload: SimulationStatisticsPayload): SimulationResultTable[] {

    const demandByTerminal = terminalDemandById(payload);
    const nodeTypeById = buildSimulationNodeTypeMap(payload);

    return [
        {
            key: "terminalSatisfaction",
            title: "Terminal Satisfaction",
            description: "Percentage of requested terminal demand satisfied across successful scenarios.",
            columns: [
                { key: "terminalId", label: "Terminal", nodePresentation: "ID", nodeType: "TERMINAL" },
                {
                    key: "minSat",
                    label: "Minimum",
                    unit: "%",
                    numeric: true,
                    format: "PERCENT",
                },
                {
                    key: "moySat",
                    label: "Mean",
                    unit: "%",
                    numeric: true,
                    format: "PERCENT",
                },
                {
                    key: "maxSat",
                    label: "Maximum",
                    unit: "%",
                    numeric: true,
                    format: "PERCENT",
                },
                {
                    key: "cVarSat5",
                    label: "CVaR 5%",
                    unit: "%",
                    numeric: true,
                    format: "PERCENT",
                },
                {
                    key: "cVarSat20",
                    label: "CVaR 20%",
                    unit: "%",
                    numeric: true,
                    format: "PERCENT",
                },
                {
                    key: "clientCount",
                    label: "Clients",
                    numeric: true,
                    format: "INTEGER",
                    detailsOnly: true,
                },
                {
                    key: "targetFlowSummary",
                    label: "Target flow / client",
                    unit: "L/s",
                    detailsOnly: true,
                },
                {
                    key: "probabilitySummary",
                    label: "Open probability / client",
                    detailsOnly: true,
                },
                {
                    key: "activeScenarioCount",
                    label: "Active scenarios",
                    title: "Scenarios in which at least one client attached to the terminal was open.",
                    numeric: true,
                    format: "INTEGER",
                    detailsOnly: true,
                },
                {
                    key: "waterShortfallCount",
                    label: "Water shortfalls",
                    title: "Successfully solved active scenarios in which delivered flow was below the target demand.",
                    numeric: true,
                    format: "INTEGER",
                    detailsOnly: true,
                },
                {
                    key: "stdSat",
                    label: "Std deviation",
                    numeric: true,
                    format: "PERCENT",
                    detailsOnly: true,
                },
                {
                    // std / mean
                    key: "cvSat",
                    label: "CV",
                    numeric: true,
                    detailsOnly: true,
                },
            ],
            rows: payload.statistics.terminalSatisfaction.map(row => {
                const demand = demandByTerminal.get(row.terminalId);
                return {
                    ...row,
                    clientCount: demand?.clientCount ?? null,
                    targetFlowSummary: targetFlowSummary(demand),
                    probabilitySummary: probabilitySummary(demand),
                };
            }),
        },
        {
            key: "sourceFlow",
            title: "Source Flow",
            description: "Flow statistics at network sources.",
            columns: [{ key: "sourceId", label: "Source", nodePresentation: "ID", nodeType: "SOURCE" },
            {
                key: "minSt",
                label: "Minimum",
                unit: unit(payload.units.sourceFlow, "L/s"),
                numeric: true,
            },
            {
                key: "moySt",
                label: "Mean",
                unit: unit(payload.units.sourceFlow, "L/s"),
                numeric: true,
            },
            {
                key: "maxSt",
                label: "Maximum",
                unit: unit(payload.units.sourceFlow, "L/s"),
                numeric: true,
            },
            {
                key: "cVarSt5",
                label: "CVaR 5%",
                unit: unit(payload.units.sourceFlow, "L/s"),
                numeric: true,
            },
            {
                key: "cVarSt95",
                label: "CVaR 95%",
                unit: unit(payload.units.sourceFlow, "L/s"),
                numeric: true,
            },
            ],
            rows: payload.statistics.sourceFlow.map(row => ({ ...row })),
        },
        {
            key: "pipeVelocity",
            title: "Pipe Velocity",
            description: "Velocity statistics for network pipes.",
            columns: [
                {
                    key: "edgeId",
                    label: "Pipe",
                },
                {
                    key: "startNodeId",
                    label: "From",
                    nodePresentation: "ID",
                    nodeTypeKey: "startNodeType",
                },
                {
                    key: "endNodeId",
                    label: "To",
                    nodePresentation: "ID",
                    nodeTypeKey: "endNodeType",
                },
                {
                    key: "minVelocityMps",
                    label: "Minimum",
                    unit: unit(payload.units.pipeVelocity, "m/s"),
                    numeric: true,
                },
                {
                    key: "meanVelocityMps",
                    label: "Mean",
                    unit: unit(payload.units.pipeVelocity, "m/s"),
                    numeric: true,
                },
                {
                    key: "maxVelocityMps",
                    label: "Maximum",
                    unit: unit(payload.units.pipeVelocity, "m/s"),
                    numeric: true,
                },
            ],
            rows: payload.statistics.pipeVelocity.map(row => ({
                ...row,
                startNodeType: nodeTypeById.get(row.startNodeId) ?? null,
                endNodeType: nodeTypeById.get(row.endNodeId) ?? null,
            })),
        },
        {
            key: "pressure",
            title: "Pressure",
            description: "Pressure-head statistics at terminal and transit nodes.",
            columns: [
                {
                    key: "nodeId",
                    label: "Node",
                    nodePresentation: "ID",
                    nodeTypeKey: "nodeType",
                },
                {
                    key: "nodeType",
                    label: "Type",
                    nodePresentation: "TYPE",
                    nodeTypeKey: "nodeType",
                },
                {
                    key: "minPressureM",
                    label: "Minimum",
                    unit: unit(payload.units.terminalPressure, "m"),
                    numeric: true,
                },
                {
                    key: "meanPressureM",
                    label: "Mean",
                    unit: unit(payload.units.terminalPressure, "m"),
                    numeric: true,
                },
                {
                    key: "maxPressureM",
                    label: "Maximum",
                    unit: unit(payload.units.terminalPressure, "m"),
                    numeric: true,
                },
            ],
            rows: buildPressureRows(payload),
        },
    ];
}