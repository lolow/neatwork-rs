# Decisions

Each decision of the port is a short record. Records keep their numbers. A changed decision gets a new record that supersedes the old one.

## ADR-001: Port the engine and a command line, not the window

Status: accepted (2026-11-20)

Context: NeatWork 2024 is a Swing application of about 20,000 lines. `rust-anything analyze --whole-app` reports a CLI, but `Loader.main` opens a window. The engine (`core`, `project`, `solver.SolverDisk`) has no Swing code.

Decision: Port the engine to the library `neatwork-core` and add the command line `neatwork`. The window is out of scope for this port.

Consequences: The library can carry a later desktop or web interface. The command line repeats the default choices of the windows (ADR-011).

## ADR-002: Solve the design LP with HiGHS

Status: accepted (2026-11-20)

Context: The Java code solves the design LP with MOSEK, a commercial solver that needs a license. GLPK code exists but is not in use.

Decision: Use the `highs` crate 2.4.0, which builds HiGHS from source. The solver runs on one thread with the simplex method. A non-optimal status gives zero columns and a warning, as the Java code does.

Consequences: The build needs cmake and a C++ compiler. If several optima exist, HiGHS and MOSEK can return different ones. On `El_Bihajua`, both give the same design.

## ADR-003: Equivalence bar

Status: accepted (2026-11-20)

Context: The user asked for the same file formats, with numbers equal within a relative tolerance of 1e-6. Later, the user asked for the same output on the sample files.

Decision: The port reads and writes the Java formats. The recorded cases must match byte for byte. A tolerance of 1e-6 is allowed, but no case needs it.

Consequences: The port copies Java number formatting and hash table order (ADR-009).

## ADR-004: Record the Java engine with a headless driver

Status: accepted (2026-11-20)

Context: The MOSEK and GLPK Java libraries of the original repository exist only for Windows, so the Java design cannot run here.

Decision: Copy the Java sources to `reference/neatwork2024`. Add a driver in `reference/harness` that calls `SolverDisk` without a window. A stub `Solver` writes the LP and reads its solution from a file. `neatwork lp-solve` solves the LP with HiGHS.

Consequences: Java and Rust are compared on the same LP solution. The comparison covers the LP construction and every later step. The LP solver itself is not part of the comparison.

## ADR-005: Seeded random generator

Status: accepted (2026-11-20)

Context: The Java code opens taps with `Math.random()` without a seed.

Decision: The port uses SplitMix64 with a `--seed` option. Random simulations are compared by statistics. Tap-by-tap, handmade and all-open random simulations are compared byte for byte.

Consequences: A random simulation with the same seed gives the same file. Random files differ from Java files value by value.

## ADR-006: Repository layout

Status: accepted (2026-11-20)

Decision: Cargo workspace with `crates/neatwork-core` and `crates/neatwork-cli`. The Java reference and driver are in `reference/`. The port records and the recorded cases are in `rust/`. The original repository `../neatwork2024` stays unchanged.

## ADR-007: Locales

Status: accepted (2026-11-20)

Context: Three localized strings reach the output files: the `.sim` parameter header, the label of the average row and the name of an added source.

Decision: `--locale en|fr|sp` (and `es`) selects these strings from the Java bundles. The default is English. Reading does not depend on the locale.

## ADR-008: Keep Java behavior that looks wrong

Status: accepted (2026-11-20)

Context: Equivalence (ADR-003) needs the Java behavior, also where it looks like a defect. Examples: iron pipes get no friction, inner pipe flows stay 0, tap heads leave out the faucet loss.

Decision: Translate the behavior as it is. List each case in `docs/java-compatibility.md`.

Consequences: A fix changes the output files. Each fix needs its own record and new recorded cases.

## ADR-009: Copy Java hash table order

Status: accepted (2026-11-20)

Context: The parameter, load factor and diameter sections come from `Hashtable` and `Properties` iteration. `Properties` uses a `ConcurrentHashMap` in JDK 9 and later.

Decision: `neatwork_core::java::collections` copies the single-thread algorithms of JDK 25 (read from the OpenJDK sources).

Consequences: The order matches on all recorded cases. A tree bin of `ConcurrentHashMap` (8 keys in one bin of a table of 64 or more) is not copied. NeatWork property sets do not reach that size.

## ADR-010: Refresh diameters before a simulation

Status: accepted (2026-11-20)

Context: On each design that it opens, the window calls `Design.refreshDesign(database)`. The `.sim` files of the window thus contain catalog values, not the values of the `.dsg` file.

Decision: `simulate` refreshes the diameters from the catalog (shipped, or `--diameters`).

## ADR-011: Default choices of the design window

Status: accepted (2026-11-20)

Decision: `design` selects every catalog row, uses the computed load factors and applies no constraint. The library accepts constraints (`Constraint`). The command line has no option for them yet.

## ADR-012: The current Java code is the reference

Status: accepted (2026-11-20)

Context: The shipped `.sim` files and the orifice format of `El_Bihajua.dsg` come from older versions of the program. The current Java code does not give them again.

Decision: The reference is the output of the current Java code through the driver of ADR-004. The shipped files are a secondary comparison only.

## ADR-013: Use of rust-anything

Status: accepted (2026-11-20)

Context: `rust-anything analyze` reports the engine as one tangled piece of about 15,000 lines with the window (decision D6). The per-unit flow (`bridge`, `shadow`, JNI) needs a running Java application, and here the design step cannot run with MOSEK.

Decision: Use `analyze` for the plan, `app record` and `app replay` for the equivalence cases, and port the engine as one piece. The checks are `cargo fmt`, `cargo clippy -D warnings`, `cargo test` and `app replay`, in place of `gate` on unit crates.

Consequences: No bridge crate and no front door exist. The Rust program is standalone from the start.
