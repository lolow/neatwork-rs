package com.neatwork.backend.request;

import com.neatwork.backend.enums.ProjectArtifactType;
import jakarta.validation.constraints.NotNull;

public record UploadArtifactTypeRequest(
        @NotNull ProjectArtifactType type
        ) {
}
