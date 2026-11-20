package com.neatwork.backend.response;

import com.neatwork.backend.enums.ComparisonCompatibilityLevel;

import java.util.List;
import java.util.UUID;

public record DomainSnapshotCompatibilityResponse(
        UUID leftSnapshotId,
        UUID rightSnapshotId,

        ComparisonCompatibilityLevel level,
        boolean sameTopology,
        boolean sameDemandStructure,
        boolean sameEffectiveInput,

        boolean directMetricComparisonAllowed,
        boolean partialMetricComparisonAllowed,

        DomainComparisonChangeSummary changes,
        List<String> warnings,

        ComparableEntitySummary terminals,
        ComparableEntitySummary sources,
        ComparableEntitySummary transitNodes,
        ComparableEntitySummary pipes

) {
}
