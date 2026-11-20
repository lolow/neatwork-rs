package com.neatwork.backend.response;

public record SimulationEdgeResult(
        String id,
        double flowRateLps,
        double velocityMps
) {
}
