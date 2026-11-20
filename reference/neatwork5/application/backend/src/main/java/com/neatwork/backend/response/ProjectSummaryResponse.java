package com.neatwork.backend.response;

import com.neatwork.backend.enums.ProjectDisplayLengthUnit;
import com.neatwork.backend.enums.ProjectSourceFormat;

import java.time.Instant;
import java.util.UUID;

public record ProjectSummaryResponse(
        UUID id,
        String name,
        Instant createdAt,
        ProjectSourceFormat sourceFormat,
        ProjectDisplayLengthUnit displayUnit
) {
}
