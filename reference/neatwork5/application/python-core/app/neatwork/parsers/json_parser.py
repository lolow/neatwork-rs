import json

from app.neatwork.schemas import (
    NeatworkDomainPayload,
    NeatworkFileFormat,
    NeatworkTables,
    VisualEdge,
    VisualNode,
    VisualGraph,
)


def parse_json_graph(name: str, content: str) -> tuple[NeatworkDomainPayload, VisualGraph]:
    payload = json.loads(content)

    if "graph" in payload and "domain" in payload:
        graph_payload = payload["graph"]
        domain_payload = payload["domain"]
    else:
        graph_payload = payload
        domain_payload = {}

    graph = VisualGraph(
        id=str(graph_payload.get("id", "imported-json")),
        name=str(graph_payload.get("name", name)),
        nodes=[VisualNode(**node) for node in graph_payload.get("nodes", [])],
        edges=[VisualEdge(**edge) for edge in graph_payload.get("edges", [])],
    )

    domain = NeatworkDomainPayload(
        name=graph.name,
        sourceFormat=NeatworkFileFormat.JSON,
        params= domain_payload.get("params", {}),
        tables=NeatworkTables(
            nodes=domain_payload.get("tables", {}).get("nodes", []),
            pipes=domain_payload.get("tables", {}).get("pipes", []),
            clients=domain_payload.get("tables", {}).get("clients", []),
            bornes=domain_payload.get("tables", {}).get("bornes", []),
            diameters=domain_payload.get("tables", {}).get("diameters", []),
            orifices=domain_payload.get("tables", {}).get("orifices", []),
        ),
        warnings=[]
    )

    return domain, graph
