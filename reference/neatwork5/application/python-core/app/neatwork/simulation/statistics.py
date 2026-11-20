"""Statistical processing of completed hydraulic simulation batches."""

from __future__ import annotations

import warnings
from collections import Counter
from dataclasses import dataclass

import numpy as np
import pandas as pd

from .data import SimulationData
from .exceptions import StatisticsProcessingError
from .models import SimulationBatchResult
from app.neatwork.hydraulic_units import m3s_to_lps


SATISFACTION_SHORTFALL_TOLERANCE = 1.0e-9
COEFFICIENT_OF_VARIATION_MEAN_EPSILON = 1.0e-12

@dataclass(frozen=True, slots=True)
class SimulationStatistics:
    """Statistical tables derived from successful simulation scenarios.

    Attributes:
        terminal_satisfaction: Satisfaction statistics by terminal.
        source_flow: Source-flow statistics in litres per second.
        pipe_velocity: Velocity statistics for source and transit pipes.
        terminal_pressure: Pressure statistics for served terminals.
        transit_pressure: Pressure statistics for transit nodes.
        run_summary: One-row batch execution summary.
        failures: Structured unsuccessful-scenario records.
        successful_scenario_mask: Scenarios included in numerical statistics.
    """

    terminal_satisfaction: pd.DataFrame
    source_flow: pd.DataFrame
    pipe_velocity: pd.DataFrame
    terminal_pressure: pd.DataFrame
    transit_pressure: pd.DataFrame

    run_summary: pd.DataFrame
    failures: pd.DataFrame

    successful_scenario_mask: np.ndarray

    def to_legacy_dict(self) -> dict[str, pd.DataFrame]:
        """Return the historical in-memory statistics table mapping."""

        return {
            "TableSat": self.terminal_satisfaction,
            "TableSt": self.source_flow,
            "TableResV": self.pipe_velocity,
            "TablePress": self.terminal_pressure,
            "TablePrB": self.transit_pressure,
        }


def _masked_row_statistics(data: np.ndarray, mask: np.ndarray, *, lower_tail_percentiles: tuple[int, ...] = ()) -> dict[str, np.ndarray]:
    """Compute row-wise statistics for selected observations.
    Standard deviation uses the complete evaluated simulation population (ddof=0).
    Coefficient of variation is standard deviation divided by the mean. It remains NaN when the mean is effectively zero.
    Args:
        data: Two-dimensional numerical observations.
        mask: Boolean inclusion mask with the same shape as ``data``.
        lower_tail_percentiles: Percentiles used for lower-tail means.

    Returns:
        Minimum, mean, maximum and requested lower-tail means by row.

    Raises:
        StatisticsProcessingError: If ``data`` and ``mask`` are misaligned.
    """

    if data.shape != mask.shape:
        raise StatisticsProcessingError(
            "Statistics data and mask have different shapes.",
            details={"dataShape": list(data.shape), "maskShape": list(mask.shape)},
        )

    masked_data = np.where(mask, data, np.nan)

    with warnings.catch_warnings():
        warnings.simplefilter("ignore", category=RuntimeWarning)

        minimum = np.nanmin(masked_data, axis=1)
        mean = np.nanmean(masked_data, axis=1)
        maximum = np.nanmax(masked_data, axis=1)
        
        # Std of the complete evaluated simulation population (ddof=0)
        standard_deviation = np.nanstd(masked_data, axis=1, ddof=0)
        # Compute a coefficient of variation (CV) row-wise for the whole population, not the sample. 
        coefficient_of_variation = np.full(mean.shape, np.nan, dtype=float)
        valid_cv = np.isfinite(mean) & (np.abs(mean) > COEFFICIENT_OF_VARIATION_MEAN_EPSILON)
        coefficient_of_variation[valid_cv] = standard_deviation[valid_cv] / mean[valid_cv]

        result = {
            "min": np.nan_to_num(minimum, nan=0.0),
            "mean": np.nan_to_num(mean, nan=0.0),
            "max": np.nan_to_num(maximum, nan=0.0),
            "std": np.nan_to_num(standard_deviation, nan=0.0),
            # NaN is intentionally preserved here.
            # JSON export converts it to null.
            "cv": coefficient_of_variation,
        }

        for percentile in lower_tail_percentiles:
            threshold = np.nanpercentile(masked_data, percentile, axis=1, keepdims=True)
            tail_values = np.where(masked_data <= threshold, masked_data, np.nan)
            result[f"cvar{percentile}"] = np.nan_to_num(np.nanmean(tail_values, axis=1), nan=0.0)
    return result

def _row_tail_mean(data: np.ndarray, percentile: float, *, upper_tail: bool) -> np.ndarray:
    """Compute a row-wise mean below or above a percentile threshold.

    Args:
        data: Two-dimensional numerical observations.
        percentile: Percentile defining the selected tail.
        upper_tail: Select observations above the threshold when ``True``.

    Returns:
        Conditional tail mean for every row.
    """

    with warnings.catch_warnings():
        warnings.simplefilter("ignore", category=RuntimeWarning)
        threshold = np.nanpercentile(data, percentile, axis=1, keepdims=True)

        if upper_tail:
            selected = np.where(data >= threshold, data, np.nan)
        else:
            selected = np.where(data <= threshold, data, np.nan)

        return np.nan_to_num(np.nanmean(selected, axis=1), nan=0.0)


class SimulationStatisticsProcessor:
    """Compute hydraulic statistics from a completed simulation batch.

    Only successful scenarios are included in numerical statistics.

    Attributes:
        instance: Prepared network data used to identify pipes and nodes.
    """

    def __init__(self,instance: SimulationData) -> None:
        """Initialize the processor with prepared network data."""

        self.instance = instance

    def compute(self,batch: SimulationBatchResult) -> SimulationStatistics:
        """Compute every supported hydraulic statistics table.

        Args:
            batch: Typed output produced by ``SimulationBatchRunner``.

        Returns:
            Statistical tables, execution summary and failure information.

        Raises:
            StatisticsProcessingError: If no scenario succeeded or required
                hydraulic data is invalid.
        """

        if batch.successful_scenario_count <= 0:
            raise StatisticsProcessingError(
                "Statistics cannot be computed because no scenario succeeded.",
                details={
                    "scenarioCount": batch.scenario_count,
                    "failedScenarioCount": batch.failed_scenario_count,
                },
            )

        success_mask = batch.success_mask

        return SimulationStatistics(
            terminal_satisfaction=self._compute_terminal_satisfaction(batch,success_mask),
            source_flow=self._compute_source_flow(batch,success_mask),
            pipe_velocity=self._compute_pipe_velocity(batch,success_mask),
            terminal_pressure=self._compute_terminal_pressure(batch,success_mask),
            transit_pressure=self._compute_transit_pressure(batch,success_mask),
            run_summary=self._build_run_summary(batch),
            failures=self._build_failure_table(batch),
            successful_scenario_mask=success_mask.copy(),
        )
    
    def _compute_terminal_satisfaction(self,batch: SimulationBatchResult,success_mask: np.ndarray) -> pd.DataFrame:
        """Compute reliability statistics for each terminal.

        A terminal is active in one scenario when at least one attached client is open.

        Satisfaction statistics use only scenarios that are both:
        - successfully solved; and
        - active for the terminal.

        Water shortfall is a valid hydraulic outcome, not an execution failure.
        """

        satisfaction = batch.satisfaction_by_terminal[:, success_mask]
        open_client_counts = np.asarray(batch.open_client_count_by_terminal,dtype=int)
        if (open_client_counts.shape != batch.satisfaction_by_terminal.shape):
            raise StatisticsProcessingError("Open-client counts are not aligned with terminal satisfaction.",
            details={
                "openClientCountShape": list(open_client_counts.shape),
                "satisfactionShape": list(batch.satisfaction_by_terminal.shape),
            },
        )

        # Exact business definition: at least one consumer at this outlet is open.
        active_mask_all_scenarios = (open_client_counts > 0)
        evaluated_active_mask = active_mask_all_scenarios[:, success_mask]

        stats = _masked_row_statistics(satisfaction, evaluated_active_mask, lower_tail_percentiles=(5, 20))

        active_scenario_count = np.sum(active_mask_all_scenarios, axis=1).astype(int)
        evaluated_active_scenario_count = np.sum(evaluated_active_mask, axis=1).astype(int)
        water_shortfall_mask = evaluated_active_mask & (satisfaction < (1.0 - SATISFACTION_SHORTFALL_TOLERANCE))
        water_shortfall_count = np.sum(water_shortfall_mask, axis=1).astype(int)

        # A terminal with no successfully evaluated active scenario has no meaningful satisfaction distribution.
        # Export these statistics as null rather than pretending that satisfaction was zero.

        no_evaluated_observation = evaluated_active_scenario_count == 0

        nullable_statistic_keys = ("min", "mean", "max", "std", "cv", "cvar5", "cvar20",)

        for key in nullable_statistic_keys:
            values = np.asarray(stats[key],dtype=float).copy()
            values[no_evaluated_observation] = np.nan
            stats[key] = values
    
        return pd.DataFrame(
            {
                "minSat": stats["min"],
                "moySat": stats["mean"],
                "maxSat": stats["max"],
                "cVarSat5": stats["cvar5"],
                "cVarSat20": stats["cvar20"],
                "stdSat":stats["std"],
                "cvSat":stats["cv"],
                "activeScenarioCount":active_scenario_count,
                "evaluatedActiveScenarioCount":evaluated_active_scenario_count,
                "waterShortfallCount":water_shortfall_count,
            },
            index=batch.terminal_ids,
        )
    
    def _compute_source_flow(self,batch: SimulationBatchResult,success_mask: np.ndarray) -> pd.DataFrame:
        """Compute source-flow statistics in litres per second."""

        source_pipe_mask = np.asarray(self.instance.dat["idAsrc"], dtype=bool)
        source_node_mask = np.asarray(self.instance.dat["idNsrc"],dtype=bool)

        source_ids = self.instance.nodes.loc[source_node_mask, "noeud"].astype(str).to_numpy()

        source_flow_m3s = batch.flow_by_pipe_m3s[source_pipe_mask, :][:, success_mask]

        if len(source_ids) != source_flow_m3s.shape[0]:
            raise StatisticsProcessingError(
                "Source-node and source-pipe counts are not aligned.",
                details={
                    "sourceNodeCount": len(source_ids),
                    "sourcePipeCount": source_flow_m3s.shape[0],
                },
            )

        source_flow_lps = m3s_to_lps(source_flow_m3s)

        return pd.DataFrame(
            {
                "minSt": np.min(source_flow_lps, axis=1),
                "moySt": np.mean(source_flow_lps, axis=1),
                "maxSt": np.max(source_flow_lps, axis=1),
                "cVarSt5": _row_tail_mean(source_flow_lps, 5, upper_tail=False),
                "cVarSt95": _row_tail_mean(source_flow_lps, 95, upper_tail=True),
            },
            index=source_ids,
        )
    
    def _compute_pipe_velocity(self,batch: SimulationBatchResult,success_mask: np.ndarray) -> pd.DataFrame:
        """Compute velocity statistics for source and transit pipes.

        Returns:
            Velocity statistics in metres per second.

        Raises:
            StatisticsProcessingError: If pipe diameters are unavailable or
                invalid.
        """

        intermediate_mask = (np.asarray(self.instance.dat["idAtra"], dtype=bool) | np.asarray(self.instance.dat["idAsrc"], dtype=bool))

        if "diam1" not in self.instance.pipes.columns:
            raise StatisticsProcessingError("Pipe diameter column 'diam1' is required for velocity statistics.")

        diameters = self.instance.pipes["diam1"].to_numpy(dtype=float)[intermediate_mask]

        if not np.isfinite(diameters).all() or np.any(diameters <= 0):
            raise StatisticsProcessingError("Pipe diameters contain invalid values.")

        flow = np.abs(batch.flow_by_pipe_m3s[intermediate_mask, :][:, success_mask])

        area = np.pi * diameters**2 / 4.0
        velocity = flow / area[:, None]

        start_nodes = self.instance.pipes.loc[intermediate_mask, "pred"].astype(str).to_numpy()
        end_nodes = self.instance.pipes.loc[intermediate_mask, "succ"].astype(str).to_numpy()
        edge_ids = np.asarray(self.instance.dat["edge_ids"], dtype=str,)[intermediate_mask]

        return pd.DataFrame(
            {
                "edgeId": edge_ids,
                "startNodeId": start_nodes,
                "endNodeId": end_nodes,
                "minVelocityMps": np.min(velocity, axis=1),
                "meanVelocityMps": np.mean(velocity, axis=1),
                "maxVelocityMps": np.max(velocity, axis=1),
            }
        )

    def _compute_terminal_pressure(self,batch: SimulationBatchResult,success_mask: np.ndarray) -> pd.DataFrame:
        """Compute pressure statistics for hydraulically served terminals."""

        terminal_pipe_mask = np.asarray(self.instance.dat["idAter"],dtype=bool)
        terminal_charge = batch.charge_by_pipe_m[terminal_pipe_mask,:,][:,success_mask]
        served_mask = batch.served_terminal_mask_by_scenario[:,success_mask]
        stats = _masked_row_statistics(-terminal_charge,served_mask)

        return pd.DataFrame(
            {
                "minPressureM": stats["min"],
                "meanPressureM": stats["mean"],
                "maxPressureM": stats["max"],
            },
            index=batch.terminal_ids,
        )
    
    def _compute_transit_pressure(self,batch: SimulationBatchResult,success_mask: np.ndarray) -> pd.DataFrame:
        """Compute pressure statistics at transit nodes in metres."""

        transit_node_mask = np.asarray(self.instance.dat["idNtra"],dtype=bool)
        heights = self.instance.nodes.loc[transit_node_mask,"haut"].to_numpy(dtype=float)
        transit_ids = self.instance.nodes.loc[transit_node_mask, "noeud"].astype(str).to_numpy()
        dual_values = batch.node_dual_by_transit_node[:, success_mask]

        if dual_values.shape[0] != len(heights):
            raise StatisticsProcessingError(
                "Transit-node dual values are not aligned with transit nodes.",
                details={
                    "dualCount": dual_values.shape[0],
                    "transitNodeCount": len(heights),
                },
            )

        pressure = -heights[:, None] + dual_values

        return pd.DataFrame(
            {
                "minPressureM": np.min(pressure, axis=1),
                "meanPressureM": np.mean(pressure, axis=1),
                "maxPressureM": np.max(pressure, axis=1),
            },
            index=transit_ids,
        )
    
    def _build_run_summary(self,batch: SimulationBatchResult) -> pd.DataFrame:
        """Build a one-row execution summary for the complete batch."""

        successful_iterations = batch.iterations[batch.success_mask]

        min_iterations = int(np.min(successful_iterations))
        mean_iterations = float(np.mean(successful_iterations))
        max_iterations = int(np.max(successful_iterations))

        # Batch failures represent execution failures only.
        # Valid hydraulic outcomes with insufficient terminal water are reported per terminal and do not make the scenario fail.
        failure_counts = Counter(failure.code for failure in batch.failures)

        return pd.DataFrame(
            [
                {
                    "scenarioCount": batch.scenario_count,
                    "successfulScenarios": batch.successful_scenario_count,
                    "failedScenarios": batch.failed_scenario_count,
                    "successRate": (batch.successful_scenario_count / batch.scenario_count),
                    "seed": batch.seed,
                    "durationSeconds": batch.duration_seconds,
                    "minIterations": min_iterations,
                    "meanIterations": mean_iterations,
                    "maxIterations": max_iterations,
                    "failureCountByCode": dict(failure_counts),
                }
            ]
        )
    
    def _build_failure_table(self,batch: SimulationBatchResult) -> pd.DataFrame:
        """Build structured records for unsuccessful scenarios."""

        columns = [
            "scenarioIndex",
            "stage",
            "code",
            "message",
            "details",
        ]

        if not batch.failures:
            return pd.DataFrame(columns=columns)

        rows = [
            {
                "scenarioIndex": failure.scenario_index,
                "stage": failure.stage.value,
                "code": failure.code,
                "message": failure.message,
                "details": dict(failure.details),
            }
            for failure in batch.failures
        ]

        return pd.DataFrame(rows, columns=columns)
