import type { DomainPayload, TableRow } from "./tableUtils";


export type CatalogReference = {
    entityId: string;
    location: string;
};

export type CatalogDeletionCheck = {
    blocked: boolean;
    catalogType: "PIPE_DIAMETER" | "ORIFICE_DIAMETER" | "NODE" | null;
    itemLabel: string;
    references: CatalogReference[];
};

function normalizeKey(value: string): string {
    return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

function readValue(row: Record<string, unknown>, candidates: string[]): unknown {
    const normalizedCandidates = new Set(candidates.map(normalizeKey));

    for (const [key, value] of Object.entries(row)) {
        if (normalizedCandidates.has(normalizeKey(key))) {
            return value;
        }
    }

    return undefined;
}

function asRow(value: unknown): Record<string, unknown> {
    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
        return value as Record<string, unknown>;
    }
    return {};
}

function findTable(domain: DomainPayload, aliases: string[]): TableRow[] {

    const tables = domain.tables ?? {};

    const normalizedAliases = new Set(aliases.map(normalizeKey));

    for (const [key, rows] of Object.entries(tables)) {
        if (normalizedAliases.has(normalizeKey(key)) && Array.isArray(rows)) {
            return rows as TableRow[];
        }
    }

    return [];
}

const PIPE_TABLE_ALIASES = [
    "pipes",
    "pipe_list",
    "pipelist",
    "arcs",
    "edges",
];


const PIPE_DIAMETER_ALIASES = [
    "diameters",
    "diameter_list",
    "diameterlist",
    "pipe_diameters",
    "pipe_diameter_database",
];


const ORIFICE_ALIASES = [
    "orifices",
    "orifice_list",
    "orificelist",
    "orifice_diameters",
    "orifice_diameter_database",
];


const NODE_ALIASES = [
    "nodes",
    "node_list",
    "nodelist",
];

const CLIENT_ALIASES = [
    "clients",
    "client_list",
    "clientlist",
    "generated_clients",
];


const BORNE_ALIASES = [
    "bornes",
    "taps",
    "tap_list",
    "taplist",
    "borne_list",
    "bornelist",
    "terminal_outlets",
    "outlets",
];

function normalizeReferenceId(value: unknown): string | null {
    if (value === null || value === undefined) {
        return null;
    }

    const text = String(value).trim().toLowerCase();

    if (text === "" || text === "n" || text === "-") {
        return null;
    }

    return text;
}

function pipeDiameterId(row: Record<string, unknown>): string | null {
    return normalizeReferenceId(
        readValue(row, [
            "ID",
            "id",
            "diameter_id",
            "diameterId",
            "Nominal",
            "nominal",
        ])
    );
}

function pipeEntityLabel(row: Record<string, unknown>, index: number): string {

    const id = readValue(row, [
        "id",
        "ID",
        "pipe",
        "edge",
        "name",
    ]);

    if (id !== null && id !== undefined && String(id).trim() !== "") {
        return String(id);
    }

    const from = readValue(row, ["pred", "from", "startNode"]);
    const to = readValue(row, ["succ", "to", "endNode"]);

    if (from !== undefined && to !== undefined) {
        return `${String(from)} → ${String(to)}`;
    }

    return `Pipe row ${index + 1}`;
}

function findPipeDiameterReferences(domain: DomainPayload, diameterRow: Record<string, unknown>): CatalogReference[] {
    const diameterId = pipeDiameterId(diameterRow);

    if (!diameterId) {
        return [];
    }

    const pipes = findTable(domain, PIPE_TABLE_ALIASES);
    const references: CatalogReference[] = [];

    pipes.forEach((rawPipe, index) => {
        const pipe = asRow(rawPipe);
        const id1 = normalizeReferenceId(readValue(pipe, [
            "ID1",
            "id1",
            "segment1DiameterId",
            "diameterId1",
        ]));

        const id2 = normalizeReferenceId(readValue(pipe, [
            "ID2",
            "id2",
            "segment2DiameterId",
            "diameterId2",
        ]));

        const label = pipeEntityLabel(pipe, index);
        if (id1 === diameterId) {
            references.push({
                entityId: label,
                location: "Segment 1 diameter",
            });
        }

        const segment2LengthRaw = readValue(pipe, ["lon2", "segment2Length", "length2",]);
        const segment2Length = Number(segment2LengthRaw);
        const segment2Active = Number.isFinite(segment2Length) && Math.abs(segment2Length) > 1e-12;
        if (segment2Active && id2 === diameterId) {
            references.push({
                entityId: label,
                location: "Segment 2 diameter",
            });
        }
    }
    );

    return references;
}


function toFiniteNumber(value: unknown): number | null {
    if (typeof value === "number" && Number.isFinite(value)) {
        return value;
    }

    if (typeof value === "string") {
        const parsed = Number(value.trim());
        if (Number.isFinite(parsed)) {
            return parsed;
        }
    }

    return null;
}

/* function orificeCatalogDiameter(row: Record<string, unknown>): number | null {
    const metres = toFiniteNumber(
        readValue(row, [
            "diameter_m",
            "diameterM",
            "diameter",
            "in_diam",
            "inDiam",
        ]));

    if (metres !== null) {
        return metres;
    }
    const millimetres = toFiniteNumber(readValue(row, ["diameter_mm", "diameterMm"]));
    if (millimetres !== null) {
        return millimetres / 1000;
    }
    return null;
}

function sameDiameter(left: number, right: number): boolean {
    const absoluteTolerance = 1e-9;
    const relativeTolerance = 1e-7;
    const scale = Math.max(1, Math.abs(left), Math.abs(right));

    return (Math.abs(left - right) <=
        Math.max(absoluteTolerance, relativeTolerance * scale));
} */

function nodeEntityLabel(row: Record<string, unknown>, index: number): string {

    const value = readValue(row, [
        "noeud",
        "node",
        "nodeId",
        "node_id",
        "id",
    ]);


    return (value !== undefined && value !== null && String(value).trim() !== ""
        ? String(value)
        : `Node row ${index + 1}`);
}

function nodeReferenceId(row: Record<string, unknown>): string | null {
    return normalizeReferenceId(
        readValue(row, [
            "noeud",
            "node",
            "nodeId",
            "node_id",
            "id",
        ]));
}

function findNodePipeReferences(domain: DomainPayload, nodeId: string): CatalogReference[] {
    const pipes = findTable(domain, PIPE_TABLE_ALIASES);
    const references: CatalogReference[] = [];

    pipes.forEach((rawPipe, index) => {
        const pipe = asRow(rawPipe);
        const from = normalizeReferenceId(readValue(pipe, [
            "pred",
            "from",
            "startNode",
            "startNodeId",
        ]));

        const to = normalizeReferenceId(readValue(pipe, [
            "succ",
            "to",
            "endNode",
            "endNodeId",
        ]));
        if (from === nodeId || to === nodeId) {
            references.push({
                entityId: pipeEntityLabel(pipe, index),
                location: from === nodeId
                    ? "Pipe start node"
                    : "Pipe end node",
            });
        }
    });

    return references;
}

function findNodeClientReferences(domain: DomainPayload, nodeId: string): CatalogReference[] {

    const clients = findTable(domain, CLIENT_ALIASES);
    const references: CatalogReference[] = [];
    clients.forEach((rawClient, index) => {
        const client = asRow(rawClient);
        const terminal = normalizeReferenceId(readValue(client, [
            "borne",
            "terminalId",
            "terminal_id",
            "outlet",
        ]));

        if (terminal !== nodeId) {
            return;
        }
        const clientId = readValue(client, [
            "client",
            "clientId",
            "client_id",
            "id",
        ]);

        references.push({
            entityId: clientId !== undefined
                &&
                clientId !== null ? String(clientId)
                : `Client row ${index + 1}`,
            location: "Client terminal",
        });
    });

    return references;
}

function findNodeBorneReferences(domain: DomainPayload, nodeId: string): CatalogReference[] {
    const bornes = findTable(domain, BORNE_ALIASES);
    const references: CatalogReference[] = [];
    bornes.forEach(
        (rawBorne, index) => {
            const borne = asRow(rawBorne);
            const referencedNode = normalizeReferenceId(readValue(borne, ["borne",
                "node",
                "nodeId",
                "node_id",
                "outlet",
                "id",
            ]));

            if (referencedNode !== nodeId) {
                return;
            }
            references.push({
                entityId: `Outlet row ${index + 1}`,
                location: "Terminal outlet",
            });
        });

    return references;
}

function findNodeReferences(domain: DomainPayload, nodeRow: Record<string, unknown>): CatalogReference[] {

    const nodeId = nodeReferenceId(nodeRow);
    if (!nodeId) {
        return [];
    }

    return [
        ...findNodePipeReferences(domain, nodeId),
        ...findNodeClientReferences(domain, nodeId),
        ...findNodeBorneReferences(domain, nodeId),
    ];
}

function nodeStatus(row: Record<string, unknown>): number | null {

    const raw = readValue(row, [
        "statut",
        "status",
        "statusCode",
        "nodeStatus",
    ]);

    const numeric = Number(raw);

    return Number.isFinite(numeric) ? numeric : null;
}

/* function findOrificeReferences(domain: DomainPayload, orificeRow: Record<string, unknown>): CatalogReference[] {
    const diameter = orificeCatalogDiameter(orificeRow);

    if (diameter === null) {
        return [];
    }
    const nodes = findTable(domain, NODE_ALIASES);
    const references: CatalogReference[] = [];
    nodes.forEach(
        (rawNode, index) => {
            const node = asRow(rawNode);

            // Source and junction nodes do not use terminal-orifice diameters.
            if (nodeStatus(node) !== 2) {
                return;
            }

            const ideal = toFiniteNumber(readValue(node, [
                "orif_ideal",
                "idealOrifice",
                "ideal_orifice_diameter_m",
            ]));

            const commercial = toFiniteNumber(readValue(node, [
                "orif_com",
                "commercialOrifice",
                "commercial_orifice_diameter_m",
            ]));

            const nodeId = nodeEntityLabel(node, index);

            if (ideal !== null && sameDiameter(ideal, diameter)) {
                references.push({
                    entityId: nodeId,
                    location: "Ideal orifice",
                });
            }

            if (commercial !== null && sameDiameter(commercial, diameter)) {
                references.push({
                    entityId: nodeId,
                    location: "Commercial orifice",
                });
            }
        }
    );

    return references;
} */

function isTableKey(tableKey: string, aliases: string[]): boolean {
    const normalized = normalizeKey(tableKey);

    return aliases.some(alias => normalizeKey(alias) === normalized);
}

export function checkCatalogDeletion(domain: DomainPayload, tableKey: string, rowIndex: number): CatalogDeletionCheck {
    const tables = domain.tables ?? {};

    const rawRows = tables[tableKey];

    if (!Array.isArray(rawRows)) {
        return {
            blocked: false,
            catalogType: null,
            itemLabel: "",
            references: [],
        };
    }

    const row = asRow(rawRows[rowIndex]);

    if (isTableKey(tableKey, NODE_ALIASES)) {
        const nodeId = nodeReferenceId(row);
        const references = findNodeReferences(domain, row);
        return {
            blocked: references.length > 0,
            catalogType: "NODE",
            itemLabel: nodeId ?? `row ${rowIndex + 1}`,

            references,
        };
    }

    if (isTableKey(tableKey, PIPE_DIAMETER_ALIASES)) {
        const id = pipeDiameterId(row);
        const references = findPipeDiameterReferences(domain, row);
        return {
            blocked: references.length > 0,
            catalogType: "PIPE_DIAMETER",
            itemLabel: id ?? `row ${rowIndex + 1}`,

            references,
        };
    }

    /* if (isTableKey(tableKey, ORIFICE_ALIASES)) {
        const diameter = orificeCatalogDiameter(row);
        const references = findOrificeReferences(domain, row);
        return {
            blocked: references.length > 0,
            catalogType: "ORIFICE_DIAMETER",
            itemLabel: diameter !== null
                ? `${(diameter * 1000).toFixed(2)} mm`
                : `row ${rowIndex + 1}`,
            references,
        };
    } */
    return {
        blocked: false,
        catalogType: null,
        itemLabel: "",
        references: [],
    };
}