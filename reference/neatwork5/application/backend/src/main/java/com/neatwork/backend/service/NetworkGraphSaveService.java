package com.neatwork.backend.service;

import com.neatwork.backend.dao.ProjectRepository;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.model.entity.NetworkEdgeEntity;
import com.neatwork.backend.model.entity.NetworkNodeEntity;
import com.neatwork.backend.model.mapper.NetworkGraphMapper;
import com.neatwork.backend.request.NetworkGraphSaveRequest;
import com.neatwork.backend.response.NetworkGraphResponse;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.UUID;

@Service
@RequiredArgsConstructor
public class NetworkGraphSaveService {

    private final ProjectRepository projectRepository;
    private final NetworkGraphMapper mapper;

    @Transactional
    public NetworkGraphResponse replaceGraph(UUID projectId, NetworkGraphSaveRequest request) {
        var project = projectRepository.findWithNodesAndEdgesById(projectId)
                .orElseThrow(() -> new ResourceNotFoundException("Project not found: " + projectId));

        project.getNodes().clear();
        project.getEdges().clear();

        request.nodes().forEach(node -> project.getNodes().add(
                NetworkNodeEntity.builder()
                        .project(project)
                        .externalId(node.id())
                        .label(node.label())
                        .type(node.type())
                        .x(node.x())
                        .y(node.y())
                        .elevationM(node.elevationM())
                        .pressureM(node.pressureM())
                        .lowPressure(node.lowPressure())
                        .sourceX(node.sourceX())
                        .sourceY(node.sourceY())
                        .layoutGenerated(Boolean.TRUE.equals(node.layoutGenerated()))
                        .build()
        ));

        request.edges().forEach(edge -> project.getEdges().add(
                NetworkEdgeEntity.builder()
                        .project(project)
                        .externalId(edge.id())
                        .sourceExternalId(edge.sourceId())
                        .targetExternalId(edge.targetId())
                        .type(edge.type())
                        .status(edge.status())
                        .flowRateLps(edge.flowRateLps())
                        .velocityMps(edge.velocityMps())
                        .lengthM(edge.lengthM())
                        .diameterId(edge.diameterId())
                        .nominalSize(edge.nominalSize())
                        .sdr(edge.sdr())
                        .innerDiameter(edge.innerDiameter())
                        .cost(edge.cost())
                        .pmax(edge.pmax())
                        .pipeTypeCode(edge.pipeTypeCode())
                        .roughness(edge.roughness())
                        .build()
        ));

        return mapper.toGraph(projectRepository.save(project));
    }
}