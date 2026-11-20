package com.neatwork.backend.request;

import com.neatwork.backend.enums.EdgeStatus;
import com.neatwork.backend.enums.EdgeType;

public record SimulationEdgeRequest(
        String id,
        String sourceId,
        String targetId,
        EdgeType type,
        EdgeStatus status,
        double flowRateLps,
        double velocityMps
) {
}
