package com.neatwork.backend.request;

import com.fasterxml.jackson.annotation.JsonInclude;

import com.neatwork.backend.enums.SimulationOrificeMode;
import jakarta.validation.constraints.DecimalMax;
import jakarta.validation.constraints.DecimalMin;
import jakarta.validation.constraints.Min;


/**
 * Runtime configuration for one hydraulic simulation.
 *
 * @param scenarioCount number of probabilistic scenarios
 * @param seed optional reproducibility seed
 * @param openTapsProbability optional uniform opening probability
 * @param targetFlowLps optional uniform target flow in litres per second
 * @param maxGenerationBatches maximum scenario-generation batches
 * @param oversamplingFactor scenario-generation oversampling factor
 * @param maxDrawsPerBatch maximum random draws per generation batch
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record SimulationOptionsRequest(

        @Min(1)
        Integer scenarioCount,

         // Optional user-selected reproducibility seed.
         // A null value means that Python Core must generate a fresh seed for this simulation execution.
        @Min(0)
        Long seed,

        @DecimalMin(value = "0.0", inclusive = false)
        @DecimalMax("1.0")
        Double openTapsProbability,

        @DecimalMin(value = "0.0", inclusive = false)
        Double targetFlowLps,

        SimulationOrificeMode orificeMode,

        Boolean boundedFlow,

        @DecimalMin("0.0")
        @DecimalMax("1.0")
        Double serviceQuality,

        @DecimalMin("0.0")
        Double requiredPressureM,

        @DecimalMin(value = "0.0", inclusive = false)
        Double orificeCoefficient,

        @DecimalMin("0.0")
        Double faucetCoefficient,

        @DecimalMin("0.0")
        Double waterTemperatureC,

        @DecimalMin(value = "0.0", inclusive = false)
        Double commercialPipeLengthM,

        @Min(1)
        Integer maxGenerationBatches,

        @DecimalMin(value = "1.0", inclusive = false)
        Double oversamplingFactor,

        @Min(1)
        Integer maxDrawsPerBatch

) {

    // Return an empty options object so Python can apply domain defaults.
    public static SimulationOptionsRequest empty() {
        return new SimulationOptionsRequest(
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null
        );
    }

    //Removes legacy per-run client-demand overrides.
    // Client demand is owned exclusively by the pinned domain snapshot's Client demand table.
    public SimulationOptionsRequest withoutClientDemandOverrides() {
        return new SimulationOptionsRequest(
                scenarioCount,
                seed,
                null,   // openTapsProbability
                null,   // targetFlowLps
                orificeMode,
                boundedFlow,
                serviceQuality,
                requiredPressureM,
                orificeCoefficient,
                faucetCoefficient,
                waterTemperatureC,
                commercialPipeLengthM,
                maxGenerationBatches,
                oversamplingFactor,
                maxDrawsPerBatch
        );
    }
}