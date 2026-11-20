package com.neatwork.backend.response;

import java.util.List;

public record NetworkGraphResponse(
        String id,
        String name,
        List<NetworkNodeResponse> nodes,
        List<NetworkEdgeResponse> edges
) {
}
