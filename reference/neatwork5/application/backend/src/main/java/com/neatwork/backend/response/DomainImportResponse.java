package com.neatwork.backend.response;

public record DomainImportResponse(
        ProjectDomainSnapshotResponse snapshot,
        NetworkGraphResponse graph
) {
}
