package com.neatwork.backend.response;

import java.util.Map;

public record NeatworkParseResponse(
        Map<String, Object> domain,
        NetworkGraphResponse graph
) {
}
