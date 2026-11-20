package com.neatwork.backend.model.entity;

import com.neatwork.backend.enums.NodeType;
import jakarta.persistence.*;
import lombok.*;
import org.hibernate.annotations.ColumnDefault;

import java.util.UUID;

@Entity
@Table(name = "network_nodes")
@Getter
@Setter
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class NetworkNodeEntity {

    @Id
    @GeneratedValue(strategy = GenerationType.UUID)
    private UUID id;

    @Column(nullable = false, length = 80)
    private String externalId;

    @Column(nullable = false, length = 120)
    private String label;

    @Enumerated(EnumType.STRING)
    @Column(nullable = false, length = 40)
    private NodeType type;

    @Column(nullable = false)
    private double x;

    @Column(nullable = false)
    private double y;

    @Column(nullable = false)
    private double pressureM;

    @Column(nullable = false)
    private boolean lowPressure;

    @Column(name = "elevation_m", nullable = false, columnDefinition = "FLOAT(53) DEFAULT 0")
    private double elevationM;

    @Column(name = "source_x")
    private Double sourceX;

    @Column(name = "source_y")
    private Double sourceY;

    @Column(name = "layout_generated", nullable = false, columnDefinition = "boolean default false")
    @ColumnDefault("false")
    private boolean layoutGenerated;

    @ManyToOne(fetch = FetchType.LAZY, optional = false)
    @JoinColumn(name = "project_id")
    private ProjectEntity project;
}