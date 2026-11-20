package com.neatwork.backend.service;

import com.neatwork.backend.dao.ProjectDomainSnapshotRepository;
import com.neatwork.backend.dao.ProjectRepository;

import com.neatwork.backend.enums.EdgeStatus;
import com.neatwork.backend.enums.ProjectArtifactType;

import com.neatwork.backend.exception.ResourceNotFoundException;

import com.neatwork.backend.model.entity.ProjectDomainSnapshotEntity;
import com.neatwork.backend.model.entity.ProjectEntity;

import com.neatwork.backend.model.mapper.NetworkGraphMapper;

import com.neatwork.backend.request.SimulationEdgeOverrideRequest;
import com.neatwork.backend.request.SimulationOptionsRequest;
import com.neatwork.backend.request.SimulationRunRequest;

import com.neatwork.backend.response.*;

import lombok.RequiredArgsConstructor;

import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.List;
import java.util.UUID;
import java.util.function.Function;
import java.util.stream.Collectors;

@Service
@RequiredArgsConstructor
public class SimulationService {

    private final ProjectRepository projectRepository;
    private final PythonSimulationClient pythonSimulationClient;
    private final NetworkGraphMapper mapper;
    private final ProjectDomainSnapshotRepository snapshotRepository;
    private final ProjectArtifactService projectArtifactService;

    /**
     * Execute the hydraulic simulation and persist aggregate graph values.
     */
    @Transactional
    public SimulatedGraphResponse runSimulation(UUID projectId, UUID domainSnapshotId, SimulationOptionsRequest options, List<SimulationEdgeOverrideRequest> edgeOverrides, String executionId) {
        var project = loadProject(projectId);
        var snapshot = loadSnapshot(projectId, domainSnapshotId);
        var request = buildPythonRequest(project, snapshot, options, edgeOverrides, executionId);
        var response = pythonSimulationClient.runSimulation(request);

        var resultGraph = buildResultGraph(project, response);

        return new SimulatedGraphResponse(resultGraph, response.summary(), response.statistics(), response.executionParameters());
    }

    /**
     * Generate and register the JSON statistics document.
     */
    @Transactional
    public ProjectArtifactResponse exportSimulation(UUID projectId, SimulationOptionsRequest options) {
        var project = loadProject(projectId);
        var snapshot = loadLatestSnapshot(projectId);
        var request = buildPythonRequest(project, snapshot, options, buildCurrentEdgeOverrides(project),null);
        var content = pythonSimulationClient.exportStatistics(request);
        var filename = "simulation_statistics_" + project.getName() + ".json";

        return projectArtifactService.registerGeneratedArtifact(
                        projectId,
                        ProjectArtifactType.SIMULATION_JSON,
                        filename,
                        content
                );
    }

    @Transactional(readOnly = true)
    public ProjectDomainSnapshotEntity latestSnapshot(UUID projectId) {
        return loadLatestSnapshot(projectId);
    }

    // Builds a Python request exclusively from execution inputs pinned when the job started.
    private SimulationRunRequest buildPythonRequest(ProjectEntity project, ProjectDomainSnapshotEntity snapshot, SimulationOptionsRequest options, List<SimulationEdgeOverrideRequest> edgeOverrides, String executionId) {
        var effectiveOptions = options == null ? SimulationOptionsRequest.empty() : options;

        return new SimulationRunRequest(project.getId().toString(),
                executionId,
                snapshot.getDomainJson(),
                effectiveOptions,
                edgeOverrides);
    }

    private ProjectEntity loadProject(UUID projectId) {
        return projectRepository.findWithNodesAndEdgesById(projectId)
                .orElseThrow(() ->
                                new ResourceNotFoundException("Project not found: " + projectId));
    }

    private ProjectDomainSnapshotEntity loadLatestSnapshot(UUID projectId) {
        return snapshotRepository
                .findFirstByProjectIdOrderByCreatedAtDesc(projectId)
                .orElseThrow(() -> new ResourceNotFoundException("No domain snapshot found for project: " + projectId));
    }

    /**
     * Load exactly the snapshot pinned by the execution.
     * Never replace this lookup with latestSnapshot(), otherwise an edit made during simulation could silently change the recorded input dataset.
     */
    private ProjectDomainSnapshotEntity loadSnapshot(UUID projectId, UUID snapshotId) {
        return snapshotRepository.findByIdAndProjectId(snapshotId, projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Domain snapshot not found: " + snapshotId));
    }

    // Overlay one simulation result on the persistent topology without mutating the project entities.
    // This prevents concurrent executions from overwriting each other's visual results.
    private NetworkGraphResponse buildResultGraph(ProjectEntity project, SimulationRunResponse response) {
        var baseGraph = mapper.toGraph(project);

        var nodeResults = response.nodes().stream()
                .collect(Collectors.toMap(SimulationNodeResult::id, Function.identity()));

        var edgeResults = response.edges().stream().collect(
                Collectors.toMap(SimulationEdgeResult::id, Function.identity()));

        var nodes = baseGraph.nodes().stream().map(node -> {
            var result = nodeResults.get(node.id());
            if (result == null) {
                return node;
            }
            return new NetworkNodeResponse(
                    node.id(),
                    node.label(),
                    node.type(),
                    node.x(),
                    node.y(),
                    node.elevationM(),
                    result.pressureM(),
                    result.lowPressure(),
                    node.sourceX(),
                    node.sourceY(),
                    node.layoutGenerated()
            );
        }).toList();


        var edges = baseGraph.edges().stream()
                .map(edge -> {
                    var result = edgeResults.get(edge.id());
                    if (result == null) {
                        return edge;
                    }
                    return new NetworkEdgeResponse(
                            edge.id(),
                            edge.sourceId(),
                            edge.targetId(),
                            edge.type(),
                            edge.status(),
                            result.flowRateLps(),
                            result.velocityMps(),
                            edge.lengthM(),
                            edge.diameterId(),
                            edge.nominalSize(),
                            edge.sdr(),
                            edge.innerDiameter(),
                            edge.cost(),
                            edge.pmax(),
                            edge.pipeTypeCode(),
                            edge.roughness()
                    );
                }).toList();
        return new NetworkGraphResponse(
                baseGraph.id(),
                baseGraph.name(),
                nodes,
                edges
        );
    }

    private List<SimulationEdgeOverrideRequest> buildCurrentEdgeOverrides(ProjectEntity project) {
        return project.getEdges().stream().map(edge ->
                        new SimulationEdgeOverrideRequest(edge.getExternalId(),
                                edge.getStatus() == EdgeStatus.OPEN)
                ).toList();
    }
}