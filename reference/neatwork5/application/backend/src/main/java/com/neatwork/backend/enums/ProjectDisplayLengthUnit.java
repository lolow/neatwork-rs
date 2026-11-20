package com.neatwork.backend.enums;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonValue;
import java.util.Locale;

/**
 * Project-wide display unit for hydraulic length-like values.
 * Internal calculations remain in canonical SI units.
 * This preference controls presentation and user input only.
 */
public enum ProjectDisplayLengthUnit {
    METER("m"),
    FEET("ft"),
    INCHES("in");

    private final String symbol;

    ProjectDisplayLengthUnit(String symbol) {
        this.symbol = symbol;
    }

    // Expose the same compact unit values used by the TypeScript frontend.
    @JsonValue
    public String symbol() {
        return symbol;
    }

    // Accept frontend values such as m, ft and in.
    @JsonCreator
    public static ProjectDisplayLengthUnit fromJson(String value) {
        if (value == null) {
            throw new IllegalArgumentException("Display unit is required.");
        }
        return switch (value.trim().toLowerCase(Locale.ROOT)) {

            case "m", "meter", "metre" ->
                    METER;
            case "ft", "feet", "foot" ->
                    FEET;
            case "in", "inch", "inches" ->
                    INCHES;
            default ->
                    throw new IllegalArgumentException("Unsupported display unit: " + value);
        };
    }
}