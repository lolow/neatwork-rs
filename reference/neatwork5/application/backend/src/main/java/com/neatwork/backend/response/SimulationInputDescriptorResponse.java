package com.neatwork.backend.response;

import java.util.UUID;

//Identifies the exact project input revision associated with one simulation execution.
public record SimulationInputDescriptorResponse(
        String networkName,
        UUID artifactId,
        String filename,
        Integer version,
        String sourceFormat,
        UUID domainSnapshotId
) {
}