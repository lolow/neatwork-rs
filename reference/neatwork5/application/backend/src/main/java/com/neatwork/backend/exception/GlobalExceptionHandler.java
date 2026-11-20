package com.neatwork.backend.exception;

import lombok.RequiredArgsConstructor;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.ExceptionHandler;
import org.springframework.web.bind.annotation.ResponseStatus;
import org.springframework.web.bind.annotation.RestControllerAdvice;
import tools.jackson.databind.ObjectMapper;

import java.time.Instant;
import java.util.Map;
import java.util.Objects;

@RestControllerAdvice
@RequiredArgsConstructor
public class GlobalExceptionHandler {

    private final ObjectMapper objectMapper;

    @ExceptionHandler(ResourceNotFoundException.class)
    @ResponseStatus(HttpStatus.NOT_FOUND)
    public Map<String, Object> handleNotFound(ResourceNotFoundException e) {
        return Map.of(
                "timestamp", Instant.now(),
                "status", 404,
                "message", e.getMessage()
        );
    }

    @ExceptionHandler(IllegalArgumentException.class)
    @ResponseStatus(HttpStatus.BAD_REQUEST)
    public Map<String, Object> handleBadRequest(IllegalArgumentException e) {
        return Map.of(
                "timestamp", Instant.now(),
                "status", 400,
                "message", Objects.toString(e.getMessage(), "Bad request")
        );
    }

    @ExceptionHandler(PythonSimulationException.class)
    public ResponseEntity<Object> handlePythonSimulation(PythonSimulationException e){
        Object pythonBody;
        try {
            pythonBody = objectMapper.readValue(e.getResponseBody(), Object.class);
        } catch (Exception parseException) {
            pythonBody = Map.of("message", e.getResponseBody());
        }
        return ResponseEntity.status(e.getStatusCode())
                .body(Map.of("timestamp", Instant.now(),
                    "status", e.getStatusCode(),
                    "message", "Hydraulic simulation failed.",
                    "pythonCore", pythonBody)
                );
    }

    @ExceptionHandler(SimulationRunConflictException.class)
    @ResponseStatus(HttpStatus.CONFLICT)
    public Map<String, Object> handleSimulationRunConflict(SimulationRunConflictException exception) {
        return Map.of("timestamp", Instant.now(),
                "status", 409, "message",
                Objects.toString(exception.getMessage(), "Simulation state conflict")
        );
    }

    @ExceptionHandler(DomainIntegrityException.class)
    @ResponseStatus(HttpStatus.BAD_REQUEST)
    public Map<String, Object> handleDomainIntegrity(DomainIntegrityException exception) {
        return Map.of("timestamp", Instant.now(), "status", 400,
                "message", Objects.toString(exception.getMessage(), "Domain integrity validation failed"));
    }
}
