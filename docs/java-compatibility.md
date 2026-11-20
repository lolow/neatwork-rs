# Java behavior kept on purpose

The port gives the same output as the Java engine, including where the Java code looks wrong. This page lists that behavior. Each item is a candidate for a later fix. A fix changes the output files, so it needs a decision in `rust/DECISIONS.md` and new recorded cases.

## Engine behavior

1. Iron pipes have no friction coefficient. For pipe type 2, `DiametersVector.calculIron` writes its results into local copies that the caller never reads. Iron pipes thus get A = B = 0, so p = 2, q = 5 and beta = 0. In the design, an iron pipe causes no head loss.
2. The simulation uses a fixed coefficient for the first diameter of each pipe. `CoreDesign` sets `beta1 = 0.00089396` instead of the value of the diameter.
3. The simulation does not compute flows in the inner pipes. `SimulFlows` writes only the tap flows. In each run, every inner pipe counts as a failure, and its flow and speed are 0.
4. The head at a tap does not include the loss in the faucet and the orifice. `TapsStatistic` looks the tap up with `==` on two different `String` objects, so it never finds it.
5. A node with height exactly 0, other than a tap, does not count as an intermediate node in the simulation matrices.
6. Speeds use 3.14 for pi.
7. The design step adds the orifices `1.0E-6` and `1.0` to the orifice list, and the `.dsg` file keeps them.
8. A failed linear program gives a design with zero lengths and no error. If MOSEK reports no optimal status, the Java code does not read the solution and continues with zeros.
9. The quantile table of `calculQuartile` sorts the runs with a special rule that moves zero flows to the end.

## Java runtime behavior

The output also depends on the Java runtime. The `java` module of `neatwork-core` copies this behavior.

- Numbers: `Double.toString` gives the shortest form, with `1.0E-6` notation outside 0.001 to 10^7. `DecimalFormat` rounds half to even on the exact binary value. It writes `-0` for small negative numbers, `NaN`, and the infinity sign.
- Line order: the `.dsg` and `.sim` files write some sections in the iteration order of a Java hash table. These sections are the parameters, the load factors and the diameters. The order depends on the Java hash codes and on the history of the table. The port copies the JDK 25 algorithms of `Hashtable`, `ConcurrentHashMap` and `Properties`.
- Text: fields split as with `StringTokenizer`, so empty fields disappear. Names compare with `equalsIgnoreCase` in the engine, and with `equals` in the project files.
- `Math.max` and `Math.min` treat NaN and signed zeros in a specific way, and the port copies that behavior.

## Behavior of the Java windows

The NeatWork window does more than the engine. The command line repeats these steps:

- The design uses every row of the catalogs, the computed load factors and no constraint. These are the defaults of the window.
- The heights of the topography move so that the source is at 0 before the design, as `MakeDesignDialog` does.
- Before a simulation, the program replaces each diameter of the design with the equal catalog entry. The window does this on each design that it opens.
- A simulation file gets the next free number, `<design>.<N>.sim`.

## Not ported

- The Swing user interface, the client-server mode and the printing of tables.
- `RunSimulation.Resolution` and `Solver.nlp`. The Java code does not call them.
- The Apache POI import of `SimulFlows`. The Java code does not use it.
- The diameter constraints of the window. The library supports them (`Constraint`), but the command line has no option for them yet.
