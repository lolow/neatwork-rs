"""Run the NW5 python-core simulation on a DSG file and dump statistics + raw batch arrays.
Usage: python ref_run.py <dsg> <seed> <nscen> <outprefix> [--fixture <json>]"""
import json, sys
from pathlib import Path
import numpy as np

CORE = Path("/home/lolow/Sync/06-DEVEL/neatwork/neatwork-rs/reference/neatwork5/application/python-core")
sys.path.insert(0, str(CORE))
from app.neatwork.parsers.dsg_parser import parse_dsg
from app.neatwork.simulation import Simulation, SimulationRunOptions, build_simulation_data
from app.schemas.simulation_domain import SimulationDomainInput

dsg, seed, n, out = Path(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), Path(sys.argv[4])
dom, _ = parse_dsg(name=dsg.stem, content=dsg.read_text(encoding="utf-8"))
domain = SimulationDomainInput.model_validate(dom.model_dump(mode="json", by_alias=True))
inst = build_simulation_data(domain)
sim = Simulation(inst=inst, options=SimulationRunOptions(scenario_count=n, seed=seed))
batch = sim.run()
sim.compute_stats()
payload = sim.statistics_payload()
payload["summary"].pop("durationSeconds", None)
out.with_suffix(".stats.json").write_text(json.dumps(payload, indent=2, allow_nan=False))
def L(a):
    a = np.asarray(a)
    return [[None if (isinstance(v, float) and not np.isfinite(v)) else v for v in row] for row in a.tolist()] if a.ndim == 2 else a.tolist()
raw = {
  "params": {k: (None if isinstance(v, float) and not np.isfinite(v) else v) for k, v in inst.param.items()},
  "edge_ids": batch.edge_ids.tolist(), "terminal_ids": batch.terminal_ids.tolist(),
  "transit_node_ids": batch.transit_node_ids.tolist(),
  "pipe_pred": inst.pipes["pred"].tolist(), "pipe_succ": inst.pipes["succ"].tolist(),
  "beta": inst.dat["beta"].tolist(), "gravite": inst.dat["gravite"].tolist(),
  "alpha_ideal": inst.pipes["alpha_ideal"].tolist(), "alpha_com": inst.pipes["alpha_com"].tolist(),
  "demand_by_terminal_lps": L(batch.demand_by_terminal_lps),
  "open_client_count_by_terminal": L(batch.open_client_count_by_terminal),
  "flow_by_pipe_m3s": L(batch.flow_by_pipe_m3s), "charge_by_pipe_m": L(batch.charge_by_pipe_m),
  "node_dual": L(batch.node_dual_by_transit_node), "satisfaction": L(batch.satisfaction_by_terminal),
  "alpha_by_pipe": L(batch.alpha_by_pipe), "iterations": batch.iterations.tolist(),
  "status": [s.value for s in batch.status_by_scenario],
  "scenario_meta": {k: v for k, v in sim.sim_res["ScenarioMetadata"].items()},
}
out.with_suffix(".raw.json").write_text(json.dumps(raw))
print("success", batch.successful_scenario_count, "/", batch.scenario_count, "iters min/mean/max",
      batch.iterations.min(), batch.iterations.mean(), batch.iterations.max())
print("meta", sim.sim_res["ScenarioMetadata"]["generatedDrawCount"], sim.sim_res["ScenarioMetadata"]["generationBatchCount"])

if "--fixture" in sys.argv:
    exp = json.loads(Path(sys.argv[sys.argv.index("--fixture") + 1]).read_text())
    worst = [0.0, ""]
    def cmp(a, e, p):
        if isinstance(e, dict):
            for k in e:
                if isinstance(a, dict) and k in a: cmp(a[k], e[k], p + "." + k)
                else: print("missing in actual:", p + "." + k)
            if isinstance(a, dict):
                for k in a:
                    if k not in e: print("new in actual:", p + "." + k)
        elif isinstance(e, list):
            if len(a) != len(e): print("len mismatch", p); return
            for i, (x, y) in enumerate(zip(a, e)): cmp(x, y, f"{p}[{i}]")
        elif isinstance(e, bool) or e is None or isinstance(e, str):
            if a != e: print("diff", p, a, e)
        elif isinstance(e, (int, float)):
            d = abs(a - e) / max(abs(e), 1e-8)
            if d > worst[0]: worst[:] = [d, p]
    cmp(payload, exp, "$")
    print("max rel diff vs fixture:", worst)
