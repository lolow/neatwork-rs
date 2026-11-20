# Window specification

This file lists the windows of the NeatWork 2024 Swing interface that the Slint interface rebuilds. It comes from a reading of `reference/neatwork2024/src/neatwork/gui`. File references are relative to that folder. Labels are the English texts of `neatwork_en.properties`.

## Main window (`FrameNeatwork`)

- Title `NeatWork`, size 750 x 550.
- Content: the project area. The open projects are tabs at the bottom (`ProjectPane`, `SingleRowTabbedPane`). A tab shows the short name (13 characters and `...` above 15) and the full name as a tooltip.
- With no open project, one tab `NeatWork` shows the welcome page (`PresentationPanel`): the name in large italic text and the line "A tool for the design and the analysis of gravity driven water distribution systems".
- Closing the window asks "Do you really want to quit?".

### Menus

| Menu | Item | Effect |
| --- | --- | --- |
| File | New Topography... | File dialog in "new" mode. Adds an empty topography. |
| File | Open Topography... | File dialog in "open" mode. Adds the project with a free name. |
| File | Open Design... | Same, then replaces the diameters with the catalog entries. |
| File | Close | Asks "Do you want to close the file <name>?". |
| File | Close All | Asks "Do you want to close all the files?". |
| File | Save | Writes the file. If no file has this name yet, it runs Save as. |
| File | Save as... | File dialog in "save" mode. Renames the project. |
| File | Delete a Topography... / Delete a Design... | File dialog in "delete" mode. Deleting a design also deletes its simulations. |
| File | Exit | Asks for confirmation. |
| Topography | Quick check... | Quick Check dialog. |
| Topography | Network summary... | Topography statistics. |
| Topography | Report in HTML... | Saves an HTML report. |
| Topography | Start Design... | Make design dialog. The network must be a tree, and all pipe lengths must be above 0. |
| Design | Extract topography | Adds a topography made from the design (tree designs only). |
| Design | Report in HTML... | Saves an HTML report. |
| Design | Design parameters... | Read-only parameter table. |
| Design | Design Load Factors... | Read-only load factor table. |
| Database | Edit database... | Database dialog. |
| Help | Measurement Units | Unit table (display only). |
| Help | Language | English, Spanish, French. The choice goes to the file `neatwork.locale`. |
| Help | About | About dialog. |

Enabled state: Save, Save as and Close need an open project. Close All needs two. The Topography menu shows for a topography, the Design menu for a design.

### Names of projects

- Two open projects of the same type cannot have the same name. The program adds ` (2)`, ` (3)`, and so on (`ProjectManager.isAlreadyLoaded`).
- A new design takes the first name without a file: `name`, `name (2)`, ... (`AbstractFileManager.getFirstNameCompatible`).

## File dialog (`FileManagerDialog`)

- Modes: open, new, save, delete. Title "Open a", "New ", "Save a", "Delete a" followed by "topography" or "design".
- Top: "Project Path: " with the folder, and a Browse button (folder chooser).
- Center: the list of project files of the type, sorted, without extension.
- Bottom: "Filename:" text field, the action button (Open, New, Save, Delete), Cancel (Close in delete mode), and a status line.
- A click in the list copies the name. A double click runs the action. The action button is disabled while the name is empty.
- Save over an existing file asks "The file <name> exists. Do you wish to overwrite it?".
- Files are in `<file.path>/projects`. The catalogs are in `<file.path>/db`.

## Topography project (`TopographiePane`)

Tabs: Tables, Tree View, Text. The Slint interface adds a Network tab (drawing).

### Tables

Two editable tables, nodes above and pipes below. Each table has a tool bar: insert, delete, copy, paste, undo, apply.

| Table | Columns |
| --- | --- |
| Nodes | ID, Height, X, Y, # of faucets, Nature (read-only: TANK, BRANCHING NODE, FAUCET NODE) |
| Pipes | Begin, End, Length |

- Insert adds a row after the selected row. A new node is named `N`, `N1`, `N2`, ... A new pipe goes from the last node to the last node, with length 0.
- In the topography node table, a name with `_` is cut at the first `_`.
- Copy puts the whole table on the clipboard, cells separated by tabs.
- Paste replaces the whole table. Each line needs the number of columns, otherwise "Clipboard is not well formated".
- Edits change the project only on Apply. Apply rebuilds the project text and parses it. On an error, it shows "Invalid data" and the messages.
- After an edit the table background turns gray until Apply or undo.

### Tree View

- A search field ("Node Search: ") and a Go button.
- A grid: column = depth from the source, row = position by number of downstream faucets. Line cells link the nodes. A click on a node highlights its path to the source.
- Colors: tank green, branching gray, faucet blue.

### Text

The project text in an editor, with Undo and Apply.

## Design project (`DesignPane`)

Tabs: Tables, Simulation, Tree View, Text, plus Network (drawing).

| Table | Columns |
| --- | --- |
| Nodes | ID, Height, X, Y, Ideal Orifice, Commercial Orifice, Nature |
| Pipes | Begin, End, Length, Length 1, Diam 1, Length 2, Diam 2 |

- Orifice value 10000 shows `-`. Typing a value with `-` stores 10000.
- Diam 1 and Diam 2 are chosen from the catalog (`diam <d>:sdr <s>:(PVC)`). If Length 2 is 0, Diam 2 is `-` and read-only.
- Below the tables: "Design total cost: " with the cost.

## Make design dialog (`MakeDesignDialog`)

Tabs: Hardware, Parameters, Constraints, Load Factors. Buttons: Abort, Design.

- Hardware: diameter catalog with a check box per row (Nominal, SDR, Diam, Cost, Max Pressure, Type, Roughness), and orifice catalog with check boxes. "All" and "None" buttons. At first, no row is selected. Design refuses an empty diameter selection: "Select at least one type of pipe".
- Parameters: open faucet fraction, service quality, target flow, budget limit, water temperature, commercial pipe length, orifice coefficient, faucet coefficient. Reset and Apply. Apply writes the values to the topography.
- Constraints: pipe, kind (greater than, lower than, equal to, two diameters), values. "Add constraint" and "Delete". The list shows `[ <pipe>] diameter must be greater than <d>` and so on.
- Load Factors: Begin, End, # of faucets, Suggested load factor, Modified load factor (editable). A reset button.
- Run: a progress dialog ("In progress", status, progress bar, Close at 100%). Without a solution: "There is no feasible solution!". With a solution: the pressure table (Node ID, Height, Forecast Pressure Loss, Potential Problem), then a new unsaved design.

## Simulation tab (`SimulationPane`)

- Buttons: New Simulation..., delete, HTML report. A list of the simulations of the design (`simu <n>`).
- Parameter text: number of simulations, open faucet fraction, orifices, type.
- Table choice: Flows at faucets, Percentiles, Speed in pipes, Node pressures.
- Flows: Faucet ID, # of occurences, Min, Average, Max, Variability, < min critical flow, > max critical flow, # of failures. Columns 6 to 8 show a bar: green above 0, yellow above 60, red above 80.
- The speed table shows `-` in every cell in the Java program.
- A simulation needs a saved design: "You must save the design to run a simulation."

### New simulation dialog (`NewSimuDialog`)

Fields:

- Number of simulations
- Fraction of open faucets
- critical flows, low and high
- Target Flow
- Orifices in use (ideal, commercial)
- Type of simulation (monte-carlo sampling, individual faucets, user-defined)

Buttons: Close and "Run Simulation...". User-defined opens the faucet selection (check box per faucet, All, None, OK). The values become the defaults of the design.

## Other dialogs

| Dialog | Content |
| --- | --- |
| Quick Check | Critical threshold and faucet coefficient fields, Apply. Table: Faucet ID, flow min, target flow, each `Ok` or `No` from `-height > (flow/1000)^2 / faucet coefficient`. |
| Topography information | Name, nodes (branching, faucets, individual faucets), total height change (m), pipes, total length (m). |
| Design Parameters | Name and value of each `topo.*` parameter. |
| Load Factors | Ending node and load factor. |
| Hardware database | Tabs Diameters (Nominal, SDR, Diameter, Cost/m, Max Pressure, Type, Roughness) and Orifices (Diameter). Same tool bar as the tables, with "Load data", "Save data" and an HTML export. |
| Measurement Units | Read-only table of units (meter, meter per second, meter H2O, liter per second). |
| About | Logo, credits, version. |
| In progress | Status, progress bar, Close. |
