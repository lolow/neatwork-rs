# Architecture

The port is a Cargo workspace with two crates. `neatwork-core` is the engine library. `neatwork-cli` is the `neatwork` program, a thin layer over the library.

## Data flow

Design:

1. `Topographie::new` parses the `.tpo` text and applies the parameters on top of the application properties.
2. `make_expanded_topo` splits each multi-tap node into one faucet per tap. `load_taps` and `load_factor` compute the load factor of each pipe.
3. `core_topographie` builds the engine network. `MakeDesign::run` builds the linear program, calls the solver, splits each pipe into at most two diameters and computes the orifices.
4. `Design::new` parses the result as the Java program does, and `Design::content` writes the `.dsg` text.

Simulation:

1. `Design::new` parses the `.dsg` text. `Design::refresh` replaces each diameter with the catalog entry.
2. `core_design` builds the engine network with the pipe coefficients.
3. `MakeSimulation::run` runs the flow computation for each set of open taps and collects the statistics.
4. `simulate` writes the design, the four result tables and the parameters.

## Modules and Java classes

Each Rust module translates one or more Java classes. The translation keeps the Java structure, loop order and error paths. The port gives the Java output for the recorded cases. A later change to idiomatic Rust can use these cases as a safety net.

| Rust module | Java classes |
| --- | --- |
| `lib.rs` (`design`, `simulate`) | `Loader`, `SolverDisk`, the glue in `FrameNeatwork`, `MakeDesignDialog`, `NewSimuDialog`, `SimulationPane` |
| `project::network` | `project.Network`, `Node`, `Pipe` |
| `project::topographie` | `project.Topographie` |
| `project::design` | `project.Design`, `Diameter`, `Database` |
| `engine::defs` | `core.defs.*` (`Nodes`, `Pipes`, `Taps`, `Diameters`, vectors, `CoreTopographie`, `CoreDesign`) |
| `engine::make_design` | `core.MakeDesign`, `core.run.RunMakeDesign`, `core.run.Decoupe` |
| `engine::lp` | `solver.Solver.lp` (MOSEK replaced by HiGHS) |
| `engine::simulation` | `core.MakeSimulation`, `core.run.RunSimulation` |
| `engine::simul_flows` | `core.SimulFlows`, `Gradient`, `Hessien`, `Ndir`, `Amax`, `Dat`, `Utils` |
| `engine::loi_normale` | `utils.LoiNormale` |
| `java::format` | `Double.toString`, `DecimalFormat`, `Double.parseDouble`, `Math.round` |
| `java::collections` | `Hashtable`, `ConcurrentHashMap`, `Properties` (iteration order) |
| `java` | `Tools.readCSV`, `String.equalsIgnoreCase`, `FileManagerDisk.writeFile` |
| `messages` | the strings of the `neatwork_*.properties` bundles that reach files |

## Shared objects

In Java, a sub-network holds references to the nodes, pipes and taps of the full network. A change through the sub-network thus changes the full network. The Rust engine keeps one owned list of each kind. A sub-network (`Sub` in `engine::simulation`) holds indexes into these lists.

## Linear program

`RunMakeDesign` builds the linear program in the MOSEK column layout: bound keys, bounds, and a sparse matrix stored by column. `engine::lp::Lp` keeps this layout and gives it to HiGHS as a `ColProblem`. HiGHS runs on one thread with the simplex method, so the result is the same on each run.

The solver is a callback of `MakeDesign::run`. The command line uses this to write the linear program, to read a solution from a file, or to solve with HiGHS.

## Flow computation

`SimulFlows` computes the tap flows of one run with a projected, reduced Newton method. Each iteration computes the Hessian, a Newton direction on the taps that are not at zero flow (Cholesky factorization), and a step length by backtracking. The Rust code keeps the Java loops and summation order, so the floating-point results are the same.

## Errors

The Java engine throws an exception on inconsistent data, for example a pipe with a diameter that is not in the catalog. The port returns `neatwork_core::Error` in these cases. The command line prints the message and returns 1.
