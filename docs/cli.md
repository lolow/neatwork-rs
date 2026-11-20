# Command line

The program is `neatwork`. It has four commands: `design`, `simulate`, `convert` and `lp-solve`.

`design` and `simulate` use one of two engines:

- NeatWork 5 (NW5), the default. It reads the NW5 files with clients (`!Clients`) and required pressures per terminal (`!Borne`). The rules are in `rust/nw5/specs/`.
- NeatWork 4 (NW4), with `--engine nw4`. It is the engine of the Java program NeatWork 2024, and its output files are byte-identical to the Java output.

## Global options

| Option | Values | Default | Effect |
| --- | --- | --- | --- |
| `--engine` | `nw5`, `nw4` | `nw5` | Engine of `design` and `simulate`. |
| `--locale` | `en`, `fr`, `sp` (or `es`) | `en` | NW4 only: language of the localized strings that the program writes to files. |

The NW4 engine changes three strings with the locale:

- the header of the simulation parameters in a `.sim` file
- the label of the first row of the flow table
- the name of a source that the program adds itself

The program reads files in any locale.

## design (NW5)

```sh
neatwork design <INPUT> [-o OUTPUT.dsg] [options]
```

The command reads a network and writes a design. The input can be a `.tpo` file or any input of `convert`. The default output is `<network name>.dsg` in the folder of the input. The command prints the optimal cost and the final cost after the rounding of the pipe lengths.

The design file holds the chosen pipes, the orifices, and a section `!loadflow` with the design flow and the head margin of each link (ADR-022 in `rust/DECISIONS.md`).

If the input has no pipe catalog (`!Diameters`), the program looks in the folder of the input, as the MATLAB code does. It uses the first file that exists: `DiamBase<file name>.db`, `DiamBase<folder name>.db`, `diameters.db`. If the input has no orifice catalog, it uses `orifices.db` from the same folder. If no file exists, it uses the catalog shipped with NeatWork 2024. It prints the name of each catalog that it uses. A file given with `--diameters` or `--orifices` must exist.

| Option | Effect |
| --- | --- |
| `-o, --output FILE` | Write the design to `FILE`. The extension gives the format, as in `convert`. |
| `--diameters FILE` | Use this pipe catalog. |
| `--orifices FILE` | Use this orifice catalog. |
| `--cvar` | Compute the design loads with the conditional value at risk (CVaR). The default is the value at risk (VaR). |
| `--monte-carlo` | Compute the design loads by Monte Carlo draws, also for a uniform demand. |
| `--seed S` | Seed of the Monte Carlo draws. The default is 0. |
| `--param NAME=VALUE` | Replace a parameter of the file, for example `--param vmax=1.2`. You can give the option more than one time. |
| `--pressures` | Print the table `succ,design flow (L/s),margin (m)` to standard output. |

The parameters `fusion`, `split` and `use_efficient_pipes` stay in the files but have no effect, as in the current MATLAB code (ADR-023).

If the linear program has no solution, the error message gives the likely cause when the program can find it:

- a node that has no head left for the losses in the pipes: the node is at or above the source, or its required pressure is too high
- a link where no catalog pipe can hold the static pressure

The options `--lp-dump`, `--lp-solution-out` and `--lp-solution` need `--engine nw4`.

## simulate (NW5)

```sh
neatwork simulate <INPUT.dsg> [-o OUTPUT.json] [options]
```

The command draws random scenarios of open clients and solves the flows of each scenario. It writes the statistics as JSON (the payload of the NW5 python-core). The default output is `<design>.sim.json` in the folder of the input. The command prints the number of successful scenarios, the seed and the time.

| Option | Values | Default | Effect |
| --- | --- | --- | --- |
| `-o, --output FILE` | path | see above | Write the statistics to `FILE`. |
| `--nbsim N` | integer | parameter `nbsim` of the file, else 100 | Number of scenarios with at least one open client. |
| `--seed S` | integer | drawn by the program | Seed of the random generator. The same seed gives the same scenarios. |
| `--opentaps P` | number | file value | Opening probability. It applies to a uniform demand only. |
| `--orifice` | `commercial`, `ideal` | file value | Orifice size to use at the terminals. |
| `--diameters FILE` | path | see `design` | Pipe catalog, if the design has none. |

The NW5 engine runs random scenarios only. The options `--type tapbytap`, `--type handmade` and `--open` need `--engine nw4`.

## convert

```sh
neatwork convert <INPUT> -o <OUTPUT> [options]
```

The command converts a NW5 network between formats. The extension of each file gives its format.

| Format | Input | Output |
| --- | --- | --- |
| `.tpo` topography | yes | yes |
| `.dsg` design | yes | yes |
| `.json` MATLAB NW-json-v1 | yes (detected) | yes (default) |
| `.json` application domain JSON | yes | yes, with `--json domain` |
| `.inp` EPANET | yes | yes |
| Txt folder (Cruscade files) | yes (a folder as input) | no |

| Option | Effect |
| --- | --- |
| `--upgrade` | Read a NW4 `.dsg` file (no `!Clients` section) and upgrade it to NW5. |
| `--json matlab` or `--json domain` | Kind of JSON to write. The default is `matlab`. |
| `--txt-prefix P` | Txt folder: prefix of the five files, for example `CruscadeDet2406`. |
| `--txt-m3s` | Txt folder: the target flows of the clients are in m³/s. |
| `--inp-open-demands` | EPANET input: make one client that is always open for each demand. |

The command prints a warning for each part of the network that the output format cannot hold.

## design (NW4)

```sh
neatwork --engine nw4 design <INPUT.tpo> [-o OUTPUT.dsg] [options]
```

The command reads a topography and writes a design. The default output is `<name>.dsg` in the folder of the input.

The command repeats the choices that the NeatWork window makes by default:

- It selects every row of the diameter catalog and of the orifice catalog.
- It computes the load factor of each pipe from the number of taps downstream.
- It applies no diameter constraint.

| Option | Effect |
| --- | --- |
| `-o, --output FILE` | Write the design to `FILE`. |
| `--diameters FILE` | Use this diameter catalog instead of the shipped one. |
| `--orifices FILE` | Use this orifice catalog instead of the shipped one. |
| `--pressures` | Print the table `node,height,pressure,suction` to standard output. |
| `--lp-dump FILE` | Write the linear program to `FILE` (JSON, MOSEK column layout). |
| `--lp-solution-out FILE` | Write the solution of the linear program, one value per line. |
| `--lp-solution FILE` | Do not solve. Read the solution from `FILE`. |

If HiGHS finds no optimal solution, the program prints a warning and continues with zero lengths, as the Java program did. The design is then not usable.

## simulate (NW4)

```sh
neatwork --engine nw4 simulate <INPUT.dsg> [-o OUTPUT.sim] [options]
```

The command reads a design and writes a simulation. The default output is `<design>.<N>.sim` in the folder of the input. `N` is one above the largest number in use, and the first file is `<design>.0.sim`.

Before the simulation, the program replaces each diameter of the design with the equal entry of the catalog. The NeatWork window also does this replacement on each design that it opens.

| Option | Values | Default | Effect |
| --- | --- | --- | --- |
| `-o, --output FILE` | path | see above | Write the simulation to `FILE`. |
| `--type` | `random`, `tapbytap`, `handmade` | `random` | Kind of simulation. |
| `--nbsim N` | integer | `100` | Number of runs of a random simulation. |
| `--opentaps P` | number | `0.4` | Probability that a tap is open in a random run. |
| `--orifice` | `commercial`, `ideal` | `commercial` | Orifice size to use at the taps. |
| `--open A,B,...` | tap names | none | Open taps of a handmade simulation. |
| `--seed S` | integer | `0` | Seed of the random generator. |
| `--diameters FILE` | path | shipped catalog | Catalog for the diameter replacement. |

The three kinds of simulation:

- `random` does `--nbsim` runs. In each run, each tap is open with probability `--opentaps`.
- `tapbytap` does one run for each tap, with only that tap open. The program sets the number of runs to the number of taps.
- `handmade` does one run with the taps in `--open`.

The Java program used `Math.random()` without a seed, so its random simulations never repeat. The port uses a seeded generator, so a random simulation with the same seed gives the same file. Random results agree with the Java results in distribution, not value by value.

## lp-solve

```sh
neatwork lp-solve <LP.json>
```

The command reads a linear program in the JSON format of `design --lp-dump` and prints the solution, one value per line. The Java reference driver uses this command to solve the linear program that the Java code builds. See [equivalence.md](equivalence.md).

## Exit status

The program returns 0 on success. On an error it prints `neatwork: <message>` to standard error and returns 1. Wrong options return 2.
