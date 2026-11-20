# NW5 reference runs

Scripts that run the NeatWork 5 code of `reference/neatwork5` to record reference outputs for the Rust port.

| File | Content |
| --- | --- |
| `scripts/ref_run.py` | Runs the python-core simulation on a `.dsg` file: `python ref_run.py <dsg> <seed> <scenarios> <out prefix>`. It writes `<prefix>.stats.json` and `<prefix>.raw.json`. |
| `scripts/multibatch.py`, `scripts/cmp.py`, `scripts/spec_check.py`, `scripts/pcg_check.py` | Batch runs and comparisons used to check `rust/nw5/specs/simulation.md`. |
| `scripts/nw5_design.py` | A Python version of the MATLAB design `makeReinfDsg` with HiGHS, used to check `rust/nw5/specs/design.md`. |
| `shim/sksparse` | Stand-in for the CHOLMOD library (scikit-sparse is not installed). Put `shim/` first on `PYTHONPATH`. |
| `out/` | Reference outputs (seed 42, 100 scenarios, and other runs listed in `rust/nw5/specs/simulation.md`, section 7.4). |

Setup:

```sh
python3 -m venv .venv
.venv/bin/pip install -r reference/harness-nw5/requirements.txt
PYTHONPATH=reference/harness-nw5/shim .venv/bin/python reference/harness-nw5/scripts/ref_run.py <dsg> 42 100 out/name
```

The outputs come from the CHOLMOD stand-in, not from CHOLMOD itself.
