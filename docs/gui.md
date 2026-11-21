# Desktop interface

`neatwork-gui` is the desktop interface of the port. It rebuilds the windows of the NeatWork 2024 Swing program with [Slint](https://slint.dev) 1.19, and it adds a drawing of the network. The engine is `neatwork-core`, so the interface writes the same files as the command line.

## Start

```sh
cargo run --release -p neatwork-gui -- [FILE.tpo | FILE.dsg ...]
```

The program uses the working folder as the Java program does. Projects are in `projects/` and the catalogs are in `db/`. If a catalog file is missing, the program uses the catalog of NeatWork 2024. Files on the command line open at start.

The build needs Rust 1.95 or later (for Slint 1.19). On Linux, the file chooser of the Browse button and of the HTML reports needs the XDG desktop portal or GTK 3.

## Windows

The interface has the windows of the Java program. `rust/GUI-SPEC.md` lists them with their content.

- Main window: menus File, Topography, Design, Database and Help. The open projects are tabs at the bottom.
- Topography project: tabs Tables, Tree View, Network, Text.
- Design project: tabs Tables, Simulation, Tree View, Network, Text.
- Project dialogs: file chooser, make design (Hardware, Parameters, Constraints, Load Factors), progress, node and faucet pressures.
- Simulation dialogs: new simulation, faucet selection.
- Other dialogs: quick check, topography information, design parameters, load factors, hardware database, measurement units, About.

The dialogs open over the main window and block it, as the modal Java dialogs do.

## NeatWork 5 projects

File > New NW5 Project, Open NW5 Project, Import Network and Delete NW5 Project work on NeatWork 5 projects (`rust/nw5/specs/app.md`, ADR-024, ADR-025). A NW5 project is a folder `projects/<name>/` managed by `neatwork-nw5-project`. It opens as a project tab with an orange mark, next to the NW4 projects. A `.tpo` or `.dsg` file opened with Open Topography, Open Design or on the command line still opens the NW4 windows.

- Import Network takes a `.tpo`, `.dsg`, NW-json or application `.json`, or EPANET `.inp` file (native file chooser). JSON and INP files are converted to `.dsg` (a design) or `.tpo`. A `.tpo` without `!Diameters` opens the "Complementary databases" dialog, with the catalogues found next to the file. The project accepts one source format (F7).
- Simulation bar: Design... (NW5 design: VaR or CVaR loads, Monte Carlo loads, seed, overrides of common parameters; it runs in the background with the progress dialog and saves the result as a new `.dsg` revision), Simulation... (defaults with their source, overrides), Run, Cancel and a progress bar. One run at a time, on a background thread.
- Tabs: Network (Visual, Raw 2D and Linear layouts, node drag in the Visual layout, inspector, Close valve / Open valve, node search, coloring by the results of the last run), Tables (read-only domain tables), Text (source file, domain JSON, NW-json), Revisions (set active, archive, export as `.tpo`, `.dsg`, NW-json, `.inp`), Simulation (run history, details, export of the statistics JSON, delete), Review (one run, four sortable result tables), Comparison (two runs, compatibility, unified table, numeric panel).
- Help > Set global units chooses m, ft or in for the project.

The NW5 texts have `Nw5.*` keys in `src/i18n.rs`. They are in English for the three languages.

## Network drawing

The Network tab is new. It draws the nodes and the pipes of the project.

| Control | Choices |
| --- | --- |
| Layout | Tree (depth from the source), Profile (distance along the pipes against height), Coordinates (X and Y of the nodes) |
| Color | Node type, Height. For a design also: Pipe diameter, Mean flow at faucets, Mean pressure |
| Zoom | Slider, Fit, 1:1. Ctrl with the mouse wheel also zooms. |

The flow and pressure colors use the simulation that is selected in the Simulation tab. A click on a node shows its data and highlights its path to the source, in the drawing and in the Tree View. If no node has coordinates, the Coordinates layout shows the tree layout.

## Texts and languages

The interface reads the texts of the Java message bundles (`neatwork_en`, `neatwork_fr`, `neatwork_sp`). The new controls have `Gui.*` texts in `src/i18n.rs`. Help > Language changes the language at once and writes `neatwork.locale` for the next start. The Java program needs a restart.

## Differences from the Java interface

The interface keeps the behavior of the Java windows, with these changes:

| Java behavior | Rust interface |
| --- | --- |
| A language change needs a restart. | The language changes at once. |
| The Design Load Factors dialog shows `!topo.0.name!` instead of the node names. | It shows the node names, sorted. |
| Apply in the design tables removes the orifice list of the design. | The orifice list stays. |
| The orifice choice of a new simulation is stored as the translated label. | It is stored as `commercial` or `ideal`. |
| Choosing an iron diameter in a design table compares the type wrongly (read from the code, not tested). | The type of the diameter is compared correctly. |
| No solution: the feasibility test reads a pressure value. | No solution: the LP solver status is not optimal. |
| The HTML reports use the language of the bundles. | The HTML reports are in English. |

The "Speed in pipes" table shows `-` in every cell, as in the Java program.

## Checks without a mouse

The variable `NEATWORK_GUI_STEPS` runs interface actions at start, separated by `;`. With these steps a script can drive the interface and take screenshots.

| Steps | Effect |
| --- | --- |
| `tab=N`, `mdtab=N`, `dbtab=N`, `simtable=N` | Select a tab or a table. |
| `menu=ACTION` | Run a menu action, for example `start-design` or `sim-new`. |
| `layout=N`, `color=N`, `select=NODE` | Set the drawing. |
| `action=TABLE:ACTION` | Press a table button, for example `20:select-all`. |
| `mdrun`, `pgclose`, `tdclose` | Run the make design, close the progress and table dialogs. |
| `fdname=NAME`, `fdaction` | Fill the file dialog and press its action button. |
| `nstype=N`, `nsrun` | Set the simulation type and run it. |
| `wait=MS` | Wait before the next steps. |
| `nw5new=NAME`, `nw5open=FOLDER` | Create a NW5 project in `projects/`, or open a project folder (absolute or relative to `projects/`). |
| `nw5import=FILE` | Import a network file in the current NW5 project (the catalogue dialog of a `.tpo` is confirmed). |
| `nw5tab=N` | Select a workspace tab: 0 Network, 1 Tables, 2 Text, 3 Revisions, 4 Simulation, 5 Review, 6 Comparison. |
| `nw5table=N`, `nw5text=N`, `nw5layout=N`, `nw5zoom=Z` | Select a domain table, a text view (0 source, 1 JSON, 2 NW-json), a layout (0 Visual, 1 Raw 2D, 2 Linear), a zoom. |
| `nw5select=NODE`, `nw5edge=ARC`, `nw5details` | Search and select a node, select an arc, toggle Show Details. |
| `nw5valve=ARC` | Open or close an arc (stored in `graph.json`). |
| `nw5design`, `nw5design=var`, `nw5design=cvar` | Run the NW5 design with the dialog values. Then `pgclose` shows the result. |
| `nw5save`, `nw5save=NAME` | Save the design as the next revision, or under a custom name. |
| `nw5active=FILE`, `nw5units=m\|ft\|in` | Set the active revision, the display unit. |
| `nw5configure`, `nw5run=NBSIM:SEED` | Open the simulation dialog; start a run with a scenario count and a seed. |
| `nw5review=RUN`, `nw5reviewtable=N`, `nw5compare=A:B` | Show the review of a run, a review table, the comparison of two runs. |
| `nw5details=RUN`, `nw5export=RUN:FILE` | Show the execution details of a run; export its statistics JSON to a file (no dialog). |
| `nw5menu=ACTION`, `nw5close` | Run a NW5 menu action (`nw5-new`, `nw5-open`, `nw5-units`, ...); close the NW5 dialogs. |

For example, these steps make a design of El_Bihajua with every catalog row and save it as `GuiDesign`:

```sh
NEATWORK_GUI_STEPS="menu=start-design;action=20:select-all;action=21:select-all;mdrun;wait=2500;pgclose;tdclose;menu=save;fdname=GuiDesign;fdaction" \
  neatwork-gui projects/El_Bihajua.tpo
```

These steps make a NW5 project from Ceiba, design it (revision 2), run 200 scenarios with seed 1 and export the statistics:

```sh
NEATWORK_GUI_STEPS="nw5new=Ceiba demo;nw5import=reference/neatwork5/examples/tpo_examples/Ceiba/Ceiba.tpo;nw5design;wait=2500;pgclose;nw5save;nw5run=200:1;wait=3000;nw5export=1:stats.json" \
  neatwork-gui
```

On 2026-11-21, the exported statistics were equal to the payload of `neatwork-nw5-project` for the same revision and options (test `ceiba_design_revision_and_run`), except the measured duration `summary.durationSeconds`.

On 2026-11-20, the design saved this way and a tap-by-tap simulation started from the interface were byte-identical to the files of the command line.
