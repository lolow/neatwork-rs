# Command line

The program is `neatwork`. It has three commands: `design`, `simulate` and `lp-solve`.

## Global option

| Option | Values | Default | Effect |
| --- | --- | --- | --- |
| `--locale` | `en`, `fr`, `sp` (or `es`) | `en` | Language of the localized strings that the program writes to files. |

Three strings change with the locale:

- the header of the simulation parameters in a `.sim` file
- the label of the first row of the flow table
- the name of a source that the program adds itself

The program reads files in any locale.

## design

```sh
neatwork design <INPUT.tpo> [-o OUTPUT.dsg] [options]
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

## simulate

```sh
neatwork simulate <INPUT.dsg> [-o OUTPUT.sim] [options]
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
