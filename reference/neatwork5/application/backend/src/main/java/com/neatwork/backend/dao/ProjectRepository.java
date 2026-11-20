package com.neatwork.backend.dao;

import com.neatwork.backend.model.entity.ProjectEntity;
import jakarta.persistence.LockModeType;
import org.springframework.data.jpa.repository.EntityGraph;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Lock;
import org.springframework.data.jpa.repository.Query;
import org.springframework.data.repository.query.Param;
import org.springframework.stereotype.Repository;

import java.util.List;
import java.util.Optional;
import java.util.UUID;

@Repository
public interface ProjectRepository extends JpaRepository<ProjectEntity, UUID> {

    List<ProjectEntity> findAllByOrderByCreatedAtDesc();

    @EntityGraph(attributePaths = {"nodes", "edges"})
    Optional<ProjectEntity> findWithNodesAndEdgesById(UUID id);

    /**
     * Locks a project while its input format is being
     * validated or assigned.
     *
     * This prevents concurrent requests from importing
     * incompatible network formats.
     */
    @Lock(LockModeType.PESSIMISTIC_WRITE)
    @Query("""
    SELECT p
    FROM ProjectEntity p
    WHERE p.id = :projectId
    """)
    Optional<ProjectEntity> findByIdForUpdate(@Param("projectId") UUID projectId);
}