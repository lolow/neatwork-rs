from app.neatwork.parsers.dsg_parser import parse_dsg
from app.neatwork.parsers.json_parser import parse_json_graph
from app.neatwork.parsers.tpo_parser import parse_tpo
from app.neatwork.schemas import NeatworkFileFormat, NeatworkParseRequest, NeatworkParseResponse


def parse_neatwork(request: NeatworkParseRequest) -> NeatworkParseResponse:
    if request.format == NeatworkFileFormat.DSG:
        domain, graph = parse_dsg(request.name, request.primaryContent)
    elif request.format == NeatworkFileFormat.TPO:
        domain, graph = parse_tpo(
            request.name,
            request.primaryContent,
            request.diametersDbContent,
            request.orificesDbContent,
        )
    elif request.format == NeatworkFileFormat.JSON:
        domain, graph = parse_json_graph(request.name, request.primaryContent)
    else:
        raise ValueError(f"Unsupported format: {request.format}")

    return NeatworkParseResponse(domain=domain, graph=graph)