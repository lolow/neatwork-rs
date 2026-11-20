// Domain table data processing and presentation utilities: formatting, labels, table metadata

import { convertTableMeasurement, getDisplayUnitSymbol, getPhysicalColumnUnit } from "./unitSystem";
import type { DisplayLengthUnit } from "./unitSystem";
import { formatAdaptiveNumber } from "../formatting/numericDisplay";


export type TableRow = Record<string, unknown>;

export type DomainPayload = {
    name?: string;
    sourceFormat?: string;
    params?: Record<string, unknown>;
    technicalParams?: Record<string, unknown>;
    designParams?: Record<string, unknown>;
    warnings?: string[];
    tables?: Record<string, unknown[]>;
};

export type DomainTableView = {
    key: string;
    title: string;
    description: string;
    rows: TableRow[];
};

type TableGroup = {
    title: string;
    description: string;
    aliases: string[];
};

const TABLE_GROUPS: TableGroup[] = [

    {
        title: "Network nodes",
        description: "Raw nodes imported from the network topology file.",
        aliases: ["nodes", "node_list", "nodelist"],
    },
    {
        title: "Network pipes",
        description: "Pipe links imported from the network topology file.",
        aliases: ["pipes", "pipe_list", "pipelist", "arcs", "arc_list", "arclist", "edges", "edge_list", "edgelist"],
    },
    {
        title: "Pipe diameter catalog",
        description: "Available pipe diameters imported from the DSG file or diameter database.",
        aliases: ["diameters", "diameter_list", "diameterlist", "pipe_diameters", "pipe_diameter_database",],
    },
    {
        title: "Orifice diameter catalog",
        description: "Available orifice diameters imported from the DSG file or orifice database.",
        aliases: ["orifices", "orifice_list", "orificelist", "orifice_diameters", "orifice_diameter_database",],
    },
    {
        title: "Client behaviors",
        description: "Reusable demand behaviour definitions shared by client instances.",
        aliases: ["clientBehaviors", "client_behaviors", "clientBehaviorTypes", "client_behavior_types"],
    },
    {
        title: "Client demand table",
        description: "Client instances attached to terminal outlets and linked to reusable demand behaviors.",
        aliases: ["clients", "client_list", "clientlist", "generated_clients"],
    },
    {
        title: "Terminal outlets",
        description: "Faucet or terminal outlet nodes used as consumption points.",
        aliases: ["bornes", "taps", "tap_list", "taplist", "borne_list", "bornelist", "terminal_outlets", "outlets"],
    },
];

const COLUMN_LABELS: Record<string, string> = {
    noeud: "Node ID",
    Noeud: "Node ID",
    node: "Node ID",
    id: "ID",
    ID: "ID",
    haut: "Elevation",
    Haut: "Elevation",
    xdata: "X coordinate",
    Xdata: "X coordinate",
    ydata: "Y coordinate",
    Ydata: "Y coordinate",
    orif_ideal: "Ideal orifice",
    orif_com: "Commercial orifice",
    statut: "Status",
    status: "Status",
    node_type: "Node type",
    pred: "From",
    succ: "To",
    lon: "Total pipe length",
    lon1: "Segment 1 length",
    ID1: "Segment 1 diameter ID",
    lon2: "Segment 2 length",
    ID2: "Segment 2 diameter ID",
    N: "NeatWork flag",
    Nominal: "Nominal size",
    SDR: "SDR",
    diam: "Inner diameter",
    Diam: "Inner diameter",
    cost: "Cost",
    Pmax: "Maximum pressure",
    Type: "Pipe type code",
    Roughness: "Roughness",
    source_index: "Source row",
    diameter_m: "Diameter",
    diameter_mm: "Diameter",
    outlet: "Outlet",
    elevation_m: "Elevation",
    ideal_orifice_diameter_m: "Ideal orifice diameter",
    commercial_orifice_diameter_m: "Commercial orifice diameter",
    client: "Client",
    borne: "Outlet",
    proba: "Open probability",
    cible: "Target flow",
    pressure: "Pressure",
    Pressure: "Pressure",
    pression: "Pressure",
    behaviorId: "Behavior ID",
    behavior_id: "Behavior ID",
    behavior: "Behavior",
    behaviorName: "Behavior name",
    name: "Name",
    openProbability: "Open probability",
    open_probability: "Open probability",
    targetFlowLps: "Target flow",
    target_flow_lps: "Target flow",
    profileId: "Demand profile",
    profile_id: "Demand profile",
};

// Units for non-length hydraulic input columns.
// Length-like fields continue to use unitSystem.ts because their display unit can change.
const FIXED_COLUMN_UNITS: Record<string, string> = {
    cible: "L/s",
    target_flow: "L/s",
    target_flow_lps: "L/s",
    proba: "ratio",
    probability: "ratio",
    open_probability: "ratio",
    targetflowlps: "L/s",
    openprobability: "ratio",
};

export function asDomainPayload(value: unknown): DomainPayload {

    if (!value || typeof value !== "object" || Array.isArray(value)) {
        return {};
    }

    return value as DomainPayload;
}

export function normalizeTableKey(value: string): string {

    return value.trim().toLowerCase().replace(/[-\s]+/g, "_");
}

export function prettifyTableName(value: string): string {

    return value.replace(/_/g, " ").replace(/\b\w/g, letter => letter.toUpperCase());
}

export function getTableColumns(rows: TableRow[]): string[] {

    const columns = new Set<string>();

    for (const row of rows) {
        for (const key of Object.keys(row)) {
            columns.add(key);
        }
    }

    return Array.from(columns);
}

export function toTableRows(rows: unknown[]): TableRow[] {

    return rows
        .filter(
            (row): row is TableRow =>
                row !== null && typeof row === "object" && !Array.isArray(row)
        )
        .map(row => ({ ...row }));
}

/**
 * Build a human-readable column label using the active display unit.
 */
export function labelTableColumn(column: string, displayUnit: DisplayLengthUnit): string {

    const baseLabel = COLUMN_LABELS[column]
        ?? COLUMN_LABELS[normalizeTableKey(column)]
        ?? prettifyTableName(column);

    const physicalUnit = getPhysicalColumnUnit(column);
    if (physicalUnit) {
        return (`${baseLabel} ` + `(${getDisplayUnitSymbol(displayUnit)})`);
    }

    const fixedUnit = FIXED_COLUMN_UNITS[normalizeTableKey(column)];
    if (fixedUnit) {
        return (`${baseLabel} ` + `(${fixedUnit})`);
    }
    return baseLabel;
}

// Formats a continuous table measurement. Canonical precision is preserved in the domain. This function affects presentation only.
function formatMeasurement(value: number): string {
    return formatAdaptiveNumber(value);
}

// Numeric-looking columns that represent identifiers or categorical codes rather than measurements.
const DISCRETE_NUMERIC_COLUMNS =
    new Set([
        "noeud",
        "node",
        "id",

        "pred",
        "succ",

        "id1",
        "id2",

        "client",
        "borne",
        "outlet",

        "statut",
        "status",

        "source_index",

        "n",
        "nominal",
    ]);

/**
 * Format one imported table cell for display.
 * Continuous numeric values use two decimals.
 * Identifiers and categorical numeric codes remain unchanged. Physical values are converted only at render time.
 */
export function formatTableCell(value: unknown, column: string, displayUnit: DisplayLengthUnit): string {

    if (value === null || value === undefined || value === "") {
        return "-";
    }

    const discrete = isDiscreteNumericColumn(column);

    if (typeof value === "number") {
        if (discrete) {
            return String(value);
        }
        const converted = convertTableMeasurement(column, value, displayUnit);
        if (converted !== null) {
            return formatAdaptiveNumber(converted);
        }
        return formatAdaptiveNumber(value);
    }

    if (typeof value === "string") {
        if (discrete) {
            return value;
        }
        const trimmed = value.trim();
        if (trimmed === "") {
            return "-";
        }
        const parsed = Number(trimmed);
        if (Number.isFinite(parsed)) {
            const converted = convertTableMeasurement(column, parsed, displayUnit);
            return formatAdaptiveNumber(converted ?? parsed);
        }
        return value;
    }

    if (typeof value === "boolean") {
        return value ? "Yes" : "No";
    }

    if (typeof value === "object") {
        return JSON.stringify(value);
    }

    return String(value);
}

function findTable(tables: Record<string, unknown[]>, aliases: string[],): [string, unknown[]] | null {

    const entries = Object.entries(tables);
    const normalizedAliases = aliases.map(normalizeTableKey);
    for (const alias of normalizedAliases) {

        const found = entries.find(([key, rows]) =>

            normalizeTableKey(key) === alias
            &&
            Array.isArray(rows)
            &&
            rows.length > 0
        );
        if (found) {
            return found;
        }
    }
    for (const alias of normalizedAliases) {
        const found = entries.find(([key, rows]) =>

            normalizeTableKey(key) === alias
            &&
            Array.isArray(rows)
        );
        if (found) {
            return found;
        }
    }
    return null;
}

function markGroupAsUsed(tables: Record<string, unknown[]>, aliases: string[], usedKeys: Set<string>): void {

    const normalizedAliases = new Set(aliases.map(normalizeTableKey));
    for (const key of Object.keys(tables)) {
        if (normalizedAliases.has(normalizeTableKey(key))) {
            usedKeys.add(normalizeTableKey(key));
        }
    }
}

export function buildDomainTableViews(tables: Record<string, unknown[]>): DomainTableView[] {

    const usedKeys = new Set<string>();
    const views: DomainTableView[] = [];
    for (const group of TABLE_GROUPS) {
        const found = findTable(tables, group.aliases);

        if (!found) {
            continue;
        }
        const [key, rows] = found;
        const optionalTable = group.aliases.some(alias => ["clients", "bornes", "taps", "terminal_outlets"]
            .includes(alias)
        );

        if (rows.length === 0 && optionalTable) {
            markGroupAsUsed(tables, group.aliases, usedKeys);
            continue;
        }

        views.push({
            key,
            title: group.title,
            description: group.description,
            rows: toTableRows(rows)
        });

        markGroupAsUsed(tables, group.aliases, usedKeys);
    }

    for (const [key, rows] of Object.entries(tables)) {

        if (!Array.isArray(rows) || usedKeys.has(normalizeTableKey(key))) {
            continue;
        }

        views.push({
            key,
            title: prettifyTableName(key),
            description: `Imported table: ${key}`,
            rows: toTableRows(rows)
        });
    }

    return views;
}

function isDiscreteNumericColumn(column: string): boolean {
    return DISCRETE_NUMERIC_COLUMNS.has(normalizeTableKey(column));
}

// Formats one editable physical value in the project display unit without changing the canonical domain value.
export function formatEditableTableCell(value: unknown, column: string, displayUnit: DisplayLengthUnit): string {

    if (value === null || value === undefined) {
        return "";
    }

    const numeric = typeof value === "number" ? value : typeof value === "string"
        ? Number(value) : Number.NaN;

    if (Number.isFinite(numeric)) {
        const converted = convertTableMeasurement(column, numeric, displayUnit);

        if (converted !== null) {

            // Editing keeps additional precision.
            // The normal read-only table still uses the two-decimal rule from Step 10.
            return String(Number(converted.toPrecision(10)));
        }
    }

    return String(value);
}

export function behaviorNameById(tables: Record<string, unknown[]>): Map<string, string> {

    const rows = (tables.clientBehaviors ?? tables.client_behaviors ?? []);
    const result = new Map<string, string>();
    for (const raw of rows) {
        if (!raw || typeof raw !== "object") {
            continue;
        }
        const row = raw as Record<string, unknown>;
        const id = String(row.behaviorId ?? row.behavior_id ?? "");
        if (!id) {
            continue;
        }
        result.set(id, String(row.name ?? id));
    }
    return result;
}
