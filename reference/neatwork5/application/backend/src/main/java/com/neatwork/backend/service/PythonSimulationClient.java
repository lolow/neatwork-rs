package com.neatwork.backend.service;

import com.neatwork.backend.config.PythonCoreProperties;
import com.neatwork.backend.exception.PythonSimulationException;
import com.neatwork.backend.request.SimulationRunRequest;
import com.neatwork.backend.response.SimulationRunResponse;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;
import tools.jackson.databind.ObjectMapper;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.Map;

@Service
@Slf4j
public class PythonSimulationClient {

    private static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);

    private static final Duration SIMULATION_TIMEOUT = Duration.ofMinutes(10);

    private final PythonCoreProperties properties;
    private final ObjectMapper objectMapper;

    private final HttpClient httpClient;

    /**
     * Create a Python simulation client using HTTP/1.1.
     *
     * @param properties Python Core connection properties
     * @param objectMapper Spring JSON serializer
     */
    public PythonSimulationClient(PythonCoreProperties properties, ObjectMapper objectMapper) {
        this.properties = properties;
        this.objectMapper = objectMapper;
        this.httpClient = HttpClient
                        .newBuilder()
                        .version(HttpClient.Version.HTTP_1_1)
                        .connectTimeout(CONNECT_TIMEOUT)
                        .build();
    }


    /**
     * Execute the complete hydraulic simulation pipeline.
     *
     * @param request imported domain, runtime options and edge overrides
     * @return aggregate graph values and statistics
     */
    public SimulationRunResponse runSimulation(SimulationRunRequest request) {
        var response = sendJson("/api/simulation/run", request,
                        HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8)
                );

        try {
            return objectMapper.readValue(
                            response.body(),
                            SimulationRunResponse.class
                    );

        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not deserialize the Python simulation response.", exception);
        }
    }

    /**
     * Execute the simulation and return its JSON export bytes.
     *
     * @param request imported domain, runtime options and edge overrides
     * @return strict JSON statistics document
     */
    public byte[] exportStatistics(SimulationRunRequest request) {
        var response = sendJson("/api/simulation/export", request,
                        HttpResponse.BodyHandlers.ofByteArray()
                );

        return response.body();
    }

    /**
     * Serialize and send one request to a Python simulation endpoint.
     */
    private <T> HttpResponse<T> sendJson(String path, Object request, HttpResponse.BodyHandler<T> bodyHandler) {
        try {
            var jsonBody = objectMapper.writeValueAsString(request);

            var endpoint = URI.create(properties.baseUrl()
                                    .replaceAll("/+$", "") + path
                    );

            log.info("Sending Python simulation request: endpoint={}, jsonLength={}, httpVersion=HTTP_1_1", endpoint, jsonBody.length());

            var httpRequest = HttpRequest.newBuilder(endpoint)
                            .version(HttpClient.Version.HTTP_1_1)
                            .timeout(SIMULATION_TIMEOUT)
                            .header("Content-Type", "application/json; charset=UTF-8")
                            .header("Accept", "application/json")
                            .POST(HttpRequest.BodyPublishers.ofString(jsonBody, StandardCharsets.UTF_8))
                            .build();

            var response = httpClient.send(httpRequest, bodyHandler);

            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                var responseBody = bodyToErrorText(response.body());

                throw new PythonSimulationException(response.statusCode(), responseBody, null);
            }
            return response;

        } catch (PythonSimulationException exception) {
            throw exception;
        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
            throw new IllegalArgumentException("Python simulation request was interrupted.", exception);
        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not send the Python simulation request.", exception);
        }
    }

    /**
     * Convert a textual or binary error response to readable text.
     */
    private String bodyToErrorText(Object body) {
        if (body == null) {return "";}

        if (body instanceof byte[] bytes) {
            return new String(bytes, StandardCharsets.UTF_8);
        }

        return String.valueOf(body);
    }

    /**
     * Request cooperative cancellation of one Python execution.
     */
    public void cancelExecution(String executionId) {
        try {var endpoint = URI.create(properties
                                    .baseUrl()
                                    .replaceAll("/+$", "")
                                    + "/api/simulation/executions/"
                                    + executionId
                                    + "/cancel"
                    );

            var request = HttpRequest.newBuilder(endpoint)
                            .version(HttpClient.Version.HTTP_1_1)
                            .timeout(Duration.ofSeconds(10))
                            .header("Accept", "application/json")
                            .POST(HttpRequest.BodyPublishers.noBody())
                            .build();

            var response = httpClient.send(request, HttpResponse.BodyHandlers.ofString());


            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                log.warn("Python cancellation returned HTTP {}.", response.statusCode());
            }
        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
        } catch (Exception exception) {
            log.warn("Could not request Python simulation cancellation.", exception);
        }
    }

    /**
     * Convert persisted simulation statistics into an XLSX workbook.
     */
    public byte[] exportSpreadsheet(Map<String, Object> statistics) {

        var request = Map.of("statistics", statistics);
        var response = sendJson(
                        "/api/simulation/exports/xlsx",
                        request, HttpResponse.BodyHandlers.ofByteArray()
                );

        return response.body();
    }
}

