package com.neatwork.backend.response;

import com.neatwork.backend.enums.NodeType;

public record NetworkNodeResponse(
        String id,
        String label,
        NodeType type,
        double x,
        double y,
        double elevationM,
        double pressureM,
        boolean lowPressure,
        Double sourceX,
        Double sourceY,
        Boolean layoutGenerated
) {
}
