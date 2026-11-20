package com.neatwork.backend.controller;

import com.neatwork.backend.response.NetworkGraphResponse;
import com.neatwork.backend.service.NetworkQueryService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api/network")
@RequiredArgsConstructor
public class NetworkController {

    private final NetworkQueryService networkQueryService;

    @GetMapping("/demo/graph")
    public NetworkGraphResponse getDemoGraph() {
        return networkQueryService.loadDemoGraph();
    }
}
