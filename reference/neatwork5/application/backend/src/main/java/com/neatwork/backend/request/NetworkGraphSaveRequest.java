package com.neatwork.backend.request;

import com.neatwork.backend.response.NetworkEdgeResponse;
import com.neatwork.backend.response.NetworkNodeResponse;
import jakarta.validation.constraints.NotEmpty;

import java.util.List;

public record NetworkGraphSaveRequest(
        @NotEmpty List<NetworkNodeResponse> nodes,
        @NotEmpty List<NetworkEdgeResponse> edges
) {
}