import type { NetworkGraph } from "../network/types";
import type { ProjectArtifact } from "../artifacts/types";

export type NeatworkFileFormat = "TPO" | "DSG" | "JSON";

export type DomainSnapshot = {
    id: string;
    projectId: string;
    sourceFormat: NeatworkFileFormat;
    domain: unknown;
    createdAt: string;
    primaryArtifactId: string | null;
    diametersArtifactId: string | null;
    orificesArtifactId: string | null;
}

export type DomainImportResponse = {
    snapshot: DomainSnapshot;
    graph: NetworkGraph;
}

export type RevisionNamingMode = "DEFAULT" | "CUSTOM";

export type DomainRevisionResponse = {
    artifact: ProjectArtifact;
    snapshot: DomainSnapshot;
    graph: NetworkGraph;
};