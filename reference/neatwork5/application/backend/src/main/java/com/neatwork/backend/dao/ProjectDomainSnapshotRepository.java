package com.neatwork.backend.dao;

import com.neatwork.backend.model.entity.ProjectDomainSnapshotEntity;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.Optional;
import java.util.UUID;

@Repository
public interface ProjectDomainSnapshotRepository extends JpaRepository<ProjectDomainSnapshotEntity, UUID> {

    Optional<ProjectDomainSnapshotEntity> findFirstByProjectIdOrderByCreatedAtDesc(UUID projectId);

    /**
     * Delete every imported domain snapshot owned by one project
     * @param projectId owning project identifier
     */
    void deleteAllByProjectId(UUID projectId);

    Optional<ProjectDomainSnapshotEntity> findByIdAndProjectId(UUID id, UUID projectId);
}
