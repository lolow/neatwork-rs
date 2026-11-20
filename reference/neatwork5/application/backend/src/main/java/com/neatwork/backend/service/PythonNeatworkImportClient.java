package com.neatwork.backend.service;

import com.neatwork.backend.config.PythonCoreProperties;
import com.neatwork.backend.request.NeatworkParseRequest;
import com.neatwork.backend.response.NeatworkParseResponse;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;
import tools.jackson.databind.ObjectMapper;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;

@Slf4j
@Service
public class PythonNeatworkImportClient {

    private static final String PARSE_PATH = "/api/neatwork/parse";
    private static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);
    private static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(60);

    private final PythonCoreProperties properties;
    private final ObjectMapper objectMapper;
    private final HttpClient httpClient;

    /**
     * Create a Python import client that always uses HTTP/1.1.
     *
     * Uvicorn does not support the clear-text HTTP/2 upgrade attempted by
     * some Java HTTP client configurations. Forcing HTTP/1.1 prevents the
     * JSON body from being interpreted as a separate invalid request.
     *
     * @param properties Python Core connection properties
     * @param objectMapper JSON serializer configured by Spring Boot
     */

    public PythonNeatworkImportClient(PythonCoreProperties properties, ObjectMapper objectMapper) {
        this.properties = properties;
        this.objectMapper = objectMapper;
        this.httpClient = HttpClient
                        .newBuilder()
                        .version(HttpClient.Version.HTTP_1_1)
                        .connectTimeout(CONNECT_TIMEOUT)
                        .build();
    }

    /**
     * Parse one uploaded NeatWork document through Python Core.
     *
     * @param request source format, primary contents and optional DB contents
     * @return parsed hydraulic domain and visual graph
     * @throws IllegalArgumentException if serialization, transport, parsing or
     *         response deserialization fails
     */
    public NeatworkParseResponse parse(NeatworkParseRequest request) {
        var jsonBody = serializeRequest(request);
        var endpoint = buildEndpoint();
        log.info("Sending Python import request: endpoint={}, jsonLength={}, httpVersion=HTTP_1_1",
                endpoint,
                jsonBody.length());

        var httpRequest = HttpRequest
                        .newBuilder(endpoint)
                        .version(HttpClient.Version.HTTP_1_1)
                        .timeout(REQUEST_TIMEOUT)
                        .header("Content-Type", "application/json; charset=UTF-8")
                        .header("Accept", "application/json")
                        .POST(HttpRequest
                                        .BodyPublishers
                                        .ofString(jsonBody, StandardCharsets.UTF_8))
                        .build();

        try {
            var response =
                    httpClient.send(httpRequest, HttpResponse
                                    .BodyHandlers
                                    .ofString(StandardCharsets.UTF_8)
                    );
            return handleResponse(response);

        } catch (
                InterruptedException exception
        ) {
            Thread.currentThread().interrupt();
            throw new IllegalArgumentException("Python Core import request was interrupted.", exception);

        } catch (
                Exception exception
        ) {
            throw new IllegalArgumentException("Could not send the import request to Python Core: " + exception.getMessage(), exception);
        }
    }

    /**
     * Serialize the complete Python import request as JSON.
     *
     * @param request request model to serialize
     * @return non-empty JSON document
     */
    private String serializeRequest(NeatworkParseRequest request) {
        try {
            var json = objectMapper.writeValueAsString(request);
            if (json == null || json.isBlank()) {
                throw new IllegalArgumentException("The serialized Python import request is empty.");
            }
            return json;
        } catch (
                IllegalArgumentException exception
        ) {
            throw exception;
        } catch (
                Exception exception
        ) {
            throw new IllegalArgumentException("Could not serialize the Python import request.", exception);
        }
    }

    /**
     * Build the absolute Python Core parsing endpoint.
     *
     * @return validated absolute endpoint URI
     */
    private URI buildEndpoint() {
        var baseUrl = properties.baseUrl().replaceAll("/+$", "");

        return URI.create(baseUrl + PARSE_PATH);
    }


    /**
     * Validate and deserialize one Python Core response.
     *
     * @param response completed HTTP response
     * @return parsed import response
     */
    private NeatworkParseResponse handleResponse(HttpResponse<String> response) {
        var statusCode = response.statusCode();
        var responseBody = response.body();

        log.info("Python import response: status={}, bodyLength={}", statusCode,
                responseBody == null
                        ? 0
                        : responseBody.length()
        );
        if (statusCode < 200 || statusCode >= 300) {
            throw new IllegalArgumentException("Python Core import failed with HTTP " + statusCode + ": " +
                    (responseBody == null ||
                                    responseBody.isBlank()
                                    ? "Empty error response."
                                    : responseBody
                    )
            );
        }
        if (responseBody == null || responseBody.isBlank()
        ) {
            throw new IllegalArgumentException("Python Core returned an empty import response.");
        }

        try {
            return objectMapper.readValue(responseBody, NeatworkParseResponse.class);
        } catch (
                Exception exception
        ) {
            throw new IllegalArgumentException("Could not deserialize the Python Core import response.", exception);
        }
    }
}
