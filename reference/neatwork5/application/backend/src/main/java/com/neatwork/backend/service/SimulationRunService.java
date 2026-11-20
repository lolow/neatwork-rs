
package com.neatwork.backend.service;


import com.neatwork.backend.dao.ProjectArtifactRepository;
import com.neatwork.backend.dao.ProjectDomainSnapshotRepository;
import com.neatwork.backend.dao.ProjectRepository;
import com.neatwork.backend.enums.EdgeStatus;
import com.neatwork.backend.enums.SimulationConfigurationMode;
import com.neatwork.backend.enums.SimulationJobStatus;
import com.neatwork.backend.exception.SimulationRunConflictException;
import com.neatwork.backend.model.entity.ProjectDomainSnapshotEntity;
import com.neatwork.backend.request.SimulationEdgeOverrideRequest;
import com.neatwork.backend.response.*;
import tools.jackson.core.type.TypeReference;
import tools.jackson.databind.ObjectMapper;

import com.neatwork.backend.dao.SimulationRunRepository;

import com.neatwork.backend.exception.ResourceNotFoundException;

import com.neatwork.backend.model.entity.SimulationRunEntity;

import com.neatwork.backend.request.SimulationOptionsRequest;

import lombok.RequiredArgsConstructor;

import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.UUID;


@Service
@RequiredArgsConstructor
public class SimulationRunService {

    private final SimulationRunRepository repository;
    private final ProjectRepository projectRepository;
    private final ProjectDomainSnapshotRepository snapshotRepository;
    private final ProjectArtifactRepository artifactRepository;
    private final ObjectMapper objectMapper;

    // Return the latest simulation result available for export.
    @Transactional(readOnly = true)
    public SimulationRunEntity latest(UUID projectId) {
        return repository.findFirstByProjectIdAndStatusInOrderByCompletedAtDescCreatedAtDesc(projectId, COMPLETED_STATUSES)
                .orElseThrow(() -> new ResourceNotFoundException("No completed simulation exists for project: " + projectId));
    }

    /**
     * Return every persisted simulation for one project.
     * <p>Runs are ordered from oldest to newest. The index in this ordered list
     * becomes the human-readable Simulation #1, #2, #3, ... number.</p>
     * @param projectId owning project
     * @return lightweight chronological simulation history
     */
    @Transactional(readOnly = true)
    public List<SimulationRunSummaryResponse> listRuns(UUID projectId) {

        ensureProjectExists(projectId);
        var runs = repository.findByProjectIdAndStatusInOrderByCreatedAtAscIdAsc(projectId, COMPLETED_STATUSES);
        return runs.stream()
                .map(this::toSummary)
                .toList();
    }

    /**
     * Return one complete historical simulation result.
     *
     * @param projectId owning project identifier
     * @param runId simulation-run identifier
     * @return complete persisted runtime options and statistics
     */
    @Transactional(readOnly = true)
    public SimulationRunDetailsResponse getRun(UUID projectId, UUID runId) {
        ensureProjectExists(projectId);
        /*
         * Load the complete ordered history as well so the same deterministic
         * run number used by the list endpoint is returned here.
         */
        var run = repository.findByIdAndProjectId(runId, projectId).orElseThrow(() ->
                new ResourceNotFoundException("Simulation run not found: " + runId));

        return toDetails(run);
    }

    /**
     * Persist one simulation attempt before it is submitted to the background executor.
     * A short project lock is used only while allocating the stable run number and pinning the current domain snapshot. Simulation execution itself remains fully asynchronous and concurrent.
     */
    @Transactional
    public SimulationRunEntity createPendingRun(UUID projectId, SimulationOptionsRequest requestedOptions) {
        projectRepository.findByIdForUpdate(projectId).orElseThrow(() ->
                        new ResourceNotFoundException("Project not found: " + projectId));

        var project = projectRepository.findWithNodesAndEdgesById(projectId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("Project not found: " + projectId));

        // Pin the exact OPEN/CLOSED state of every pipe.
        // Later project edits must not change the hydraulic  input of this already submitted execution.
        List<Map<String, Object>> edgeOverridesJson = project.getEdges().stream()
                .map(edge -> {
                    Map<String, Object> value = new java.util.LinkedHashMap<>();
                    value.put("edgeId", edge.getExternalId());
                    value.put("enabled", edge.getStatus() == EdgeStatus.OPEN);
                    return value;})
                .toList();

        var snapshot = snapshotRepository.findFirstByProjectIdOrderByCreatedAtDesc(projectId)
                .orElseThrow(() ->
                        new ResourceNotFoundException("No domain snapshot found for project: " + projectId));

        // Resolve immutable input-file provenance before the asynchronous worker is submitted.
        var sourceArtifact = snapshot.getPrimaryArtifactId() == null ? null : artifactRepository
              .findById(snapshot.getPrimaryArtifactId()).orElse(null);

        var nextRunNumber = repository.findMaxRunNumberByProjectId(projectId).orElse(0) + 1;

        var inputName = resolveInputName(snapshot);

        var requestedOptionsJson = compactRequestedOptions(requestedOptions);
        var configurationMode = requestedOptionsJson.isEmpty()
                ? SimulationConfigurationMode.DEFAULT
                : SimulationConfigurationMode.MANUAL;

        var run = SimulationRunEntity.builder()
                .projectId(projectId)
                .runNumber(nextRunNumber)
                // Pin the exact domain revision used by this execution.
                .domainSnapshotId(snapshot.getId())
                .configurationMode(configurationMode)
                .requestedOptionsJson(requestedOptionsJson)
                .edgeOverridesJson(edgeOverridesJson)
                .inputArtifactId(sourceArtifact == null ? null : sourceArtifact.getId())
                .inputName(inputName)
                .inputFilename(sourceArtifact == null ? null : sourceArtifact.getOriginalFilename())
                .inputVersion(sourceArtifact == null ? null : sourceArtifact.getVersion())
                .inputSourceFormat(snapshot.getSourceFormat())
                .status(SimulationJobStatus.PENDING)
                .cancelRequested(false)
                .build();

        return repository.saveAndFlush(run);
    }

    /**
     * Return the complete lifecycle history of one project.
     * Unlike the comparison API, this method includes pending, running, failed and cancelled executions.
     */
    @Transactional(readOnly = true)
    public List<SimulationHistoryItemResponse> listHistory(UUID projectId) {
        ensureProjectExists(projectId);
        return repository.findByProjectIdOrderByRunNumberAsc(projectId)
                .stream()
                .map(this::toHistoryItem)
                .toList();
    }

    // Convert one persistent execution into a history-row response.
    private SimulationHistoryItemResponse toHistoryItem(SimulationRunEntity run) {
        return new SimulationHistoryItemResponse(
                run.getId(),
                run.getRunNumber(),
                run.getProjectId(),
                run.getDomainSnapshotId(),
                run.getStatus(),
                run.isCancelRequested(),
                run.getConfigurationMode(),
                run.getInputName(),
                run.getInputFilename(),
                run.getInputVersion(),
                run.getInputSourceFormat(),
                run.getSeed(),
                run.getScenarioCount(),
                run.getResultStatus(),
                run.getCreatedAt(),
                run.getStartedAt(),
                run.getCompletedAt(),
                run.getErrorMessage()
        );
    }

    /**
     * Move one persisted execution from PENDING to RUNNING.
     * When cancellation was requested while the task was waiting in the executor queue, the task is cancelled without contacting Python.
     */
    @Transactional
    public ExecutionContext beginExecution(UUID runId) {
        var run = repository.findByIdForUpdate(runId).orElseThrow(() ->
                                new ResourceNotFoundException("Simulation execution not found: " + runId)
                        );
        if (isTerminal(run.getStatus())) {
            return null;
        }
        if (run.isCancelRequested() || run.getStatus() == SimulationJobStatus.CANCEL_REQUESTED) {
            markCancelledEntity(run);
            return null;
        }
        run.setStatus(SimulationJobStatus.RUNNING);
        run.setStartedAt(Instant.now());
        repository.save(run);
        var options = objectMapper.convertValue(run.getRequestedOptionsJson(), SimulationOptionsRequest.class);

        var edgeOverrides = objectMapper.convertValue(run.getEdgeOverridesJson(),
                new TypeReference<List<SimulationEdgeOverrideRequest>>() {});

        return new ExecutionContext(run.getId(), run.getProjectId(), run.getDomainSnapshotId(), options, edgeOverrides);
    }

    /**
     * Persist the successful Python response and move
     * the execution to its final lifecycle state.
     */
    @Transactional
    public void completeRun(UUID runId, SimulatedGraphResponse response) {
        var run = repository.findByIdForUpdate(runId).orElseThrow(() ->
                                new ResourceNotFoundException("Simulation execution not found: " + runId));

        // Cancellation wins when it was accepted before the successful result was committed.
        if (run.isCancelRequested()) {
            markCancelledEntity(run);
            return;
        }

        Map<String, Object> resultJson = objectMapper.convertValue(response, new TypeReference<>() {});
        run.setResultJson(resultJson);

        // Persist the exact values used by Python instead of  attempting to reconstruct them later in Java.
        if (response.executionParameters() != null) {
            run.setResolvedOptionsJson(new java.util.LinkedHashMap<>(
                    response.executionParameters().values())
            );
            run.setParameterSourcesJson(new java.util.LinkedHashMap<>(
                    response.executionParameters().sources())
            );
        }
        run.setStatisticsJson(response.statistics());
        run.setSeed(response.summary().seed());
        run.setScenarioCount(response.summary().scenarioCount());
        run.setResultStatus(response.summary().status());

        /*
         * COMPLETED_WITH_ERRORS is reserved for execution or solver
         * failures. Hydraulic water shortfalls are valid simulation
         * outcomes and must not change the job lifecycle status.
         */
        run.setStatus(response.summary().failedScenarios() > 0
                        ? SimulationJobStatus.COMPLETED_WITH_ERRORS
                        : SimulationJobStatus.COMPLETED
        );
        run.setCompletedAt(Instant.now());
        run.setErrorMessage(null);
        repository.save(run);
    }

    /**
     * Permanently deletes one terminal simulation run.
     * Only the execution record and its persisted result are removed. Project files, domain snapshots and other simulation runs are deliberately preserved.
     */
    @Transactional
    public void deleteRun(UUID projectId, UUID runId) {
        var run = repository.findByIdAndProjectIdForUpdate(runId, projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Simulation run not found: " + runId));

        if (!isTerminal(run.getStatus())) {
            throw new SimulationRunConflictException("An active simulation cannot be deleted. Cancel it first and wait until it reaches a terminal state.");
        }

        // Deleting a simulation must never delete its source domain snapshot or input artifact.
        repository.delete(run);
    }

    // Persist a terminal execution failure.
    @Transactional
    public void failRun(UUID runId, String errorMessage) {
        var run = repository.findByIdForUpdate(runId).orElseThrow(() ->
                                new ResourceNotFoundException("Simulation execution not found: " + runId));

        if (run.isCancelRequested()) {
            markCancelledEntity(run);
            return;
        }
        run.setStatus(SimulationJobStatus.FAILED);
        run.setErrorMessage(errorMessage);
        run.setCompletedAt(Instant.now());
        repository.save(run);
    }

    // Persist a cooperative cancellation request.
    @Transactional
    public SimulationRunEntity requestCancellation(UUID projectId, UUID runId) {

        var run = repository.findByIdAndProjectIdForUpdate(runId, projectId).orElseThrow(() ->
                new ResourceNotFoundException("Simulation execution not found: " + runId));
        if (isTerminal(run.getStatus())) {
            return run;
        }

        run.setCancelRequested(true);
        run.setStatus(SimulationJobStatus.CANCEL_REQUESTED);
        return repository.save(run);
    }

    // Mark one execution as cancelled.
    @Transactional
    public void markCancelled(UUID runId) {
        var run = repository.findByIdForUpdate(runId).orElseThrow(() ->
                new ResourceNotFoundException("Simulation execution not found: " + runId));
        markCancelledEntity(run);
    }

    // Return the persisted state of one asynchronous job.
    @Transactional(readOnly = true)
    public SimulationJobResponse getJob(UUID projectId, UUID runId) {
        var run = repository.findByIdAndProjectId(runId, projectId)
                        .orElseThrow(() ->
                                new ResourceNotFoundException("Simulation job not found: " + runId));

        return toJobResponse(run);
    }

    // Return unfinished executions so the frontend can reconnect to them after a project switch.
    @Transactional(readOnly = true)
    public List<SimulationJobResponse> listActiveJobs(UUID projectId) {
        ensureProjectExists(projectId);

        return repository
                .findByProjectIdAndStatusInOrderByCreatedAtAsc(projectId, ACTIVE_STATUSES)
                .stream()
                .map(this::toJobResponse)
                .toList();
    }

    /**
     * Mark executions interrupted by a backend restart as failed.
     * Automatically rerunning them would risk duplicate Python executions, so recovery is intentionally conservative.
     */
    @Transactional
    public int failInterruptedExecutions() {
        var interrupted = repository.findByStatusIn(ACTIVE_STATUSES);
        var now = Instant.now();

        for (var run : interrupted) {
            run.setStatus(SimulationJobStatus.FAILED);
            run.setErrorMessage("Simulation interrupted because the backend restarted.");
            run.setCompletedAt(now);
        }
        repository.saveAll(interrupted);

        return interrupted.size();
    }

    public boolean isCancelRequested(UUID runId) {
        return repository.findById(runId)
                .map(SimulationRunEntity::isCancelRequested).orElse(false);
    }

    private void markCancelledEntity(SimulationRunEntity run) {
        run.setCancelRequested(true);
        run.setStatus(SimulationJobStatus.CANCELLED);
        run.setCompletedAt(Instant.now());
        repository.save(run);
    }


    private boolean isTerminal(SimulationJobStatus status) {

        return switch (status) {
            case CANCELLED,
                 COMPLETED,
                 COMPLETED_WITH_ERRORS,
                 FAILED -> true;
            default -> false;
        };
    }

    private SimulationJobResponse toJobResponse(SimulationRunEntity run) {
        SimulatedGraphResponse result = null;

        if (run.getResultJson() != null) {
            result = objectMapper.convertValue(run.getResultJson(), SimulatedGraphResponse.class);
        }

        return new SimulationJobResponse(
                run.getId(),
                run.getProjectId(),
                run.getStatus(),
                run.isCancelRequested(),
                run.getCreatedAt(),
                run.getCompletedAt(),
                result,
                run.getErrorMessage()
        );
    }

    // Immutable input required by one background worker.
    public record ExecutionContext(
            UUID runId,
            UUID projectId,
            UUID domainSnapshotId,
            SimulationOptionsRequest options,
            List<SimulationEdgeOverrideRequest> edgeOverrides
    ) {
    }

    private static final List<SimulationJobStatus> COMPLETED_STATUSES = List.of(SimulationJobStatus.COMPLETED, SimulationJobStatus.COMPLETED_WITH_ERRORS);

    private static final List<SimulationJobStatus> ACTIVE_STATUSES = List.of(
            SimulationJobStatus.PENDING,
            SimulationJobStatus.RUNNING,
            SimulationJobStatus.CANCEL_REQUESTED
    );

    /**
     * Convert one entity into the lightweight history representation.
     */
    private SimulationRunSummaryResponse toSummary(SimulationRunEntity run) {
        return new SimulationRunSummaryResponse(
                run.getId(),
                run.getRunNumber(),
                run.getProjectId(),
                run.getDomainSnapshotId(),
                run.getSeed(),
                run.getScenarioCount(),
                run.getStatus(),
                run.getCreatedAt()
        );
    }

    /**
     * Convert one persisted execution into its complete API representation.
     */
    private SimulationRunDetailsResponse toDetails(SimulationRunEntity run) {

        var input = new SimulationInputDescriptorResponse(
                run.getInputName(),
                run.getInputArtifactId(),
                run.getInputFilename(),
                run.getInputVersion(),
                run.getInputSourceFormat(),
                run.getDomainSnapshotId()
        );

        var requestedOptions = run.getRequestedOptionsJson() == null
                ? Map.<String, Object>of()
                : run.getRequestedOptionsJson();

        var edgeOverrides = run.getEdgeOverridesJson() == null
                ? List.<Map<String, Object>>of()
                : run.getEdgeOverridesJson();

        return new SimulationRunDetailsResponse(
                run.getId(),
                run.getRunNumber(),
                run.getProjectId(),
                run.getStatus().name(),
                run.getResultStatus(),
                run.getCreatedAt(),
                run.getStartedAt(),
                run.getCompletedAt(),
                run.getSeed(),
                run.getScenarioCount(),
                run.getConfigurationMode(),
                input,
                requestedOptions,
                run.getResolvedOptionsJson(),
                run.getParameterSourcesJson(),
                edgeOverrides,
                run.getStatisticsJson(),
                run.getErrorMessage()
        );
    }

    /**
     * Fail clearly when the requested project does not exist.
     */
    private void ensureProjectExists(UUID projectId) {
        if (!projectRepository.existsById(projectId)) {
            throw new ResourceNotFoundException("Project not found: " + projectId);
        }
    }

    /**
     * Converts one request into the exact runtime overrides supplied by the frontend.
     * Null values are removed so an empty map unambiguously represents DEFAULT execution.
     */
    private Map<String, Object> compactRequestedOptions(SimulationOptionsRequest request) {
        var effectiveRequest = request == null
                ? SimulationOptionsRequest.empty() : request;

        Map<String, Object> values = new java.util.LinkedHashMap<>(
                objectMapper.convertValue(effectiveRequest, new TypeReference<>() {}));

        values.entrySet().removeIf(entry -> entry.getValue() == null);

        return values;
    }

    // Reads the network name stored in the immutable domain snapshot.
    private String resolveInputName(ProjectDomainSnapshotEntity snapshot) {
        if (snapshot.getDomainJson() == null) {
            return "Network";
        }
        var value = snapshot.getDomainJson().get("name");
        if (value == null) {
            return "Network";
        }
        var name = String.valueOf(value).trim();

        return name.isBlank() ? "Network" : name;
    }
}