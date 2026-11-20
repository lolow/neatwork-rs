package com.neatwork.backend.request;

import com.neatwork.backend.enums.EdgeStatus;
import jakarta.validation.constraints.NotNull;

public record UpdateEdgeStatusRequest(
        @NotNull EdgeStatus status
        ) {
}
