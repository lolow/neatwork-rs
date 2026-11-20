import { httpClient } from "./httpClient";

export type PythonCoreHealthResponse = {
    service: string;
    status: string;
    timestamp: string;
};

export async function fetchPythonCoreHealth(): Promise<PythonCoreHealthResponse> {
    const response = await httpClient.get<PythonCoreHealthResponse>("/api/python-core/health");
    return response.data;
}