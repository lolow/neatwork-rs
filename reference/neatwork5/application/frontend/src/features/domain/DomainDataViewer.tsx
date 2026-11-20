import { useMemo, useState, useEffect, type ReactNode } from "react";
import type { DomainSnapshot, RevisionNamingMode } from "./types";
import {
    getDisplayUnitSymbol,
    metersToOrificeDisplayUnit,
    getOrificeDisplayUnitSymbol,
    metersToDisplayUnit
} from "./unitSystem";
import type { DisplayLengthUnit } from "./unitSystem";
import {
    asDomainPayload,
    buildDomainTableViews,
    formatTableCell,
    getTableColumns,
    labelTableColumn,
    normalizeTableKey,
    behaviorNameById,
} from "./tableUtils";
import type { DomainTableView } from "./tableUtils";
import type { ProjectArtifact } from "../artifacts/types";
import { DomainEditor } from "./DomainEditor";
import { NodeIdBadge, NodeTypeBadge, nodeSemanticFromStatus } from "../network/nodeSemantics";

type Props = {
    snapshot: DomainSnapshot | null;
    displayUnit: DisplayLengthUnit;
    sourceArtifact: ProjectArtifact | null;
    onSaveRevision: (domain: unknown,
        namingMode: RevisionNamingMode,
        customFilename?: string | null,
    ) => Promise<ProjectArtifact>;
};

// Normalizes imported column names so DSG/TPO variants can be matched safely.
function normalizeImportedKey(value: string): string {
    return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

// Reads one semantic value from an imported row without depending on exact source capitalization or separator style.
function readImportedValue(row: Record<string, unknown>, candidates: string[]): unknown {
    const expected = new Set(candidates.map(normalizeImportedKey));
    for (const [key, value] of Object.entries(row)) {
        if (expected.has(normalizeImportedKey(key))) {
            return value;
        }
    }
    return undefined;
}

// Returns the hydraulic node identifier used to match network nodes with terminal outlet rows.
function nodeIdentifier(row: Record<string, unknown>): string | null {
    const value =
        readImportedValue(row,
            [
                "noeud",
                "node",
                "node_id",
                "nodeId",
                "id",
            ]
        );

    if (value === undefined || value === null) {
        return null;
    }
    const normalized = String(value).trim();
    return normalized === "" ? null : normalized;
}

// Returns the node identifier referenced by one terminal-outlet row.
function terminalIdentifier(row: Record<string, unknown>): string | null {
    const value = readImportedValue(row, [
        "borne",
        "outlet",
        "noeud",
        "node",
        "node_id",
        "nodeId",
        "id",
    ]
    );
    if (value === undefined || value === null) {
        return null;
    }
    const normalized = String(value).trim();
    return normalized === "" ? null : normalized;
}

/**
 * Enriches Network nodes with terminal-only fields for table presentation.
 * The underlying domain snapshot remains unchanged.
 */
function enrichNetworkNodeRows(nodeRows: Record<string, unknown>[], terminalRows: Record<string, unknown>[]): Record<string, unknown>[] {

    const terminalsByNode = new Map<string, Record<string, unknown>>();
    for (const terminal of terminalRows) {
        const id = terminalIdentifier(terminal);
        if (!id) continue;
        terminalsByNode.set(id, terminal);
    }
    return nodeRows.map(
        node => {
            const id = nodeIdentifier(node);
            const terminal = id
                ? terminalsByNode.get(id)
                : undefined;
            const pressure = terminal ? readImportedValue(terminal, [
                "pressure",
                "pression",
                "pressure_m",
                "required_pressure",
                "requiredPressureM",
            ]
            ) : null;

            const neatworkFlag = terminal ? readImportedValue(terminal, [
                "N",
                "neatwork_flag",
                "neatworkFlag",
                "flag",
            ]
            ) : null;

            // pressure and N are intentionally added after the original node fields so they appear at the end of the table.
            return {
                ...node,
                pressure: pressure ?? null,
                N: neatworkFlag ?? null,
            };
        }
    );
}

function readField(record: Record<string, unknown>, keys: string[]): unknown {
    for (const key of keys) {
        if (key in record) {
            return record[key];
        }
    }

    const normalizedLookup = new Map(
        Object.keys(record).map((key) => [normalizeTableKey(key), key])
    );

    for (const key of keys) {
        const actualKey = normalizedLookup.get(normalizeTableKey(key));
        if (actualKey) {
            return record[actualKey];
        }
    }

    return undefined;
}

function toNumber(value: unknown): number | null {
    if (typeof value === "number" && Number.isFinite(value)) {
        return value;
    }

    if (typeof value === "string" && value.trim() !== "") {
        const parsed = Number(value);
        return Number.isFinite(parsed) ? parsed : null;
    }

    return null;
}

function mapOriginalNature(value: unknown): string | null {
    if (typeof value !== "string" || value.trim() === "") {
        return null;
    }

    const normalized = normalizeTableKey(value);

    if (["tank", "source", "source_tank", "reservoir"].includes(normalized)) {
        return "Source tank";
    }

    if (["branching_node", "branch", "junction", "node"].includes(normalized)) {
        return "Junction";
    }

    if (["faucet", "tap", "borne", "terminal", "terminal_node", "terminal_outlet", "outlet"].includes(normalized)) {
        return "Faucet / terminal outlet";
    }

    return value;
}

function inferNodeType(record: Record<string, unknown>): string {
    const existingType = mapOriginalNature(
        readField(record, ["node_type", "nodeType", "type", "nature"])
    );

    if (existingType) {
        return existingType;
    }

    const nodeId = String(readField(record, ["node", "id", "ID", "Noeud", "label"]) ?? "")
        .trim()
        .toUpperCase();

    const status = toNumber(readField(record, ["statut", "status", "status_code"]));
    const tapCount = toNumber(readField(record, ["nbrob", "number_of_taps", "taps"]));

    if (nodeId === "TP" || nodeId === "TANK" || nodeId === "SOURCE") {
        return "Source tank";
    }

    if (status === 0) {
        return "Source tank";
    }

    if (status === 2) {
        return "Faucet / terminal outlet";
    }

    if (tapCount !== null && tapCount > 0) {
        return "Faucet / terminal outlet";
    }

    if (status === 1) {
        return "Junction";
    }

    return "Junction";
}

// Hides an unused second pipe segment by rendering zero length as an em dash.
function formatSegment2Length(value: unknown, column: string, displayUnit: DisplayLengthUnit): string {
    const numeric = toNumber(value);
    if (numeric !== null && Math.abs(numeric) < 1e-12) {
        return "—";
    }
    return formatTableCell(
        value,
        column,
        displayUnit
    );
}

/* function enrichNetworkNodeRows(rows: unknown[]): unknown[] {
    return rows.map((row) => {
        if (!row || typeof row !== "object" || Array.isArray(row)) {
            return row;
        }

        const record = row as Record<string, unknown>;
        const enriched: Record<string, unknown> = {};
        let nodeTypeInserted = false;

        for (const [key, value] of Object.entries(record)) {
            enriched[key] = value;

            if (!nodeTypeInserted && ["node", "id", "ID", "Noeud", "label"].includes(key)) {
                enriched.node_type = inferNodeType(record);
                nodeTypeInserted = true;
            }
        }

        if (!nodeTypeInserted) {
            enriched.node_type = inferNodeType(record);
        }

        return enriched;
    });
} */

export function DomainDataViewer({ snapshot, displayUnit, sourceArtifact, onSaveRevision }: Props) {
    const [selectedTableKey, setSelectedTableKey] = useState<string | null>(null);
    const [editing, setEditing] = useState(false);
    const [saving, setSaving] = useState(false);
    const [saveMessage, setSaveMessage] = useState<string | null>(null);
    const [saveError, setSaveError] = useState<string | null>(null);

    useEffect(() => {
        setSelectedTableKey(null);
    }, [snapshot?.id]);

    const domain = asDomainPayload(snapshot?.domain);
    const tables = domain.tables ?? {};
    const behaviorNames = useMemo(() => behaviorNameById(tables), [tables]);

    const rawTableViews = useMemo(() => buildDomainTableViews(tables), [tables]);

    const pipeDiameterCatalog = useMemo(() =>
        rawTableViews.find(table =>
            table.title === "Pipe diameter catalog")
        ?? null,
        [rawTableViews]
    );

    const tableViews = useMemo(() => {
        const networkNodes = rawTableViews.find(table => table.title === "Network nodes");
        const terminalOutlets = rawTableViews.find(table => table.title === "Terminal outlets");

        // Terminal outlets are represented as additional columns of Network nodes in the read-only Tables workspace.
        return rawTableViews
            .filter(table => table.title !== "Terminal outlets")
            .map(table => {
                if (table.title !== "Network nodes") {
                    return table;
                }
                return {
                    ...table,
                    description: "Network topology nodes with terminal pressure and NeatWork metadata.",
                    rows: enrichNetworkNodeRows(networkNodes?.rows ?? table.rows, terminalOutlets?.rows ?? []),
                };
            }
            );
    },
        [rawTableViews]
    );

    const nodeStatusLookup = useMemo(() => {
        const networkNodes = tableViews.find(table => table.title === "Network nodes");
        return buildNodeStatusLookup(networkNodes?.rows ?? []);
    },
        [tableViews]
    );

    const pipeDiameterLookup = useMemo(() =>
        buildPipeDiameterLookup(pipeDiameterCatalog?.rows ?? []),
        [pipeDiameterCatalog]
    );

    const selectedTable =
        tableViews.find((table) => table.key === selectedTableKey) ?? tableViews[0];

    if (!snapshot) {
        return (
            <section className="domain-data-viewer empty">
                <h2>No imported domain yet</h2>
                <p>
                    Upload project files, then use NeatWork domain import to parse them
                    into tables and a visual graph.
                </p>
            </section>
        );
    }

    if (editing && snapshot && sourceArtifact) {
        return (
            <DomainEditor
                snapshot={snapshot}
                sourceArtifact={sourceArtifact}
                displayUnit={displayUnit}
                saving={saving}
                onCancel={() => setEditing(false)}
                onSave={async (domain, namingMode, customFilename) => {
                    setSaving(true);
                    setSaveError(null);
                    try {
                        const artifact = await onSaveRevision(domain, namingMode, customFilename);
                        setSaveMessage(`Saved as ${artifact.originalFilename}`);
                        setEditing(false);
                    } catch (error) {
                        const message = error instanceof Error ? error.message : "The edited file could not be saved.";
                        setSaveError(message);
                        throw error;

                    } finally {
                        setSaving(false);
                    }
                }}
            />
        );
    }

    return (
        <section className="domain-data-viewer">
            <div className="domain-data-header">
                <div className="domain-data-heading">
                    <h2>{domain.name ?? "Imported NeatWork domain"}</h2>
                    <p>
                        Source: <strong>{domain.sourceFormat ?? snapshot.sourceFormat}</strong>
                        {" · "}
                        Imported: <strong>{new Date(snapshot.createdAt).toLocaleString()}</strong>
                    </p>
                </div>
                <div className="domain-data-header-actions">
                    {sourceArtifact ? (
                        <button
                            type="button"
                            className="project-modal__primary"
                            onClick={() => {
                                setSaveMessage(null);
                                setSaveError(null);
                                setEditing(true);
                            }}
                        >
                            Edit input data
                        </button>
                    ) : (
                        <span className="domain-edit-unavailable">Re-import this legacy network to enable versioned editing.</span>
                    )}

                    {saveMessage && (<div className="domain-save-success">{saveMessage}</div>)}
                    {saveError && (<div className="project-modal__error">{saveError}</div>)}

                </div>
            </div>


            <div className="domain-data-content">
                {domain.warnings && domain.warnings.length > 0 && (
                    <div className="domain-warning-box">
                        <strong>Import warnings</strong>
                        <ul>
                            {domain.warnings.map((warning, index) => (
                                <li key={`${warning}-${index}`}>{warning}</li>
                            ))}
                        </ul>
                    </div>
                )}

                {tableViews.length === 0 ? (
                    <div className="domain-empty-table">
                        No tables found in the imported domain.
                    </div>
                ) : (
                    <div className="neatwork-table-layout">
                        <aside className="neatwork-table-tabs">
                            {tableViews.map((table) => (
                                <button
                                    key={table.key}
                                    type="button"
                                    className={selectedTable?.key === table.key ? "active" : ""}
                                    onClick={() => setSelectedTableKey(table.key)}
                                >
                                    <span>{table.title}</span>
                                    <small>{table.rows.length} rows</small>
                                </button>
                            ))}
                        </aside>

                        <NeatworkTable table={selectedTable} displayUnit={displayUnit} pipeDiameterLookup={pipeDiameterLookup} nodeStatusLookup={nodeStatusLookup} behaviorNames={behaviorNames} />
                    </div>
                )}
            </div>
        </section>
    );

}


function isNodeStatusColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);
    return ["statut", "status", "nodestatus", "nodestatuscode", "nodetypecode"]
        .includes(normalized);
}

function isNeatworkFlagColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);
    return normalized === "n" || normalized === "neatworkflag";
}

function isTerminalPressureColumn(column: string): boolean {
    return normalizeImportedKey(column) === "pressure";
}

// Detects the primary identifier column of the Network nodes table.
function isNodeIdColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);

    return [
        "noeud",
        "node",
        "nodeid",
        "id",
    ].includes(normalized);
}

// Reads the raw NeatWork status code associated with one network node.
function readNodeStatus(record: Record<string, unknown>): unknown {
    return readImportedValue(record, [
        "statut",
        "status",
        "status_code",
        "node_status",
        "nodeStatus",
        "nodeStatusCode",
        "node_type_code",
    ]
    );
}

// Displays one node identifier using the semantic color of its Source, Junction or Terminal type.


// Normalizes a node identifier for cross-table lookup.
function normalizeNodeId(value: unknown): string | null {
    if (value === null || value === undefined) {
        return null;
    }
    const text = String(value).trim().toLowerCase();
    return text === "" ? null : text;
}

/**
 * Builds a lookup from network-node IDs to their persisted NeatWork status codes.
 * This allows Network pipes to reuse the exact semantic node classification from Network nodes instead of guessing from identifier prefixes.
 */
function buildNodeStatusLookup(rows: unknown[]): NodeStatusLookup {
    const lookup: NodeStatusLookup = new Map();
    for (const row of rows) {
        if (!row || typeof row !== "object" || Array.isArray(row)) {
            continue;
        }
        const record = row as Record<string, unknown>;
        const nodeId = readImportedValue(record, [
            "node",
            "node_id",
            "nodeId",
            "id",
            "ID",
            "Noeud",
            "label",
        ]
        );
        const normalizedId = normalizeNodeId(nodeId);

        if (!normalizedId) {
            continue;
        }
        const status = readNodeStatus(record);
        if (status === null || status === undefined || status === "") {
            continue;
        }
        lookup.set(normalizedId, status);
    }

    return lookup;
}

function isIdealOrificeColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);

    return [
        "orifideal",
        "idealorifice",
        "idealorificediameter",
        "idealorificediameterm",
    ].includes(normalized);
}

function isCommercialOrificeColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);

    return [
        "orifcom",
        "commercialorifice",
        "commercialorificediameter",
        "commercialorificediameterm",
    ].includes(normalized);
}

function isNodeOrificeColumn(column: string): boolean {
    return (
        isIdealOrificeColumn(column)
        ||
        isCommercialOrificeColumn(column)
    );
}

/**
 * Formats a terminal-orifice value using the project presentation policy.
 * - metre project -> millimetres
 * - feet project  -> feet
 * - inch project  -> inches
 * Source and junction nodes do not have terminal orifices.
 */
function formatNodeOrifice(value: unknown, record: Record<string, unknown>, displayUnit: DisplayLengthUnit): string {
    const semanticType = nodeSemanticFromStatus(readNodeStatus(record));
    if (semanticType !== "TERMINAL") {
        return "—";
    }
    const numericValue = toNumber(value);
    if (numericValue === null) {
        return "—";
    }
    return metersToOrificeDisplayUnit(numericValue, displayUnit).toFixed(2);
}

// Detects the canonical metre-based diameter column of the imported orifice catalog.
function isOrificeCatalogDiameterColumn(column: string): boolean {
    return normalizeImportedKey(column) === "diameterm";
}

// Detects the derived millimetre helper column. 
// This field is useful internally but should not be shown beside diameter_m because it represents the same value.
function isDerivedOrificeMillimeterColumn(column: string): boolean {
    return normalizeImportedKey(column) === "diametermm";
}

//Keeps enough precision for small orifice diameters when feet or inches are selected.
function formatOrificeDisplayValue(value: number, displayUnit: DisplayLengthUnit): string {
    const converted = metersToOrificeDisplayUnit(value, displayUnit);
    if (displayUnit === "m") {
        return converted.toFixed(2);
    }

    if (displayUnit === "ft") {
        return converted.toFixed(4);
    }

    return converted.toFixed(3);
}

// Provides presentation-specific labels for the read-only network tables.
function labelReadOnlyTableColumn(table: DomainTableView, column: string, displayUnit: DisplayLengthUnit): string {
    if (table.title === "Network nodes") {
        const orificeUnit = getOrificeDisplayUnitSymbol(displayUnit);
        if (isIdealOrificeColumn(column)) {
            return `Ideal orifice (${orificeUnit})`;
        }
        if (isCommercialOrificeColumn(column)) {
            return `Commercial orifice (${orificeUnit})`;
        }
    }
    if (table.title === "Orifice diameter catalog" && isOrificeCatalogDiameterColumn(column)) {
        return `Diameter (${getOrificeDisplayUnitSymbol(displayUnit)})`;
    }

    //All ordinary columns keep the standard project-wide display-unit labelling.
    return labelTableColumn(column, displayUnit);
}

// Keeps the original NeatWork flag untouched while rendering the legacy "N" value as an em dash.
function formatNeatworkFlag(value: unknown): string {
    if (value === null || value === undefined) {
        return "—";
    }
    const text = String(value).trim();
    if (text === "" || text.toUpperCase() === "N") {
        return "—";
    }

    return text;
}

// Detects the upstream-node column of a Network pipes table.
function isPipeFromColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);
    return [
        "pred",
        "from",
        "fromnode",
        "startnode",
        "startnodeid",
    ].includes(normalized);
}

// Detects the downstream-node column of a Network pipes table.
function isPipeToColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);
    return [
        "succ",
        "to",
        "tonode",
        "endnode",
        "endnodeid",
    ].includes(normalized);
}

/**
 * Resolves the real node status for a pipe endpoint.
 * The persisted Network nodes table is preferred.
 * Resolves the persisted node status for one pipe endpoint.
 * The Network nodes table remains the semantic source of truth.
 */
function pipeNodeStatus(value: unknown, lookup: NodeStatusLookup): unknown {
    const nodeId = normalizeNodeId(value);
    if (!nodeId) {
        return null;
    }
    return lookup.get(nodeId) ?? null;
}

// Detects the first pipe-segment diameter reference.
function isSegment1DiameterIdColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);
    return [
        "id1",
        "segment1diameterid",
        "diameterid1",
    ].includes(normalized);
}

function isSegment2DiameterIdColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);

    return [
        "id2",
        "segment2diameterid",
        "diameterid2",
    ].includes(normalized);
}

// Detects the optional second-segment length.
function isSegment2LengthColumn(column: string): boolean {
    const normalized = normalizeImportedKey(column);
    return [
        "lon2",
        "segment2length",
        "length2",
    ].includes(normalized);
}

function readSegment2Length(record: Record<string, unknown>): number | null {
    return toNumber(readImportedValue(record, [
        "lon2",
        "segment2_length",
        "segment2Length",
        "length2",
    ]));
}

type PipeDiameterLookup = Map<string, number>;

// Maps a network-node ID to its persisted NeatWork status code.
type NodeStatusLookup = Map<string, unknown>;

// Normalises a raw imported pipe-diameter reference into a canonical key.
function normalizePipeDiameterId(value: unknown): string | null {
    if (value === null || value === undefined) {
        return null;
    }
    const text = String(value).trim().toLowerCase();
    return text === "" ? null : text;
}

// Resolves the identifier used by a pipe diameter catalog row.
// Nominal is included because legacy NeatWork files may use the nominal value as the pipe reference.
function pipeDiameterCatalogId(row: Record<string, unknown>): string | null {
    const value = readImportedValue(row, [
        "id",
        "ID",
        "diameter_id",
        "diameterId",
        "nominal",
        "Nominal",
    ]);
    return normalizePipeDiameterId(value);
}

// Reads the canonical inner diameter from one pipe-diameter catalog row.
function pipeCatalogInnerDiameter(row: Record<string, unknown>): number | null {
    const value = readImportedValue(row, [
        "diam",
        "Diam",
        "inner_diameter",
        "innerDiameter",
        "inner_diameter_m",
        "innerDiameterM",
        "diameter_m",
    ]);
    return toNumber(value);
}

// Builds a fast lookup from pipe diameter IDs to their canonical inner diameters.
// The source catalog itself is never modified.
function buildPipeDiameterLookup(rows: Record<string, unknown>[]): PipeDiameterLookup {
    const lookup: PipeDiameterLookup = new Map();
    for (const row of rows) {
        const id = pipeDiameterCatalogId(row);
        const innerDiameter = pipeCatalogInnerDiameter(row);
        if (!id || innerDiameter === null) {
            continue;
        }
        lookup.set(id, innerDiameter);
    }

    return lookup;
}

function formatPipeDiameterDisplayValue(diameterM: number, displayUnit: DisplayLengthUnit): string {
    const converted = metersToDisplayUnit(diameterM, displayUnit);

    if (displayUnit === "m") {
        return converted.toFixed(3);
    }

    if (displayUnit === "ft") {
        return converted.toFixed(2);
    }

    return converted.toFixed(2);
}

// Displays a pipe-diameter ID together with the catalog inner diameter converted to the active project display unit.
function formatPipeDiameterReference(value: unknown, pipeDiameterLookup: PipeDiameterLookup, displayUnit: DisplayLengthUnit): ReactNode {
    const normalizedId = normalizePipeDiameterId(value);
    if (!normalizedId) {
        return "—";
    }
    const displayId = String(value).trim();
    const diameterM = pipeDiameterLookup.get(normalizedId);
    if (diameterM === undefined) {
        return displayId;
    }
    return (
        <span className="pipe-diameter-reference">
            <strong>{displayId}</strong>

            <span className="pipe-diameter-reference__value">
                (
                {formatPipeDiameterDisplayValue(diameterM, displayUnit)}
                {" "}
                {getDisplayUnitSymbol(displayUnit)}
                )
            </span>
        </span>
    );
}

type NeatworkTableProps = {
    table: DomainTableView;
    displayUnit: DisplayLengthUnit;
    pipeDiameterLookup: PipeDiameterLookup;
    nodeStatusLookup: NodeStatusLookup;
    behaviorNames: Map<string, string>;
};

function NeatworkTable({ table, displayUnit, pipeDiameterLookup, nodeStatusLookup, behaviorNames }: NeatworkTableProps) {
    const rawColumns = getTableColumns(table.rows);

    // The orifice catalog stores the same physical diameter in both metres and derived millimetres.
    // Only the canonical metre column is rendered. Its value is converted at presentation time according to the current project display unit.
    const columns = rawColumns.filter(column => {
        if (table.title === "Orifice diameter catalog" && isDerivedOrificeMillimeterColumn(column)) {
            return false;
        }

        // Node type is already communicated through the semantic S/J/P identifier colors.
        // Keep the persisted status in the domain, but hide the redundant read-only column.
        if (table.title === "Network nodes" && isNodeStatusColumn(column)) {
            return false;
        }
        return true;
    });

    return (
        <section className="neatwork-table-panel">
            <div className="neatwork-table-panel-header">
                <div>
                    <h3>{table.title}</h3>
                    <p>{table.description}</p>
                </div>

                <div className="neatwork-table-panel-meta">
                    {table.title === "Network nodes" && (
                        <div className="network-node-semantic-legend">
                            <span>
                                <NodeIdBadge nodeId="Source" type="SOURCE" />
                            </span>
                            <span>
                                <NodeIdBadge nodeId="Junction" type="JUNCTION" />
                            </span>
                            <span>
                                <NodeIdBadge nodeId="Terminal" type="TERMINAL" />
                            </span>

                        </div>

                    )}
                    <span>{table.rows.length} rows</span>
                    <span className="table-unit-badge">
                        {table.title === "Network nodes"
                            ? (<>
                                Length:{" "}
                                {getDisplayUnitSymbol(displayUnit)}
                                {" · "}
                                Orifice:{" "}
                                {getOrificeDisplayUnitSymbol(displayUnit)}
                            </>
                            )
                            : (
                                <>
                                    Unit:{" "}
                                    {table.title === "Orifice diameter catalog"
                                        ? getOrificeDisplayUnitSymbol(displayUnit)
                                        : getDisplayUnitSymbol(displayUnit)
                                    }
                                </>
                            )}
                    </span>
                </div>
            </div>

            <div className="neatwork-table-scroll">
                <table>
                    <thead>
                        <tr>
                            <th className="row-index-column">#</th>
                            {columns.map(column => (
                                <th key={column}>{labelReadOnlyTableColumn(table, column, displayUnit)}</th>
                            ))}
                        </tr>
                    </thead>

                    <tbody>
                        {table.rows.map((row, rowIndex) => {
                            const record =
                                row && typeof row === "object" && !Array.isArray(row)
                                    ? (row as Record<string, unknown>)
                                    : {};

                            return (
                                <tr key={rowIndex}>
                                    <td className="row-index-column">{rowIndex + 1}</td>

                                    {columns.map(column => {
                                        const isNetworkNodes = table.title === "Network nodes";
                                        const isNetworkPipes = table.title === "Network pipes";
                                        const value = record[column];
                                        let content: ReactNode;
                                        if (isNetworkNodes && isNodeIdColumn(column)) {
                                            content = (<NodeIdBadge nodeId={row[column]}
                                                type={nodeSemanticFromStatus(readImportedValue(row, [
                                                    "status",
                                                    "statut",
                                                    "statusCode",
                                                    "nodeStatus",
                                                ]))} />);
                                        }
                                        else if (isNetworkNodes && isNodeStatusColumn(column)) {
                                            content = (<NodeTypeBadge type={nodeSemanticFromStatus(row[column])} />);
                                        }
                                        else if (isNeatworkFlagColumn(column)) {
                                            // Every NeatWork flag uses the same presentation rule across read-only tables.
                                            content = formatNeatworkFlag(value);
                                        }
                                        else if (isNetworkNodes && isNodeOrificeColumn(column)) {
                                            content = formatNodeOrifice(value, record, displayUnit);
                                        }
                                        else if (isNetworkNodes && isTerminalPressureColumn(column) && (
                                            value === null || value === undefined || value === ""
                                        )
                                        ) {
                                            content = "—";
                                        }
                                        else if (isNetworkPipes && isPipeFromColumn(column)) {
                                            content = (<NodeIdBadge nodeId={value} type={nodeSemanticFromStatus(pipeNodeStatus(value, nodeStatusLookup))} />);
                                        }
                                        else if (isNetworkPipes && isPipeToColumn(column)) {
                                            content = (<NodeIdBadge nodeId={value} type={nodeSemanticFromStatus(pipeNodeStatus(value, nodeStatusLookup))} />);
                                        }
                                        else if (isNetworkPipes && isSegment1DiameterIdColumn(column)) {
                                            content = formatPipeDiameterReference(value, pipeDiameterLookup, displayUnit);
                                        }
                                        else if (isNetworkPipes && isSegment2LengthColumn(column)) {
                                            content = formatSegment2Length(value, column, displayUnit);
                                        }
                                        else if (isNetworkPipes && isSegment2DiameterIdColumn(column)) {
                                            const segment2Length = readSegment2Length(record);
                                            if (segment2Length !== null && Math.abs(segment2Length) < 1e-12) {
                                                content = "—";
                                            }
                                            else {
                                                content = formatPipeDiameterReference(value, pipeDiameterLookup, displayUnit);
                                            }
                                        }
                                        else if (table.title === "Orifice diameter catalog" &&
                                            isOrificeCatalogDiameterColumn(column)) {
                                            const numericValue = toNumber(value);
                                            content = numericValue === null ? "—" :
                                                formatOrificeDisplayValue(numericValue, displayUnit);
                                        } else if (normalizeTableKey(table.key) === "clients" && (column === "behaviorId" || column === "behavior_id")) {
                                            const behaviorId = String(value ?? "");
                                            content = behaviorNames.get(behaviorId) ?? behaviorId;
                                        }
                                        else {
                                            content = formatTableCell(value, column, displayUnit);
                                        }
                                        return (
                                            <td key={column}>{content}</td>
                                        );
                                    })}
                                </tr>
                            );
                        })}
                    </tbody>
                </table>
            </div>
        </section>
    );
}
