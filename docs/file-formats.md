# File formats

NeatWork project files are plain text. Each line is a list of fields separated by commas. A line that starts with `!` is a section header for people. The program ignores headers and blank lines, and it skips empty fields. It finds the meaning of a line from its number of fields.

The program writes numbers in Java notation, for example `200.0`, `0.0015` and `1.0E-6`. Some tables use rounded numbers, for example `0.12`.

Units: heights, lengths and diameters are in meters. Flows are in liters per second. Speeds are in meters per second. Heads and pressures are in meters of water.

## Topography (`.tpo`)

| Fields | Line | Meaning |
| --- | --- | --- |
| 6 | node | `name, height, x, y, number of taps, type` |
| 3 | pipe | `begin node, end node, length` |
| 2 | parameter | `name, value` |

Node types: `0` is the source (reservoir), `1` is a branching node, `2` is a faucet node. The program sets the type of each node again from the pipes. A node with an outgoing pipe is a branching node. Other nodes are faucet nodes.

A faucet node with more than one tap becomes a branching node with one faucet per tap, named `<node>_a`, `<node>_b`, and so on.

Parameters (written in the `!Default properties` section):

| Name | Default | Meaning |
| --- | --- | --- |
| `pipelength` | 6 | Length of one commercial pipe (m). |
| `watertemp` | 20 | Water temperature (C). |
| `orifcoef` | 0.59 | Discharge coefficient of an orifice. |
| `faucetcoef` | 2.0E-8 | Loss coefficient of a faucet. |
| `targetflow` | 0.2 | Target flow at a tap (l/s). |
| `servicequal` | 0.65 | Required quality of service. |
| `opentaps` | 0.4 | Fraction of taps open at the same time, for the design. |
| `limitbudget` | 1.0E9 | Budget limit. |
| `seuil` | 0.1 | Threshold of the "quick check" window of the Java program. The engine does not use it. |

Example:

```
!Nodes
Source,0.0,0.0,0.0,0,0
A,-10.0,0.0,0.0,0,1
P2,-9.0,0.0,0.0,3,2
!Pipes
Source,A,20.0
A,P2,8.0
!Default properties
targetflow,0.2
```

## Design (`.dsg`)

| Fields | Line | Meaning |
| --- | --- | --- |
| 7 | node | `name, height, x, y, ideal orifice, commercial orifice, type` |
| 8 | pipe | `begin, end, length, length 1, diameter ref 1, length 2, diameter ref 2, N` |
| 2 | parameter | `name, value` |
| 3 | load factor | `node, load factor, 0` |
| 9 | diameter | `ref, nominal, SDR, diameter, cost, max head, type, roughness, N` |
| 6 | orifice | `diameter, N, N, N, N, N` |

A pipe has one diameter (`length 2` is 0 and `ref 2` is `0`) or two diameters. An orifice of `10000.0` means "no orifice". The orifice list includes the two bounds `1.0E-6` and `1.0` that the design step adds.

Sections in a written file: `!Nodes`, `!Pipes`, `!Default properties`, `!loadfactor`, `!Diameters`, `!Orifices`.

## Simulation (`.sim`)

A simulation file starts with the full design. Five tables follow:

| Fields | Table | Columns |
| --- | --- | --- |
| 10 | flows per tap | `tap, runs, min, mean, max, variability %, % below min critical flow, % above max critical flow, failures, -1` |
| 11 | quantiles per tap | `tap, runs, min, 10%, 25%, 50%, 75%, 90%, max, N, N` |
| 5 | speed per pipe | `begin -> end, runs, mean speed, max speed, -1` |
| 4 | head per node | `node, min, mean, max` |
| 3 | parameter | `name, value, N` |

The first row of the flow table is the average over all taps (`Global average` in English). A failure is a run where the tap flow is below 0.005 l/s. In the tables, the taps of a multi-tap node are named `<node>_1`, `<node>_2`, and so on.

The parameter table has a localized header (`!Default properties` in English) and these parameters: `nbsim`, `simopentaps`, `targetflow`, `mincriticalflow`, `maxcriticalflow`, `typeorifice`, `typesimu`. For tap-by-tap and handmade simulations, `simopentaps` is `-`.

## Catalogs

The diameter catalog (`diameters.db`) has one line per commercial pipe:

```
nominal, SDR, inner diameter (m), cost per meter, max head (m), type, roughness (mm)
```

Type `1` is PVC and type `2` is iron. The orifice catalog (`orifices.db`) has one orifice diameter per line. The program ships both catalogs from NeatWork 2024. The `--diameters` and `--orifices` options replace them.

## Line order

Some sections come from Java hash tables, and their line order looks random. The order is part of the format: the port writes the same order as the Java program. See [java-compatibility.md](java-compatibility.md).
