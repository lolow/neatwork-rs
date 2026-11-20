from dataclasses import replace


import numpy as np
import pandas as pd
from pathlib import Path

from .json_exporter import SimulationJsonExporter
from .statistics import SimulationStatistics, SimulationStatisticsProcessor

from .data import SimulationData
from .solver import InteriorPointSolver
from .options import resolve_simulation_options, resolve_solver_options
from .scenario_generator import ScenarioGenerator

from .models import ScenarioProblem,SimulationRunOptions
from .batch_runner import SimulationBatchResult, SimulationBatchRunner
from .problem_builder import ScenarioProblemBuilder
from collections.abc import Callable


class Simulation:
	"""
	Orchestrate scenario generation and hydraulic batch execution.

	The class prepares runtime parameters and generated scenarios, while
	dedicated components build individual hydraulic problems, solve them and
	execute the complete scenario batch.

	Attributes:
		inst: Prepared hydraulic network.
		requested_options: Runtime options supplied by the caller.
		options: Resolved scenario-generation options.
		solver_options: Validated numerical IPM options.
		params: Runtime hydraulic parameter mapping.
		scenarios: Generated probabilistic demand scenarios.
		problem_builder: Builder for solver-ready scenario problems.
		batch_result: Most recent typed batch result, if execution has run.
		sim_res : dict
			Dictionary to store the results of the simulations, including flows, satisfaction, iterations, etc.
		statistics: Most recently computed statistical simulation results.
	Methods
	-------
	__init__(self, inst)
		Initializes the simulation by generating scenarios and preparing result storage.
	genscen(self)
		Generates probabilistic scenarios of demand based on client probabilities.
	init_sim_res(self)
		Initializes the dictionary to store simulation results.
	prepare_sim(self, k)
		Prepares the data for the k-th scenario to be fed into the solver.
	runSimul(self)
		Runs the flow solver for all scenarios and stores the results.
	stat_processing(self)
		Processes the results of the simulations to compute statistics and prints them."""
	def __init__(self, inst: SimulationData, seed: int | None = None, options: SimulationRunOptions | None = None):
		"""
		Create one simulation execution.

		Parameters
		----------
		inst:
			Validated hydraulic network.
		seed:
			Backward-compatible random seed.
		options:
			Structured runtime options.
		solver_options:
			Validated numerical settings used by the interior-point solver.
		"""
		if seed is not None and options is not None:
			raise ValueError(
				"Pass either 'seed' or 'options', not both."
			)

		if options is None:
			options = SimulationRunOptions(seed=seed)

		if not inst.is_prepared:
			inst.prepare()
		self.inst = inst
		self.requested_options = options
		
		self.options = resolve_simulation_options(inst, options)
		self.solver_options = resolve_solver_options(inst)

		self.params = {
			**inst.param,
			"nbsim": self.options.scenario_count,
		}

		if self.options.open_taps_probability is not None:
			self.params["opentaps"] = (
				self.options.open_taps_probability
			)

		if self.options.target_flow_lps is not None:
			self.params["targetflow"] = (
				self.options.target_flow_lps
			)

		self.scenarios = ScenarioGenerator(inst, self.options).generate()
        # Build problems for each scenario
		self.problem_builder = ScenarioProblemBuilder(
			instance=inst,
			scenarios=self.scenarios,
			runtime_params=self.params
		)
		# Empty batch result until run()
		self.batch_result: SimulationBatchResult | None = None
		# Computed statistics when run() has been called
		self.statistics: SimulationStatistics | None = None

		# Backward-compatible attribute names used by prepare_sim().
		self.demande_borne = self.scenarios.demand_by_terminal_lps
		self.nb_clients = self.scenarios.open_client_count_by_terminal

		self.sim_res = self.init_sim_res()

		self.sim_res["ScenarioMetadata"] = {
		"scenarioCount": self.scenarios.scenario_count,
		"seed": self.scenarios.seed,
		"validScenarioProbability": (
			self.scenarios.valid_scenario_probability
		),
		"generatedDrawCount": (
			self.scenarios.generated_draw_count
		),
		"generationBatchCount": (
			self.scenarios.generation_batch_count
		),
		"terminalIds": (
			self.scenarios.terminal_ids.tolist()
		),
		"demandUnit": "L/s",
		}
		

	def genscen(self, seed: int | None = None) -> tuple[np.ndarray, np.ndarray]:
		"""
		Backward-compatible access to probabilistic scenario generation.
		New code should use self.scenarios.
		"""

		if seed is None:
			return (
				self.scenarios.demand_by_terminal_lps.copy(),
				self.scenarios
				.open_client_count_by_terminal
				.copy(),
			)

		requested_options = replace(self.requested_options, seed=seed)
		resolved_options = resolve_simulation_options(self.inst, requested_options)

		scenarios = ScenarioGenerator(self.inst, resolved_options).generate()

		return (
			scenarios.demand_by_terminal_lps,
			scenarios.open_client_count_by_terminal,
		)

	def init_sim_res(self):
		scenario_count = self.options.scenario_count

		return {
        "X": np.zeros((self.inst.dat["nbA"], scenario_count)),
        "Satis": np.zeros((self.inst.dat["nbAter"], scenario_count)),
        "Ouv": np.zeros((self.inst.dat["nbA"], scenario_count), dtype=bool),
        "Charge": np.zeros((self.inst.dat["nbA"], scenario_count)),
        "Alpha": np.zeros((self.inst.dat["nbAter"], scenario_count)),
        "Iter": np.zeros(scenario_count, dtype=int),
        "U": np.zeros((self.inst.dat["nbNtra"], scenario_count)),
        "dem_tot": np.zeros(scenario_count),
		}
	
	def build_scenario_problem(self, scenario_index: int) -> ScenarioProblem:
		"""Build a typed hydraulic problem for one scenario."""

		return self.problem_builder.build(scenario_index)


	def prepare_sim(self, k: int) -> dict:
		"""
		Backward-compatible adapter for the current solver.

		New code should use build_scenario_problem().
		"""

		problem = self.build_scenario_problem(k)
		return problem.to_solver_work()

	def runSimul(self) -> None:
		"""Run the batch and populate legacy ``sim_res`` structures.

		New code should call :meth:`run`. This compatibility method remains until
		the statistical pipeline is refactored.
		"""

		batch = self.run()

		self._sync_legacy_results(batch)
				
	def compute_stats(self) -> SimulationStatistics:
		"""Compute statistics from the latest typed batch result.

		Returns:
			Processed simulation statistics.

		Raises:
			RuntimeError: If the simulation batch has not been executed.
		"""
		if self.batch_result is None:
			raise RuntimeError("Simulation must be executed before computing statistics.")

		processor = SimulationStatisticsProcessor(self.inst)
		self.statistics = processor.compute(self.batch_result)

		# Temporary compatibility for remaining legacy consumers.
		self.stats = self.statistics.to_legacy_dict()

		return self.statistics

	def display_stats(self, print_flag: bool = True) -> None:
		"""Print the latest statistical tables for manual inspection."""
		if not print_flag:
			return

		if self.statistics is None:
			raise RuntimeError("Statistics must be computed before display.")

		print("\nTerminal satisfaction")
		print(self.statistics.terminal_satisfaction)

		print("\nSource flow")
		print(self.statistics.source_flow)

		print("\nPipe velocity")
		print(self.statistics.pipe_velocity)

		print("\nTerminal pressure")
		print(self.statistics.terminal_pressure)

		print("\nTransit pressure")
		print(self.statistics.transit_pressure)

		print("\nSimulation summary")
		print(self.statistics.run_summary)

		if not self.statistics.failures.empty:
			print("\nScenario failures")
			print(self.statistics.failures)

	def export_stats(self, destination: str | Path | None = None, *, indent: int | None = 2) -> Path:
		"""Export the latest statistics to a strict UTF-8 JSON file.

		Args:
			destination: Optional output path. A network-based filename is used
				when omitted.
			indent: JSON indentation level, or ``None`` for compact output.

		Returns:
			Resolved path of the generated JSON file.

		Raises:
			RuntimeError: If statistics have not been computed.
		"""

		if self.statistics is None:
			raise RuntimeError("Statistics must be computed before export.")

		if destination is None:
			version_suffix = f"_{self.inst.version}" if self.inst.version else ""

			destination = Path(f"simulation_statistics_{self.inst.name}{version_suffix}.json")

		exporter = SimulationJsonExporter()

		return exporter.export(
			self.statistics,
			destination,
			network_name=self.inst.name,
			network_version=self.inst.version,
			indent=indent,
		)

	def statistics_payload(self) -> dict:
		"""Return the latest statistics as an API-friendly JSON payload.

		Returns:
			Strictly JSON-compatible statistics dictionary.

		Raises:
			RuntimeError: If statistics have not been computed.
		"""

		if self.statistics is None:
			raise RuntimeError("Statistics must be computed before serialization.")

		exporter = SimulationJsonExporter()

		return exporter.build_payload(
			self.statistics,
			network_name=self.inst.name,
			network_version=self.inst.version,
		)

	def stat_processing(self, print_flag: bool = True, export_flag: bool = False, destination: str | Path | None = None) -> SimulationStatistics:
		"""Compute statistics and optionally display or export them as JSON.

		Args:
			print_flag: Print computed tables when ``True``.
			export_flag: Write a JSON file when ``True``.
			destination: Optional JSON destination used during export.

		Returns:
			Newly computed simulation statistics.
		"""
		self.compute_stats()
		self.display_stats(print_flag=print_flag)
		if export_flag:
			self.export_stats(destination=destination)

		return self.statistics
	
	# TODO : refactor this method, and delete it later
	def compare_stats(self, filename=None):
		if filename is None:
			filename = f"../test_examples/simul_debit_{self.inst.name}_{self.inst.version}.xlsx"
		
		sheets = {
			'Sheet1': ('TableSat', True),
			'Sheet2': ('TableSt', True),
			'Sheet3': ('TableResV', False),
			'Sheet4': ('TablePress', True),
			'Sheet5': ('TablePrB', True)
		}
		
		for sheet, (name, has_index) in sheets.items():
			df_calc = self.stats[name]
			idx_col = 1 if has_index else None
			
			df_excel = pd.read_excel(filename, sheet_name=sheet, skiprows=1, index_col=idx_col)
			df_excel = df_excel.dropna(axis=1, how='all')
			
			num_calc = df_calc.select_dtypes(include=[np.number]).to_numpy()
			num_excel = df_excel.select_dtypes(include=[np.number]).to_numpy()
			
			if num_calc.shape == num_excel.shape:
				max_diff = np.nanmax(np.abs(num_calc - num_excel))
				if max_diff > 1e-6:
					print(f"{name} : {max_diff:.4e}")
				else:
					print(f"{name} : OK")
			else:
				print(f"{name} : {num_calc.shape} vs {num_excel.shape}")

	def run(self, *, cancel_requested: Callable[[], bool] | None = None) -> SimulationBatchResult:
		"""Execute all generated scenarios and return typed batch results.

		Returns:
			Structured numerical outputs, statuses and failures for the batch.
		"""

		solver = InteriorPointSolver(self.solver_options)

		runner = SimulationBatchRunner(
			problem_builder=self.problem_builder,
			solver=solver,
			target_flow_lps=self.options.target_flow_lps,
			cancel_requested=cancel_requested,
		)

		self.batch_result = runner.run()

		return self.batch_result
	
	def _sync_legacy_results(self, batch: SimulationBatchResult) -> None:
		"""Copy typed batch outputs into the temporary legacy result dictionary."""

		legacy_iterations = batch.iterations.copy()
		legacy_iterations[~batch.success_mask] = -1

		self.sim_res["X"] = batch.flow_by_pipe_m3s.copy()

		self.sim_res["Satis"] = batch.satisfaction_by_terminal.copy()

		self.sim_res["Ouv"] = batch.open_pipe_mask_by_scenario.copy()

		self.sim_res["Charge"] = batch.charge_by_pipe_m.copy()

		self.sim_res["Alpha"] = batch.alpha_by_pipe[self.inst.dat["idAter"], :].copy()

		self.sim_res["Iter"] = legacy_iterations

		self.sim_res["U"] = batch.node_dual_by_transit_node.copy()

		self.sim_res["dem_tot"] = batch.total_demand_lps.copy()

		self.sim_res["TempsIPM"] = batch.duration_seconds

		self.sim_res["ScenarioFailures"] = [failure.to_dict() for failure in batch.failures]

		self.sim_res["BatchMetadata"] = {
			"scenarioCount": batch.scenario_count,
			"successfulScenarioCount": batch.successful_scenario_count,
			"failedScenarioCount": batch.failed_scenario_count,
			"seed": batch.seed,
			"durationSeconds": batch.duration_seconds,
			"statusByScenario": [
				status.value
				for status in batch.status_by_scenario
			],
		}
                