/**
 * Supported display units for hydraulic length-like values.
 */
export type DisplayLengthUnit = "m" | "ft" | "in";

/**
 * Metadata displayed in the unit selector.
 */
export type DisplayUnitDefinition = {
    value: DisplayLengthUnit;
    label: string;
    symbol: string;
};

export const DISPLAY_LENGTH_UNITS: DisplayUnitDefinition[] = [
    {
        value: "m",
        label: "Meter",
        symbol: "m",
    },
    {
        value: "ft",
        label: "Feet",
        symbol: "ft",
    },
    {
        value: "in",
        label: "Inches",
        symbol: "in",
    },
];

/**
 * Physical representation of an imported numeric column.
 * "meter" means that the raw value is stored in metres.
 * "millimeter" is used only when the source field explicitly stores mm.
 */
export type PhysicalColumnUnit = "meter" | "millimeter";

/**
 * Imported fields that represent hydraulic lengths.
 *
 * Keys are normalized before lookup.
 */
const PHYSICAL_COLUMN_UNITS: Record<string, PhysicalColumnUnit> = {
    /* -------------------------
           Network nodes
    ------------------------- */
    haut: "meter",
    elevation: "meter",
    elevation_m: "meter",
    xdata: "meter",
    ydata: "meter",
    x: "meter",
    y: "meter",
    /* -------------------------
           Pipes
    ------------------------- */
    lon: "meter",
    lon1: "meter",
    lon2: "meter",
    length: "meter",
    length_m: "meter",
    pipe_length: "meter",
    pipe_length_m: "meter",
    /* -------------------------
           Pipe diameters
    ------------------------- */
    diam: "meter",
    diameter: "meter",
    diameter_m: "meter",
    diam1: "meter",
    diam2: "meter",
    diameter_mm: "millimeter",
    /* -------------------------
           Orifices
    ------------------------- */
    orif_ideal: "meter",
    orif_com: "meter",
    ideal_orifice_diameter_m: "meter",
    commercial_orifice_diameter_m: "meter",
    /* -------------------------
           Hydraulic pressure head
    ------------------------- */
    pressure: "meter",
    pressure_m: "meter",
    required_pressure_m: "meter",

    /* -------------------------
       Simulation / project defaults
    ------------------------- */
    pipelength: "meter",
    commercial_pipe_length_m: "meter",
    commercialpipelengthm: "meter",
    requiredpressurem: "meter",
};

/**
 * Normalize imported column names before metadata lookup.
 */
function normalizeColumnKey(value: string): string {
    return value.trim().toLowerCase().replace(/[-\s]+/g, "_");
}

/**
 * Return the physical storage unit of one imported column.
 */
export function getPhysicalColumnUnit(column: string): PhysicalColumnUnit | null {
    return PHYSICAL_COLUMN_UNITS[normalizeColumnKey(column)] ?? null;
}

/**
 * Conversion factor from metres to feet.
 */
const METERS_TO_FEET = 3.280839895013123;

/**
 * Conversion factor from metres to inches.
 */
const METERS_TO_INCHES = 39.37007874015748;

// Converts canonical metres to millimetres for fields that are conventionally easier to read in millimetres, such as terminal orifices.
export function metersToMillimeters(meters: number): number {
    return meters * 1000;
}

/**
 * Convert an imported value to metres before applying the display unit.
 */
function sourceValueToMeters(value: number, sourceUnit: PhysicalColumnUnit): number {

    switch (sourceUnit) {
        case "millimeter":
            return value / 1000;

        case "meter":
        default:
            return value;
    }
}

/**
 * Convert metres to the selected display unit.
 */
export function metersToDisplayUnit(meters: number, targetUnit: DisplayLengthUnit): number {
    switch (targetUnit) {
        case "ft":
            return meters * METERS_TO_FEET;

        case "in":
            return meters * METERS_TO_INCHES;

        case "m":
        default:
            return meters;
    }
}

/**
 * Convert the active project display unit back to canonical metres.
 * Use this whenever a user edits a length-like value that will be sent to Backend/Python.
 */
export function displayUnitToMeters(value: number, sourceUnit: DisplayLengthUnit): number {
    switch (sourceUnit) {
        case "ft":
            return value / METERS_TO_FEET;

        case "in":
            return value / METERS_TO_INCHES;

        case "m":
        default:
            return value;
    }
}

// Convert canonical metres back to the physical storage representation expected by one source field.
function metersToSourceValue(meters: number, sourceUnit: PhysicalColumnUnit): number {
    switch (sourceUnit) {
        case "millimeter":
            return meters * 1000;

        case "meter":
        default:
            return meters;
    }
}

// Converts one user-entered project-display value back to the canonical/raw value stored by the imported domain.
// Non-physical fields return null.
export function convertDisplayMeasurementToTableValue(column: string, value: number, displayUnit: DisplayLengthUnit): number | null {
    const sourceUnit = getPhysicalColumnUnit(column);
    if (!sourceUnit) {
        return null;
    }
    const meters = displayUnitToMeters(value, displayUnit);
    return metersToSourceValue(meters, sourceUnit);
}

// Convert one imported numeric table value to the selected display unit.
// Non-physical columns return null and should be displayed unchanged.
export function convertTableMeasurement(column: string, value: number, targetUnit: DisplayLengthUnit): number | null {

    const sourceUnit = getPhysicalColumnUnit(column);

    if (!sourceUnit) {
        return null;
    }
    const meters = sourceValueToMeters(value, sourceUnit);

    return metersToDisplayUnit(meters, targetUnit);
}

/**
 * Return the short symbol used in table headers.
 */
export function getDisplayUnitSymbol(unit: DisplayLengthUnit): string {

    switch (unit) {

        case "ft":
            return "ft";

        case "in":
            return "in";

        case "m":
        default:
            return "m";
    }
}

// Returns the presentation unit used for orifice diameters.
// When the project uses metres, small orifice values are shown in millimetres for readability. Feet and inches remain aligned with the project-wide display unit.
export function getOrificeDisplayUnitSymbol(displayUnit: DisplayLengthUnit): string {
    if (displayUnit === "m") {
        return "mm";
    }
    return getDisplayUnitSymbol(displayUnit);
}

// Converts a canonical orifice diameter stored in metres to the unit used by the current project UI.
// Meter projects display orifices in millimetres. Feet and inch projects keep their global display unit.
export function metersToOrificeDisplayUnit(meters: number, displayUnit: DisplayLengthUnit): number {
    if (displayUnit === "m") {
        return metersToMillimeters(meters);
    }

    return metersToDisplayUnit(meters, displayUnit);
}

// Returns the selected length unit per second.
export function getVelocityDisplayUnitSymbol(unit: DisplayLengthUnit): string {

    return (`${getDisplayUnitSymbol(unit)}/s`);
}