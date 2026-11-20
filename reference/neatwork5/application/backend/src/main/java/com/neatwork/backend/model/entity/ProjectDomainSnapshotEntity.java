package com.neatwork.backend.model.entity;

import jakarta.persistence.*;
import lombok.*;
import org.hibernate.annotations.JdbcTypeCode;
import org.hibernate.type.SqlTypes;

import java.time.Instant;
import java.util.Map;
import java.util.UUID;

@Entity
@Table(name = "project_domain_snapshots")
@Getter
@Setter
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ProjectDomainSnapshotEntity {

    @Id
    @GeneratedValue(strategy = GenerationType.UUID)
    private UUID id;

    @Column(nullable = false)
    private UUID projectId;

    @Column(nullable = false, length = 20)
    private String sourceFormat;

    /**
     * Exact DSG or TPO artifact used to create this immutable domain snapshot.
     */
    @Column(name = "primary_artifact_id")
    private UUID primaryArtifactId;

    /**
     * Optional diameters database artifact used during domain import.
     */
    @Column(name = "diameters_artifact_id")
    private UUID diametersArtifactId;

    /**
     * Optional orifices database artifact used during domain import.
     */
    @Column(name = "orifices_artifact_id")
    private UUID orificesArtifactId;

    @JdbcTypeCode(SqlTypes.JSON)
    @Column(nullable = false, columnDefinition = "jsonb")
    private Map<String, Object> domainJson;

    @Column(nullable = false)
    private Instant createdAt;

    @PrePersist
    void onCreate(){
        if (createdAt == null){
            createdAt = Instant.now();
        }
    }
}
