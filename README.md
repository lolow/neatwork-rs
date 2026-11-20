# NeatWork in Rust

This repository holds a Rust port of the engine of NeatWork 2024. NeatWork designs gravity-driven water distribution networks for rural areas. APLV (Agua Para La Vida) and Logilab (University of Geneva) wrote the original program in Java.

The port reads and writes the same project files as the Java program. On the sample projects, its output files are byte-identical to the output of the Java engine.

## What the port contains

The Java program has a Swing user interface and an engine. This port contains the engine, a command line interface, and a desktop interface made with Slint. The desktop interface rebuilds the Java windows and adds a drawing of the network (see [docs/gui.md](docs/gui.md)).

The engine does two operations:

- `design` reads a topography (`.tpo`) and writes a design (`.dsg`). A design gives each pipe one or two commercial diameters and gives each tap an orifice. A linear program chooses the cheapest diameters that deliver the target flow.
- `simulate` reads a design (`.dsg`) and writes a simulation (`.sim`). A simulation computes the flow at each tap and the head at each node for sets of open taps.

The Java program solved the linear program with MOSEK. The port uses HiGHS, an open-source solver.

## Build

You need Rust 1.85 or later, cmake and a C++ compiler. The `highs` crate builds the HiGHS solver from source.

```sh
cargo build --release
```

The binary is `target/release/neatwork`.

## Use

Make a design from a topography:

```sh
neatwork design reference/neatwork2024/sample_projects/El_Bihajua.tpo -o El_Bihajua.dsg
```

Simulate the design with each tap open alone:

```sh
neatwork simulate El_Bihajua.dsg --type tapbytap -o El_Bihajua.0.sim
```

Run 100 random simulations where each tap is open with probability 0.4:

```sh
neatwork simulate El_Bihajua.dsg --type random --nbsim 100 --opentaps 0.4 --seed 1
```

The full list of commands and options is in [docs/cli.md](docs/cli.md).

Start the desktop interface (Rust 1.95 or later):

```sh
cargo run --release -p neatwork-gui -- reference/neatwork2024/sample_projects/El_Bihajua.dsg
```

## Repository layout

| Path | Content |
| --- | --- |
| `crates/neatwork-core` | The engine library: file parsing, design, simulation, Java compatibility layer. |
| `crates/neatwork-cli` | The `neatwork` command line program and the replay test. |
| `crates/neatwork-gui` | The `neatwork-gui` desktop interface (Slint). |
| `reference/neatwork2024` | A copy of the Java sources and sample projects (commit in `SOURCE_COMMIT`). It is the reference for the port. |
| `reference/harness` | A headless driver for the Java engine and its build script. |
| `rust/` | Port records: decisions, glossary, map, recorded test cases. |
| `docs/` | Documentation of the port. |

## Documentation

- [docs/cli.md](docs/cli.md): commands and options.
- [docs/gui.md](docs/gui.md): the desktop interface.
- [docs/file-formats.md](docs/file-formats.md): the `.tpo`, `.dsg`, `.sim` and catalog files.
- [docs/architecture.md](docs/architecture.md): modules and how they map to the Java classes.
- [docs/equivalence.md](docs/equivalence.md): how the port is compared with the Java engine, and the results.
- [docs/java-compatibility.md](docs/java-compatibility.md): Java behavior that the port keeps on purpose.
- [rust/DECISIONS.md](rust/DECISIONS.md): the decisions of the port.

## Tests

```sh
cargo test
```

The test suite contains unit tests and a replay of 16 cases recorded from the Java engine. For each case, every output file must be byte-identical to the Java output.

## License

The Java sources carry no license statement. Ask APLV before you distribute this port.
