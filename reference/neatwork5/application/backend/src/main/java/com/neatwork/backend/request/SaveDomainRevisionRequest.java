package com.neatwork.backend.request;

import com.neatwork.backend.enums.RevisionNamingMode;

import jakarta.validation.constraints.NotNull;

import java.util.Map;
import java.util.UUID;

public record SaveDomainRevisionRequest(

        @NotNull
        UUID sourceSnapshotId,

        @NotNull
        Map<String, Object> domain,

        @NotNull
        RevisionNamingMode namingMode,

        String customFilename
) {
}