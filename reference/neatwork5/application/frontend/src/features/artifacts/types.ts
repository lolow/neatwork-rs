export type ProjectArtifactType =
    | "TPO_INPUT"
    | "DSG_INPUT"
    | "DB_DIAMETERS_INPUT"
    | "DB_ORIFICES_INPUT"
    | "JSON_INPUT"
    | "DSG_OUTPUT"
    | "INP_OUTPUT"
    | "JSON_OUTPUT"
    | "FUSION_META_JSON"
    | "TABLE_FUSION_CSV"
    | "SIMULATION_XLSX"
    | "SIMULATION_JSON"
    | "LOG_FILE"
    | "OTHER";

export type ProjectArtifact = {
    id: string;
    projectId: string;
    type: ProjectArtifactType;
    originalFilename: string;
    storedFilename: string;
    sizeBytes: number;
    version: number;
    generated: boolean;
    archived: boolean;

    parentArtifactId: string | null;
    revisionFamilyId: string | null;
    createdAt: string;
};