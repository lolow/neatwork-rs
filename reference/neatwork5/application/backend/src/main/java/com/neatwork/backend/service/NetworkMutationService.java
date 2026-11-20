package com.neatwork.backend.service;

import com.neatwork.backend.dao.NetworkEdgeRepository;
import com.neatwork.backend.dao.NetworkNodeRepository;
import com.neatwork.backend.enums.EdgeStatus;
import com.neatwork.backend.exception.ResourceNotFoundException;
import com.neatwork.backend.model.mapper.NetworkGraphMapper;
import com.neatwork.backend.response.NetworkEdgeResponse;
import com.neatwork.backend.response.NetworkNodeResponse;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.UUID;

@Service
@RequiredArgsConstructor
public class NetworkMutationService {

    private final NetworkEdgeRepository edgeRepository;
    private final NetworkNodeRepository nodeRepository;
    private final NetworkGraphMapper mapper;

    @Transactional
    public NetworkEdgeResponse updateEdgeStatus(UUID projectId, String edgeId, EdgeStatus status) {
        var edge = edgeRepository.findByProjectIdAndExternalId(projectId, edgeId)
                .orElseThrow(() -> new ResourceNotFoundException("Edge not found: " + edgeId));

        edge.setStatus(status);

        if (status == EdgeStatus.CLOSED) {
            edge.setFlowRateLps(0.0);
            edge.setVelocityMps(0.0);
        } else if (edge.getFlowRateLps() == 0.0) {
            edge.setFlowRateLps(0.35);
            edge.setVelocityMps(0.20);
        }

        return mapper.toEdge(edge);
    }

    @Transactional
    public NetworkNodeResponse updateNodePosition(UUID projectId, String nodeId, double x, double y) {
        var node = nodeRepository.findByProjectIdAndExternalId(projectId, nodeId)
                .orElseThrow(() -> new ResourceNotFoundException("Node not found: " + nodeId));

        node.setX(x);
        node.setY(y);

        return mapper.toNode(node);
    }
}