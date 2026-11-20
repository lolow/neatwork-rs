package com.neatwork.backend.response;

public record SimulationNodeResult(
        String id,
        double pressureM,
        boolean lowPressure
) {
}
