import { httpClient } from "./httpClient";
import type { ProjectArtifact, ProjectArtifactType } from "../features/artifacts/types";
import type { TpoArtifactRequirements } from "../features/import/types";

export async function fetchProjectArtifacts(projectId: string): Promise<ProjectArtifact[]> {
    const response = await httpClient.get<ProjectArtifact[]>(`/api/projects/${projectId}/artifacts`);
    return response.data;
}

export async function uploadProjectArtifact(
    projectId: string,
    type: ProjectArtifactType,
    file: File
): Promise<ProjectArtifact> {
    const formData = new FormData();
    formData.append("type", type);
    formData.append("file", file);

    const response = await httpClient.post<ProjectArtifact>(
        `/api/projects/${projectId}/artifacts`,
        formData,
        { headers: { "Content-Type": "multipart/form-data" } }
    );

    return response.data;
}

export function artifactDownloadUrl(projectId: string, artifactId: string): string {
    const baseUrl = import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080";
    return `${baseUrl}/api/projects/${projectId}/artifacts/${artifactId}/download`;
}

export async function fetchArtifactLibrary(type: ProjectArtifactType): Promise<ProjectArtifact[]> {
    const response = await httpClient.get<ProjectArtifact[]>(
        `/api/artifacts`, { params: { type } });
    return response.data;
}


export async function copyArtifactToProject(projectId: string, sourceArtifactId: string): Promise<ProjectArtifact> {
    const response = await httpClient.post<ProjectArtifact>(
        `/api/projects/${projectId}/artifacts/${sourceArtifactId}/copy`
    );
    return response.data;
}

export async function fetchTpoArtifactRequirements(artifactId: string): Promise<TpoArtifactRequirements> {
    const response = await httpClient.get<TpoArtifactRequirements>(
        `/api/artifacts/${artifactId}/tpo-requirements`
    );
    return response.data;
}

export async function archiveDatabaseArtifact(artifactId: string): Promise<void> {
    await httpClient.delete(`/api/artifacts/${artifactId}/library`);
}

// Removes one DSG or TPO file from the active project library.
// The backend preserves historical dependencies.
export async function deleteProjectInputArtifact(projectId: string, artifactId: string): Promise<void> {
    await httpClient.delete(`/api/projects/${projectId}/artifacts/${artifactId}`);
}

// Reads the original text stored in one project artifact.
// This endpoint is intended for the read-only source-file viewer and does not modify or download the artifact.
export async function fetchArtifactText(projectId: string, artifactId: string): Promise<string> {
    const response = await httpClient.get<string>(
        `/api/projects/${projectId}/artifacts/${artifactId}/text`, { responseType: "text" }
    );

    return response.data;
}