package com.neatwork.backend.response;

import java.util.UUID;

public record TpoArtifactRequirementsResponse(
        UUID artifactId,
        boolean hasDiametersSection,
        boolean hasOrificesSection,
        boolean diametersDatabaseRequired
) {
}
