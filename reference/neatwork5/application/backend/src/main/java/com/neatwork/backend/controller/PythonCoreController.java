package com.neatwork.backend.controller;

import com.neatwork.backend.response.PythonCoreHealthResponse;
import com.neatwork.backend.service.PythonCoreClient;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api/python-core")
@RequiredArgsConstructor
public class PythonCoreController {

    private final PythonCoreClient pythonCoreClient;

    @GetMapping("/health")
    public PythonCoreHealthResponse health() {
        return pythonCoreClient.getHealth();
    }
}
