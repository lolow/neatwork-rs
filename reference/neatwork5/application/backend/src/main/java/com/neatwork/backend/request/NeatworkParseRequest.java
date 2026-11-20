package com.neatwork.backend.request;

import jakarta.validation.constraints.NotBlank;
import jakarta.validation.constraints.NotNull;

public record NeatworkParseRequest(
        @NotBlank
        String name,

        @NotBlank
        String format,

        @NotNull
        String primaryContent,
        String diametersDbContent,
        String orificesDbContent
) {
}
