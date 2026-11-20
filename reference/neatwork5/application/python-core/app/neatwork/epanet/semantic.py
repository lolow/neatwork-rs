from __future__ import annotations

from app.neatwork.epanet.models import (
    EpanetCoordinate,
    EpanetDemand,
    EpanetEmitter,
    EpanetInpDocument,
    EpanetIssueSeverity,
    EpanetJunction,
    EpanetMappingIssue,
    EpanetPipe,
    EpanetReservoir,
    EpanetSemanticModel,
)

TYPED_SECTIONS = {
    "TITLE",
    "JUNCTIONS",
    "RESERVOIRS",
    "PIPES",
    "DEMANDS",
    "EMITTERS",
    "COORDINATES",
    "PATTERNS",
    "OPTIONS",
}

def _float_value(value: str, *, field_name: str,section: str,line_number: int,issues:list[EpanetMappingIssue]) -> float | None:
    try:
        return float(value)
    except (TypeError, ValueError):
        issues.append(EpanetMappingIssue(
                code="INVALID_NUMBER",
                severity=EpanetIssueSeverity.BLOCKING,
                message=f"Invalid numeric value for {field_name}: {value!r}",
                section=section,
                line_number=line_number,
            ))
        return None

OPTION_PREFIXES = (
    "MINIMUM PRESSURE",
    "REQUIRED PRESSURE",
    "PRESSURE EXPONENT",
    "DEMAND MULTIPLIER",
    "EMITTER EXPONENT",
    "DEMAND MODEL",
    "SPECIFIC GRAVITY",
    "UNITS",
    "HEADLOSS",
    "PATTERN",
)

def _parse_options(document: EpanetInpDocument) -> dict[str, str]:
    options = {
        "UNITS": "GPM",
        "HEADLOSS": "H-W",
    }

    section = document.section("OPTIONS")
    if section is None:
        return options
    for line in section.lines:
        upper_tokens = [token.upper() for token in line.tokens]
        matched = False
        for key in OPTION_PREFIXES:
            key_tokens = key.split()
            if upper_tokens[:len(key_tokens)] == key_tokens:
                value_tokens = line.tokens[len(key_tokens):]
                options[key] = " ".join(value_tokens)
                matched = True
                break
        if not matched:
            options[upper_tokens[0]] = " ".join(line.tokens[1:])
    return options

def parse_epanet_semantics(
    document: EpanetInpDocument,
) -> tuple[EpanetSemanticModel, list[EpanetMappingIssue]]:
    model = EpanetSemanticModel()
    issues: list[EpanetMappingIssue] = []
    model.options = _parse_options(document)
    title_section = document.section("TITLE")
    if title_section:
        model.title = "\n".join(line.data for line in title_section.lines)
    junction_section = document.section("JUNCTIONS")
    if junction_section:
        for line in junction_section.lines:
            if len(line.tokens) < 2:
                issues.append(
                    EpanetMappingIssue(
                        code= "INVALID_JUNCTION_ROW",
                        severity= EpanetIssueSeverity.BLOCKING,
                        message= "Junction rows require at least ID and elevation.",
                        section= "JUNCTIONS",
                        line_number=line.line_number,
                    ))
                continue
            elevation = _float_value(
                    line.tokens[1],
                    field_name="junction elevation",
                    section="JUNCTIONS",
                    line_number=line.line_number,
                    issues=issues,
                )
            if elevation is None:
                continue

            demand = 0.0

            if len(line.tokens) >= 3:
                parsed_demand = (_float_value(
                        line.tokens[2],
                        field_name="junction demand",
                        section="JUNCTIONS",
                        line_number=line.line_number,
                        issues=issues,
                    ))

                if parsed_demand is None:
                    continue
                demand = parsed_demand
            model.junctions.append(
                EpanetJunction(
                    node_id=line.tokens[0],
                    elevation=elevation,
                    base_demand=demand,
                    pattern_id=(
                        line.tokens[3]
                        if len(line.tokens) >= 4
                        else None
                    ),
                    source_line=line.line_number,
                )
            )

    reservoir_section = document.section("RESERVOIRS")
    if reservoir_section:
        for line in reservoir_section.lines:
            if len(line.tokens) < 2:
                continue
            head = _float_value(
                line.tokens[1],
                field_name="reservoir head",
                section="RESERVOIRS",
                line_number=line.line_number,
                issues=issues,
            )
            if head is None:
                continue
            model.reservoirs.append(
                EpanetReservoir(
                    node_id=line.tokens[0],
                    head=head,
                    pattern_id=(
                        line.tokens[2]
                        if len(line.tokens) >= 3
                        else None
                    ),
                    source_line=line.line_number,
                )
            )

    pipe_section = document.section("PIPES")
    if pipe_section:
        for line in pipe_section.lines:
            if len(line.tokens) < 6:
                issues.append(
                    EpanetMappingIssue(
                        code="INVALID_PIPE_ROW",
                        severity=EpanetIssueSeverity.BLOCKING,
                        message="Pipe rows require ID, two nodes, length, diameter and roughness.",
                        section="PIPES",
                        line_number=line.line_number,
                    )
                )
                continue
            length = _float_value(
                line.tokens[3],
                field_name="pipe length",
                section="PIPES",
                line_number=line.line_number,
                issues=issues,
            )

            diameter = _float_value(
                line.tokens[4],
                field_name="pipe diameter",
                section="PIPES",
                line_number=line.line_number,
                issues=issues,
            )

            roughness = _float_value(
                line.tokens[5],
                field_name="pipe roughness",
                section="PIPES",
                line_number=line.line_number,
                issues=issues,
            )

            if length is None or diameter is None or roughness is None:
                continue

            minor_loss = 0.0
            if len(line.tokens) >= 7:
                parsed_minor_loss = (_float_value(line.tokens[6],
                        field_name="minor loss coefficient",
                        section="PIPES",
                        line_number=line.line_number,
                        issues=issues,
                    ))
                if parsed_minor_loss is None:
                    continue
                minor_loss = parsed_minor_loss

            status = line.tokens[7].upper() if len(line.tokens) >= 8 else "OPEN"
            model.pipes.append(
                EpanetPipe(
                    pipe_id=line.tokens[0],
                    start_node_id=line.tokens[1],
                    end_node_id=line.tokens[2],
                    length=length,
                    diameter=diameter,
                    roughness=roughness,
                    minor_loss=minor_loss,
                    status=status,
                    source_line=line.line_number,
                ))

    demand_section = document.section("DEMANDS")
    if demand_section:
        for line in demand_section.lines:
            if len(line.tokens) < 2:
                continue

            base_demand = (_float_value(
                    line.tokens[1],
                    field_name="base demand",
                    section="DEMANDS",
                    line_number=line.line_number,
                    issues=issues,
                ))

            if base_demand is None:
                continue

            model.demands.append(
                EpanetDemand(
                    node_id=line.tokens[0],
                    base_demand=base_demand,
                    pattern_id=(
                        line.tokens[2] if len(line.tokens) >= 3 else None
                    ),
                    category=(
                        " ".join(line.tokens[3:]) if len(line.tokens) >= 4
                        else None
                    ),
                    source_line=line.line_number,
                ))

    emitter_section = document.section("EMITTERS")
    if emitter_section:
        for line in emitter_section.lines:
            if len(line.tokens) < 2:
                continue
            coefficient = _float_value(
                    line.tokens[1],
                    field_name="emitter coefficient",
                    section="EMITTERS",
                    line_number=line.line_number,
                    issues=issues,
                )
            if coefficient is None:
                continue
            model.emitters.append(
                EpanetEmitter(
                    node_id=line.tokens[0],
                    coefficient=coefficient,
                    source_line=line.line_number,
                )
            )

    coordinate_section = document.section("COORDINATES")
    if coordinate_section:
        for line in coordinate_section.lines:
            if len(line.tokens) < 3:
                continue
            x = _float_value(
                line.tokens[1],
                field_name="X coordinate",
                section="COORDINATES",
                line_number=line.line_number,
                issues=issues,
            )

            y = _float_value(
                line.tokens[2],
                field_name="Y coordinate",
                section="COORDINATES",
                line_number=line.line_number,
                issues=issues,
            )
            if x is None or y is None:
                continue
            model.coordinates.append(
                EpanetCoordinate(
                    node_id=line.tokens[0],
                    x=x,
                    y=y,
                    source_line=line.line_number,
                ))

    pattern_section = document.section("PATTERNS")
    if pattern_section:
        for line in pattern_section.lines:
            if not line.tokens:
                continue
            pattern_id = line.tokens[0]
            values:list[float] = []
            valid = True
            for token in line.tokens[1:]:
                value = _float_value(
                    token,
                    field_name="pattern multiplier",
                    section="PATTERNS",
                    line_number=line.line_number,
                    issues=issues,
                )
                if value is None:
                    valid = False
                    break
                values.append(value)
            if not valid:
                continue
            model.patterns.setdefault(pattern_id, []).extend(values)

    for section_name, section in document.sections.items():
        if section_name in TYPED_SECTIONS:
            continue
        model.preserved_sections[section_name] = list(section.raw_lines)
    
    return model, issues
    