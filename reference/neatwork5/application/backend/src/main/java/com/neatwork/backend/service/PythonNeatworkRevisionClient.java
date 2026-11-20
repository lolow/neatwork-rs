package com.neatwork.backend.service;

import com.neatwork.backend.config.PythonCoreProperties;
import com.neatwork.backend.request.NeatworkRevisionSerializeRequest;
import com.neatwork.backend.response.NeatworkRevisionSerializeResponse;

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
public class PythonNeatworkRevisionClient {

    private static final String PATH = "/api/neatwork/revisions/serialize";
    private static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);
    private static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(60);

    private final PythonCoreProperties properties;
    private final ObjectMapper objectMapper;
    private final HttpClient httpClient;

    /**
     * Creates the HTTP client used to serialize edited NeatWork revisions through Python Core.
     */
    public PythonNeatworkRevisionClient(PythonCoreProperties properties, ObjectMapper objectMapper) {
        this.properties = properties;
        this.objectMapper = objectMapper;
        this.httpClient = HttpClient.newBuilder()
                .version(HttpClient.Version.HTTP_1_1)
                .connectTimeout(CONNECT_TIMEOUT)
                .build();
    }

    /**
     * Serializes an edited domain through Python Core.
     * Python Core also parses the generated document again before returning it, so only valid DSG/TPO revisions reach persistence.
     */
    public NeatworkRevisionSerializeResponse serialize(NeatworkRevisionSerializeRequest request) {
        var jsonBody = serializeRequest(request);
        var endpoint = URI.create(properties.baseUrl().replaceAll("/+$", "") + PATH);

        var httpRequest = HttpRequest
                .newBuilder(endpoint)
                .version(HttpClient.Version.HTTP_1_1)
                .timeout(REQUEST_TIMEOUT)
                .header("Content-Type", "application/json; charset=UTF-8")
                .header("Accept", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(jsonBody, StandardCharsets.UTF_8))
                .build();

        try {
            var response = httpClient.send(httpRequest,
                    HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8)
            );

            if (response.statusCode() < 200 || response.statusCode() >= 300) {
                throw new IllegalArgumentException(
                        "Python revision serialization failed with HTTP "
                                + response.statusCode() + ": " + response.body());
            }

            if (response.body() == null || response.body().isBlank()) {
                throw new IllegalArgumentException("Python Core returned an empty revision response.");
            }

            return objectMapper.readValue(response.body(), NeatworkRevisionSerializeResponse.class);

        } catch (InterruptedException exception) {
            Thread.currentThread().interrupt();
            throw new IllegalArgumentException("Revision serialization was interrupted.", exception);

        } catch (IllegalArgumentException exception) {
            throw exception;

        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not serialize NeatWork revision.", exception);
        }
    }

    /**
     * Converts the Java request into the JSON payload expected by Python Core.
     */
    private String serializeRequest(NeatworkRevisionSerializeRequest request) {
        try {
            var json = objectMapper.writeValueAsString(request);

            if (json == null || json.isBlank()) {
                throw new IllegalArgumentException("The serialized revision request is empty.");
            }

            return json;

        } catch (IllegalArgumentException exception) {
            throw exception;

        } catch (Exception exception) {
            throw new IllegalArgumentException("Could not serialize the revision request.", exception);
        }
    }
}