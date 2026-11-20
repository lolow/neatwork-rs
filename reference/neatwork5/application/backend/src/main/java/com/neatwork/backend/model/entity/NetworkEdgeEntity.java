package com.neatwork.backend.model.entity;

import com.neatwork.backend.enums.EdgeStatus;
import com.neatwork.backend.enums.EdgeType;
import jakarta.persistence.*;
import lombok.*;

import java.util.UUID;

@Entity
@Table(name = "network_edges")
@Getter
@Setter
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class NetworkEdgeEntity {

    @Id
    @GeneratedValue(strategy = GenerationType.UUID)
    private UUID id;

    @Column(nullable = false, length = 80)
    private String externalId;

    @Column(nullable = false, length = 80)
    private String sourceExternalId;

    @Column(nullable = false, length = 80)
    private String targetExternalId;

    @Enumerated(EnumType.STRING)
    @Column(nullable = false, length = 40)
    private EdgeType type;

    @Enumerated(EnumType.STRING)
    @Column(nullable = false, length = 40)
    private EdgeStatus status;

    @Column(nullable = false)
    private double flowRateLps;

    @Column(nullable = false)
    private double velocityMps;

    @Column(name = "length_m")
    private Double lengthM;

    @Column(name = "diameter_id", length = 80)
    private String diameterId;

    @Column(name = "nominal_size", length = 80)
    private String nominalSize;

    @Column(name = "sdr")
    private Double sdr;

    @Column(name = "inner_diameter")
    private Double innerDiameter;

    @Column(name = "cost")
    private Double cost;

    @Column(name = "pmax")
    private Double pmax;

    @Column(name = "pipe_type_code", length = 80)
    private String pipeTypeCode;

    @Column(name = "roughness")
    private Double roughness;

    @ManyToOne(fetch = FetchType.LAZY, optional = false)
    @JoinColumn(name = "project_id")
    private ProjectEntity project;
}