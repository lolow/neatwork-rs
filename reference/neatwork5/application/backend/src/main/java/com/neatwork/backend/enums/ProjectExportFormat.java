package com.neatwork.backend.enums;


/**
 * User-selectable project export formats.
 */
public enum ProjectExportFormat {
    /**
     * Canonical simulation statistics document.
     */
    SIMULATION_JSON,
    /**
     * Spreadsheet representation of the latest simulation statistics.
     */
    SIMULATION_XLSX,
    /**
     * NeatWork design-network file.
     */
    DSG,
    /**
     * EPANET input network file.
     */
    INP
}