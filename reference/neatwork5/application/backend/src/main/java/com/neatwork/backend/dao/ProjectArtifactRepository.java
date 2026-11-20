package com.neatwork.backend.dao;

import com.neatwork.backend.enums.ProjectArtifactType;
import com.neatwork.backend.model.entity.ProjectArtifactEntity;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.List;
import java.util.Optional;
import java.util.UUID;

public interface ProjectArtifactRepository extends JpaRepository<ProjectArtifactEntity, UUID> {

    List<ProjectArtifactEntity> findByProjectIdOrderByCreatedAtDesc(UUID projectId);

    // this query allows us to get for example all: DSG_INPUTs across the entire database
    // Returns active, non-generated artifacts of one type.
    List<ProjectArtifactEntity> findByTypeAndGeneratedFalseAndArchivedFalseOrderByCreatedAtDesc(ProjectArtifactType type);

    int countByProjectIdAndType(UUID projectId, ProjectArtifactType type);

    /**
     * Delete every artifact metadata row owned by one project.
     *
     * @param projectId owning project identifier
     */
    void deleteAllByProjectId(UUID projectId);

    Optional<ProjectArtifactEntity> findFirstByProjectIdAndTypeOrderByCreatedAtDesc(UUID projectId, ProjectArtifactType type);

    /**
     * Checks whether a project already contains an input artifact of the specified type.
     */
    boolean existsByProjectIdAndType(UUID projectId, ProjectArtifactType type);

    /**
     * Returns only active artifacts belonging to a project.
     */
    List<ProjectArtifactEntity>
    findByProjectIdAndArchivedFalseOrderByCreatedAtDesc(UUID projectId);

    /**
     * Returns the latest active artifact of a specific type.
     */
    Optional<ProjectArtifactEntity>
    findFirstByProjectIdAndTypeAndArchivedFalseOrderByCreatedAtDesc(UUID projectId, ProjectArtifactType type);

    /**
     * Checks whether an active artifact of the requested type exists inside a project.
     */
    boolean existsByProjectIdAndTypeAndArchivedFalse(UUID projectId, ProjectArtifactType type);

    /**
     * Finds one artifact and verifies its project ownership.
     */
    Optional<ProjectArtifactEntity>
    findByIdAndProjectId(UUID artifactId, UUID projectId);

    /**
     * Returns the newest revision from one file family.
     */
    Optional<ProjectArtifactEntity>
    findFirstByProjectIdAndRevisionFamilyIdOrderByVersionDesc(UUID projectId, UUID revisionFamilyId);

}