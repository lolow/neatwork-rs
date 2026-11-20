import { httpClient } from "./httpClient";
import type {
    NetworkGraph,
    NetworkEdge,
    NetworkNode,
    EdgeStatus
} from "../features/network/types";
import type { ProjectSummary } from "../features/projects/types";
import type {
    SimulationRunRequestOptions,
    SimulationJob,
    SimulationRunDetails,
    SimulationRunSummary,
    SimulationHistoryItem,
} from "../features/simulation/types";
import type { ProjectArtifact } from "../features/artifacts/types";
import type { ProjectExportAvailability, ProjectExportFormat } from "../features/export/types";
import type { DisplayLengthUnit } from "../features/domain/unitSystem";
import type { DomainSnapshotCompatibility } from "../features/comparison/types";

/* export type SimulatedGraphResponse = {
    graph: NetworkGraph;
    summary: SimulationSummary;
} */

/**
 * Fetch a paginated list of all projects.
 */
export async function fetchProjects(): Promise<ProjectSummary[]> {
    const response = await httpClient.get<ProjectSummary[]>("/api/projects");
    return response.data;
}

/**
 * Create a new project.
 */
export async function createProject(name: string): Promise<ProjectSummary> {
    const response = await httpClient.post<ProjectSummary>("/api/projects", { name });
    return response.data;
}

/**
 * Delete one project and all of its persisted project data.
 */
export async function deleteProject(projectId: string): Promise<void> {
    await httpClient.delete(`/api/projects/${projectId}`);
}

/**
 * Fetch the network graph for a specific project.
 */
export async function fetchProjectGraph(projectId: string): Promise<NetworkGraph> {
    const response = await httpClient.get<NetworkGraph>(`/api/projects/${projectId}/graph`);
    return response.data;
}

/**
 * Fetch unfinished simulation executions persisted
 * by the backend.
 */
export async function fetchActiveSimulationJobs(projectId: string): Promise<SimulationJob[]> {
    const response = await httpClient.get<SimulationJob[]>(
        `/api/projects/${projectId}/simulation/jobs`);
    return response.data;
}

/**
 * Update the status of a specific edge.
 */
export async function updateEdgeStatus(
    projectId: string,
    edgeId: string,
    status: EdgeStatus
): Promise<NetworkEdge> {
    const response = await httpClient.patch<NetworkEdge>(
        `/api/projects/${projectId}/edges/${edgeId}/status`,
        { status }
    );
    return response.data
}

/**
 * Update the position of a specific node.
 */
export async function updateNodePosition(
    projectId: string,
    nodeId: string,
    x: number,
    y: number

): Promise<NetworkNode> {
    const response = await httpClient.patch<NetworkNode>(
        `/api/projects/${projectId}/nodes/${nodeId}/position`,
        { x, y }
    );

    return response.data;
}

/**
 * Starts one asynchronous hydraulic simulation.
 *
 * Passing an empty options object delegates parameter resolution entirely to the active domain and Python Core.
 */
export async function startSimulationJob(projectId: string, options: SimulationRunRequestOptions): Promise<SimulationJob> {
    const response = await httpClient.post<SimulationJob>(
        `/api/projects/${projectId}/simulation/jobs`, options);

    return response.data;
}

export async function fetchSimulationJob(projectId: string, jobId: string): Promise<SimulationJob> {
    const response = await httpClient.get<SimulationJob>(
        `/api/projects/${projectId}/simulation/jobs/${jobId}`
    );

    return response.data;
}

export async function cancelSimulationJob(projectId: string, jobId: string): Promise<SimulationJob> {
    const response = await httpClient.delete<SimulationJob>(
        `/api/projects/${projectId}/simulation/jobs/${jobId}`
    );

    return response.data;
}

export async function fetchExportAvailability(projectId: string): Promise<ProjectExportAvailability[]> {
    const response = await httpClient.get<ProjectExportAvailability[]>(
        `/api/projects/${projectId}/exports/availability`
    );

    return response.data;
}

export async function exportProject(projectId: string, format: ProjectExportFormat): Promise<ProjectArtifact> {
    const response = await httpClient.post<ProjectArtifact>(
        `/api/projects/${projectId}/exports`, { format }
    );

    return response.data;
}

/**
 * Fetch the complete persisted simulation history for one project.
 */
export async function fetchSimulationRuns(projectId: string): Promise<SimulationRunSummary[]> {
    const response = await httpClient.get<SimulationRunSummary[]>(
        `/api/projects/${projectId}/simulation/runs`
    );

    return response.data;
}

/**
 * Fetch runtime options and canonical statistics for one historical run.
 */
export async function fetchSimulationRun(projectId: string, runId: string,): Promise<SimulationRunDetails> {
    const response = await httpClient.get<SimulationRunDetails>(
        `/api/projects/${projectId}/simulation/runs/${runId}`
    );

    return response.data;
}

// Fetch every persisted simulation attempt, including active, failed and cancelled runs.
export async function fetchSimulationHistory(projectId: string): Promise<SimulationHistoryItem[]> {
    const response = await httpClient.get<SimulationHistoryItem[]>(
        `/api/projects/${projectId}/simulation/history`
    );

    return response.data;
}

// Permanently deletes one terminal simulation run.
export async function deleteSimulationRun(projectId: string, runId: string): Promise<void> {
    await httpClient.delete(`/api/projects/${projectId}/simulation/runs/${runId}`);
}

// Updates the display unit used throughout the selected project.
export async function updateProjectDisplayUnit(projectId: string, displayUnit: DisplayLengthUnit): Promise<ProjectSummary> {
    const response = await httpClient.patch<ProjectSummary>(
        `/api/projects/${projectId}/settings/display-unit`, { displayUnit }
    );

    return response.data;
}

//Fetches the structural compatibility between two domain snapshots.
export async function fetchDomainSnapshotCompatibility(
    projectId: string, leftSnapshotId: string, rightSnapshotId: string
): Promise<DomainSnapshotCompatibility> {

    const response = await httpClient.get<DomainSnapshotCompatibility>(
        `/api/projects/${projectId}/domain/comparison/compatibility`,
        {
            params: { leftSnapshotId, rightSnapshotId },
        });

    return response.data;
}
