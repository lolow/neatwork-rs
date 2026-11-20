package com.neatwork.backend.response;

/**
 * Compact summary of one hydraulic simulation execution.
 * failedScenarios counts execution failures only:
 * preparation, numerical solving or convergence failures.
 * A successfully solved scenario containing one or more terminals with insufficient delivered water remains a
 * successful execution scenario. Those shortfalls are stored in per-terminal statistics.
 */
public record SimulationSummaryResponse(
        String status,
        int scenarioCount,
        int successfulScenarios,
        int failedScenarios,
        double successRate,

        long seed,

        double durationSeconds,

        int minIterations,
        double meanIterations,
        int maxIterations,

        int openEdges,
        int closedEdges,
        int lowPressureNodes
) {
}
