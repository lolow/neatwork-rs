package com.neatwork.backend.exception;


/**
 * Represent an HTTP error returned by Python simulation endpoints.
 */
public class PythonSimulationException extends RuntimeException {

    private final int statusCode;
    private final String responseBody;

    public PythonSimulationException(int statusCode, String responseBody, Throwable cause) {
        super("Python hydraulic simulation failed.", cause);

        this.statusCode = statusCode;
        this.responseBody = responseBody;
    }

    public int getStatusCode() {
        return statusCode;
    }

    public String getResponseBody() {
        return responseBody;
    }
}