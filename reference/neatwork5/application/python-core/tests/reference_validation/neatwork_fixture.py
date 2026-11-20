from __future__ import annotations

from app.schemas.simulation_domain import SimulationDomainInput


HAZEN_WILLIAMS_C = 140.0
HW_FLOW_EXPONENT = 1.852
HW_DIAMETER_EXPONENT = 4.871
HW_SI_CONSTANT = 10.667

def hazen_williams_bescal(roughness_c: float) -> float:
    """
    Convert one uniform Hazen-Williams C-factor into
    the global beta scale used by the current
    NeatWork pipe resistance formula.
    """
    return HW_SI_CONSTANT * roughness_c ** (-HW_FLOW_EXPONENT)

def build_reference_neatwork_domain() -> SimulationDomainInput:
    """
    Build the NeatWork equivalent of reference_pda_hw.inp.
    """

    return (
        SimulationDomainInput
        .model_validate({
            "name": "reference_pda_hw",
            "sourceFormat": "EPANET_REFERENCE",
            "params": {
                # Force the NeatWork friction law to exactly match Hazen-Williams for one common C-factor.
                "lambda": HW_FLOW_EXPONENT,
                "nu": HW_DIAMETER_EXPONENT,
                "bescal": hazen_williams_bescal(HAZEN_WILLIAMS_C),
                # Match EPANET PDA: Pmin = 0, Preq = 20 m, exponent = 0.5
                "bounded_flow": 1,
                "uniform_dem": 1,
                "opentaps": 1.0,
                "targetflow": 1.0,
                "nbsim": 1,
            },
            "tables": {
                "nodes": [
                    {
                        "nodeId": "S1",
                        # For this controlled case, source datum represents the fixed reservoir hydraulic head.
                        "elevationM": 12.0,
                        "x": 0,
                        "y": 0,
                        "status": 0,
                    },
                    {
                        "nodeId": "J1",
                        "elevationM": 0.0,
                        "x": 1,
                        "y": 0,
                        "status": 1,
                    },
                    {
                        "nodeId": "P1",
                        "elevationM": 0.0,
                        "x": 2,
                        "y": 0,
                        "status": 2,
                    },
                ],
                "pipes": [
                    {
                        "edgeId": "E1",
                        "predecessorId": "S1",
                        "successorId": "J1",
                        "lengthM": 100.0,
                        "firstLengthM": 100.0,
                        "firstDiameterId": "D63",
                        "secondLengthM": 0.0,
                    },
                    {
                        "edgeId": "E2",
                        "predecessorId": "J1",
                        "successorId": "P1",
                        "lengthM": 100.0,
                        "firstLengthM": 100.0,
                        "firstDiameterId": "D63",
                        "secondLengthM": 0.0,
                    },
                ],
                "clients": [
                    {
                        "clientId": "C1",
                        "terminalId": "P1",
                        "openProbability": 1.0,
                        "targetFlowLps": 1.0,
                    },
                ],
                "bornes": [
                    {
                        "terminalId": "P1",
                        "requiredPressureM": 20.0,
                    },
                ],
                "diameters": [
                    {
                        "diameterId": "D63",
                        "nominalSize": "63 mm",
                        "innerDiameter": 0.063,
                        "roughness": HAZEN_WILLIAMS_C,
                    },
                ],
                "orifices": [],
            },
        })
    )