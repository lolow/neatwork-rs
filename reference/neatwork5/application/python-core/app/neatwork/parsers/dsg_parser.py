from collections import defaultdict
from app.neatwork.domain.client_behavior import normalize_client_behaviors

from app.neatwork.parsers.parsing_utils import (
    clean_lines,
    get_section,
    node_type_from_status,
    parse_params,
    rows_to_dicts,
    safe_float,
    split_csv_row,
)

from app.neatwork.schemas import (
    NeatworkDomainPayload,
    NeatworkFileFormat,
    NeatworkTables,
    VisualEdge,
    VisualNode,
    VisualGraph
)

DSG_DEFAULT_PARAMS = {
    "bescal": 8.9396e-04,
    "bounded-flow": 0,
    "faucetcoef": 1.0,
    "fusion": 0,
    "lambda": 0,
    "limitbudget": float("inf"),
    "nbsim": 100,
    "nu": 4.7810,
    "opentaps": 1,
    "orif_com": 1,
    "orifcoef": 1.0,
    "pipelength": 6.0,
    "pressure": 2,
    "secu": 1.0,
    "serviceequal": 0.65,
    "seuil": 0.0,
    "split": 0,
    "targetflow": 0.0,
    "uniform_dem": 1,
    "use_efficient_pipes": 0,
    "vmax": 1.5,
    "vmax_secu": 0.95,
    "watertemp": 20.0,
}

def parse_dsg(name: str, content: str) -> tuple[NeatworkDomainPayload, VisualGraph]:
    lines = clean_lines(content)
    warnings: list[str] = []
    comments = get_section(lines, "Commentaires")

    raw_nodes = rows_to_dicts(
        get_section(lines, "Nodes"),
        ["noeud", "haut", "xdata", "ydata", "orif_ideal", "orif_com", "statut"],
    )

    pipes = rows_to_dicts(
        get_section(lines, "Pipes"),
        ["pred", "succ", "lon", "lon1", "ID1", "lon2", "ID2", "N"],
    )

    orifices = _parse_dsg_orifices(get_section(lines, "Orifices"))

    clients = rows_to_dicts(
        get_section(lines, "Clients"),
        ["client", "borne", "proba", "cible"],
    )

    diameters = rows_to_dicts(
        get_section(lines, "Diameters"),
                ["ID", "Nominal", "SDR", "diam", "cost", "Pmax", "Type", "Roughness", "N"],
    )

    bornes = rows_to_dicts(
        get_section(lines, "Borne"),
        ["borne", "pressure", "N"],
    )

    if not bornes:
        bornes = _build_terminal_outlets_from_dsg_nodes(raw_nodes)

    params = {
        **DSG_DEFAULT_PARAMS,
        **parse_params(get_section(lines, "Default properties")),
    }

    if not raw_nodes:
        raise ValueError("DSG import failed: missing !Nodes section")
    if not pipes:
        raise ValueError("DSG import failed: missing !Pipes section")
    if not clients:
        warnings.append("DSG has no !Clients section. Simulation requires client demand data before running.")

    layout_nodes = _generate_layout_if_missing(
        [node.copy() for node in raw_nodes],
        pipes,
    )

    raw_nodes_by_id = {
        str(node.get("noeud")): node
        for node in raw_nodes
    }

    graph_nodes: list[VisualNode] = []

    for node in layout_nodes:
        node_id = str(node["noeud"])
        raw_node = raw_nodes_by_id.get(node_id, node)
        
        # coordinate source from file
        source_x = safe_float(raw_node.get("xdata"), 0.0)
        source_y = safe_float(raw_node.get("ydata"), 0.0)
    
        # coordinate visual from layout
        visual_x = safe_float(node.get("xdata"), 0.0)
        visual_y = safe_float(node.get("ydata"), 0.0)
    
        # checking if the layout has changed the coordinates
        layout_generated = (visual_x != source_x or visual_y != source_y)
    

        graph_nodes.append(
            VisualNode(
            id=node_id,
            label=node_id,
            type=node_type_from_status(node.get("statut", 1)),
            x=visual_x,
            y=visual_y,
            #pressureM=safe_float(node.get("haut"), 0.0),
            elevationM=safe_float(node.get("haut"), 0.0),
            pressureM=0.0,
            sourceX=source_x,  # coordinate source from file
            sourceY=source_y,  # coordinate source from file
            layoutGenerated=layout_generated,  # checking if the layout has changed the coordinates
        )
    )

    graph_edges = []

    for index, pipe in enumerate(pipes):
        diameter_row = _diameter_row_for_pipe(pipe, diameters)
        
        graph_edges.append(
        VisualEdge(
            id=f"A{index + 1}",
            sourceId=str(pipe["pred"]),
            targetId=str(pipe["succ"]),
            type="PIPE",
            status="OPEN",

            lengthM=safe_float(pipe.get("lon"), 0.0),
            
            diameterId=str(pipe.get("ID1")).strip() if pipe.get("ID1") else None,
            nominalSize=str(diameter_row.get("Nominal")) if diameter_row else None,
            sdr=safe_float(diameter_row.get("SDR"), 0.0) if diameter_row else None,
            innerDiameter=safe_float(diameter_row.get("diam"), 0.0) if diameter_row else None,
            cost=safe_float(diameter_row.get("cost"), 0.0) if diameter_row else None,
            pmax=safe_float(diameter_row.get("Pmax"), 0.0) if diameter_row else None,
            pipeTypecode=str(diameter_row.get("Type")) if diameter_row else None,
            roughness=safe_float(diameter_row.get("Roughness"), 0.0) if diameter_row else None,
        )
    )
    

    domain = NeatworkDomainPayload(
        name=name,
        sourceFormat=NeatworkFileFormat.DSG,
        params=params,
        technicalParams={},
        comments=comments,
        tables=NeatworkTables(
            nodes=raw_nodes,
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
        id="imported_dsg",
        name=name,
        nodes=graph_nodes,
        edges=graph_edges,
    )

def _build_terminal_outlets_from_dsg_nodes(nodes: list[dict]) -> list[dict]:
    outlets: list[dict] = []

    for node in nodes:
        if int(float(node.get("statut", 1))) != 2:
            continue

        outlets.append({
            "outlet": str(node["noeud"]),
            "elevation_m": safe_float(node.get("haut"), 0.0),
            "ideal_orifice_diameter_m": safe_float(node.get("orif_ideal"), 0.0),
            "commercial_orifice_diameter_m": safe_float(node.get("orif_com"), 0.0),
        })

    return outlets

def _parse_dsg_orifices(rows: list[str]) -> list[dict]:
    result: list[dict] = []

    for index, row in enumerate(rows):
        values = split_csv_row(row)

        if not values:
            continue

        diameter_m = safe_float(values[0], 0.0)

        result.append({
            "source_index": index + 1,
            "diameter_m": diameter_m,
            "diameter_mm": diameter_m * 1000,
        })

    return result

def _diameter_for_pipe(pipe: dict, diameters: list[dict]) -> float | None:
    id1= str(pipe.get("ID1", "")).strip()
    lookup = {str(row.get("ID")): row for row in diameters}

    if id1 in lookup:
        return safe_float(lookup[id1].get("diam"), 0.0)

    return None

def _generate_layout_if_missing(nodes: list[dict], pipes: list[dict]) -> list[dict]:
    has_coordinates = any(
        safe_float(node.get("xdata"), 0.0) != 0.0 or safe_float(node.get("ydata"), 0.0) != 0.0
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

        valid_children = [child for child in children[node_id] if child in node_by_id]

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
        
def _diameter_row_for_pipe(pipe:dict, diameters: list[dict]) -> dict | None:
    id1 = str(pipe.get("ID1", "")).strip()
    lookup = {str(row.get("ID")).strip(): row for row in diameters}
    return lookup.get(id1)