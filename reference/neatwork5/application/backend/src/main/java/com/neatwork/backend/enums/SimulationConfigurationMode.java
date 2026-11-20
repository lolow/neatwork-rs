package com.neatwork.backend.enums;


// Describes how runtime simulation parameters were selected by the user.
public enum SimulationConfigurationMode {

    /**
     * No runtime parameter overrides were requested.
     * Python Core resolves values from the active domain.
     */
    DEFAULT,

    // The user opened the manual configuration workflow and explicitly submitted runtime parameters.
    MANUAL
}