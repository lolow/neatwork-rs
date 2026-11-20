package com.neatwork.backend.service;

import com.neatwork.backend.config.PythonCoreProperties;
import com.neatwork.backend.response.PythonCoreHealthResponse;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;
import org.springframework.web.client.RestClient;
import org.springframework.web.client.RestClientException;

import java.time.Instant;

@Service
@RequiredArgsConstructor
public class PythonCoreClient {

    private final PythonCoreProperties properties;

    public PythonCoreHealthResponse getHealth() {
        try {
            return RestClient.create(properties.baseUrl())
                    .get()
                    .uri("/api/health")
                    .retrieve()
                    .body(PythonCoreHealthResponse.class);
        } catch (RestClientException exception) {
            return new PythonCoreHealthResponse(
                    "neatwork-python-core",
                    "DOWN",
                    Instant.now().toString()
            );
        }
    }
}
