package com.neatwork.backend.config;

import org.springframework.boot.context.properties.ConfigurationProperties;

@ConfigurationProperties(prefix = "neatwork.workspace")
public record WorkspaceProperties(
        String rootDir
) {
    public WorkspaceProperties{
        if (rootDir == null || rootDir.isBlank()) {
            rootDir = "./neatwork-workspace";
        }

        rootDir = rootDir.trim();

        if (rootDir.startsWith("root-dir:")) {
            rootDir = rootDir.substring("root-dir:".length()).trim();
        }
    }
}
