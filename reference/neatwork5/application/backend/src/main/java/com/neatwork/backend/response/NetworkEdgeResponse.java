package com.neatwork.backend.response;

import com.neatwork.backend.enums.EdgeStatus;
import com.neatwork.backend.enums.EdgeType;

public record NetworkEdgeResponse(
        String id,
        String sourceId,
        String targetId,
        EdgeType type,
        EdgeStatus status,
        double flowRateLps,
        double velocityMps,

        Double lengthM,
        String diameterId,
        String nominalSize,
        Double sdr,
        Double innerDiameter,
        Double cost,
        Double pmax,
        String pipeTypeCode,
        Double roughness
) {
}
