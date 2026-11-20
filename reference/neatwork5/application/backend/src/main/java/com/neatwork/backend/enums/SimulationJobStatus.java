package com.neatwork.backend.enums;


/**
 * Lifecycle state of one asynchronous simulation job.
 */
public enum SimulationJobStatus {
    PENDING,
    RUNNING,
    CANCEL_REQUESTED,
    CANCELLED,
    COMPLETED,
    COMPLETED_WITH_ERRORS,
    FAILED
}