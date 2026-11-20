import type { SimulationOrificeMode, SimulationRunOptions } from "./types";
import { resolveClientDemands } from "../domain/clientBehaviorUtils";


type JsonObject = Record<string, unknown>;

export type SimulationDialogDefaults = {
    options: SimulationRunOptions;
    uniformDemand: boolean;
    sourceFormat: string | null;
    // Demand values are owned exclusively by the Client demand table.
    clientDemand: ClientDemandSummary;
};

export type ClientDemandSummary = {
    clientCount: number;
    completeClientCount: number;
    uniform: boolean;
    // A scalar value exists only when all clients use the same opening probability.
    openProbability: number | null;
    // A scalar value exists only when all clients use the same target flow.
    targetFlowLps: number | null;
};

const DEFAULT_OPTIONS: SimulationRunOptions = {
    scenarioCount: 100,
    orificeMode: "COMMERCIAL",
    boundedFlow: false,
    serviceQuality: 0.65,
    requiredPressureM: 2,
    orificeCoefficient: 1,
    faucetCoefficient: 1,
    waterTemperatureC: 20,
    commercialPipeLengthM: 6,
    maxGenerationBatches: 20,
    oversamplingFactor: 1.1,
    maxDrawsPerBatch: 100000,
};

function asObject(value: unknown): JsonObject {
    return (
        value !== null
        && typeof value === "object"
        && !Array.isArray(value)
    )
        ? value as JsonObject
        : {};
}

function firstValue(object: JsonObject, keys: readonly string[]): unknown {
    for (const key of keys) {
        if (object[key] !== undefined && object[key] !== null) {
            return object[key];
        }
    }

    return undefined;
}

function firstValueFromSources(sources: readonly JsonObject[], keys: readonly string[],): unknown {
    for (const source of sources) {
        const value = firstValue(source, keys);

        if (value !== undefined && value !== null) {
            return value;
        }
    }

    return undefined;
}

function numberValue(object: JsonObject, keys: readonly string[],): number | undefined {
    const raw = firstValue(object, keys);

    if (typeof raw === "number" && Number.isFinite(raw)) {
        return raw;
    }

    if (typeof raw === "string") {
        const parsed = Number(raw.trim().replace(",", "."));

        if (Number.isFinite(parsed)) {
            return parsed;
        }
    }

    return undefined;
}

function booleanValue(object: JsonObject, keys: readonly string[],): boolean | undefined {
    const raw = firstValue(object, keys);

    if (typeof raw === "boolean") {
        return raw;
    }

    if (typeof raw === "number" && Number.isFinite(raw)) {
        return raw !== 0;
    }

    if (typeof raw === "string") {
        const normalized = raw.trim().toLowerCase();

        if (["1", "true", "yes", "on"].includes(normalized)) {
            return true;
        }

        if (["0", "false", "no", "off"].includes(normalized)) {
            return false;
        }
    }

    return undefined;
}

function numberValueFromSources(sources: readonly JsonObject[], keys: readonly string[],): number | undefined {
    for (const source of sources) {
        const value = numberValue(source, keys);

        if (value !== undefined) {
            return value;
        }
    }

    return undefined;
}

function booleanValueFromSources(sources: readonly JsonObject[], keys: readonly string[],): boolean | undefined {
    for (const source of sources) {
        const value = booleanValue(source, keys);

        if (value !== undefined) {
            return value;
        }
    }

    return undefined;
}

function stringValue(object: JsonObject, keys: readonly string[],): string | undefined {
    const raw = firstValue(object, keys);

    if (typeof raw !== "string") {
        return undefined;
    }

    const normalized = raw.trim();
    return normalized.length > 0 ? normalized : undefined;
}

function orificeModeValue(sources: readonly JsonObject[],): SimulationOrificeMode {
    const raw = firstValueFromSources(
        sources,
        [
            "orif_com",
            "orificeCommercial",
            "orifice_commercial",
            "orificeMode",
            "orifice_mode",
        ],
    );

    if (typeof raw === "string") {
        const normalized = raw.trim().toUpperCase();

        if (normalized === "IDEAL" || normalized === "0") {
            return "IDEAL";
        }

        if (normalized === "COMMERCIAL" || normalized === "1") {
            return "COMMERCIAL";
        }
    }

    if (typeof raw === "boolean") {
        return raw ? "COMMERCIAL" : "IDEAL";
    }

    if (typeof raw === "number" && Number.isFinite(raw)) {
        return raw === 1 ? "COMMERCIAL" : "IDEAL";
    }

    return DEFAULT_OPTIONS.orificeMode ?? "COMMERCIAL";
}

function uniqueNumbers(rows: unknown[], keys: readonly string[],): number[] {
    return [
        ...new Set(
            rows
                .map((row) => numberValue(asObject(row), keys))
                .filter((value): value is number => value !== undefined),
        ),
    ];
}

/**
 * Build editable simulation defaults from the latest imported domain snapshot.
 * Parameter precedence mirrors Python Core:
 * technicalParams < designParams < params.
 */
export function buildSimulationDefaults(domain: unknown): SimulationDialogDefaults {
    const root = asObject(domain);
    const technicalParams = asObject(root.technicalParams ?? root.technical_params);
    const designParams = asObject(root.designParams ?? root.design_params);
    const generalParams = asObject(root.params);

    // Highest-precedence source first. This remains correct even if two
    // sources use different aliases for the same parameter.
    const parameterSources = [generalParams, designParams, technicalParams];

    const tables = asObject(root.tables);
    const clients = Array.isArray(tables.clients) ? tables.clients : [];
    const bornes = Array.isArray(tables.bornes) ? tables.bornes : [];

    //const probabilities = uniqueNumbers(clients, ["proba", "openProbability", "open_probability"]);
    //const targets = uniqueNumbers(clients, ["cible", "targetFlowLps", "target_flow_lps"]);
    const bornePressures = uniqueNumbers(bornes, ["pressure", "requiredPressureM", "required_pressure_m"]);

    const resolvedClients = resolveClientDemands(tables);
    const probabilities = resolvedClients.map(client => client.openProbability);
    const targets = resolvedClients.map(client => client.targetFlowLps);
    const probabilitySet = new Set(probabilities);
    const targetSet = new Set(targets);

    // Python Core makes the same decision from the client table.
    const demandDataComplete = clients.length > 0 && resolvedClients.length === clients.length;
    const uniformDemand = demandDataComplete && probabilitySet.size === 1 && targetSet.size === 1;

    const uniformOpenProbability = uniformDemand ? probabilities[0] : null;
    const uniformTargetFlowLps = uniformDemand ? targets[0] : null;


    return {
        sourceFormat:
            stringValue(root, ["sourceFormat", "source_format"])?.toUpperCase()
            ?? null,

        uniformDemand,
        clientDemand: {
            clientCount: clients.length,
            completeClientCount: resolvedClients.length,
            uniform: uniformDemand,
            openProbability: uniformOpenProbability,
            targetFlowLps: uniformTargetFlowLps,
        },

        options: {
            scenarioCount:
                numberValueFromSources(
                    parameterSources,
                    [
                        "nbsim",
                        "scenarioCount",
                        "scenario_count",
                        "numberOfScenarios",
                        "number_of_scenarios",
                    ],
                )
                ?? DEFAULT_OPTIONS.scenarioCount,


            orificeMode: orificeModeValue(parameterSources),

            boundedFlow:
                uniformDemand
                    ? (
                        booleanValueFromSources(
                            parameterSources,
                            [
                                "bounded_flow",
                                "bounded-flow",
                                "boundedFlow",
                                "boundedFlowEnabled",
                                "bounded_flow_enabled",
                            ],
                        )
                        ?? DEFAULT_OPTIONS.boundedFlow
                    )
                    : true,

            serviceQuality:
                numberValueFromSources(
                    parameterSources,
                    [
                        "servicequal",
                        "serviceequal",
                        "serviceQuality",
                        "service_quality",
                    ],
                )
                ?? DEFAULT_OPTIONS.serviceQuality,

            requiredPressureM:
                numberValueFromSources(
                    parameterSources,
                    ["pressure", "requiredPressureM", "required_pressure_m"],
                )
                ?? (
                    bornePressures.length === 1
                        ? bornePressures[0]
                        : undefined
                )
                ?? DEFAULT_OPTIONS.requiredPressureM,

            orificeCoefficient:
                numberValueFromSources(
                    parameterSources,
                    ["orifcoef", "orificeCoefficient", "orifice_coefficient"],
                )
                ?? DEFAULT_OPTIONS.orificeCoefficient,

            faucetCoefficient:
                numberValueFromSources(
                    parameterSources,
                    ["faucetcoef", "faucetCoefficient", "faucet_coefficient"],
                )
                ?? DEFAULT_OPTIONS.faucetCoefficient,

            waterTemperatureC:
                numberValueFromSources(
                    parameterSources,
                    [
                        "watertemp",
                        "waterTemperatureC",
                        "water_temperature_c",
                        "waterTemperature",
                        "water_temperature",
                    ],
                )
                ?? DEFAULT_OPTIONS.waterTemperatureC,

            commercialPipeLengthM:
                numberValueFromSources(
                    parameterSources,
                    [
                        "pipelength",
                        "commercialPipeLengthM",
                        "commercial_pipe_length_m",
                        "pipeLength",
                        "pipe_length",
                    ],
                )
                ?? DEFAULT_OPTIONS.commercialPipeLengthM,

            maxGenerationBatches:
                numberValueFromSources(
                    parameterSources,
                    ["maxGenerationBatches", "max_generation_batches"],
                )
                ?? DEFAULT_OPTIONS.maxGenerationBatches,

            oversamplingFactor:
                numberValueFromSources(
                    parameterSources,
                    ["oversamplingFactor", "oversampling_factor"],
                )
                ?? DEFAULT_OPTIONS.oversamplingFactor,

            maxDrawsPerBatch:
                numberValueFromSources(
                    parameterSources,
                    ["maxDrawsPerBatch", "max_draws_per_batch"],
                )
                ?? DEFAULT_OPTIONS.maxDrawsPerBatch,
        },
    };
}