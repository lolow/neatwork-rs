package com.neatwork.backend.request;


/**
 * Current hydraulic state of one project edge.
 *
 * @param edgeId external graph edge identifier
 * @param enabled true when the pipe is hydraulically open
 */
public record SimulationEdgeOverrideRequest(
        String edgeId,
        boolean enabled
) {
}
