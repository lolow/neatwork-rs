package com.neatwork.backend.response;

import java.time.Instant;

public record HealthResponse(
        String service,
        String status,
        Instant timestamp
    ) {
}
