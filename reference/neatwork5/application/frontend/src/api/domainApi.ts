import { httpClient } from "./httpClient";
import type { DomainImportResponse, DomainSnapshot, NeatworkFileFormat, DomainRevisionResponse, RevisionNamingMode } from "../features/domain/types";

export async function importDomainFromArtifacts(params: {
    projectId: string;
    name: string;
    format: NeatworkFileFormat;
    primaryArtifactId?: string | null;
    diametersDbArtifactId?: string | null;
    orificesDbArtifactId?: string | null;
}): Promise<DomainImportResponse> {
    const response = await httpClient.post<DomainImportResponse>(
        `/api/projects/${params.projectId}/domain/import-from-artifacts`,
        {
            name: params.name,
            format: params.format,
            primaryArtifactId: params.primaryArtifactId,
            diametersDbArtifactId: params.diametersDbArtifactId ?? null,
            orificesDbArtifactId: params.orificesDbArtifactId ?? null,
        }
    );

    return response.data;
}

export async function fetchLatestDomainSnapshot(projectId: string): Promise<DomainSnapshot> {
    const response = await httpClient.get<DomainSnapshot>(
        `/api/projects/${projectId}/domain/latest`
    );

    return response.data;
}

/**
 * Saves edited input data as a new immutable DSG/TPO file revision.
 */
export async function saveDomainRevision(params: {
    projectId: string;
    sourceSnapshotId: string;
    domain: unknown;
    namingMode: RevisionNamingMode;
    customFilename?: string | null;
}): Promise<DomainRevisionResponse> {

    const response = await httpClient.post<DomainRevisionResponse>(
        `/api/projects/${params.projectId}/domain/revisions`,
        {
            sourceSnapshotId: params.sourceSnapshotId,
            domain: params.domain,
            namingMode: params.namingMode,
            customFilename: params.customFilename ?? null,
        }
    );

    return response.data;
}