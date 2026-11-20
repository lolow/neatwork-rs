package com.neatwork.backend.model.mapper;

import com.neatwork.backend.model.entity.NetworkEdgeEntity;
import com.neatwork.backend.model.entity.NetworkNodeEntity;
import com.neatwork.backend.model.entity.ProjectEntity;
import com.neatwork.backend.response.NetworkEdgeResponse;
import com.neatwork.backend.response.NetworkGraphResponse;
import com.neatwork.backend.response.NetworkNodeResponse;
import com.neatwork.backend.response.ProjectSummaryResponse;
import org.springframework.stereotype.Component;

@Component
public class NetworkGraphMapper {

    public ProjectSummaryResponse toSummary(ProjectEntity project) {
        return new ProjectSummaryResponse(
                project.getId(),
                project.getName(),
                project.getCreatedAt(),
                project.getSourceFormat(),
                project.getDisplayLengthUnit()
        );
    }

    public NetworkGraphResponse toGraph(ProjectEntity project) {
        return new NetworkGraphResponse(
                project.getId().toString(),
                project.getName(),
                project.getNodes().stream().map(this::toNode).toList(),
                project.getEdges().stream().map(this::toEdge).toList()
        );
    }

    public NetworkNodeResponse toNode(NetworkNodeEntity node) {
        return new NetworkNodeResponse(
                node.getExternalId(),
                node.getLabel(),
                node.getType(),
                node.getX(),
                node.getY(),
                node.getElevationM(),
                node.getPressureM(),
                node.isLowPressure(),
                node.getSourceX(),
                node.getSourceY(),
                node.isLayoutGenerated()
        );
    }

    public NetworkEdgeResponse toEdge(NetworkEdgeEntity edge) {
        return new NetworkEdgeResponse(
                edge.getExternalId(),
                edge.getSourceExternalId(),
                edge.getTargetExternalId(),
                edge.getType(),
                edge.getStatus(),
                edge.getFlowRateLps(),
                edge.getVelocityMps(),

                edge.getLengthM(),
                edge.getDiameterId(),
                edge.getNominalSize(),
                edge.getSdr(),
                edge.getInnerDiameter(),
                edge.getCost(),
                edge.getPmax(),
                edge.getPipeTypeCode(),
                edge.getRoughness()
        );
    }


}
