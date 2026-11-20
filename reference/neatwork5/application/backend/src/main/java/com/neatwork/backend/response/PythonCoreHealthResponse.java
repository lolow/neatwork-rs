package com.neatwork.backend.response;

public record PythonCoreHealthResponse(
        String service,
        String status,
        String timestamp
) {
}
