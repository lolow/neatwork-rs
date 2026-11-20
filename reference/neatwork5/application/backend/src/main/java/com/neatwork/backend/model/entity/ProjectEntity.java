package com.neatwork.backend.model.entity;

import com.neatwork.backend.enums.ProjectDisplayLengthUnit;
import com.neatwork.backend.enums.ProjectSourceFormat;
import jakarta.persistence.*;
import lombok.*;

import java.time.Instant;
import java.util.*;

@Entity
@Table(name = "projects")
@Getter
@Setter
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ProjectEntity {

    @Id
    @GeneratedValue(strategy = GenerationType.UUID)
    private UUID id;

    @Column(nullable = false, length = 160)
    private String name;

    // Stores the active network input format.
    // The initial value is NONE. The format is assigned after the first successful network import.
    @Builder.Default
    @Enumerated(EnumType.STRING)
    @Column(name = "source_format", nullable = false, length = 20)
    private ProjectSourceFormat sourceFormat = ProjectSourceFormat.NONE;

    // Controls how hydraulic length-like values are displayed throughout this project.
    // Stored numerical values remain in canonical units.
    @Builder.Default
    @Enumerated(EnumType.STRING)
    @Column(name = "display_length_unit", nullable = false, length = 16)
    private ProjectDisplayLengthUnit displayLengthUnit = ProjectDisplayLengthUnit.METER;

    @Column(nullable = false)
    private Instant createdAt;

    @Builder.Default
    @OneToMany(mappedBy = "project", cascade = CascadeType.ALL, orphanRemoval = true)
    private Set<NetworkNodeEntity> nodes = new HashSet<>();

    @Builder.Default
    @OneToMany(mappedBy = "project", cascade = CascadeType.ALL, orphanRemoval = true)
    private Set<NetworkEdgeEntity> edges = new HashSet<>();

    @PrePersist
    void onCreate() {
        if (createdAt == null) {
            createdAt = Instant.now();
        }
    }
}