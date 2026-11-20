package com.neatwork.backend.response;

import java.util.Map;

// Fully resolved simulation parameters returned by Python Core.
public record SimulationExecutionParametersResponse(
        Map<String, Object> values,
        Map<String, String> sources
) {
}