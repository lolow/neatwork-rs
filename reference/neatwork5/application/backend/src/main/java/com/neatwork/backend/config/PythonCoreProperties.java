package com.neatwork.backend.config;

import org.springframework.boot.context.properties.ConfigurationProperties;

@ConfigurationProperties(prefix = "neatwork.python-core")
public record PythonCoreProperties (String baseUrl) {
    public PythonCoreProperties {
        if (baseUrl == null || baseUrl.isBlank()) {
            baseUrl = "http://localhost:8000";
        }
    }
}
