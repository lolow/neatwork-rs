package com.neatwork.backend.request;

import java.util.Map;

public record NeatworkRevisionSerializeRequest(
        String name,
        String format,
        String sourceContent,
        Map<String, Object> domain
) {
}