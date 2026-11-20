package com.neatwork.backend.service;

import com.neatwork.backend.enums.SimulationJobStatus;
import com.neatwork.backend.exception.PythonSimulationException;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.request.SimulationOptionsRequest;
import com.neatwork.backend.response.SimulationJobResponse;

import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.stereotype.Service;

import java.util.List;
import java.util.UUID;
import java.util.concurrent.Executor;

/**
 * Manage asynchronous and cancellable simulation jobs.
 */
@Service
@Slf4j
public class SimulationJobService {

    private final SimulationService simulationService;
    private final PythonSimulationClient pythonSimulationClient;
    private final Executor executor;
    private final SimulationRunService simulationRunService;



    public SimulationJobService(SimulationService simulationService,
            PythonSimulationClient pythonSimulationClient,
            @Qualifier("simulationTaskExecutor") Executor executor,
            SimulationRunService simulationRunService
    ) {
        this.simulationService = simulationService;
        this.pythonSimulationClient = pythonSimulationClient;
        this.executor = executor;
        this.simulationRunService = simulationRunService;
    }

    /**
     * Persist and submit one asynchronous simulation.
     * The database row is created before the task is handed to the executor, so every attempt is durable.
     */
    public SimulationJobResponse start(UUID projectId, SimulationOptionsRequest options) {
        if (options != null && (options.openTapsProbability() != null || options.targetFlowLps() != null)) {
            log.warn("Ignoring legacy per-run demand overrides; client demand is read from the pinned domain snapshot");
        }
        var canonicalOptions = options == null ? SimulationOptionsRequest.empty() : options.withoutClientDemandOverrides();

        var run = simulationRunService.createPendingRun(projectId, canonicalOptions);
        executor.execute(() -> execute(run.getId()));

        return simulationRunService.getJob(projectId, run.getId());
    }

     // Read current job state from PostgreSQL.
    public SimulationJobResponse get(UUID projectId, UUID jobId) {
        return simulationRunService.getJob(projectId, jobId);
    }

    // Return every currently unfinished execution.
    public List<SimulationJobResponse> listActive(UUID projectId) {
        return simulationRunService.listActiveJobs(projectId);
    }

    /**
     * Request cooperative cancellation.
     * The cancellation state is persisted before the Python cancellation endpoint is called.
     */
    public SimulationJobResponse cancel(UUID projectId, UUID jobId) {
        var run = simulationRunService.requestCancellation(projectId, jobId);

        if (isTerminal(run.getStatus())) {
            return simulationRunService.getJob(projectId, jobId);
        }
        pythonSimulationClient.cancelExecution(jobId.toString());

        return simulationRunService.getJob(projectId, jobId);
    }

    /**
     * Execute one persisted job in a background worker.
     * No mutable execution state is stored in a Java map. The database remains the source of truth.
     */
    private void execute(UUID runId) {
        SimulationRunService.ExecutionContext context;

        try {
            context = simulationRunService.beginExecution(runId);
            if (context == null) {
                return;
            }
        } catch (Exception exception) {
            log.error("Could not initialize simulation {}.", runId, exception);
            safeFail(runId, exception.getMessage());
            return;
        }

        try {
            var result = simulationService.runSimulation(
                    context.projectId(),
                    context.domainSnapshotId(),
                    context.options(),
                    context.edgeOverrides(),
                    runId.toString()
            );
            simulationRunService.completeRun(runId, result);

        } catch (PythonSimulationException exception) {
            if (exception.getStatusCode() == 409 || simulationRunService.isCancelRequested(runId)) {
                simulationRunService.markCancelled(runId);
                return;
            }

            simulationRunService.failRun(runId, exception.getResponseBody());

        } catch (Exception exception) {
            if (simulationRunService.isCancelRequested(runId)) {
                simulationRunService.markCancelled(runId);
                return;
            }
            simulationRunService.failRun(runId, exception.getMessage());
        }
    }

    // Avoid masking the original initialization error if the execution row disappeared concurrently.
    private void safeFail(UUID runId, String message) {
        try {
            simulationRunService.failRun(runId, message);
        } catch (ResourceNotFoundException ignored) {
            log.warn("Simulation execution {} disappeared before failure could be persisted.", runId);
        }
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

}