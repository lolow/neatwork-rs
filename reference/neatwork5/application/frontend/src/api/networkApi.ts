import { httpClient } from "./httpClient";
import type { NetworkGraph } from "../features/network/types";

export async function fetchDemoNetworkGraph(): Promise<NetworkGraph> {
    const response = await httpClient.get<NetworkGraph>("/api/network/demo/graph");
    return response.data;

}