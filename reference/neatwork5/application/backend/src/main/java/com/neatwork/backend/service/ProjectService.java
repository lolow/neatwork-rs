package com.neatwork.backend.service;

import com.neatwork.backend.dao.ProjectArtifactRepository;
import com.neatwork.backend.dao.ProjectDomainSnapshotRepository;
import com.neatwork.backend.dao.ProjectRepository;
import com.neatwork.backend.dao.SimulationRunRepository;
import com.neatwork.backend.enums.ProjectDisplayLengthUnit;
import com.neatwork.backend.enums.ProjectSourceFormat;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.model.entity.ProjectEntity;
import com.neatwork.backend.model.mapper.NetworkGraphMapper;
import com.neatwork.backend.request.CreateProjectRequest;
import com.neatwork.backend.response.NetworkGraphResponse;
import com.neatwork.backend.response.ProjectSummaryResponse;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.List;
import java.util.UUID;

@Service
@RequiredArgsConstructor
public class ProjectService {

    private final ProjectRepository projectRepository;
    private final ProjectArtifactRepository artifactRepository;
    private final ProjectDomainSnapshotRepository snapshotRepository;
    private final SimulationRunRepository simulationRunRepository;
    private final ProjectArtifactService projectArtifactService;
    private final NetworkGraphMapper mapper;

    @Transactional(readOnly = true)
    public List<ProjectSummaryResponse> listProjects() {
        return projectRepository.findAllByOrderByCreatedAtDesc()
                .stream()
                .map(mapper::toSummary)
                .toList();
    }

    @Transactional(readOnly = true)
    public NetworkGraphResponse getProjectGraph(UUID projectId) {
        var project = projectRepository.findWithNodesAndEdgesById(projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Project not found: " + projectId));

        return mapper.toGraph(project);
    }

    /**
     * Creates an empty project without demo network data.
     * The project source format remains NONE until the first successful DSG or TPO import.
     * @param request project creation data
     * @return summary of the newly created project
     */
    @Transactional
    public ProjectSummaryResponse createProject(CreateProjectRequest request) {
        var project = ProjectEntity.builder()
                .name(request.name().trim())
                .build();
        var savedProject = projectRepository.save(project);

        return mapper.toSummary(savedProject);
    }

    /**
     * Delete one project and all project-owned persistent data.
     * <p>Network nodes and edges are removed through the JPA cascade configured
     * on {@link ProjectEntity}. Artifact metadata and domain snapshots are
     * removed explicitly because they store only the project UUID.</p>
     * @param projectId identifier of the project to remove
     */
    @Transactional
    public void deleteProject(UUID projectId) {
        var project = projectRepository.findWithNodesAndEdgesById(projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Project not found: " + projectId));

        /*
         * Simulation history references project and snapshot UUIDs directly,
         * so it must be removed before project-owned snapshots disappear.
         */
        simulationRunRepository.deleteAllByProjectId(projectId);
        artifactRepository.deleteAllByProjectId(projectId);

        snapshotRepository.deleteAllByProjectId(projectId);

        projectRepository.delete(project);

        projectArtifactService.deleteProjectWorkspace(projectId);
    }

    /**
     * Updates the display-unit preference of one project.
     * This operation changes presentation metadata only.
     * Hydraulic values stored in the database are not converted or rewritten.
     */
    @Transactional
    public ProjectSummaryResponse updateDisplayUnit(UUID projectId, ProjectDisplayLengthUnit displayUnit) {

        var project = projectRepository.findByIdForUpdate(projectId)
                        .orElseThrow(() ->
                                new ResourceNotFoundException("Project not found: " + projectId));

        project.setDisplayLengthUnit(displayUnit);

        return mapper.toSummary(project);
    }
}