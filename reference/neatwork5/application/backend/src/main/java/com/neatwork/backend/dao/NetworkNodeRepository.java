package com.neatwork.backend.dao;

import com.neatwork.backend.model.entity.NetworkNodeEntity;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.Optional;
import java.util.UUID;

@Repository
public interface NetworkNodeRepository extends JpaRepository<NetworkNodeEntity, UUID> {

    Optional<NetworkNodeEntity> findByProjectIdAndExternalId(UUID projectId, String externalId);
}