from typing import Any

from app.neatwork.parsers.dsg_parser import parse_dsg
from app.neatwork.parsers.tpo_parser import parse_tpo
from app.neatwork.parsers.parsing_utils import (
    normalize_section_name,
)

from app.neatwork.schemas import (
    NeatworkDomainPayload,
    NeatworkFileFormat,
    NeatworkRevisionSerializeRequest,
    NeatworkRevisionSerializeResponse,
)
from app.neatwork.domain.client_behavior import materialize_serializable_clients
from app.schemas.simulation_domain import SimulationDomainInput
from app.neatwork.simulation.domain_adapter import build_simulation_data

def _value(value: Any) -> str:
    """
    Convert one domain value back to the textual
    representation expected by NeatWork files.
    """

    if value is None:
        return ""

    if isinstance(value, bool):
        return "1" if value else "0"

    if isinstance(value, float):
        if value.is_integer():
            return str(int(value))

    return str(value)


def _row(record: dict[str, Any], columns: list[str]) -> str:
    """
    Serialize one structured domain row using
    the exact NeatWork column order.
    """

    return ",".join(_value(record.get(column)) for column in columns )


def _is_simulation_seed_parameter(key: str,) -> bool:
    """
    Return True for legacy project-level seed keys.
    Random seed is execution metadata and must not be written into new DSG/TPO project revisions.
    """
    normalized = (
        key
        .strip()
        .lower()
        .replace("_", "")
        .replace("-", "")
        .replace(" ", "")
    )

    return normalized in {
        "seed",
        "randomseed",
    }

def _parameter_rows(domain: NeatworkDomainPayload) -> list[str]:
    """
    Serialize persistent project parameters.
    Execution-specific random seed values are deliberately excluded from new revisions.
    """
    return [
        f"{key},{_value(value)}"
        for key, value in domain.params.items()
        if not _is_simulation_seed_parameter(key)
    ]


def _borne_row(record: dict[str, Any]) -> str:
    """
    Support both explicit !Borne records and
    generated terminal-outlet representations.
    """

    borne = (record.get("borne") or record.get("outlet") or "" )
    pressure = (record.get("pressure") or record.get("required_pressure_m") or 0 )
    flag = record.get("N", "N")

    return ",".join([_value(borne), _value(pressure), _value(flag)])


def _dsg_sections(domain: NeatworkDomainPayload) -> dict[str, list[str] | None]:

    tables = domain.tables

    return {
        "commentaires": domain.comments,

        "nodes": [ _row(row,[
                    "noeud",
                    "haut",
                    "xdata",
                    "ydata",
                    "orif_ideal",
                    "orif_com",
                    "statut",
                ],
            )
            for row in tables.nodes
        ],

        "pipes": [ _row(row,[
                    "pred",
                    "succ",
                    "lon",
                    "lon1",
                    "ID1",
                    "lon2",
                    "ID2",
                    "N",
                ],
            )
            for row in tables.pipes
        ],

        "default properties": _parameter_rows(domain),

        "diameters": [ _row(row,[
                    "ID",
                    "Nominal",
                    "SDR",
                    "diam",
                    "cost",
                    "Pmax",
                    "Type",
                    "Roughness",
                    "N",
                ],
            )
            for row in tables.diameters
        ],

        "orifices": [",".join([_value(
                        row.get("diameter_m",
                            row.get("in_diam", 0))),
                    "N",
                    "N",
                    "N",
                    "N",
                    "N",
                ]
            )
            for row in tables.orifices
        ],

        "clients": [_row(row,[
                    "client",
                    "borne",
                    "proba",
                    "cible",
                ],
            )
            for row in materialize_serializable_clients(domain)
        ],

        "borne": [_borne_row(row) for row in tables.bornes],
    }


def _tpo_sections(domain: NeatworkDomainPayload) -> dict[str, list[str] | None]:

    tables = domain.tables

    return {
        "nodes": [_row(row,[
                    "noeud",
                    "haut",
                    "xdata",
                    "ydata",
                    "nbrob",
                    "statut",
                ],
            )
            for row in tables.nodes
        ],

        "pipes": [_row(row,[
                    "pred",
                    "succ",
                    "length",
                ],
            )
            for row in tables.pipes
        ],

        "default properties":_parameter_rows(domain),

        
        # TPO revisions are intentionally made self-contained. Diameter/orifice data
        # loaded from external DB files are embedded into the new revision.
        
        "diameters": [_row(row,[
                    "size",
                    "SDR",
                    "diam",
                    "cost",
                    "pressure",
                    "type",
                    "rugo",
                ],
            )
            for row in tables.diameters
        ],

        "orifices": [",".join([_value(
                        row.get("in_diam",
                            row.get("diameter_m",0))),
                    _value(row.get("N", "N")),
                ]
            )
            for row in tables.orifices
        ],

        "clients": [_row(row,[
                    "client",
                    "borne",
                    "proba",
                    "cible",
                ],
            )
            for row in materialize_serializable_clients(domain)
        ],

        "borne": [_borne_row(row) for row in tables.bornes],
    }


CANONICAL_SECTION_NAMES = {
    "commentaires": "Commentaires",
    "nodes": "Nodes",
    "pipes": "Pipes",
    "default properties": "Default properties",
    "diameters": "Diameters",
    "orifices": "Orifices",
    "clients": "Clients",
    "borne": "Borne",
}


def _merge_sections(source_content: str, replacements: dict[str, list[str] | None]) -> str:
    """
    Replace only sections understood by NW5.

    Unknown sections from the original file are
    preserved unchanged. This is important for
    legacy NeatWork data such as !loadfactor.
    """
    source_lines = source_content.splitlines()
    preamble: list[str] = []
    sections: list[ tuple[str, str, list[str]] ] = []

    current_header: str | None = None
    current_name: str | None = None
    current_body: list[str] = []

    for line in source_lines:
        stripped = line.strip()

        if stripped.startswith("!"):
            if current_header is not None:
                sections.append((current_header, current_name or "", current_body))
            current_header = line
            current_name = stripped[1:].strip()
            current_body = []

            continue

        if current_header is None:
            preamble.append(line)
        else:
            current_body.append(line)

    if current_header is not None:
        sections.append((current_header, current_name or "", current_body))

    output: list[str] = [*preamble]

    emitted: set[str] = set()

    for header, name, body in sections:

        normalized = normalize_section_name(name)
        replacement = replacements.get(normalized)

        if normalized not in replacements:
            output.append(header)
            output.extend(body)
            continue

         # None means that the structured snapshot
         # does not own this section. Preserve the exact original content.
         
        if replacement is None:
            output.append(header)
            output.extend(body)
            continue

        output.append(f"!{CANONICAL_SECTION_NAMES[normalized]}")
        output.extend(replacement)
        emitted.add(normalized)

    for normalized, replacement in replacements.items():

        if normalized in emitted or replacement is None or len(replacement) == 0:
            continue

        output.append(f"!{CANONICAL_SECTION_NAMES[normalized]}")
        output.extend(replacement)


    return "\n".join(output).rstrip() + "\n"

def _restore_client_behavior_names(edited_domain: NeatworkDomainPayload,parsed_domain: NeatworkDomainPayload) -> NeatworkDomainPayload:
    """
    Preserve UI-level client behavior names after DSG/TPO serialization and re-parsing.
    Legacy NeatWork source formats store probability and target flow but do not persist the canonical behavior display name.
    """
    edited_names = {
        str(row.get("behaviorId")):
            row.get("name")
        for row
        in edited_domain.tables.clientBehaviors
        if row.get("behaviorId")
    }

    result = parsed_domain.model_copy(deep=True)

    for behavior in result.tables.clientBehaviors:
        behavior_id = str(behavior.get("behaviorId","" ))

        if behavior_id in edited_names:
            behavior["name"] = edited_names[behavior_id]

    return result

def serialize_revision(request: NeatworkRevisionSerializeRequest,) -> NeatworkRevisionSerializeResponse:
    """
    Serialize edited domain data and immediately parse the generated file again.

    Re-parsing guarantees that no invalid revision is persisted by the Java backend.
    """
    _validate_revision_domain(request.domain)

    if request.format == NeatworkFileFormat.DSG:

        replacements = _dsg_sections(request.domain)

        content = _merge_sections(request.sourceContent,replacements)

        domain, graph = parse_dsg(request.name,content)
        domain = _restore_client_behavior_names(request.domain,domain)
        _validate_revision_domain(domain)

    elif request.format == NeatworkFileFormat.TPO:

        replacements = _tpo_sections(request.domain)

        content = _merge_sections(request.sourceContent,replacements)

        domain, graph = parse_tpo(request.name,content,None,None)
        domain = _restore_client_behavior_names(request.domain,domain)
        _validate_revision_domain(domain)

    else:
        raise ValueError("Only DSG and TPO revisions are supported.")

    return NeatworkRevisionSerializeResponse(
        content=content,
        domain=domain,
        graph=graph,
    )

def _validate_revision_domain(domain) -> None:
    """
    Validate an edited revision through the same typed hydraulic domain used by simulation.
    """
    typed_domain = (
        SimulationDomainInput
        .model_validate(domain.model_dump(mode="json", by_alias=True))
    )

    # Building SimulationData validates numeric fields, client demand, references, topology and units.
    build_simulation_data(typed_domain)