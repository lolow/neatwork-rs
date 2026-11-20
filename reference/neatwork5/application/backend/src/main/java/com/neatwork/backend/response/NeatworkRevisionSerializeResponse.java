package com.neatwork.backend.response;

import java.util.Map;

public record NeatworkRevisionSerializeResponse(
        String content,
        Map<String, Object> domain,
        NetworkGraphResponse graph
) {
}