package com.neatwork.backend.dao;

import com.neatwork.backend.enums.SimulationJobStatus;
import com.neatwork.backend.model.entity.SimulationRunEntity;

import jakarta.persistence.LockModeType;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Lock;
import org.springframework.data.jpa.repository.Query;
import org.springframework.data.repository.query.Param;

import java.util.Collection;
import java.util.List;
import java.util.Optional;
import java.util.UUID;


public interface SimulationRunRepository extends JpaRepository<SimulationRunEntity, UUID> {

    /**
     * Return one simulation only when it belongs to the requested project.
     * This prevents a run ID from another project from being accessed through the current project's URL.
     */
    Optional<SimulationRunEntity>
    findByIdAndProjectId(UUID id, UUID projectId);

    // Lock one execution before a lifecycle transition.
    @Lock(LockModeType.PESSIMISTIC_WRITE)
    @Query("""
        SELECT run
        FROM SimulationRunEntity run
        WHERE run.id = :runId
        """)
    Optional<SimulationRunEntity>
    findByIdForUpdate(@Param("runId") UUID runId);

     // Lock one project-owned execution before cancellation or another state mutation.
    @Lock(LockModeType.PESSIMISTIC_WRITE)
    @Query("""
        SELECT run
        FROM SimulationRunEntity run
        WHERE run.id = :runId
          AND run.projectId = :projectId
        """)
    Optional<SimulationRunEntity>
    findByIdAndProjectIdForUpdate(@Param("runId") UUID runId, @Param("projectId") UUID projectId);

    /**
     * Return completed simulation history.
     * Pending, failed and cancelled attempts remain persisted but are not yet part of comparison history.
     */
    List<SimulationRunEntity>
    findByProjectIdAndStatusInOrderByCreatedAtAscIdAsc(UUID projectId, Collection<SimulationJobStatus> statuses);

    // Return the latest completed result available for export.
    Optional<SimulationRunEntity>
    findFirstByProjectIdAndStatusInOrderByCompletedAtDescCreatedAtDesc(UUID projectId, Collection<SimulationJobStatus> statuses);

    // Find active executions for restart recovery or frontend monitoring.
    List<SimulationRunEntity>
    findByProjectIdAndStatusInOrderByCreatedAtAsc(UUID projectId, Collection<SimulationJobStatus> statuses);

    // Find unfinished executions after backend restart.
    List<SimulationRunEntity>
    findByStatusIn(Collection<SimulationJobStatus> statuses);

    // Delete simulation history when a project is removed.
    void deleteAllByProjectId(UUID projectId);

    /**
     * Returns the greatest simulation number already
     * allocated inside one project.
     */
    @Query("""
    SELECT MAX(run.runNumber)
    FROM SimulationRunEntity run
    WHERE run.projectId = :projectId
    """)
    Optional<Integer> findMaxRunNumberByProjectId(@Param("projectId") UUID projectId);

    // Return every persisted simulation attempt, including unfinished and failed executions.
    List<SimulationRunEntity> findByProjectIdOrderByRunNumberAsc(UUID projectId);
}