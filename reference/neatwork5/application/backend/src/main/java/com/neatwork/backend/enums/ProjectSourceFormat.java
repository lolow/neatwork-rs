
package com.neatwork.backend.enums;

/**
 * Defines the primary source format of a project.
 * A project can use DSG or TPO as its active source, but it cannot use both simultaneously.
 */
public enum ProjectSourceFormat {

    // No network source has been imported yet.
    NONE,
    // The active network source is a DSG file.
    DSG,
    // The active network source is a TPO file.
    TPO,
    // Existing data requires manual review before new imports.
    REVIEW_REQUIRED
}
