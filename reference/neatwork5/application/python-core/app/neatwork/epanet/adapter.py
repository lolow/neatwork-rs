from __future__ import annotations

import hashlib
from collections import defaultdict

from pydantic import BaseModel

from app.neatwork.epanet.models import (
    EpanetAdapterOptions,
    EpanetDemand,
    EpanetDemandPolicy,
    EpanetIssueSeverity,
    EpanetMappingReport,
    EpanetSemanticModel,
)
from app.neatwork.epanet.units import EpanetUnitContext
from app.schemas.simulation_domain import SimulationDomainInput
from app.neatwork.domain.client_behavior import build_behavior_id

class EpanetAdaptationResult(BaseModel):
    domain: SimulationDomainInput
    report: EpanetMappingReport


def _diameter_catalog_id(
    *,
    diameter_m: float,
    roughness: float,
    headloss: str,
) -> str:
    """
    Build an order-independent diameter catalogue ID.
    Reordering the INP file must not change IDs or invalidate revisions.
    """
    identity = (
        f"{diameter_m:.12g}"
        "|"
        f"{roughness:.12g}"
        "|"
        f"{headloss.upper()}"
    )

    digest = hashlib.sha1(identity.encode("utf-8")).hexdigest()[:10].upper()

    return f"EPD_{digest}"


def _coordinate_lookup(model: EpanetSemanticModel) -> dict[str, tuple[float, float]]:
    """Return a node_id → (x, y) map from the semantic model."""
    return {
        coordinate.node_id: (coordinate.x, coordinate.y)
        for coordinate in model.coordinates
    }


def _all_demands(model: EpanetSemanticModel) -> list[EpanetDemand]:
    """
    Collect explicit [DEMANDS] records plus implicit junction base demands.
    Zero-demand junctions are skipped.
    """
    result = list(model.demands)

    for junction in model.junctions:
        if junction.base_demand == 0:
            continue
        result.append(
            EpanetDemand(
                node_id=junction.node_id,
                base_demand=junction.base_demand,
                pattern_id=junction.pattern_id,
                category="PRIMARY",
                source_line=junction.source_line,
            )
        )

    return result


def _is_validated_hw_subset(
    *,
    headloss_upper: str,
    roughness_values: set[float],
    has_minor_loss: bool,
) -> bool:
    """
    H-W subset is validated only if:
      - headloss == H-W
      - single roughness C across all pipes
      - no minor loss
    """
    return (
        headloss_upper == "H-W"
        and len(roughness_values) == 1
        and not has_minor_loss
    )


def adapt_epanet_to_neatwork(
    model: EpanetSemanticModel,
    *,
    options: EpanetAdapterOptions | None = None,
    name: str = "EPANET import prototype",
    enable_hw_param_mapping: bool = False,
) -> EpanetAdaptationResult:
    """
    Adapt an EPANET semantic model into a NeatWork SimulationDomainInput.

    enable_hw_param_mapping:
        keep False until the regression suite is green.
        When True and the H-W subset is validated, injects lambda/nu/bescal.
    """
    adapter_options = options or EpanetAdapterOptions()
    report = EpanetMappingReport()
    units = EpanetUnitContext.from_model(model)
    coordinates = _coordinate_lookup(model)

    # --------------------------------------------------------------
    # Headloss model policy
    # --------------------------------------------------------------
    headloss_upper = units.headloss.upper()
    roughness_values: set[float] = set()

    if headloss_upper == "H-W":
        roughness_values = {float(pipe.roughness) for pipe in model.pipes}
        if len(roughness_values) > 1:
            report.add_issue(
                code="VARIABLE_HW_ROUGHNESS_UNSUPPORTED",
                severity=EpanetIssueSeverity.BLOCKING,
                message=(
                    "NeatWork uses one global friction scale and cannot "
                    "exactly represent multiple H-W roughness values."
                ),
                section="PIPES",
            )
    elif headloss_upper == "D-W":
        report.add_issue(
            code="DW_HEADLOSS_MODEL_NOT_VALIDATED",
            severity=EpanetIssueSeverity.BLOCKING,
            message="Darcy-Weisbach headloss model is not yet validated.",
            section="OPTIONS",
        )
    elif headloss_upper == "C-M":
        report.add_issue(
            code="CM_HEADLOSS_MODEL_NOT_VALIDATED",
            severity=EpanetIssueSeverity.BLOCKING,
            message="Chezy-Manning headloss model is not yet validated.",
            section="OPTIONS",
        )
    else:
        report.add_issue(
            code="UNKNOWN_HEADLOSS_MODEL",
            severity=EpanetIssueSeverity.BLOCKING,
            message=f"Unsupported headloss model: {units.headloss!r}.",
            section="OPTIONS",
        )

    # --------------------------------------------------------------
    # PDA / DDA compatibility policy
    # --------------------------------------------------------------
    demand_model = str(model.options.get("DEMAND MODEL", "DDA")).upper()

    if demand_model in {"PDA", "PDD"}:
        minimum_pressure = float(model.options.get("MINIMUM PRESSURE", "0"))
        pressure_exponent = float(model.options.get("PRESSURE EXPONENT", "0.5"))

        if abs(minimum_pressure) > 1e-12:
            report.add_issue(
                code="PDA_MINIMUM_PRESSURE_UNSUPPORTED",
                severity=EpanetIssueSeverity.BLOCKING,
                message=(
                    "NeatWork bounded flow matches PDA only when "
                    "minimum pressure is zero."
                ),
                section="OPTIONS",
            )

        if abs(pressure_exponent - 0.5) > 1e-12:
            report.add_issue(
                code="PDA_PRESSURE_EXPONENT_UNSUPPORTED",
                severity=EpanetIssueSeverity.BLOCKING,
                message=(
                    "NeatWork uses a fixed quadratic exponent; custom PDA "
                    "pressure exponent is not supported."
                ),
                section="OPTIONS",
            )
    elif demand_model == "DDA":
        report.add_issue(
            code="DDA_NOT_SIMULATION_EQUIVALENT",
            severity=EpanetIssueSeverity.BLOCKING,
            message=(
                "DDA (fixed demand) is not equivalent to NeatWork bounded "
                "flow. Topology may be imported; simulation equivalence is "
                "not declared."
            ),
            section="OPTIONS",
        )
    else:
        report.add_issue(
            code="UNKNOWN_DEMAND_MODEL",
            severity=EpanetIssueSeverity.BLOCKING,
            message=f"Unsupported demand model: {demand_model!r}.",
            section="OPTIONS",
        )

    # --------------------------------------------------------------
    # Demands
    # --------------------------------------------------------------
    demand_records = _all_demands(model)
    demands_by_node: dict[str, list[EpanetDemand]] = defaultdict(list)

    for demand in demand_records:
        demands_by_node[demand.node_id].append(demand)

    # --------------------------------------------------------------
    # Nodes
    # --------------------------------------------------------------
    node_rows: list[dict] = []
    terminal_ids: set[str] = set()

    for junction in model.junctions:
        x, y = coordinates.get(junction.node_id, (0.0, 0.0))
        node_rows.append({
            "node_id": junction.node_id,
            "elevation_m": units.length_to_m(junction.elevation),
            "x": x,
            "y": y,
            "ideal_orifice_m": 0.0,
            "commercial_orifice_m": 0.0,
            # Becomes terminal (2) only if demand adaptation creates a client.
            "status": 1,
            "epanetBaseDemand": junction.base_demand,
            "epanetPatternId": junction.pattern_id,
        })

    report.mapped["junctions"] = len(model.junctions)

    for reservoir in model.reservoirs:
        x, y = coordinates.get(reservoir.node_id, (0.0, 0.0))
        node_rows.append({
            "node_id": reservoir.node_id,
            # Prototype: fixed head → source hydraulic datum (not yet DIRECT).
            "elevation_m": units.head_to_m(reservoir.head),
            "x": x,
            "y": y,
            "ideal_orifice_m": 0.0,
            "commercial_orifice_m": 0.0,
            "status": 0,
            "epanetReservoirHead": reservoir.head,
            "epanetPatternId": reservoir.pattern_id,
        })

        if reservoir.pattern_id is not None:
            report.add_issue(
                code="TIME_VARYING_RESERVOIR_HEAD",
                severity=EpanetIssueSeverity.BLOCKING,
                message="Reservoir head patterns are not supported.",
                section="RESERVOIRS",
                line_number=reservoir.source_line,
            )

    report.adapted["reservoirs"] = len(model.reservoirs)

    # --------------------------------------------------------------
    # Diameter catalogue + pipes
    # --------------------------------------------------------------
    diameter_rows: dict[str, dict] = {}
    pipe_diameter_ids: dict[str, str] = {}

    for pipe in model.pipes:
        diameter_m = units.pipe_diameter_to_m(pipe.diameter)
        diameter_id = _diameter_catalog_id(
            diameter_m=diameter_m,
            roughness=pipe.roughness,
            headloss=units.headloss,
        )
        pipe_diameter_ids[pipe.pipe_id] = diameter_id

        if diameter_id in diameter_rows:
            continue

        diameter_rows[diameter_id] = {
            "diameter_id": diameter_id,
            "nominal_size": (
                f"{pipe.diameter:g} "
                + ("in" if units.uses_us_units else "mm")
            ),
            "sdr": 0.0,
            "inner_diameter": diameter_m,
            "cost": 0.0,
            "maximum_pressure": 0.0,
            "type_code": "EPANET",
            # Preserved for traceability, not yet equivalent to NeatWork.
            "roughness": pipe.roughness,
            "epanetHeadloss": units.headloss,
            "epanetRoughnessRaw": pipe.roughness,
            "epanetDarcyRoughnessM": units.darcy_roughness_to_m(pipe.roughness),
        }

    pipe_rows: list[dict] = []

    for pipe in model.pipes:
        status = pipe.status.strip().upper()

        if status not in {"OPEN", "CLOSED", "CV"}:
            report.add_issue(
                code="UNKNOWN_PIPE_STATUS",
                severity=EpanetIssueSeverity.BLOCKING,
                message=f"Unsupported pipe status {pipe.status!r}.",
                section="PIPES",
                line_number=pipe.source_line,
            )

        if status == "CV":
            report.add_issue(
                code="CHECK_VALVE_NOT_SUPPORTED",
                severity=EpanetIssueSeverity.BLOCKING,
                message="Check-valve pipe semantics are not supported.",
                section="PIPES",
                line_number=pipe.source_line,
            )

        length_m = units.length_to_m(pipe.length)
        pipe_rows.append({
            "edge_id": pipe.pipe_id,
            "predecessor_id": pipe.start_node_id,
            "successor_id": pipe.end_node_id,
            "length_m": length_m,
            "first_length_m": length_m,
            "first_diameter_id": pipe_diameter_ids[pipe.pipe_id],
            "second_length_m": 0.0,
            "second_diameter_id": None,
            "enabled": status != "CLOSED",
            "operational_status": status,
            "epanetMinorLoss": pipe.minor_loss,
        })

    report.mapped["pipes"] = len(pipe_rows)

    # --------------------------------------------------------------
    # Minor losses validation
    # --------------------------------------------------------------
    has_minor_loss = any(abs(pipe.minor_loss) > 1e-12 for pipe in model.pipes)

    if has_minor_loss:
        report.add_issue(
            code="EPANET_MINOR_LOSS_UNSUPPORTED",
            severity=EpanetIssueSeverity.BLOCKING,
            message="Minor-loss coefficients are not represented by NeatWork.",
            section="PIPES",
        )

    report.adapted["diameterCatalogEntries"] = len(diameter_rows)

    # --------------------------------------------------------------
    # Clients
    # --------------------------------------------------------------
    client_rows: list[dict] = []
    client_behavior_rows: dict[str, dict] = {}

    if (
        demand_records
        and adapter_options.demand_policy
        == EpanetDemandPolicy.PRESERVE_UNRESOLVED
    ):
        report.add_issue(
            code="INTERMITTENT_PROBABILITY_UNRESOLVED",
            severity=EpanetIssueSeverity.BLOCKING,
            message=(
                f"{len(demand_records)} demands preserved, but EPANET has "
                "no client opening probability."
            ),
            section="DEMANDS",
        )
        report.preserved["demands"] = len(demand_records)

    if adapter_options.demand_policy == EpanetDemandPolicy.DETERMINISTIC_ALWAYS_OPEN:
        global_multiplier = float(model.options.get("DEMAND MULTIPLIER", "1"))
        default_pattern = model.options.get("PATTERN")

        for node_id, node_demands in demands_by_node.items():
            for index, demand in enumerate(node_demands, start=1):
                if demand.base_demand < 0:
                    report.add_issue(
                        code="NEGATIVE_DEMAND_NOT_CLIENT",
                        severity=EpanetIssueSeverity.BLOCKING,
                        message="Negative demand (inflow) cannot become a client.",
                        section="DEMANDS",
                        line_number=demand.source_line,
                    )
                    continue

                if demand.base_demand == 0:
                    continue

                effective_pattern = demand.pattern_id or default_pattern

                if effective_pattern is not None:
                    report.add_issue(
                        code="TIME_PATTERN_NOT_MAPPED",
                        severity=EpanetIssueSeverity.BLOCKING,
                        message=(
                            f"Demand at {node_id} uses a time pattern; "
                            "static client cannot preserve semantics."
                        ),
                        section="DEMANDS",
                        line_number=demand.source_line,
                    )

                target_flow_lps = (units.flow_to_lps(demand.base_demand * global_multiplier))
                behavior_id = (build_behavior_id(open_probability=1.0,
                                             target_flow_lps=target_flow_lps,
                                             profile_id=effective_pattern))


                if (behavior_id not in client_behavior_rows):
                    client_behavior_rows[behavior_id] = {"behaviorId": behavior_id,
                                                       "name": (demand.category or f"EPANET demand {node_id}"),
                                                       "openProbability": 1.0,
                                                       "targetFlowLps":target_flow_lps,
                                                       "profileId":effective_pattern}


                client_rows.append({
                        "client": f"EP_{node_id}_{index:02d}",
                        "borne":node_id,
                        "behaviorId":behavior_id})
                terminal_ids.add(node_id)

        report.adapted["deterministicClients"] = len(client_rows)

        if terminal_ids:
            for node in node_rows:
                if node["node_id"] in terminal_ids and node["status"] != 0:
                    node["status"] = 2  # terminal

    # --------------------------------------------------------------
    # Bornes (required pressure per terminal)
    # --------------------------------------------------------------
    borne_rows: list[dict] = []
    required_pressure_m: float | None = None
    epanet_required_pressure = model.options.get("REQUIRED PRESSURE")

    if epanet_required_pressure is not None:
        required_pressure_m = units.pressure_to_head_m(
            float(epanet_required_pressure)
        )
    elif adapter_options.default_required_pressure_m is not None:
        required_pressure_m = adapter_options.default_required_pressure_m

    if terminal_ids and required_pressure_m is None:
        report.add_issue(
            code="TERMINAL_PRESSURE_UNRESOLVED",
            severity=EpanetIssueSeverity.BLOCKING,
            message=(
                "Required pressure cannot be inferred; provide adapter "
                "default or EPANET REQUIRED PRESSURE."
            ),
        )
    elif required_pressure_m is not None:
        borne_rows = [
            {
                "terminal_id": terminal_id,
                "required_pressure_m": required_pressure_m,
            }
            for terminal_id in sorted(terminal_ids)
        ]

    # --------------------------------------------------------------
    # Emitters (preserved, unresolved)
    # --------------------------------------------------------------
    if model.emitters:
        report.preserved["emitters"] = len(model.emitters)
        report.add_issue(
            code="EMITTER_ORIFICE_MAPPING_UNRESOLVED",
            severity=EpanetIssueSeverity.BLOCKING,
            message="Emitter coefficients are not mapped to orifice diameters.",
            section="EMITTERS",
        )

    # --------------------------------------------------------------
    # Preserved / unsupported sections
    # --------------------------------------------------------------
    blocking_sections = {"TANKS", "PUMPS", "VALVES"}

    for section_name, raw_lines in model.preserved_sections.items():
        count = sum(
            1
            for line in raw_lines
            if line.strip() and not line.lstrip().startswith(";")
        )

        if section_name in blocking_sections and count > 0:
            report.unsupported[section_name] = count
            report.add_issue(
                code=f"UNSUPPORTED_{section_name}",
                severity=EpanetIssueSeverity.BLOCKING,
                message=(
                    f"[{section_name}] has {count} active record(s) not "
                    "representable by NeatWork."
                ),
                section=section_name,
            )
        elif count > 0:
            report.preserved[section_name] = count

    # --------------------------------------------------------------
    # Technical params (traceability)
    # --------------------------------------------------------------
    technical_params = {
        "epanet": {
            "units": units.flow_units,
            "headloss": units.headloss,
            "options": dict(model.options),
            "patterns": dict(model.patterns),
            "emitters": [emitter.model_dump() for emitter in model.emitters],
            "preservedSections": dict(model.preserved_sections),
        }
    }

    # --------------------------------------------------------------
    # H-W mapping (only when validated & enabled)
    # --------------------------------------------------------------
    domain_params: dict = {}

    if (
        enable_hw_param_mapping
        and _is_validated_hw_subset(
            headloss_upper=headloss_upper,
            roughness_values=roughness_values,
            has_minor_loss=has_minor_loss,
        )
    ):
        roughness_c = next(iter(roughness_values))
        domain_params.update({
            "lambda": 1.852,
            "nu": 4.871,
            "bescal": 10.667 * roughness_c ** (-1.852),
        })

    # --------------------------------------------------------------
    # Build domain
    # --------------------------------------------------------------
    domain = SimulationDomainInput.model_validate({
        "name": name,
        "sourceFormat": "EPANET",
        "params": domain_params,
        "technicalParams": technical_params,
        "designParams": {},
        "comments": [model.title] if model.title else [],
        "tables": {
            "nodes": node_rows,
            "pipes": pipe_rows,
            "clients": client_rows,
            "clientBehaviors":list(client_behavior_rows.values()),
            "bornes": borne_rows,
            "diameters": list(diameter_rows.values()),
            "orifices": [],
        },
    })

    report.mapped["coordinates"] = len(model.coordinates)
    if model.patterns:
        report.preserved["patterns"] = len(model.patterns)

    report.finalize()

    return EpanetAdaptationResult(domain=domain, report=report)