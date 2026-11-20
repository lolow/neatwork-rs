package com.neatwork.backend.response;

import java.util.List;
import java.util.Map;

/**
 * Hydraulic result returned by Python Core.
 */
public record SimulationRunResponse(
        String projectId,
        List<SimulationNodeResult> nodes,
        List<SimulationEdgeResult> edges,
        SimulationSummaryResponse summary,
        Map<String, Object> statistics,
        SimulationExecutionParametersResponse executionParameters
) {
}
