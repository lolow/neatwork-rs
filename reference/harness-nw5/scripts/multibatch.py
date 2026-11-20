import sys, numpy as np
from pathlib import Path
sys.path.insert(0, "/home/lolow/Sync/06-DEVEL/neatwork/neatwork-rs/reference/neatwork5/application/python-core")
sys.path.insert(0, str(Path(__file__).parent))
from app.neatwork.parsers.dsg_parser import parse_dsg
from app.neatwork.simulation import Simulation, SimulationRunOptions, build_simulation_data
from app.schemas.simulation_domain import SimulationDomainInput
import spec_check as sc
dsg = Path(sys.argv[1])
dom, _ = parse_dsg(name=dsg.stem, content=dsg.read_text())
inst = build_simulation_data(SimulationDomainInput.model_validate(dom.model_dump(mode="json", by_alias=True)))
sim = Simulation(inst=inst, options=SimulationRunOptions(scenario_count=100, seed=7, max_draws_per_batch=30))
sc_ = sim.scenarios
nodes, pipes, clients, d, b, P = sc.load(dsg); nodes, pipes, clients, A = sc.prepare(nodes, pipes, clients, d, b, P)
term, dem, cnt, draws, batches = sc.scenarios(nodes, clients, P, 7, 100, maxd=30)
print("python-core draws/batches", sc_.generated_draw_count, sc_.generation_batch_count, "clean-room", draws, batches)
print("identical demand:", np.array_equal(dem, sc_.demand_by_terminal_lps))
