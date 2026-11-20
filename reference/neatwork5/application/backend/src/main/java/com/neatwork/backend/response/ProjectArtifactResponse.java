package com.neatwork.backend.response;

import com.neatwork.backend.enums.ProjectArtifactType;

import java.time.Instant;
import java.util.UUID;

public record ProjectArtifactResponse(
        UUID id,
        UUID projectId,
        ProjectArtifactType type,
        String originalFilename,
        String storedFilename,
        long sizeBytes,
        int version,
        boolean generated,
        boolean archived,

        UUID parentArtifactId,
        UUID revisionFamilyId,
        Instant createdAt

) {
}
