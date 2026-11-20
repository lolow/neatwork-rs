package com.neatwork.backend.response;

import com.neatwork.backend.enums.SimulationJobStatus;

import java.time.Instant;
import java.util.UUID;


/**
 * Current state of one asynchronous simulation execution.
 */
public record SimulationJobResponse(

        UUID jobId,

        UUID projectId,

        SimulationJobStatus status,

        boolean cancelRequested,

        Instant createdAt,

        Instant completedAt,

        SimulatedGraphResponse result,

        String errorMessage

) {
}