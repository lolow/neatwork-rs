package com.neatwork.backend.response;

import com.neatwork.backend.enums.SimulationConfigurationMode;
import com.neatwork.backend.enums.SimulationJobStatus;

import java.time.Instant;
import java.util.UUID;


/**
 * Lifecycle information displayed in the simulation history interface.
 * Result fields remain nullable because failed, cancelled and unfinished executions may not produce hydraulic results.
 */
public record SimulationHistoryItemResponse(
        UUID id,
        int runNumber,
        UUID projectId,
        UUID domainSnapshotId,
        SimulationJobStatus status,
        boolean cancelRequested,
        SimulationConfigurationMode configurationMode,
        String inputName,
        String inputFilename,
        Integer inputVersion,
        String inputSourceFormat,
        Long seed,
        Integer scenarioCount,
        String resultStatus,
        Instant createdAt,
        Instant startedAt,
        Instant completedAt,
        String errorMessage
) {
}