package com.neatwork.backend.dao;

import com.neatwork.backend.model.entity.NetworkEdgeEntity;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.Optional;
import java.util.UUID;

@Repository
public interface NetworkEdgeRepository extends JpaRepository<NetworkEdgeEntity, UUID> {

    Optional<NetworkEdgeEntity> findByProjectIdAndExternalId(UUID projectId, String externalId);
}
