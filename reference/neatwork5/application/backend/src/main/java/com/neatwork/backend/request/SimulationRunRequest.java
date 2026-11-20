package com.neatwork.backend.request;

import java.util.List;
import java.util.Map;

/**
 * Complete hydraulic payload sent to Python Core.
 *
 * @param projectId project identifier
 * @param domain persisted hydraulic domain snapshot
 * @param options runtime simulation options
 * @param edgeOverrides current graph OPEN/CLOSED states
 */
public record SimulationRunRequest(
        String projectId,
        String executionId,
        Map<String, Object> domain,
        SimulationOptionsRequest options,
        List<SimulationEdgeOverrideRequest> edgeOverrides
) {
}
