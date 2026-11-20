from app.neatwork.parsers.dsg_parser import _generate_layout_if_missing
from pydantic import annotated_handlers
from fastapi import encoders
from app.neatwork.parsers.db_parser import parse_diameters_db, parse_orifices_db
from app.neatwork.parsers.parsing_utils import (
    clean_lines,
    get_section,
    node_type_from_status,
    parse_params,
    rows_to_dicts,
    safe_float,
)
from app.neatwork.schemas import (
    NeatworkDomainPayload,
    NeatworkFileFormat,
    NeatworkTables,
    VisualEdge,
    VisualGraph,
    VisualNode,
)

from collections import defaultdict
from app.neatwork.domain.client_behavior import normalize_client_behaviors


TECHNICAL_DEFAULTS = {
    "bescal": 8.9396e-04,
    "faucetcoef": 1.0,
    "lambda": 1.7810,
    "nu": 4.7810,
    "orifcoef": 1.0,
}

DESIGN_DEFAULTS = {
    "bounded_flow": 0,
    "fusion": 0,
    "limitbudget": float("inf"),
    "opentaps": 1,
    "pipelength": 6.0,
    "pressure": 2,
    "secu": 1.0,
    "servicequal": 0.65,
    "seuil": 0.0,
    "split": 0,
    "targetflow": 0.0,
    "uniform_dem": 1,
    "use_efficient_pipes": 0,
    "vmax": 1.5,
    "vmax_secu": 0.95,
    "watertemp": 20.0,
}


def parse_tpo(
    name: str,
    content: str,
    diameters_db_content: str | None,
    orifices_db_content: str | None,
) -> tuple[NeatworkDomainPayload, VisualGraph]:
    lines = clean_lines(content)
    warnings: list[str] = []

    nodes = rows_to_dicts(
        get_section(lines, "Nodes"),
        ["noeud", "haut", "xdata", "ydata", "nbrob", "statut"],
    )

    pipes = rows_to_dicts(
        get_section(lines, "Pipes"),
        ["pred", "succ", "length"],
    )

    default_params = parse_params(get_section(lines, "Default properties"))
    technical_params = {
        key: default_params.get(key, value)
        for key, value in TECHNICAL_DEFAULTS.items()
    }

    design_params = {
        key: default_params.get(key, value)
        for key, value in DESIGN_DEFAULTS.items()
    }

    clients = rows_to_dicts(
        get_section(lines, "Clients"),
        ["client", "borne", "proba", "cible"],
    )

    bornes = rows_to_dicts(
        get_section(lines, "Borne"),
        ["borne", "pressure", "N"],
    )

    diameters = rows_to_dicts(
        get_section(lines, "Diameters"),
        ["size", "SDR", "diam", "cost", "pressure", "type", "rugo"],
    )

    if not diameters:
        if not diameters_db_content:
            raise ValueError("TPO import failed: missing !Diameters and diameters.db.")
        diameters = parse_diameters_db(diameters_db_content)

    for index, row in enumerate(diameters):
        row.setdefault("ID", f"D{index + 1:03d}")

    orifices = rows_to_dicts(
        get_section(lines, "Orifices"),
        ["in_diam", "N"],
    )

    if not orifices:
        if orifices_db_content:
            orifices = parse_orifices_db(orifices_db_content)
        else:
            warnings.append("TPO has no !Orifices and no orifices.db was provided.")

    if not clients:
        clients = _build_clients_from_nodes(nodes, design_params)
        warnings.append(f"TPO has no !Clients section. {len(clients)} clients were generated "
            "from terminal nodes where statut = 2 using nbrob.")

    if not bornes:
        bornes = _build_bornes_from_nodes(nodes, design_params)
        warnings.append(f"TPO has no !Borne section. {len(bornes)} bornes were generated "
            "from terminal nodes where statut = 2.")

    _validate_graph(nodes, pipes, bornes)

    layout_nodes = _generate_layout_if_missing(
        [node.copy() for node in nodes],
        pipes,
    )

    raw_nodes_by_id = {
        str(node.get("noeud")): node
        for node in nodes
    }

    graph_nodes: list[VisualNode] = []
    for node in layout_nodes:
        node_id = str(node["noeud"])
        raw_node = raw_nodes_by_id.get(node_id, node)
        
        # coordinate source from file
        source_x = safe_float(raw_node.get("xdata"), 0.0)
        source_y = safe_float(raw_node.get("ydata"), 0.0)
        
        # coordinate visual (in TPO usually the same as source coordinates)
        visual_x = safe_float(node.get("xdata"), 0.0)
        visual_y = safe_float(node.get("ydata"), 0.0)
        
        # checking if the layout has changed the coordinates
        layout_generated = (visual_x != source_x) or (visual_y != source_y)

        graph_nodes.append(
            VisualNode(
                id=str(node["noeud"]),
                label=str(node["noeud"]),
                type=node_type_from_status(node.get("statut", 1)),
                x=visual_x,
                y=visual_y,
                pressureM=safe_float(node.get("haut"), 0.0),
                sourceX=source_x,  # coordinate source from file
                sourceY=source_y,  # coordinate source from file
                layoutGenerated=layout_generated,  # checking if the layout has changed the coordinates
            )
        )

    graph_edges = [
        VisualEdge(
            id=f"A{index + 1}",
            sourceId=str(pipe["pred"]),
            targetId=str(pipe["succ"]),
            type="VALVE" if safe_float(pipe.get("length"), 0.0) <= 0 else "PIPE",
            status="OPEN",
            lengthM=max(0.01, safe_float(pipe.get("length"), 0.0)),
        )
        for index, pipe in enumerate(pipes)
    ]

    domain = NeatworkDomainPayload(
        name=name,
        sourceFormat=NeatworkFileFormat.TPO,
        params={**technical_params, **design_params},
        technicalParams=technical_params,
        designParams=design_params,
        tables=NeatworkTables(
            nodes=nodes,
            pipes=pipes,
            edges=pipes,
            clients=clients,
            bornes=bornes,
            diameters=diameters,
            orifices=orifices,
        ),
        warnings=warnings,
    )

    domain = normalize_client_behaviors(domain)

    return domain, VisualGraph(
        id="imported-tpo",
        name=name,
        nodes=graph_nodes,
        edges=graph_edges,
    )

def _generate_layout_if_missing(nodes: list[dict], pipes: list[dict]) -> list[dict]:
    has_coordinates = any(
        safe_float(node.get("xdata"), 0.0) != 0.0
        or safe_float(node.get("ydata"), 0.0) != 0.0
        for node in nodes
    )

    if has_coordinates:
        return nodes

    node_by_id = {str(node["noeud"]): node for node in nodes}
    children: dict[str, list[str]] = defaultdict(list)
    incoming: set[str] = set()

    for pipe in pipes:
        pred = str(pipe["pred"])
        succ = str(pipe["succ"])

        if pred in node_by_id and succ in node_by_id:
            children[pred].append(succ)
            incoming.add(succ)

    roots = [
        str(node["noeud"])
        for node in nodes
        if int(float(node.get("statut", "1"))) == 0
    ]

    if not roots:
        roots = [node_id for node_id in node_by_id if node_id not in incoming]

    if not roots and nodes:
        roots = [str(nodes[0]["noeud"])]

    visited: set[str] = set()
    y_counter = 0

    def place(node_id: str, depth: int) -> float:
        nonlocal y_counter

        if node_id in visited:
            return safe_float(node_by_id[node_id].get("ydata"), 0.0)

        visited.add(node_id)

        valid_children = [
            child for child in children[node_id]
            if child in node_by_id
        ]

        if not valid_children:
            y = y_counter * 80.0
            y_counter += 1
        else:
            child_ys = [place(child, depth + 1) for child in valid_children]
            y = sum(child_ys) / len(child_ys)

        node_by_id[node_id]["xdata"] = depth * 160.0
        node_by_id[node_id]["ydata"] = y

        return y

    for root in roots:
        place(root, 0)

    for node_id in node_by_id:
        if node_id not in visited:
            place(node_id, 0)

    return nodes

def _build_clients_from_nodes(nodes: list[dict], design_params: dict) -> list[dict]:
    clients: list[dict] = []
    counter = 0

    for node in nodes:
        if int(float(node.get("statut", 1))) != 2:
            continue

        taps = int(float(node.get("nbrob", 0)))
        for index in range(taps):
            counter += 1
            clients.append({
                "client": f"C{counter}",
                "borne": str(node["noeud"]),
                "proba": design_params["opentaps"],
                "cible": design_params["targetflow"],
            })

    return clients


def _build_bornes_from_nodes(nodes: list[dict], design_params: dict) -> list[dict]:
    return [
        {
            "borne": str(node["noeud"]),
            "pressure": design_params["pressure"],
            "N": "",
        }
        for node in nodes
        if int(float(node.get("statut", 1))) == 2
    ]


def _validate_graph(nodes: list[dict], pipes: list[dict], bornes: list[dict]) -> None:
    node_ids = {str(node["noeud"]) for node in nodes}
    pipe_ids = {str(pipe["pred"]) for pipe in pipes} | {str(pipe["succ"]) for pipe in pipes}
    borne_ids = {str(borne["borne"]) for borne in bornes}

    unknown_pipe_nodes = pipe_ids - node_ids
    if unknown_pipe_nodes:
        raise ValueError(f"TPO import failed: unknown pipe nodes {sorted(unknown_pipe_nodes)}")

    unknown_bornes = borne_ids - node_ids
    if unknown_bornes:
        raise ValueError(f"TPO import failed: unknown bornes {sorted(unknown_bornes)}")

    for pipe in pipes:
        if str(pipe["pred"]) == str(pipe["succ"]):
            raise ValueError(f"TPO import failed: self-loop detected on node {pipe['pred']}")