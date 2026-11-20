package com.neatwork.backend.response;

import java.time.Instant;
import java.util.Map;
import java.util.UUID;

public record ProjectDomainSnapshotResponse(
        UUID id,
        UUID projectId,
        String sourceFormat,
        Map<String, Object> domain,
        Instant createdAt,
        UUID primaryArtifactId,
        UUID diametersArtifactId,
        UUID orificesArtifactId
) {
}
