
package com.neatwork.backend.service;

import com.neatwork.backend.dao.ProjectArtifactRepository;
import com.neatwork.backend.dao.ProjectRepository;
import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.enums.ProjectSourceFormat;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.model.entity.ProjectEntity;

import lombok.RequiredArgsConstructor;

import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Propagation;
import org.springframework.transaction.annotation.Transactional;

import java.util.UUID;

@Service
@RequiredArgsConstructor
public class ProjectSourcePolicy {

    private final ProjectRepository projectRepository;
    private final ProjectArtifactRepository artifactRepository;

    /**
     * Validates a network input before it is added to a project.
     *
     * The first successfully imported domain determines the permanent source format. Before that happens, uploaded artifacts must still not mix DSG and TPO.
     * This method must be called inside a write transaction.
     */
    @Transactional(propagation = Propagation.MANDATORY)
    public void assertCanStage(UUID projectId, ProjectArtifactType artifactType) {
        ProjectSourceFormat requested = toSourceFormat(artifactType);

        if (requested == ProjectSourceFormat.NONE) {
            return;
        }
        var project = lockProject(projectId);
        assertCompatible(project, requested);
        if (project.getSourceFormat() == ProjectSourceFormat.NONE) {

            var oppositeType = requested == ProjectSourceFormat.DSG
                            ? ProjectArtifactType.TPO_INPUT
                            : ProjectArtifactType.DSG_INPUT;

            if (artifactRepository.existsByProjectIdAndTypeAndArchivedFalse(projectId, oppositeType)) {
                throw new IllegalArgumentException("This project already contains an input " + "file of the opposite format.");
            }
        }
    }

    /**
     * Validates and locks the active project before importing its primary network source.
     *
     * The caller must assign the source format only after parsing and graph persistence have succeeded.
     */
    @Transactional(propagation = Propagation.MANDATORY)
    public ProjectEntity lockForImport(UUID projectId, ProjectSourceFormat requested) {
        if (requested != ProjectSourceFormat.DSG && requested != ProjectSourceFormat.TPO) {
            throw new IllegalArgumentException("Only DSG and TPO are supported.");
        }
        var project = lockProject(projectId);
        assertCompatible(project, requested);
        if (project.getSourceFormat() == ProjectSourceFormat.NONE) {

            var oppositeType = requested == ProjectSourceFormat.DSG
                            ? ProjectArtifactType.TPO_INPUT
                            : ProjectArtifactType.DSG_INPUT;

            if (artifactRepository.existsByProjectIdAndTypeAndArchivedFalse(projectId, oppositeType)) {
                throw new IllegalArgumentException("Remove the conflicting staged input " + "before importing this network.");
            }
        }

        return project;
    }

    private ProjectEntity lockProject(UUID projectId) {
        return projectRepository.findByIdForUpdate(projectId).orElseThrow(() ->
                        new ResourceNotFoundException("Project not found: " + projectId));
    }

    private void assertCompatible(ProjectEntity project, ProjectSourceFormat requested) {
        var current = project.getSourceFormat();

        if (current == ProjectSourceFormat.REVIEW_REQUIRED) {
            throw new IllegalArgumentException("This legacy project requires a source " + "format review before importing.");
        }

        if (current != ProjectSourceFormat.NONE && current != requested) {
            throw new IllegalArgumentException("This project uses " + current + ". Importing " + requested + " is not allowed.");
        }
    }

    private ProjectSourceFormat toSourceFormat(ProjectArtifactType type) {
        return switch (type) {
            case DSG_INPUT -> ProjectSourceFormat.DSG;
            case TPO_INPUT -> ProjectSourceFormat.TPO;
            default -> ProjectSourceFormat.NONE;
        };
    }
}