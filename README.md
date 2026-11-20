# NeatWork in Rust

This repository holds a Rust port of the engine of NeatWork 2024. NeatWork designs gravity-driven water distribution networks for rural areas. APLV (Agua Para La Vida) and Logilab (University of Geneva) wrote the original program in Java.

The port reads and writes the same project files as the Java program. On the sample projects, its output files are byte-identical to the output of the Java engine.

## What the port contains

The Java program has a Swing user interface and an engine. This port contains the engine, a command line interface, and a desktop interface made with Slint. The desktop interface rebuilds the Java windows and adds a drawing of the network (see [docs/gui.md](docs/gui.md)).

The engine does two operations:

- `design` reads a topography (`.tpo`) and writes a design (`.dsg`). A design gives each pipe one or two commercial diameters and gives each tap an orifice. A linear program chooses the cheapest diameters that deliver the target flow.
- `simulate` reads a design (`.dsg`) and writes a simulation (`.sim`). A simulation computes the flow at each tap and the head at each node for sets of open taps.

The Java program solved the linear program with MOSEK. The port uses HiGHS, an open-source solver.

## Install

The program has two binaries: `neatwork` (command line) and `neatwork-gui` (desktop interface).

### Linux (x86_64): release archive

The release `v0.1.0` has a ready archive for 64-bit Linux. The repository is private, so download it with the GitHub CLI:

```sh
gh release download v0.1.0 -R lolow/neatwork-rs -p 'neatwork-rs-*-x86_64-linux.tar.gz*'
sha256sum -c neatwork-rs-0.1.0-x86_64-linux.tar.gz.sha256
tar xzf neatwork-rs-0.1.0-x86_64-linux.tar.gz
cd neatwork-rs-0.1.0-x86_64-linux
./neatwork-gui projects/El_Bihajua.dsg
```

The interface needs `fontconfig` and `freetype`, which most desktop systems have. The file chooser needs the XDG desktop portal or GTK 3.

### All systems: build from source

Windows and macOS have no ready binary yet. Build the program from source. The build was tested on Linux only.

1. Install Rust 1.95 or later with [rustup](https://rustup.rs). The command line alone needs Rust 1.85.
2. Install CMake and a C++ compiler. The `highs` crate compiles the HiGHS solver.
   - Windows: install "Visual Studio Build Tools" with the "Desktop development with C++" workload. It contains the compiler and CMake.
   - macOS: run `xcode-select --install`, then `brew install cmake`.
   - Linux (Debian, Ubuntu): `sudo apt install build-essential cmake libfontconfig1-dev`.
   - Linux (Fedora): `sudo dnf install gcc-c++ cmake fontconfig-devel`.
3. Get the source and install the two binaries into `~/.cargo/bin` (on Windows, `%USERPROFILE%\.cargo\bin`):

```sh
gh repo clone lolow/neatwork-rs
cd neatwork-rs
cargo install --locked --path crates/neatwork-cli
cargo install --locked --path crates/neatwork-gui
```

To build without installing, run `cargo build --release`. The binaries are in `target/release`.

### Where the files go

As the Java program, the interface uses the folder where it starts. It keeps the projects in `projects/` and the catalogs in `db/`. If you start it with a double click on Windows, the folder is the folder of the program. The Browse button of the file dialog selects another project folder.

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

## Speed

The table compares the Java engine of NeatWork 2024 with the Rust port on the sample project El_Bihajua (56 nodes, 23 taps). Each value is the mean of 10 runs with [hyperfine](https://github.com/sharkdp/hyperfine), measured on 2026-11-20.

| Operation | Java 2024 | Rust | Rust faster by |
| --- | ---: | ---: | ---: |
| Design, same LP solution on both sides | 156 ms | 10 ms | 15x |
| Design with the HiGHS solve | | 20 ms | |
| Tap-by-tap simulation (23 runs) | 131 ms | 6 ms | 21x |
| Random simulation, 100 runs | 235 ms | 27 ms | 9x |
| Random simulation, 1,000 runs | 688 ms | 215 ms | 3x |
| Random simulation, 10,000 runs | 6,042 ms | 2,260 ms | 2.7x |
| Peak memory, 10,000 runs | 462 MB | 20 MB | 23x less |

Conditions:

- Machine: Intel Core Ultra 7 155H, Fedora Linux, OpenJDK 25, Rust 1.98 release build.
- The Java times include the start of the Java virtual machine (38 ms alone). The Java window pays this cost once, so the gap is smaller in the Java interface for short operations.
- Java runs through the headless driver of `reference/harness`. The MOSEK solve of the Java program is not part of the comparison. On the design line, both programs read the same LP solution.
- The output files are the same in both programs (see [docs/equivalence.md](docs/equivalence.md)).
- The Java program sorts the runs of each tap with a bubble sort, so a long random simulation takes time in proportion to the square of the run count. The port gets the same order with a faster sort.

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
