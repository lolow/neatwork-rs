package com.neatwork.backend.response;

public record DomainRevisionResponse(
        ProjectArtifactResponse artifact,
        ProjectDomainSnapshotResponse snapshot,
        NetworkGraphResponse graph
) {
}