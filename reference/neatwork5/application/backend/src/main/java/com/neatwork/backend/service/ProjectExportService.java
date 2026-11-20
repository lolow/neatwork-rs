package com.neatwork.backend.service;

import tools.jackson.databind.ObjectMapper;

import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.enums.ProjectExportFormat;

import com.neatwork.backend.exception.ResourceNotFoundException;

import com.neatwork.backend.response.ProjectArtifactResponse;
import com.neatwork.backend.response.ProjectExportAvailabilityResponse;

import lombok.RequiredArgsConstructor;

import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.List;
import java.util.UUID;


@Service
@RequiredArgsConstructor
public class ProjectExportService {

    private final SimulationRunService simulationRunService;

    private final ProjectArtifactService artifactService;

    private final PythonSimulationClient pythonSimulationClient;

    private final ObjectMapper objectMapper;

    /**
     * Return the export formats currently usable for one project.
     */
    @Transactional(readOnly = true)
    public List<ProjectExportAvailabilityResponse> getAvailability(UUID projectId) {
        boolean hasSimulation;
        try {simulationRunService.latest(projectId);
            hasSimulation = true;
        } catch (ResourceNotFoundException exception) {
            hasSimulation = false;
        }
        boolean hasDsg = artifactService.hasArtifactType(projectId, ProjectArtifactType.DSG_OUTPUT)
                        ||
                        artifactService.hasArtifactType(projectId, ProjectArtifactType.DSG_INPUT);


        boolean hasInp = artifactService.hasArtifactType(projectId, ProjectArtifactType.INP_OUTPUT);
        return List.of(new ProjectExportAvailabilityResponse(ProjectExportFormat.SIMULATION_JSON,
                        hasSimulation, hasSimulation
                                ? null : "Run a simulation first."
                ),

                new ProjectExportAvailabilityResponse(ProjectExportFormat.SIMULATION_XLSX,
                        hasSimulation, hasSimulation
                                ? null : "Run a simulation first."
                ),

                new ProjectExportAvailabilityResponse(ProjectExportFormat.DSG,
                        hasDsg, hasDsg ? null : "No DSG artifact exists for this project."
                ),

                new ProjectExportAvailabilityResponse(ProjectExportFormat.INP,
                        hasInp, hasInp ? null : "No EPANET INP artifact exists for this project."
                )
        );
    }

    /**
     * Export one project in the requested representation.
     */
    @Transactional
    public ProjectArtifactResponse export(UUID projectId, ProjectExportFormat format) {

        return switch (format) {
            case SIMULATION_JSON -> exportJson(projectId);
            case SIMULATION_XLSX -> exportSpreadsheet(projectId);
            case DSG -> latestDsg(projectId);
            case INP -> latestInp(projectId);
        };
    }

    private ProjectArtifactResponse latestDsg(UUID projectId) {
        if (artifactService.hasArtifactType(projectId, ProjectArtifactType.DSG_OUTPUT)) {
            return artifactService.latestArtifactOfType(projectId, ProjectArtifactType.DSG_OUTPUT);
        }
        return artifactService.latestArtifactOfType(projectId, ProjectArtifactType.DSG_INPUT);
    }

    private ProjectArtifactResponse exportJson(UUID projectId) {
        var run = simulationRunService.latest(projectId);
        try {
            byte[] content = objectMapper.writerWithDefaultPrettyPrinter()
                    .writeValueAsBytes(run.getStatisticsJson());

            return artifactService.registerGeneratedArtifact(projectId,
                            ProjectArtifactType.SIMULATION_JSON,
                            "simulation_statistics.json", content
                    );
        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not serialize simulation JSON.", exception);
        }
    }

    private ProjectArtifactResponse exportSpreadsheet(UUID projectId) {
        var run = simulationRunService.latest(projectId);
        var content = pythonSimulationClient.exportSpreadsheet(run.getStatisticsJson());


        return artifactService.registerGeneratedArtifact(
                        projectId,
                        ProjectArtifactType.SIMULATION_XLSX,
                        "simulation_statistics.xlsx",
                        content
                );
    }

    private ProjectArtifactResponse latestInp(UUID projectId) {

        return artifactService.latestArtifactOfType(projectId, ProjectArtifactType.INP_OUTPUT);
    }
}