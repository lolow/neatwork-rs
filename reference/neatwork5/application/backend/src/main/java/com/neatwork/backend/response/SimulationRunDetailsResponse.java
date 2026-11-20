package com.neatwork.backend.response;

import com.neatwork.backend.enums.SimulationConfigurationMode;
import com.neatwork.backend.enums.SimulationJobStatus;

import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.UUID;


/**
 * Complete persisted simulation result used by the comparison interface.
 *
 * @param id persistent simulation-run identifier
 * @param runNumber chronological display number within the project
 * @param projectId owning project
 * @param seed reproducibility seed
 * @param scenarioCount number of simulated scenarios
 * @param status final simulation status
 * @param createdAt run creation time
 * @param statistics canonical Step-9 statistics JSON
 */
public record SimulationRunDetailsResponse(
        UUID id,
        int runNumber,
        UUID projectId,
        String status,
        String resultStatus,
        Instant createdAt,
        Instant startedAt,
        Instant completedAt,
        Long seed,
        Integer scenarioCount,
        SimulationConfigurationMode configurationMode,
        SimulationInputDescriptorResponse input,
        Map<String, Object> requestedOptions,
        Map<String, Object> resolvedOptions,
        Map<String, Object> parameterSources,
        List<Map<String, Object>> edgeOverrides,
        Map<String, Object> statistics,
        String errorMessage
) {
}