package com.neatwork.backend.response;


import java.util.List;


public record DomainComparisonChangeSummary(

        int leftNodeCount,
        int rightNodeCount,
        int matchedNodeCount,
        List<String> addedNodes,
        List<String> removedNodes,
        List<String> changedNodeRoles,

        int leftPipeCount,
        int rightPipeCount,
        int matchedPipeCount,
        List<String> addedPipes,
        List<String> removedPipes,
        List<String> changedPipeEndpoints,

        int leftTerminalCount,
        int rightTerminalCount,
        int matchedTerminalCount,
        List<String> addedTerminals,
        List<String> removedTerminals

) {
}