package com.neatwork.backend.response;

import java.util.List;

public record ComparableEntitySummary(
        List<String> matched,
        List<String> leftOnly,
        List<String> rightOnly,
        List<String> contextChanged
) {
}
