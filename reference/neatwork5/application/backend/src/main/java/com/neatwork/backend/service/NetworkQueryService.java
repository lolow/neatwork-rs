package com.neatwork.backend.service;

import com.neatwork.backend.response.NetworkGraphResponse;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;

@Service
@RequiredArgsConstructor
public class NetworkQueryService {

    private final ProjectService projectService;

    public NetworkGraphResponse loadDemoGraph() {
        var projects = projectService.listProjects();

        if (projects.isEmpty()) {
            throw new IllegalStateException("No demo project available.");
        }

        return projectService.getProjectGraph(projects.get(0).id());
    }
}