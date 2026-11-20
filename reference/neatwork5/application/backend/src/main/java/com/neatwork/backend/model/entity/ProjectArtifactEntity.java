package com.neatwork.backend.model.entity;

import com.neatwork.backend.enums.ProjectArtifactType;
import jakarta.persistence.*;
import lombok.*;

import java.time.Instant;
import java.util.UUID;

@Entity
@Table(name = "project_artifacts")
@Getter
@Setter
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ProjectArtifactEntity {

    @Id
    @GeneratedValue(strategy = GenerationType.UUID)
    private UUID id;

    @Column(nullable = false)
    private UUID projectId;

    @Enumerated(EnumType.STRING)
    @Column(nullable = false, length = 60)
    private ProjectArtifactType type;

    @Column(nullable = false, length = 260)
    private String originalFilename;

    @Column(nullable = false, length = 260)
    private String storedFilename;

    @Column(nullable = false, length = 1024)
    private String relativePath;

    @Column(nullable = false)
    private long sizeBytes;

    @Column(nullable = false)
    private int version;

    @Column(nullable = false)
    private boolean generated;

    /**
     * Records when the artifact was removed from the active project library.
     */
    @Column(name = "archived_at")
    private Instant archivedAt;

    /**
     * References the previous artifact revision.
     * This field will be populated when the explicit versioning workflow is implemented.
     */
    @Column(name = "parent_artifact_id")
    private UUID parentArtifactId;

    /**
     * Identifies all revisions that belong to the same logical input file.
     * The root artifact uses its own identifier as the revision family identifier.
     */
    @Column(name = "revision_family_id")
    private UUID revisionFamilyId;

    @Column(nullable = false)
    private Instant createdAt;

    /**
     * Indicates whether an artifact has been removed from the active project library.
     * Archived artifacts remain available internally to preserve historical domain provenance.
     */
    @Builder.Default
    @Column(nullable = false)
    private boolean archived = false;

    @PrePersist
    void onCreate() {
        if (createdAt == null) {
            createdAt = Instant.now();
        }
    }
}