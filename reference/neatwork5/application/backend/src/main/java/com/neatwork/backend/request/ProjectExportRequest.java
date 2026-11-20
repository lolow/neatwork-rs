package com.neatwork.backend.request;

import com.neatwork.backend.enums.ProjectExportFormat;

import jakarta.validation.constraints.NotNull;

/**
 * Select one project export representation.
 *
 * @param format requested output format
 */
public record ProjectExportRequest(@NotNull ProjectExportFormat format) {
}