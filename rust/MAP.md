# Port map

Goal: a Rust version of the NeatWork 2024 engine with the same output files as the Java engine. The Java sources in `reference/neatwork2024` stay as the reference.

## Decisions

See [DECISIONS.md](DECISIONS.md): ADR-001 (scope) to ADR-013 (use of rust-anything). The terms are in [GLOSSARY.md](GLOSSARY.md).

## Units

The engine is one piece (ADR-013). All parts are done.

| Part | Java | State |
| --- | --- | --- |
| Java runtime behavior | `Double.toString`, `DecimalFormat`, `Hashtable`, `Properties` | done (2026-11-20) |
| Project files | `project.*` | done (2026-11-20) |
| Engine data | `core.defs.*` | done (2026-11-20) |
| Make design | `MakeDesign`, `RunMakeDesign`, `Decoupe`, `Solver.lp` | done (2026-11-20) |
| Simulation | `MakeSimulation`, `RunSimulation`, `SimulFlows` and helpers | done (2026-11-20) |
| Entry points | `Loader`, `SolverDisk`, window glue | done (2026-11-20) |
| Command line | none (new) | done (2026-11-20) |

Checks on 2026-11-20: `app replay` 16/16 for the Java reference and for `target/release/neatwork`, `cargo test` pass, `cargo clippy -D warnings` clean.

## Open questions

1. Fix the Java defects listed in `docs/java-compatibility.md`? Each fix changes the output files.
2. Add command line options for the diameter constraints of the design window?
3. Add a user interface on top of `neatwork-core`?

## Out of scope

- The Swing window, the client-server mode, printing.
- Code that the Java program does not call: `RunSimulation.Resolution`, `Solver.nlp`, the GLPK path.
