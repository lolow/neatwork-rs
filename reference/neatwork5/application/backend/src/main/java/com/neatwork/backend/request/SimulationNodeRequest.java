package com.neatwork.backend.request;

import com.neatwork.backend.enums.NodeType;

public record SimulationNodeRequest(
        String id,
        NodeType type,
        double x,
        double y,
        double elevationM,
        double pressureM
) {
}
