# Equivalence with the Java engine

The goal of the port is the same output as NeatWork 2024 on the same input. This page tells how the port is compared with the Java engine and gives the results.

## Reference

The reference is the current Java code in `reference/neatwork2024` (commit in `SOURCE_COMMIT`). The Java program is a Swing application, and its design step calls MOSEK. The MOSEK and GLPK Java libraries in the original repository exist only for Windows. To run the Java engine on Linux without a window, the port adds a headless driver:

- `reference/harness/src/neatwork/harness/Harness.java` repeats the calls of the windows: it loads the files, selects every catalog row, computes the load factors and calls `SolverDisk`.
- `reference/harness/src/neatwork/solver/Solver.java` replaces the MOSEK call. It writes the linear program that the Java code builds to a file, and it reads the solution from another file.
- `reference/harness/src/mosek/*.java` are two small stand-ins for the MOSEK enums that the Java code uses.
- `reference/harness/neatwork-java` gives the Java engine the command line of the Rust program.

For a design, `neatwork-java` runs the Java engine to get the linear program. Then `neatwork lp-solve` solves the program with HiGHS, and a second Java run uses that solution. Java and Rust thus use the same solver. The comparison covers the construction of the linear program and all the steps after the solution.

The build script copies the non-GUI Java sources without change. It removes one unused import of Apache POI.

## Recorded cases

`rust-anything app record` ran `neatwork-java` on 16 cases and stored the results in `rust/app-cases` (trycmd format). The inputs are in `rust/app-inputs.jsonl`.

| Cases | Input | Operation |
| --- | --- | --- |
| 3 | `SampleNetwork.tpo`, `baseexample.tpo`, `El_Bihajua.tpo` | design |
| 5 | `toy.dsg` | tap by tap, commercial and ideal orifices, handmade (two taps, no tap), random with all taps open |
| 5 | `El_Bihajua.dsg` | tap by tap, commercial and ideal orifices, handmade (5 taps, all 23 taps), random with all taps open |
| 3 | `SampleNetwork.dsg` (Java design) | tap by tap, handmade, random with all taps open. This design has multi-tap nodes. |

A random simulation with `--opentaps 1` opens every tap in every run. It is the only random case that gives the same file on each run.

## Results

| Comparison | Result |
| --- | --- |
| `rust-anything app replay .` (Java reference) | 16/16 |
| `rust-anything app replay . --build target/release/neatwork` | 16/16, every output file byte-identical |
| `cargo test` (replay of the 16 cases without Java) | pass |
| Linear programs of the 3 designs, Rust against Java | byte-identical JSON |
| `El_Bihajua.dsg` shipped with NeatWork (made with MOSEK) against the Rust design | same diameters, lengths, orifices and cost (10724.12) |
| Random simulation, 2000 runs of `El_Bihajua.dsg`, Java against Rust | mean tap flow 0.0978 and 0.098 l/s, largest per-tap z-score of the mean 2.72 over 23 taps |

The shipped `El_Bihajua.dsg` writes orifices as `0.0050`, and the current Java code writes `0.005`. The shipped file comes from an older version of the program. Apart from this format, the shipped design and the Rust design are the same.

The shipped `.sim` files also come from older versions. They have zero flows and French headers, and the current Java code does not give them again. They are not a reference.

## Run the comparison again

You need Java 17 or later and a release build of the port.

1. Build the port: `cargo build --release`.
2. Build the Java driver: `reference/harness/build.sh`.
3. Replay the Java reference: `rust-anything app replay .`. The result must be 16/16.
4. Replay the Rust program: `rust-anything app replay . --build target/release/neatwork`.

To add a case, add a line to `rust/app-inputs.jsonl` and record again:

```sh
rust-anything app record . --inputs rust/app-inputs.jsonl --no-traps --name neatwork --timeout 120 -- reference/harness/neatwork-java
```

A mismatch is a defect of the port. Do not change a recorded case to make a test pass. If a difference is intended, record the reason in `rust/DECISIONS.md` first.
