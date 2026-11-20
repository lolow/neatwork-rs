package com.neatwork.backend.request;

import jakarta.validation.constraints.Max;
import jakarta.validation.constraints.Min;

public record UpdateNodePositionRequest(
        @Min(0) @Max(5000) double x,
        @Min(0) @Max(5000) double y
) {
}
