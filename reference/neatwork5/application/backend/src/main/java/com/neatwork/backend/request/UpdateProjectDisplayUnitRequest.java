package com.neatwork.backend.request;

import com.neatwork.backend.enums.ProjectDisplayLengthUnit;

import jakarta.validation.constraints.NotNull;


// Request used to change one project's display unit.
public record UpdateProjectDisplayUnitRequest(

        @NotNull
        ProjectDisplayLengthUnit displayUnit

) {
}
