package com.neatwork.backend.response;

import com.neatwork.backend.enums.SimulationJobStatus;

import java.time.Instant;
import java.util.UUID;


/**
 * Lightweight metadata describing one persisted simulation execution.
 *
 * @param id persistent simulation-run identifier
 * @param runNumber stable chronological display number within the project
 * @param projectId owning project identifier
 * @param domainSnapshotId exact imported domain used by this simulation
 * @param seed reproducibility seed
 * @param scenarioCount number of executed scenarios
 * @param status final simulation status
 * @param createdAt creation timestamp
 */
public record SimulationRunSummaryResponse(

        UUID id,
        int runNumber,
        UUID projectId,
        UUID domainSnapshotId,
        long seed,
        int scenarioCount,
        SimulationJobStatus status,
        Instant createdAt

) {
}
