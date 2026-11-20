package com.neatwork.backend.response;

import java.util.Map;

/**
 * Updated project graph plus complete simulation statistics.
 */
public record SimulatedGraphResponse(
        NetworkGraphResponse graph,
        SimulationSummaryResponse summary,
        Map<String, Object> statistics,
        SimulationExecutionParametersResponse executionParameters
) {
}
