package com.neatwork.backend.request;

import jakarta.validation.constraints.NotBlank;
import jakarta.validation.constraints.NotNull;

import java.util.UUID;

public record ImportFromArtifactsRequest(
        @NotBlank String name,
        @NotBlank String format,
        @NotNull UUID primaryArtifactId,
        UUID diametersDbArtifactId,
        UUID orificesDbArtifactId
        ) {
}
