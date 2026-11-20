# NeatWork 5 integration plan

Goal: integrate the features of NeatWork 5 (repository `APLV-F/neatwork5-mat-2`, copied in `reference/neatwork5`, commit in `SOURCE_COMMIT`) into the Rust port. NW5 becomes the default engine. The NW4 engine stays available for NW4 projects and keeps its byte-identical output.

Decisions: ADR-018 to ADR-020 in [../DECISIONS.md](../DECISIONS.md).

## Phases

| Phase | Content | State |
| --- | --- | --- |
| 1. Specifications | `specs/simulation.md`, `specs/design.md`, `specs/formats.md`, `specs/app.md` | done (2026-11-20) |
| 2. References | Outputs of the python-core simulation on the examples (`reference/harness-nw5`). Expected values of the MATLAB design tests (`specs/design.md` section 8). | done (2026-11-20) |
| 3. Data model and formats | Model and readers in `neatwork-nw5` (done, 32 example files read). Writers, JSON, EPANET, Cruscade in `neatwork-nw5-io` | in progress |
| 4. Simulation | `neatwork-nw5-sim`: IPM engine, meshed and multi-source networks, bounded flow, statistics | in progress |
| 5. Design | `neatwork-nw5-design`: load factors (VaR, CVaR), LP model, post-processing | in progress |
| 6. Command line | `--engine nw5` by default, `--engine nw4` | open |
| 7. Interface | NW5 tables and parameters, simulation review, comparison of revisions, source and JSON views, drag and drop import, valves, node positions | open |
| 8. Documentation and release | docs, equivalence report, release | open |

## Validation

- Simulation: the Rust results must equal the python-core results within a relative tolerance of 1e-8 and an absolute tolerance of 1e-10 (the rule of `docs/design/ArchitectureNW5.md`).
- Design: MATLAB is not available. The Rust design must reproduce the expected values of `tests/matlab` and the example `.dsg` files that `makeReinfDsg` produced.
- NW4: the 16 recorded Java cases must stay byte-identical with `--engine nw4`.
- Independent checks (note of the NW5 scientific lead, 2026-11-20): the python-core references come from the Python engine itself, so the simulation also gets tests that do not depend on it: small cases solved by hand, flow conservation, head-loss consistency, residuals, and solver failure distinguished from hydraulic shortfall.
