import type { DomainSnapshot } from "../domain/types";

import { asDomainPayload, buildDomainTableViews, getTableColumns } from "../domain/tableUtils";
import type { SimulationRunDetails, SimulationRunSummary, SimulationStatisticsPayload } from "../simulation/types";

import type {
    ComparisonColumn,
    ComparisonDataset,
    ComparisonRow,
    ComparisonSourceDescriptor,
    ComparisonTable,
    ComparisonUnit,
    ComparisonValueKind,
    ComparisonSnapshotRelation,

} from "./types";
import { getPhysicalColumnUnit } from "../domain/unitSystem";
import type { NodeSemanticType } from "../network/nodeSemantics";
import { buildSimulationNodeTypeMap } from "../simulation/simulationNodeTypes";


function inferValueKind(rows: ComparisonRow[], column: string): ComparisonValueKind {

    for (const row of rows) {

        const value = row[column];
        if (value === null || value === undefined) {
            continue;
        }
        if (typeof value === "number") {
            return "number";
        }
        if (typeof value === "boolean") {
            return "boolean";
        }
        if (typeof value === "string") {
            return "text";
        }

        return "unknown";
    }

    return "unknown";
}

function buildDefaultColumns(rows: ComparisonRow[]): ComparisonColumn[] {

    return getTableColumns(rows).map(column => {

        const kind = inferValueKind(rows, column);
        return {
            key: column,
            label: column,
            valueKind: kind,
            unit: getPhysicalColumnUnit(column) ? "m" : null,
            identity: false,
            comparable: kind === "number",
        };
    }
    );
}

function inferDefaultIdentityColumns(tableKey: string, rows: ComparisonRow[]): string[] {

    const columns = new Set(getTableColumns(rows));
    const normalizedKey = tableKey.toLowerCase();

    if (normalizedKey.includes("node")) {
        for (const candidate of ["noeud", "node", "id", "ID"]) {
            if (columns.has(candidate)) {
                return [candidate];
            }
        }
    }
    if (normalizedKey.includes("pipe") ||
        normalizedKey.includes("arc") ||
        normalizedKey.includes("edge")) {
        if (columns.has("edgeId")) {
            return ["edgeId"];
        }
        if (columns.has("pred") && columns.has("succ")) {
            return ["pred", "succ"];
        }
    }
    if (columns.has("borne")) {
        return ["borne"];
    }
    if (columns.has("client")) {
        return ["client"];
    }
    if (columns.has("id")) {
        return ["id"];
    }

    return [];
}

export function buildDefaultComparisonDataset(snapshot: DomainSnapshot): ComparisonDataset {
    const domain = asDomainPayload(snapshot.domain);
    const tableViews = buildDomainTableViews(domain.tables ?? {});
    const inputLabel = getComparisonInputLabel(snapshot);

    const tables: ComparisonTable[] = tableViews.map(
        table => {
            const rows = table.rows.map(row => ({
                ...row
            })
            );

            return {
                key: `default:${table.key}`,
                label: table.title,
                description: table.description,
                rows,
                columns: buildDefaultColumns(rows),
                identityColumns: inferDefaultIdentityColumns(table.key, rows),
            };
        }
    );


    return {
        id: `default:${snapshot.id}`,
        sourceType: "DEFAULT",
        label: `${inputLabel} · Default input`,
        subtitle: `Imported ${new Date(snapshot.createdAt).toLocaleString()}`,
        inputLabel,
        projectId: snapshot.projectId,
        domainSnapshotId: snapshot.id,
        createdAt: snapshot.createdAt,
        tables,
    };
}

export function buildDefaultComparisonSource(snapshot: DomainSnapshot): ComparisonSourceDescriptor {
    const inputLabel = getComparisonInputLabel(snapshot);
    return {
        id: `default:${snapshot.id}`,
        sourceType: "DEFAULT",
        label: `${inputLabel} · Default input`,
        subtitle: `Imported ${new Date(snapshot.createdAt).toLocaleString()}`,
        inputLabel,
        domainSnapshotId: snapshot.id,
    };
}

function column(key: string, label: string, options?: {
    unit?: ComparisonUnit;
    identity?: boolean;
    sharedContext?: boolean;
    comparable?: boolean;
    valueKind?: ComparisonValueKind;
    nodePresentation?: "ID" | "TYPE";
    nodeType?: NodeSemanticType;
    nodeTypeKey?: string;
},
): ComparisonColumn {

    return {
        key,
        label,
        unit: options?.unit ?? null,
        identity: options?.identity ?? false,
        sharedContext: options?.sharedContext ?? false,
        comparable: options?.comparable ?? false,
        valueKind: options?.valueKind ?? "number",
        nodePresentation: options?.nodePresentation,
        nodeType: options?.nodeType,
        nodeTypeKey: options?.nodeTypeKey,
    };
}

function pressureRows(payload: SimulationStatisticsPayload): ComparisonRow[] {
    return [
        ...payload.statistics.terminalPressure.map(row => ({
            nodeId: row.terminalId,
            nodeType: "TERMINAL",
            minPressureM: row.minPressureM,
            meanPressureM: row.meanPressureM,
            maxPressureM: row.maxPressureM,
        })),

        ...payload.statistics.transitPressure.map(
            row => ({
                nodeId: row.transitNodeId,
                nodeType: "JUNCTION",
                minPressureM: row.minPressureM,
                meanPressureM: row.meanPressureM,
                maxPressureM: row.maxPressureM,
            })),
    ];
}

function terminalSatisfactionTable(payload: SimulationStatisticsPayload): ComparisonTable {
    return {
        key: "simulation:terminalSatisfaction",
        label: "Terminal Satisfaction",
        description: "Minimum, mean, maximum and lower-tail terminal satisfaction ratios.",
        rows: payload.statistics.terminalSatisfaction.map(row =>
            ({ ...row })),

        columns: [column("terminalId", "Terminal",
            {
                identity: true,
                comparable: false,
                valueKind: "text",
                nodePresentation: "ID",
                nodeType: "TERMINAL",
            }
        ),
        column("minSat", "Min", {
            unit: "ratio",
            comparable: true,
        }
        ),
        column("moySat", "Mean", {
            unit: "ratio",
            comparable: true,
        }
        ),
        column("maxSat", "Max", {
            unit: "ratio",
            comparable: true,
        }
        ),
        column("cVarSat5", "CVaR 5%", {
            unit: "ratio",
            comparable: true,
        }
        ),
        column("cVarSat20", "CVaR 20%", {
            unit: "ratio",
            comparable: true,
        }
        ),
        ],
        identityColumns: ["terminalId"],
        identityNodeType: "TERMINAL",
    };
}

function sourceFlowTable(payload: SimulationStatisticsPayload): ComparisonTable {
    return {
        key: "simulation:sourceFlow",
        label: "Source Flow",
        description: "Source-flow statistics across successful scenarios.",
        rows: payload.statistics.sourceFlow.map(row => ({ ...row })),
        columns: [column("sourceId", "Source", {
            identity: true,
            comparable: false,
            valueKind: "text",
            nodePresentation: "ID",
            nodeType: "SOURCE",
        }
        ),

        column("minSt", "Min", {
            unit: "L/s",
            comparable: true,
        }
        ),
        column("moySt", "Mean", {
            unit: "L/s",
            comparable: true,
        }
        ),
        column("maxSt", "Max", {
            unit: "L/s",
            comparable: true,
        }
        ),
        column("cVarSt5", "CVaR 5%", {
            unit: "L/s",
            comparable: true,
        }
        ),
        column("cVarSt95", "CVaR 95%", {
            unit: "L/s",
            comparable: true,
        }
        ),
        ],
        identityColumns: ["sourceId"],
        identityNodeType: "SOURCE",
    };
}

function pipeVelocityTable(payload: SimulationStatisticsPayload): ComparisonTable {

    const nodeTypes = buildSimulationNodeTypeMap(payload);

    return {
        key: "simulation:pipeVelocity",
        label: "Pipe Velocity",
        description: "Velocity statistics for source and transit pipes.",
        rows: payload.statistics.pipeVelocity.map(row => ({
            ...row,
            startNodeType: nodeTypes.get(row.startNodeId) ?? null,
            endNodeType: nodeTypes.get(row.endNodeId) ?? null,
        })),
        columns: [
            column("edgeId", "Edge", {
                identity: true,
                comparable: false,
                valueKind: "text",
            }
            ),
            column("startNodeId", "From", {
                sharedContext: true,
                comparable: false,
                valueKind: "text",
                nodePresentation: "ID",
                nodeTypeKey: "startNodeType",
            }
            ),

            column("endNodeId", "To", {
                sharedContext: true,
                comparable: false,
                valueKind: "text",
                nodePresentation: "ID",
                nodeTypeKey: "endNodeType",
            }
            ),
            column("minVelocityMps", "Min", {
                unit: "m/s",
                comparable: true,
            }
            ),
            column("meanVelocityMps", "Mean", {
                unit: "m/s",
                comparable: true,
            }
            ),
            column("maxVelocityMps", "Max", {
                unit: "m/s",
                comparable: true,
            }
            ),
        ],
        identityColumns: ["edgeId", "startNodeId", "endNodeId"],
    };
}

function pressureTable(payload: SimulationStatisticsPayload): ComparisonTable {
    return {
        key: "simulation:pressure",
        label: "Pressure",
        description: "Pressure-head statistics at terminal and transit nodes.",
        rows: pressureRows(payload),
        columns: [
            column("nodeId", "Node",
                {
                    identity: true,
                    comparable: false,
                    valueKind: "text",
                    nodePresentation: "ID",
                    nodeTypeKey: "nodeType",
                }
            ),
            column("nodeType", "Type",
                {
                    sharedContext: true,
                    comparable: false,
                    valueKind: "text",
                    nodePresentation: "TYPE",
                    nodeTypeKey: "nodeType",
                }
            ),
            column("minPressureM", "Minimum",
                {
                    unit: "m",
                    comparable: true,
                }
            ),
            column("meanPressureM", "Mean",
                {
                    unit: "m",
                    comparable: true,
                }
            ),
            column("maxPressureM", "Maximum",
                {
                    unit: "m",
                    comparable: true,
                }
            ),
        ],
        identityColumns: ["nodeId"],
        identityNodeTypeKey: "nodeType",
    };
}

export function buildSimulationComparisonDataset(run: SimulationRunDetails, inputLabel: string, displayRunNumber: number): ComparisonDataset {

    if (!run.statistics) {
        throw new Error("This simulation does not contain comparison statistics.");
    }
    const payload = run.statistics;

    const networkName = run.input.networkName?.trim() || inputLabel;
    return {
        id: `simulation:${run.id}`,
        sourceType: "SIMULATION",
        //label: `${networkName} · Simulation #${run.runNumber}`,
        label: `Simulation #${run.runNumber}`,
        /* subtitle: [
            run.input.filename,
            run.input.version !== null ? `v${run.input.version}` : null,
            run.configurationMode === "DEFAULT" ? "Default" : "Manual",
            run.scenarioCount !== null ? `${run.scenarioCount} scenarios` : null,
            run.seed !== null ? `seed ${run.seed}` : null,
        ].filter(Boolean).join(" · "), */
        subtitle: "",
        inputLabel: networkName,
        inputFilename: run.input.filename ?? undefined,
        inputVersion: run.input.version ?? undefined,
        configurationMode: run.configurationMode,
        requestedOptions: run.requestedOptions,
        resolvedOptions: run.resolvedOptions,
        parameterSources: run.parameterSources,

        projectId: run.projectId,
        domainSnapshotId: run.input.domainSnapshotId,
        createdAt: run.createdAt,
        runId: run.id,

        //Keep the backend historical number for audit/debug purposes.
        runNumber: run.runNumber,
        seed: run.seed ?? undefined,
        scenarioCount: run.scenarioCount ?? undefined,
        status: run.status,
        tables: [
            terminalSatisfactionTable(payload),
            sourceFlowTable(payload),
            pipeVelocityTable(payload),
            pressureTable(payload),
        ],
    };
}

export function buildSimulationComparisonSource(run: SimulationRunSummary, inputLabel: string, displayRunNumber: number): ComparisonSourceDescriptor {

    return {
        id: `simulation:${run.id}`,
        sourceType: "SIMULATION",
        label: `Simulation #${displayRunNumber}`,
        //subtitle: `${run.scenarioCount} scenarios` + ` · seed ${run.seed}` + ` · ${new Date(run.createdAt).toLocaleString()}`,
        subtitle: "",
        inputLabel,
        domainSnapshotId: run.domainSnapshotId,
        runId: run.id,
        displayRunNumber,
    };
}

export function findComparisonTable(dataset: ComparisonDataset, tableKey: string): ComparisonTable | null {

    return (
        dataset.tables.find(table => table.key === tableKey) ?? null
    );
}

export function firstComparisonTable(dataset: ComparisonDataset): ComparisonTable | null {

    return dataset.tables[0] ?? null;
}

export function shareSameDomainSnapshot(left: ComparisonDataset, right: ComparisonDataset): boolean {

    return left.domainSnapshotId === right.domainSnapshotId;
}

export function tablesShareIdentity(left: ComparisonTable, right: ComparisonTable): boolean {

    if (left.identityColumns.length === 0 || right.identityColumns.length === 0) {

        return false;
    }


    return left.identityColumns.some(column => right.identityColumns.includes(column));
}

export function compareDomainSnapshots(
    left: ComparisonDataset | null,
    right: ComparisonDataset | null,
): ComparisonSnapshotRelation {

    if (!left || !right || !left.domainSnapshotId || !right.domainSnapshotId) {
        return "UNKNOWN";
    }
    return (left.domainSnapshotId === right.domainSnapshotId)
        ? "SAME"
        : "DIFFERENT";
}

/**
 * Return the human-readable name of the imported network.
 */
export function getComparisonInputLabel(snapshot: DomainSnapshot): string {

    const domain = asDomainPayload(snapshot.domain);
    const name = domain.name?.trim();
    if (name) {
        return name;
    }
    /*
     * Fallback only when the imported domain has no explicit name.
     */
    return "Imported network";
}