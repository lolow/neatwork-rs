"""API-facing options for probabilistic hydraulic simulation."""

from pydantic import (
    AliasChoices,
    BaseModel,
    ConfigDict,
    Field,
)

from app.neatwork.simulation.models import SimulationRunOptions
from enum import Enum


class OrificeMode(str, Enum):
    """Available terminal-orifice models."""

    COMMERCIAL = "COMMERCIAL"
    IDEAL = "IDEAL"


class SimulationOptionsInput(BaseModel):
    """Optional settings supplied for one simulation execution."""

    model_config = ConfigDict(populate_by_name=True,extra="forbid")

    scenario_count: int | None = Field(
        default=None,
        gt=0,
        validation_alias=AliasChoices("scenario_count","scenarioCount","numberOfScenarios","nbsim")
    )

    # A missing seed requests a fresh random seed for the current simulation execution.
    seed: int | None = Field(default=None, ge=0)

    # Legacy request fields kept temporarily for API compatibility. They are not applied as runtime
    # overrides. Client demand comes from tables.clients.
    open_taps_probability: float | None = Field(
        default=None,
        ge=0,
        le=1,
        validation_alias=AliasChoices("open_taps_probability","openTapsProbability","opentaps")
    )

    target_flow_lps: float | None = Field(
        default=None,
        gt=0,
        validation_alias=AliasChoices("target_flow_lps","targetFlowLps","targetflow")
    )

    orifice_mode: OrificeMode | None = Field(
        default=None,
        validation_alias=AliasChoices("orifice_mode","orificeMode")
    )

    bounded_flow: bool | None = Field(
        default=None,
        validation_alias=AliasChoices("bounded_flow","boundedFlow")
    )

    service_quality: float | None = Field(
        default=None, ge=0, le=1,
        validation_alias=AliasChoices("service_quality","serviceQuality","servicequal")
    )

    required_pressure_m: float | None = Field(
        default=None,
        ge=0,
        validation_alias=AliasChoices("required_pressure_m","requiredPressureM","pressure")
    )

    orifice_coefficient: float | None = Field(
        default=None,
        gt=0,
        validation_alias=AliasChoices("orifice_coefficient","orificeCoefficient","orifcoef")
    )

    faucet_coefficient: float | None = Field(
        default=None,
        ge=0,
        validation_alias=AliasChoices("faucet_coefficient","faucetCoefficient","faucetcoef")
    )

    water_temperature_c: float | None = Field(
        default=None,
        ge=0,
        validation_alias=AliasChoices("water_temperature_c","waterTemperatureC","watertemp")
    )

    commercial_pipe_length_m: float | None = Field(
        default=None,
        gt=0,
        validation_alias=AliasChoices("commercial_pipe_length_m","commercialPipeLengthM","pipelength")
    )

    max_generation_batches: int = Field(
        default=20,
        gt=0,
        validation_alias=AliasChoices("max_generation_batches","maxGenerationBatches"),
    )

    oversampling_factor: float = Field(
        default=1.10,
        gt=1,
        validation_alias=AliasChoices("oversampling_factor","oversamplingFactor"),
    )

    max_draws_per_batch: int = Field(
        default=100_000,
        gt=0,
        validation_alias=AliasChoices("max_draws_per_batch", "maxDrawsPerBatch"),
    )

    # Scenario generation parameters for hydraulic system simulation in Domain run time
    def to_core_options(self, *, uniform_demand: bool, open_taps_probability: float | None, target_flow_lps: float | None) -> SimulationRunOptions:
        # uniform_demand is currently represented in the synchronized domain parameters, not SimulationRunOptions.
        _ = uniform_demand,
        return SimulationRunOptions(
            scenario_count=self.scenario_count or 100,
            seed=self.seed,
            open_taps_probability=open_taps_probability,
            target_flow_lps=target_flow_lps,
            max_generation_batches=self.max_generation_batches,
            oversampling_factor=self.oversampling_factor,
            max_draws_per_batch=self.max_draws_per_batch,
        )