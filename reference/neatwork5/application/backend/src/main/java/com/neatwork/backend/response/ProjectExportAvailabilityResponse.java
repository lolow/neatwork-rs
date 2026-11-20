package com.neatwork.backend.response;

import com.neatwork.backend.enums.ProjectExportFormat;


/**
 * Describe whether one export format is currently available.
 */
public record ProjectExportAvailabilityResponse(
        ProjectExportFormat format,
        boolean available,
        String reason
) {
}