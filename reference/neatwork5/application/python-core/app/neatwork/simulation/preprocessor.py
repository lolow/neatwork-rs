"""Hydraulic preprocessing for simulation-ready network data."""

from typing import TYPE_CHECKING, Any

import numpy as np
import pandas as pd
from scipy import sparse

from .models import PreparedHydraulicNetwork
from app.neatwork.hydraulic_units import lps_to_m3s

if TYPE_CHECKING:
    from .data import SimulationData


class HydraulicPreprocessor:
    """
    Convert validated raw simulation tables into solver-ready data.

    The preprocessor works on deep copies. The SimulationData instance
    remains unchanged if preprocessing fails.
    """

    def __init__(self, instance: "SimulationData") -> None:
        self.instance = instance
        self.params = instance.param

        self.nodes = instance.nodes.copy(deep=True)
        self.pipes = instance.pipes.copy(deep=True)
        self.clients = instance.clients.copy(deep=True)
        self.bornes = instance.bornes.copy(deep=True)
        self.diameters = instance.diameters.copy(deep=True)

    def prepare(self) -> PreparedHydraulicNetwork:
        self._normalize_table_types()
        self._classify_pipe_statuses()
        self._compute_gravity_terms()
        self._map_pipe_diameters_and_costs()
        self._compute_pipe_beta()
        self._compute_pipe_total_cost()
        self._compute_terminal_alpha()
        self._sort_and_align_tables()

        incidence_matrix = self._build_incidence_matrix()
        solver_data = self._build_solver_data(incidence_matrix)

        self._check_prepared_consistency(solver_data)

        return PreparedHydraulicNetwork(
            nodes=self.nodes,
            pipes=self.pipes,
            clients=self.clients,
            bornes=self.bornes,
            diameters=self.diameters,
            solver_data=solver_data,
        )
    
    def _normalize_table_types(self) -> None:
        """Normalize identifiers and numerical columns before calculations."""

        self.nodes["noeud"] = (self.nodes["noeud"].astype("string").str.strip().astype(str))

        node_numeric_columns = [
            "haut",
            "xdata",
            "ydata",
            "orif_ideal",
            "orif_com",
        ]

        self.nodes[node_numeric_columns] = self.nodes[node_numeric_columns].apply(pd.to_numeric)

        self.nodes["statut"] = pd.to_numeric(self.nodes["statut"]).astype(int)

        self.pipes["external_edge_id"] = (self.pipes["external_edge_id"].astype("string").str.strip().astype(str))

        self.pipes["pred"] = (self.pipes["pred"].astype("string").str.strip().astype(str))

        self.pipes["succ"] = (self.pipes["succ"].astype("string").str.strip().astype(str))

        self.pipes[["lon", "lon1", "lon2"]] = self.pipes[
            ["lon", "lon1", "lon2"]
        ].apply(pd.to_numeric)

        self.pipes["ID1"] = self.pipes["ID1"].astype("string")
        self.pipes["ID2"] = self.pipes["ID2"].astype("string")

        if "is_open" not in self.pipes.columns:
            self.pipes["is_open"] = True
        else:
            normalized_open_values: list[bool] = []

            for value in self.pipes["is_open"]:
                if isinstance(value, bool):
                    normalized_open_values.append(value)
                    continue

                if isinstance(value, (int, np.integer)):
                    normalized_open_values.append(bool(value))
                    continue

                normalized_value = str(value).strip().upper()

                if normalized_value in {
                    "TRUE",
                    "1",
                    "YES",
                    "OPEN",
                    "ENABLED",
                    "ON",
                }:
                    normalized_open_values.append(True)
                elif normalized_value in {
                    "FALSE",
                    "0",
                    "NO",
                    "CLOSED",
                    "DISABLED",
                    "OFF",
                    "INACTIVE",
                }:
                    normalized_open_values.append(False)
                else:
                    raise ValueError(
                        "Pipe column 'is_open' contains "
                        f"unsupported value '{value}'."
                    )

            self.pipes["is_open"] = normalized_open_values

        self.pipes["is_open"] = self.pipes["is_open"].astype(bool)

        self.clients["client"] = (self.clients["client"].astype("string").str.strip().astype(str))

        self.clients["borne"] = (self.clients["borne"].astype("string").str.strip().astype(str))

        self.clients[["proba", "cible"]] = self.clients[
            ["proba", "cible"]
        ].apply(pd.to_numeric)

        self.bornes["borne"] = (self.bornes["borne"].astype("string").str.strip().astype(str))

        self.bornes["pressure"] = pd.to_numeric(self.bornes["pressure"])

        self.diameters["ID"] = (self.diameters["ID"].astype("string").str.strip().astype(str))

        diameter_numeric_columns = [
            "SDR",
            "diam",
            "cost",
            "Pmax",
            "Type",
            "Roughness",
        ]

        existing_numeric_columns = [
            column
            for column in diameter_numeric_columns
            if column in self.diameters.columns
        ]

        self.diameters[existing_numeric_columns] = self.diameters[existing_numeric_columns].apply(pd.to_numeric)

    def _classify_pipe_statuses(self) -> None:
        """
        Assign hydraulic status to every pipe.

        Status:
            0 = source pipe
            1 = transit pipe
            2 = terminal pipe
        """

        source_ids = set(self.nodes.loc[self.nodes["statut"] == 0,"noeud"])

        terminal_ids = set(self.nodes.loc[self.nodes["statut"] == 2,"noeud"])

        self.pipes["statut"] = 1

        source_pipe_mask = self.pipes["pred"].isin(source_ids)
        terminal_pipe_mask = self.pipes["succ"].isin(terminal_ids)

        self.pipes.loc[source_pipe_mask, "statut"] = 0
        self.pipes.loc[terminal_pipe_mask, "statut"] = 2

        self.pipes["statut"] = self.pipes["statut"].astype(int)


    def _compute_gravity_terms(self) -> None:
        """
        Compute the elevation contribution used in the energy objective.

        Source elevation is subtracted on source pipes.
        Terminal elevation is added on terminal pipes.
        """

        self.pipes["gravit"] = 0.0

        source_heights = self.nodes.loc[self.nodes["statut"] == 0,["noeud", "haut"],].set_index("noeud")["haut"]

        terminal_heights = self.nodes.loc[self.nodes["statut"] == 2,["noeud", "haut"],].set_index("noeud")["haut"]

        source_pipe_mask = self.pipes["statut"] == 0
        terminal_pipe_mask = self.pipes["statut"] == 2

        self.pipes.loc[source_pipe_mask, "gravit"] = (
            -self.pipes.loc[
                source_pipe_mask,
                "pred",
            ].map(source_heights)
        )

        self.pipes.loc[terminal_pipe_mask, "gravit"] = (
            self.pipes.loc[
                terminal_pipe_mask,
                "succ",
            ].map(terminal_heights)
        )

        self.pipes["gravit"] = pd.to_numeric(self.pipes["gravit"]).fillna(0.0)

    def _map_pipe_diameters_and_costs(self) -> None:
        """Map ID1 and ID2 to physical diameter and unit cost."""

        diameter_by_id = self.diameters.set_index("ID")

        diameter_map = diameter_by_id["diam"]
        cost_map = diameter_by_id["cost"]

        first_segment_mask = self.pipes["lon1"] > 0
        second_segment_mask = self.pipes["lon2"] > 0

        # Inactive segments receive neutral finite values.
        # Their values are not used in beta or cost calculations.
        self.pipes["diam1"] = 1.0
        self.pipes["diam2"] = 1.0
        self.pipes["cost1"] = 0.0
        self.pipes["cost2"] = 0.0

        self.pipes.loc[first_segment_mask, "diam1"] = (
            self.pipes.loc[first_segment_mask, "ID1"]
            .astype(str)
            .map(diameter_map)
        )

        self.pipes.loc[first_segment_mask, "cost1"] = (
            self.pipes.loc[first_segment_mask, "ID1"]
            .astype(str)
            .map(cost_map)
        )

        self.pipes.loc[second_segment_mask, "diam2"] = (
            self.pipes.loc[second_segment_mask, "ID2"]
            .astype(str)
            .map(diameter_map)
        )

        self.pipes.loc[second_segment_mask, "cost2"] = (
            self.pipes.loc[second_segment_mask, "ID2"]
            .astype(str)
            .map(cost_map)
        )

        hydraulic_columns = [
            "diam1",
            "diam2",
            "cost1",
            "cost2",
        ]

        if not np.isfinite(
            self.pipes[hydraulic_columns].to_numpy(dtype=float)
        ).all():
            raise ValueError(
                "Pipe diameter or cost mapping produced invalid values."
            )

        active_first_diameters = self.pipes.loc[first_segment_mask,"diam1"]

        active_second_diameters = self.pipes.loc[second_segment_mask,"diam2"]

        if (active_first_diameters <= 0).any():
            raise ValueError(
                "Every active first pipe segment must have "
                "a strictly positive diameter."
            )

        if (active_second_diameters <= 0).any():
            raise ValueError(
                "Every active second pipe segment must have "
                "a strictly positive diameter."
            )

    def _compute_pipe_beta(self) -> None:
        """Compute hydraulic friction coefficient beta for each pipe."""

        bescal = float(self.params["bescal"])
        nu = float(self.params["nu"])

        first_segment_mask = self.pipes["lon1"] > 0
        second_segment_mask = self.pipes["lon2"] > 0

        self.pipes["beta"] = 0.0

        self.pipes.loc[first_segment_mask, "beta"] = (
            bescal
            * self.pipes.loc[first_segment_mask, "lon1"]
            / (
                self.pipes.loc[first_segment_mask, "diam1"]
                ** nu
            )
        )

        self.pipes.loc[second_segment_mask, "beta"] += (
            bescal
            * self.pipes.loc[second_segment_mask, "lon2"]
            / (
                self.pipes.loc[second_segment_mask, "diam2"]
                ** nu
            )
        )

        if not np.isfinite(
            self.pipes["beta"].to_numpy(dtype=float)
        ).all():
            raise ValueError(
                "Hydraulic beta calculation produced invalid values."
            )

        if (self.pipes["beta"] <= 0).any():
            invalid_pipe = self.pipes.loc[
                self.pipes["beta"] <= 0,
                "external_edge_id",
            ].iloc[0]

            raise ValueError(
                f"Pipe '{invalid_pipe}' has a non-positive "
                "hydraulic beta coefficient."
            )


    def _compute_pipe_total_cost(self) -> None:
        """Compute optional investment cost for every pipe."""

        self.pipes["total_cost"] = (
            self.pipes["lon1"] * self.pipes["cost1"]
            + self.pipes["lon2"] * self.pipes["cost2"]
        )

        self.pipes["total_cost"] = pd.to_numeric(
            self.pipes["total_cost"]
        ).fillna(0.0)

    def _compute_terminal_alpha(self) -> None:
        """Compute static terminal resistance coefficients."""

        successor_nodes = self.nodes.set_index("noeud")

        self.pipes["orif_ideal"] = (self.pipes["succ"].map(successor_nodes["orif_ideal"]).fillna(0.0))

        self.pipes["orif_com"] = (self.pipes["succ"].map(successor_nodes["orif_com"]).fillna(0.0))

        self.pipes["alpha_ideal"] = 0.0
        self.pipes["alpha_com"] = 0.0
        self.pipes["alpha"] = 0.0

        terminal_pipe_mask = self.pipes["statut"] == 2

        faucet_coefficient = float(self.params["faucetcoef"])

        faucet_alpha = (
            1.0 / faucet_coefficient
            if faucet_coefficient > 0
            else 0.0
        )

        orifice_coefficient = float(self.params["orifcoef"])

        valid_ideal_mask = (
            terminal_pipe_mask
            & (self.pipes["orif_ideal"] > 0)
        )

        valid_commercial_mask = (
            terminal_pipe_mask
            & (self.pipes["orif_com"] > 0)
        )

        self.pipes.loc[valid_ideal_mask,"alpha_ideal"] = (
            (
                orifice_coefficient
                / self.pipes.loc[
                    valid_ideal_mask,
                    "orif_ideal",
                ]
            )
            ** 4
            + faucet_alpha
        )

        self.pipes.loc[valid_commercial_mask,"alpha_com"] = (
            (
                orifice_coefficient
                / self.pipes.loc[
                    valid_commercial_mask,
                    "orif_com",
                ]
            )
            ** 4
            + faucet_alpha
        )

        if "ecart" in self.pipes.columns:
            target_flow_m3s = lps_to_m3s(float(self.params["targetflow"]))    

            if target_flow_m3s <= 0:
                raise ValueError(
                    "Parameter 'targetflow' must be positive "
                    "when pipe column 'ecart' is present."
                )

            self.pipes["alpha"] = (
                pd.to_numeric(
                    self.pipes["ecart"],
                    errors="coerce",
                ).fillna(0.0)
                / target_flow_m3s**2
            )
        else:
            self.pipes["alpha"] = self.pipes["alpha_ideal"]

        alpha_columns = [
            "alpha_ideal",
            "alpha_com",
            "alpha",
        ]

        if not np.isfinite(
            self.pipes[alpha_columns].to_numpy(dtype=float)
        ).all():
            raise ValueError(
                "Terminal alpha calculation produced invalid values."
            )

    def _sort_and_align_tables(self) -> None:
        """Sort tables while preserving stable external identifiers."""

        self.nodes = self.nodes.sort_values(
            by=["statut", "noeud"],
            kind="stable",
        ).reset_index(drop=True)

        self.pipes = self.pipes.sort_values(
            by=[
                "statut",
                "succ",
                "pred",
                "external_edge_id",
            ],
            kind="stable",
        ).reset_index(drop=True)

        self.clients = self.clients.sort_values(
            by=["borne", "client"],
            kind="stable",
        ).reset_index(drop=True)

        self.bornes = self.bornes.sort_values(
            by=["borne"],
            kind="stable",
        ).reset_index(drop=True)

        self.diameters = self.diameters.sort_values(
            by=["diam", "ID"],
            kind="stable",
        ).reset_index(drop=True)

    def _build_incidence_matrix(self) -> sparse.csr_matrix:
        """
        Build the flow-conservation incidence matrix.

        Rows correspond exclusively to transit nodes.
        Columns correspond to all pipes.

        For each pipe:
            predecessor contributes -1
            successor contributes +1
        """

        transit_nodes = self.nodes.loc[self.nodes["statut"] == 1,"noeud"]

        transit_index = pd.Index(transit_nodes)

        predecessor_indices = transit_index.get_indexer(self.pipes["pred"])

        successor_indices = transit_index.get_indexer(self.pipes["succ"])

        predecessor_mask = predecessor_indices != -1
        successor_mask = successor_indices != -1

        row_indices = np.concatenate(
            [
                predecessor_indices[predecessor_mask],
                successor_indices[successor_mask],
            ]
        )

        column_indices = np.concatenate(
            [
                np.flatnonzero(predecessor_mask),
                np.flatnonzero(successor_mask),
            ]
        )

        values = np.concatenate(
            [
                -np.ones(
                    predecessor_mask.sum(),
                    dtype=float,
                ),
                np.ones(
                    successor_mask.sum(),
                    dtype=float,
                ),
            ]
        )

        matrix = sparse.coo_matrix(
            (
                values,
                (
                    row_indices,
                    column_indices,
                ),
            ),
            shape=(
                len(transit_nodes),
                len(self.pipes),
            ),
            dtype=float,
        )

        return matrix.tocsr()

    def _build_solver_data(self,incidence_matrix: sparse.csr_matrix) -> dict[str, Any]:
        """Build solver-compatible masks, counts and hydraulic arrays."""

        node_terminal_mask = (self.nodes["statut"] == 2).to_numpy(dtype=bool)

        node_transit_mask = (self.nodes["statut"] == 1).to_numpy(dtype=bool)

        node_source_mask = (self.nodes["statut"] == 0).to_numpy(dtype=bool)

        pipe_terminal_mask = (self.pipes["statut"] == 2).to_numpy(dtype=bool)

        pipe_transit_mask = (self.pipes["statut"] == 1).to_numpy(dtype=bool)

        pipe_source_mask = (self.pipes["statut"] == 0).to_numpy(dtype=bool)

        return {
            # Original solver-compatible fields
            "totcost": float(
                self.pipes["total_cost"].sum()
            ),
            "idNter": node_terminal_mask,
            "idNtra": node_transit_mask,
            "idNsrc": node_source_mask,
            "idAter": pipe_terminal_mask,
            "idAtra": pipe_transit_mask,
            "idAsrc": pipe_source_mask,
            "nbA": len(self.pipes),
            "nbNter": int(node_terminal_mask.sum()),
            "nbNtra": int(node_transit_mask.sum()),
            "nbNsrc": int(node_source_mask.sum()),
            "nbAter": int(pipe_terminal_mask.sum()),
            "nbAtra": int(pipe_transit_mask.sum()),
            "nbAsrc": int(pipe_source_mask.sum()),
            "M_incid": incidence_matrix,
            "gravite": self.pipes["gravit"].to_numpy(dtype=float),
            "beta": self.pipes["beta"].to_numpy(dtype=float),
            "alpha": self.pipes["alpha"].to_numpy(dtype=float),

            # Stable mappings for API and UI integration
            "node_ids": self.nodes["noeud"].to_numpy(dtype=str),
            "edge_ids": self.pipes["external_edge_id"].to_numpy(dtype=str),
            "source_node_ids": self.nodes.loc[node_source_mask,"noeud"].to_numpy(dtype=str),
            "transit_node_ids": self.nodes.loc[node_transit_mask,"noeud"].to_numpy(dtype=str),
            "terminal_node_ids": self.nodes.loc[node_terminal_mask,"noeud"].to_numpy(dtype=str),
            "source_edge_ids": self.pipes.loc[pipe_source_mask,"external_edge_id"].to_numpy(dtype=str),
            "transit_edge_ids": self.pipes.loc[pipe_transit_mask,"external_edge_id"].to_numpy(dtype=str),
            "terminal_edge_ids": self.pipes.loc[pipe_terminal_mask,"external_edge_id"].to_numpy(dtype=str),
        }

    def _check_prepared_consistency(self,solver_data: dict[str, Any]) -> None:
        """Check consistency between prepared tables and solver arrays."""

        incidence_matrix = solver_data["M_incid"]

        expected_matrix_shape = (solver_data["nbNtra"], solver_data["nbA"])

        if incidence_matrix.shape != expected_matrix_shape:
            raise RuntimeError(
                "Incidence matrix shape is inconsistent with "
                "the number of transit nodes and pipes."
            )

        pipe_count = solver_data["nbA"]

        for field_name in [
            "gravite",
            "beta",
            "alpha",
            "edge_ids",
        ]:
            if len(solver_data[field_name]) != pipe_count:
                raise RuntimeError(
                    f"Prepared field '{field_name}' has an "
                    "invalid length."
                )

        if solver_data["nbAter"] != solver_data["nbNter"]:
            raise RuntimeError(
                "The number of terminal pipes must match "
                "the number of terminal nodes."
            )

        if solver_data["nbAter"] != len(self.bornes):
            raise RuntimeError(
                "The number of terminal pipes must match "
                "the number of bornes."
            )

        if solver_data["nbAsrc"] == 0:
            raise RuntimeError(
                "The prepared network has no source pipe."
            )

        if not np.isfinite(solver_data["gravite"]).all():
            raise RuntimeError(
                "Prepared gravity values contain invalid numbers."
            )

        if not np.isfinite(solver_data["beta"]).all():
            raise RuntimeError(
                "Prepared beta values contain invalid numbers."
            )

        if not np.isfinite(solver_data["alpha"]).all():
            raise RuntimeError(
                "Prepared alpha values contain invalid numbers."
            )



