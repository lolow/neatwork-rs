import type { NodeSemanticType, } from "../network/nodeSemantics";
import type { SimulationStatisticsPayload, } from "./types";

export function buildSimulationNodeTypeMap(payload: SimulationStatisticsPayload): Map<string, NodeSemanticType> {
    const result = new Map<string, NodeSemanticType>();

    payload.statistics.sourceFlow.forEach(row => result.set(row.sourceId, "SOURCE"));
    payload.statistics.transitPressure.forEach(row => result.set(row.transitNodeId, "JUNCTION"));
    payload.statistics.terminalPressure.forEach(row => result.set(row.terminalId, "TERMINAL"));

    return result;
}