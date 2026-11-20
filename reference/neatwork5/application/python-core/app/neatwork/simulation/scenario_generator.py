"""Probabilistic demand scenario generation."""

from typing import TYPE_CHECKING

import numpy as np
import pandas as pd
from scipy import sparse

from .exceptions import ScenarioGenerationError
from .models import ResolvedSimulationOptions, ScenarioSet

if TYPE_CHECKING:
    from .data import SimulationData


class ScenarioGenerator:
    """
    Generate non-empty demand scenarios without mutating network data.
    """

    def __init__(self, instance: "SimulationData", options: ResolvedSimulationOptions) -> None:
        self.instance = instance
        self.options = options

    def _resolve_client_inputs(self) -> tuple[np.ndarray, np.ndarray]:
        """
        Return client probabilities and target flows.

        Probabilities are unitless.
        Target flows are expressed in litres per second.
        """

        client_count = len(self.instance.clients)

        if self.options.uniform_demand:
            probabilities = np.full(client_count, self.options.open_taps_probability, dtype=float)
            target_flows_lps = np.full(client_count, self.options.target_flow_lps, dtype=float)

            return probabilities, target_flows_lps

        probabilities = pd.to_numeric(self.instance.clients["proba"], errors="raise").to_numpy(dtype=float)
        target_flows_lps = pd.to_numeric(self.instance.clients["cible"], errors="raise").to_numpy(dtype=float)

        return probabilities, target_flows_lps

    def _build_client_terminal_matrix(self, terminal_ids: np.ndarray) -> sparse.csr_matrix:
        """Build the client-to-terminal aggregation matrix."""

        terminal_index = pd.Index(terminal_ids.astype(str))

        client_terminal_ids = (
            self.instance.clients["borne"]
            .astype(str)
            .to_numpy()
        )

        client_terminal_positions = (
            terminal_index.get_indexer(
                client_terminal_ids
            )
        )

        unknown_mask = client_terminal_positions == -1

        if unknown_mask.any():
            unknown_terminals = np.unique(client_terminal_ids[unknown_mask]).tolist()

            raise ScenarioGenerationError(
                "Some clients cannot be mapped to terminal nodes.",
                details={
                    "unknownTerminalIds": unknown_terminals,
                },
            )

        client_count = len(self.instance.clients)

        matrix = sparse.coo_matrix(
            (
                np.ones(client_count, dtype=float),
                (
                    client_terminal_positions,
                    np.arange(client_count),
                ),
            ),
            shape=(
                len(terminal_ids),
                client_count,
            ),
        )

        return matrix.tocsr()

    def generate(self) -> ScenarioSet:
        """Generate the complete set of non-empty scenarios."""

        terminal_ids = np.asarray(self.instance.dat["terminal_node_ids"], dtype=str)
        probabilities, target_flows_lps = self._resolve_client_inputs()

        client_count = len(probabilities)
        scenario_count = self.options.scenario_count

        if client_count == 0:
            raise ScenarioGenerationError(
                "The network contains no clients."
            )

        valid_scenario_probability = (
            1.0 - float(np.prod(1.0 - probabilities))
        )

        if valid_scenario_probability <= 0:
            raise ScenarioGenerationError(
                "No non-empty demand scenario can be generated.",
                details={
                    "validScenarioProbability": (
                        valid_scenario_probability
                    ),
                },
            )

        rng = np.random.default_rng(self.options.seed)
        client_open_matrix = np.zeros(
            (client_count, scenario_count),
            dtype=bool,
        )

        generated_scenario_count = 0
        generation_batch_count = 0
        generated_draw_count = 0

        while generated_scenario_count < scenario_count:
            generation_batch_count += 1

            if (generation_batch_count > self.options.max_generation_batches):
                raise ScenarioGenerationError(
                    "Scenario generation exceeded the maximum "
                    "number of batches.",
                    details={
                        "requestedScenarios": scenario_count,
                        "generatedScenarios": (
                            generated_scenario_count
                        ),
                        "generationBatches": (
                            generation_batch_count - 1
                        ),
                        "validScenarioProbability": (
                            valid_scenario_probability
                        ),
                    },
                )

            remaining_count = (scenario_count - generated_scenario_count)

            estimated_draw_count = int(
                np.ceil(
                    self.options.oversampling_factor
                    * remaining_count
                    / valid_scenario_probability
                )
            )

            draw_count = max(remaining_count, estimated_draw_count)
            draw_count = min(draw_count, self.options.max_draws_per_batch)

            generated_draw_count += draw_count

            random_values = rng.random((client_count, draw_count))
            sampled_open_matrix = random_values < probabilities.reshape(-1, 1)

            non_empty_mask = np.any(sampled_open_matrix, axis=0)

            valid_open_matrix = sampled_open_matrix[:, non_empty_mask]

            available_count = valid_open_matrix.shape[1]

            accepted_count = min(available_count, remaining_count)

            if accepted_count == 0:
                continue

            destination_start = generated_scenario_count
            destination_end = generated_scenario_count + accepted_count

            client_open_matrix[:, destination_start:destination_end] = (
                valid_open_matrix[:, :accepted_count]
            )

            generated_scenario_count = destination_end

        client_terminal_matrix = self._build_client_terminal_matrix(terminal_ids)

        client_demand_matrix_lps = (client_open_matrix * target_flows_lps.reshape(-1, 1))

        demand_by_terminal_lps = np.asarray(
            client_terminal_matrix
            @ client_demand_matrix_lps,
            dtype=float,
        )

        open_client_count_by_terminal = np.asarray(
            client_terminal_matrix
            @ client_open_matrix.astype(float),
        )

        open_client_count_by_terminal = np.rint(open_client_count_by_terminal).astype(int)

        total_demand_lps = np.sum(demand_by_terminal_lps, axis=0)

        return ScenarioSet(
            terminal_ids=terminal_ids,
            demand_by_terminal_lps=(
                demand_by_terminal_lps
            ),
            open_client_count_by_terminal=(
                open_client_count_by_terminal
            ),
            total_demand_lps=total_demand_lps,
            scenario_count=scenario_count,
            seed=self.options.seed,
            valid_scenario_probability=(
                valid_scenario_probability
            ),
            generated_draw_count=generated_draw_count,
            generation_batch_count=(
                generation_batch_count
            ),
        )