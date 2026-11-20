# NeatWork 5 application: functional specification for the Slint port

This document lists every user-visible feature of the NeatWork 5 (NW5) application and gives the rules that a single-user Slint desktop application with local files must reproduce. It was written from a reading of the NW5 source code on 2026-11-20. Nothing was run.

## 0. Conventions

### 0.1 Sources and citations

| Prefix | Folder |
| --- | --- |
| `FE` | `reference/neatwork5/application/frontend/src` (React UI) |
| `BE` | `reference/neatwork5/application/backend/src/main/java/com/neatwork/backend` (Spring Boot) |
| `PY` | `reference/neatwork5/application/python-core/app` (FastAPI: parser, serializer, simulation) |

A citation such as `FE App.tsx:2251-2329` means lines 2251 to 2329 of `frontend/src/App.tsx`.

### 0.2 Markers

| Marker | Meaning |
| --- | --- |
| UNVERIFIED | Read from the code, or inferred, but not observed in a running application. |
| DEFECT | The code seems to do something that the feature list or the comments do not intend. Section 13 lists them with a recommendation. |
| NOT REACHABLE | Code exists, but no control in the UI calls it. |

### 0.3 Terms

| Term | Meaning |
| --- | --- |
| Project | A named container. It holds input files, the revision history, the simulation runs and the display settings. |
| Input file (artifact) | A stored DSG, TPO, diameters `.db` or orifices `.db` file. Each has a visible file name, a version number and an `archived` flag. |
| Revision | An immutable DSG/TPO input file. When the user saves edited input data, the application makes a new revision. Revisions of one original file form a revision family. |
| Domain snapshot | The parsed form of one revision (JSON `domain`: `name`, `sourceFormat`, `params`, `technicalParams`, `designParams`, `warnings`, `tables`). The newest snapshot of a project is the active domain. |
| Graph | The drawable form of the active domain: nodes (`RESERVOIR`, `JUNCTION`, `TANK`, `FAUCET`) and edges (`PIPE`, `VALVE`, `PUMP`) with visual and source coordinates. |
| Run | One simulation execution with a project-wide `runNumber`, a status, the options, the pipe states and the statistics. |
| Source / Junction / Terminal | The three semantic node roles. They come from the node status code `0`, `1`, `2` (`FE features/network/nodeSemantics.tsx:36-106`). |

## 1. Application shell

### F1. Start screen and intro transition

- The application opens on a start screen: brand "NeatWork", eyebrow "Water Network Design & Simulation", title "NeatWork", text "Design, simulate and analyse gravity-driven water distribution networks.", a **Start** button, and the footer "Gravity-driven water distribution systems Design by **APLV**" (`FE features/startup/StartScreen.tsx:5-48`).
- **Start** plays a water animation, then shows the workspace. If the system asks for reduced motion, the animation is 250 ms (`FE features/startup/WaterIntroTransition.tsx:12`). Phases: `start` → `transition` → `workspace` (`FE App.tsx:146-151, 2170-2186`).
- Slint: optional. A splash with a Start button is enough.

### F2. Menu bar

Top bar with three menus and the active project name on the right ("Project <name>" or "No project selected") (`FE features/menu/AppMenuBar.tsx:76-229`). A click outside or Escape closes the menus (`:31-59`).

| Menu | Item | Sub-text | Enabled when | Effect |
| --- | --- | --- | --- | --- |
| File | New Project | | always | F4 |
| File | Open an Existing Project | | always | F5 |
| File | Import Network... | "Choose a DSG or TPO input file" | a project is open | F12 |
| Database | Diameters Database | "Manage commercial pipe diameters" | always | F17 |
| Database | Orifices Database | "Manage commercial orifices" | always | F17 |
| Help | Set global units | | a project is open | F53 |

The Help menu has only one item (`FE features/menu/AppMenuBar.tsx:199-213`). There is no About, language or exit item.

### F3. Workspace layout and states

- Error banner under the menu bar. It shows the last global error text (`FE App.tsx:2202`).
- If a domain is active: the simulation bar (F35) (`FE App.tsx:2204-2233`).
- Network stage (`FE App.tsx:2235-2430`):
  - No active domain: empty state (`FE features/network/WorkspaceEmptyState.tsx:38-72`).
    - No project: "No project is open", "Create a new project or open an existing one to start working with a water distribution network.", buttons **New Project** and **Open Project**.
    - Project without domain: project name, "No network imported", "Import a DSG or TPO network to start working with this project.", button **Import Network**.
  - Active domain: view toolbar with **Graph ▾** (menu of three graph views, F20), **Tables** (F27), **Text** (F52). If a graph view is active, a node search box (F26) shows on the right (`FE App.tsx:2249-2360`).
- The rule for an active domain is `domainImported && graph && latestDomainSnapshot` (`FE App.tsx:190`).
- After a successful import the view switches to Visual Graph (`FE App.tsx:1201`).

## 2. Projects and revisions

### F4. New project

- Dialog "New Project": field "Project name" (max 120 characters, placeholder "Example: Toy network"), buttons **Cancel** and **Create project** ("Creating..." while busy) (`FE features/projects/NewProjectModal.tsx:15-137`).
- The name is trimmed. If the trimmed name is empty, **Create project** is disabled. Error text: "The project could not be created."
- Escape and a click on the backdrop close the dialog, except while it creates.
- The new project starts with `sourceFormat = NONE` and `displayUnit = m` (`BE model/entity/ProjectEntity.java:32,39`, `BE service/ProjectService.java:55-63`). It becomes the active project with no domain (`FE App.tsx:662-675`).
- Names do not need to be unique (UNVERIFIED: no check found in `BE service/ProjectService.java`).

### F5. Open project

- Dialog "Open Project", text "Select an existing project to continue working on it." (`FE features/projects/OpenProjectModal.tsx:24-173`).
- Search field "Search projects..." filters by name, case-insensitive substring (`:46-56`).
- List sorted by creation date, newest first (`BE service/ProjectService.java:34-38`). Each row: name, "Created <locale date>", "Currently open" for the active project, buttons **Open** and **Delete**.
- Empty texts: "No projects found in the connected database." or "No project matches your search." Load error with a **Retry** button.
- Opening a project resets all project state (`FE App.tsx:513-597`) and loads the file list, the run history and the active jobs.
- DEFECT: opening a project does not load its latest domain snapshot, graph, completed runs or export availability. The workspace shows "No network imported" until the user imports again (`FE App.tsx:571-596`. The API `fetchLatestDomainSnapshot` in `FE api/domainApi.ts:26-32` is never called). When a project opens, the port must restore the active revision, graph state, runs and comparison workspace.

### F6. Delete project

- Alert dialog "Delete Project?": "You are about to permanently delete “<name>”.", "Deleting this project also removes all data associated with it.", "This includes the project's network data, imported files, artifacts, domain snapshots, simulations and generated results.", "This action cannot be undone." (`FE features/projects/DeleteProjectConfirmModal.tsx:18-140`).
- A check box "I understand that this project and all its associated files, artifacts, imported data, simulations and results will be permanently deleted." enables **Delete Project** ("Deleting...").
- Effect: deletes runs, artifacts, snapshots, the project and its workspace folder (`BE service/ProjectService.java:72-89`), and the saved comparison workspace (`FE App.tsx:691-750`). If it was the active project, the workspace goes back to "No project is open".

### F7. Project source-format lock (source protection)

A project accepts DSG or TPO input, never both.

- `sourceFormat` values: `NONE`, `DSG`, `TPO`, `REVIEW_REQUIRED` (`BE enums/ProjectSourceFormat.java:8-18`).
- The first successful import sets `sourceFormat` from `NONE` to the imported format (`BE service/ProjectDomainImportService.java:132-134`).
- Allowed import format in the UI (`FE App.tsx:447-458`):
  1. `sourceFormat` is `DSG` or `TPO`: only that format.
  2. `sourceFormat` is `NONE`: if only DSG files are staged (not archived) in the project, DSG. If only TPO files, TPO. Otherwise any.
  3. Import is blocked in two cases: `sourceFormat = REVIEW_REQUIRED`, or `NONE` with both DSG and TPO files staged.
- Messages:
  - "Network import is blocked because this project has conflicting input formats." (`FE App.tsx:1940-1943`)
  - "This project uses <F> input. A <G> file cannot be added to it." (`FE App.tsx:1944-1949`)
  - "This input format is not allowed for the selected project." (`FE App.tsx:1267-1270`)
  - Backend: "This project already contains an input file of the opposite format.", "Remove the conflicting staged input before importing this network.", "This legacy project requires a source format review before importing.", "This project uses <F>. Importing <G> is not allowed." (`BE service/ProjectSourcePolicy.java:33-94`)
- A revision must have the project format: "Revision format does not match project format." (`BE service/ProjectDomainRevisionService.java:85-89`).
- `REVIEW_REQUIRED` has no UI to resolve it (UNVERIFIED: it seems to be set only by a data migration).

### F8. Revision model

Rules for input files (`BE service/ProjectArtifactService.java`):

- A stored file keeps its visible name (`originalFilename`) and gets a stored name `<base>_v<version><ext>` (`:630-637`).
- Network inputs (DSG/TPO): the first version number comes from the file name. A name that ends with `_v<N>` before the extension gets version `N`. Other names get `1` (`:529-542`). The file starts a new revision family (family id = its own id) (`:100-103`).
- Database inputs: version = number of files of this type in the project + 1 (`:69-72`).
- Saving an edited domain (`registerNetworkRevision`, `:454-517`):
  - `nextVersion` = highest version in the family + 1.
  - Default name: base of the source name without a trailing `_v<N>`, then `_v<nextVersion>` and the extension. Example: `Bihajua.dsg` (v1) → `Bihajua_v2.dsg` (`:548-554`).
  - Custom name: trimmed and NFKC-normalized. Must not contain `/` or `\` and must not be `.` or `..`. If the name has no dot, the extension is added. The name must end with `.dsg` or `.tpo` as the source. At most 240 characters (`:561-581`).
  - The revision records its parent file and family. The source file is never changed.
- The new revision becomes the active domain. A new snapshot is stored. Its `name` is the revision file name without extension (`BE service/ProjectDomainRevisionService.java:114-129`).
- The graph is rebuilt from the revision. Node positions and valve states set by the user are lost (`BE service/NetworkGraphSaveService.java:24-71`).
- Visible results of the old revision are cleared and the review closes (`FE App.tsx:824-843`).
- Only DSG/TPO files that are not archived can be edited. A snapshot without a source file ("legacy") cannot be edited: the Tables view shows "Re-import this legacy network to enable versioned editing." (`FE features/domain/DomainDataViewer.tsx:381-396`, `BE service/ProjectDomainRevisionService.java:63-80`).
- There is no UI to pick an older revision as the active domain. The only way is to import that file again (F13). Runs keep a link to their snapshot and revision (F41).

### F9. Save revision dialogs

- Editor **Save** opens "Save edited input" (eyebrow "Save revision"): "The original file will not be modified.", box "Original file <name> Revision <version>", two options (`FE features/domain/SaveRevisionModeModal.tsx:14-88`):
  - **Save automatically**: "NeatWork creates the next revision filename automatically.", tag "Recommended". Saves at once with the default name.
  - **Choose file name**: "Enter the visible filename for the new revision." Opens F10.
- Errors of the save show in this dialog.

### F10. Custom revision name

- Dialog "Choose file name" (eyebrow "Custom revision"): "Original file <name>", "This file will remain unchanged.", field "New file name", hint "If the extension is omitted, NeatWork adds the correct one automatically.", buttons **Back** and **Save new version** ("Saving...") (`FE features/domain/CustomRevisionNameModal.tsx:25-102`).
- Suggested name: source base without a trailing `_v<N>`, plus `_edited`, plus the extension. Example `Net_v3.dsg` → `Net_edited.dsg` (`:15-22`).
- After a save, the Tables view shows "Saved as <file name>" (`FE features/domain/DomainDataViewer.tsx:355`).

### F11. Delete an input file (archive)

- In the file library (F13), project-owned files have **Delete**. Dialog "Delete input file?": "<name>", "This file will be removed from the active project library.", "Historical file content and previous simulation results will be preserved.", "This file will no longer be available for new imports.", check box "I understand that this file will be removed from the active project library.", button **Delete file** (`FE features/artifacts/DeleteInputArtifactModal.tsx:15-137`).
- Effect: the file is marked archived, not deleted (`BE service/ProjectArtifactService.java:405-442`). Snapshots and runs stay. An archived source file cannot be read as text or edited (`:172-174`, `:466-468`), so the Source File view of a domain whose file is archived shows an error (UNVERIFIED).

## 3. Import

### F12. Import Network dialog ("Select input format")

- Eyebrow "Network input", title "Select input format", "Project: <name>" (`FE features/import/InputTypeModal.tsx:139-243`).
- Two cards: **DSG** "Open an existing NeatWork design file." `.dsg`. **TPO** "Open a NeatWork topology input file." `.tpo`. A card is disabled while busy, when import is blocked, or when F7 allows only the other format (`:165-180`).
- Drop zone (drag and drop):
  - Title: "Drop network file here", "Release file to import" while a file is over it, "Importing network..." while busy (`:211-216`).
  - Sub-text: "Network import is currently blocked for this project." / "This project accepts DSG files only." / "This project accepts TPO files only." / "Drop a .dsg or .tpo network file here." (`:123-137`).
  - "or click to browse your computer" (file chooser filtered on `.dsg,.tpo`). Enter or Space on the focused zone also opens the chooser (`:112-121`, `:202-207`).
  - Only the first dropped file is used (`:102-110`). Drag enter/leave events are counted so the highlight does not flicker (`:34-90`).
- Dropped or browsed file (`FE App.tsx:1542-1556`):
  1. The format comes from the extension only (`.dsg` or `.tpo`, case-insensitive). Other files: "Unsupported network file. Drop a .dsg or .tpo file." (`FE App.tsx:1926-1937`).
  2. F7 is checked before the upload.
  3. The file is stored in the project (F8).
  4. DSG: imported at once (F14). TPO: requirement check (F15). If the TPO has a `!Diameters` section, it is imported at once. If not, the TPO database dialog opens (`FE App.tsx:1445-1482`).
  5. On failure the dialog stays open with the error.
- Card click: opens the file library for that format (F13).

### F13. File library ("Select DSG/TPO file")

- Eyebrow "<F> input", title "Select <F> file", "Select an existing file from the artifact library." (`FE features/import/InputFileLibraryModal.tsx:95-246`).
- The library lists the non-archived, non-generated files of this type of all projects, newest first (`BE service/ProjectArtifactService.java:234-244`).
- Search "Search <F> files..." matches the file name or the owner project name (`:75-93`).
- Toolbar "Existing <F> files" and **Upload new <F>** (chooser filtered on the extension). An upload of the wrong extension: "Please select a .dsg file." / "Please select a .tpo file." (`FE App.tsx:1493-1497`).
- Row: file name, owner project ("Unknown project"), "v<version> · <size/1024, 1 decimal> KB · <date>", "Already in this project" when owned by the active project. Buttons **Import** ("Importing...") and, for files of the active project only, **Delete** (F11).
- Empty: "No <F> files are available." Loading: "Loading files...".
- **Back** returns to F12. **Close** closes the whole workflow.
- Import of a file of another project copies it into the active project first (F8 numbering), then imports the copy (`FE App.tsx:1324-1433`, `BE service/ProjectArtifactService.java:247-319`).

### F14. DSG import

- The network name is the file name without `.dsg`/`.tpo`, or "Network" (`FE App.tsx:1918-1920`).
- The parser needs `!Nodes` and `!Pipes`. A missing `!Clients` section gives a warning (README_EN "DSG requirement").
- Import is refused while a simulation runs: "Cancel the running simulation before importing a new domain." (`FE App.tsx:1156-1160`).
- A successful import stores the snapshot, replaces the graph, sets the project format and shows the Visual Graph (`BE service/ProjectDomainImportService.java:36-140`, `FE App.tsx:1147-1216`).
- Text files are read as UTF-8, then as ISO-8859-1 (`BE service/ProjectArtifactService.java:198-212`).

### F15. TPO import and complementary databases

- Requirement check: a section is present if a trimmed line equals `!diameters` / `!orifices`, case-insensitive. If `!Diameters` is missing, a diameters database is required (`BE service/ProjectArtifactService.java:322-346, 594-599`).
- Dialog "Complementary databases" (eyebrow "TPO configuration", sub-title the TPO file name). "Analysing TPO file..." while it checks (`FE features/import/TpoDatabaseModal.tsx:112-324`).
- Analysis lines: "!Diameters — Included in TPO / Not included", "!Orifices — Included in TPO / Not included".
- Card "Diameters database", label "Required" or "Optional". Selector first entry: "Use !Diameters from TPO" / "Select diameters.db" / "No external diameters DB". Other entries "<file> · <project> · v<version>". Message when required and empty: "This TPO has no !Diameters section. Select or upload a diameters database." **Upload diameters.db** (`.db` only: "Please select a .db file.").
- Card "Orifices database", always "Optional": "Use !Orifices from TPO" / "No external orifices DB", "Select an external orifices database only when required by the network.", **Upload orifices.db**.
- An uploaded database is selected at once (`:67-90`).
- **Import TPO** is enabled if three conditions are true: the analysis is done, no upload runs, and a required diameters database is chosen (`:94-98`). **Back** returns to the library. **Cancel** closes.
- Import copies the TPO and the chosen databases into the project, then parses (`FE App.tsx:1595-1697`). Server check: "TPO import requires a diameters database. ..." (`BE service/ProjectDomainImportService.java:80-87`).
- TPO pipes of length ≤ 0 become `VALVE` edges. Other pipes are `PIPE` (`PY neatwork/parsers/tpo_parser.py:175`).

### F16. JSON import

- The parser accepts a JSON file `{graph, domain}` or a bare graph (`PY neatwork/parsers/json_parser.py:13-45`), and the artifact type `JSON_INPUT` exists.
- NOT REACHABLE: the import service accepts only DSG and TPO as primary sources: "Only DSG and TPO can be imported as primary network sources." (`BE service/ProjectDomainImportService.java:51-53`). The UI offers only DSG and TPO.
- The user-visible "JSON" is the parsed-domain view of F48.

### F17. Database manager (Database menu)

- Dialog title "Diameters Database" or "Orifices Database", "Browse and manage your database files." (`FE features/database/DatabaseManagerModal.tsx:25-295`).
- Search "Search databases..." on file name or project name. **Upload .db** (needs an open project: "Open a project before uploading a database.". `.db` only).
- Rows: file name, "<project> · v<version> · <KB> KB", **Download**, **Remove**.
- **Remove** asks "Remove from Library?": "<file> will no longer appear in the database library. Existing project files and results will be preserved.", check box "I understand that this file will be removed from the Library.", **Remove** ("Removing..."). The file is archived (`BE service/ProjectArtifactService.java:349-364`).
- The databases have no viewer or editor in NW5. The domain tables "Pipe diameter catalog" and "Orifice diameter catalog" show the catalogs inside a domain (F27).

### F18. Import errors and warnings

- Import warnings of the parser show as a box "Import warnings" with a list in the Tables view (`FE features/domain/DomainDataViewer.tsx:406-415`).
- The UI shows the server message (`message`, `detail`, or `detail.message`), or a default text (`FE App.tsx:105-136`).

## 4. Network graph views

### F19. Graph data

- Node: `id`, `label`, `type` (`RESERVOIR` for status 0, `FAUCET` for 2, else `JUNCTION`), visual `x`,`y`, source `sourceX`,`sourceY`, `elevationM`, `pressureM`, `lowPressure`, `layoutGenerated` (`FE features/network/types.ts:5-17`, `PY neatwork/parsers/parsing_utils.py:83-91`).
- Edge: `id` (`A<index+1>` in pipe order), `sourceId`, `targetId`, `type`, `status` (`OPEN`/`CLOSED`), `flowRateLps`, `velocityMps`, `lengthM`, `diameterId`, `nominalSize`, `sdr`, `innerDiameter`, `cost`, `pmax`, `pipeTypeCode`, `roughness` (`FE features/network/types.ts:19-37`, `PY neatwork/parsers/dsg_parser.py:144-160`).
- Generated layout: when every node has x = y = 0, the parser places nodes as a tree. Roots are status-0 nodes, else nodes without incoming pipes, else the first node. x = depth × 160. A leaf gets y = 80 × (leaf counter). A parent gets the mean y of its children. If visual and source coordinates differ, `layoutGenerated` is true (`PY neatwork/parsers/dsg_parser.py:238-306, 95-125`).

### F20. Graph menu

**Graph ▾** opens a radio menu with **Visual Graph**, **Raw 2D Graph** and **Linear Graph**. The selected one has a ✓. Escape or an outside click closes it (`FE App.tsx:2251-2329, 2141-2168`).

### F21. Visual Graph (3D view)

- Header: graph name. "This view uses generated visual coordinates when the source file has missing or zero coordinates." (`FE features/network/NetworkCanvas.tsx:747-763`).
- Background `#08182d`.
- Projection (`:659-734`), with local coordinates relative to the bounding-box center and z = elevation:
  - **Iso** (default, azimuth 0°, tilt 63°): rotate x/y by the azimuth. Screen y = rotatedY·cos(tilt) − z·zIso, with zIso = max(xRange, yRange)·0.12 / zRange.
  - **X**: horizontal = y, vertical = −z·(yRange·0.35/zRange).
  - **Y**: horizontal = x, vertical = −z·(xRange·0.35/zRange).
  - **Z**: horizontal = the longer of x/y, vertical = −z·(range·0.35/zRange).
  - Ranges below 1 count as 1.
- Overlay controls (`:765-835`): check box **Show Details**. Zoom **−** (×0.8 about the center), **Fit**, **+** (×1.25). Orientation group with axis buttons **X**, **Y**, **Z** ("Top / height view"), rotate ↺/↻ (±15°), tilt ↑/↓ (±8°, clamped 20°–82°), **Iso**.
- Zoom limits 0.01–5000. Mouse wheel zooms about the pointer by 0.995^deltaY. Drag pans. A move of more than 2 px is a pan and does not select (`:568-657`).
- Fit: 90 px padding, same scale on both axes. If the graph, the bounds, the rotation or the view mode change, it runs again (`:133-177`).
- Node drawing (`:429-481`): radius 10 when selected, 8 for `RESERVOIR`, 5 for `FAUCET`, else 4. Fill color (`:327-334`):

| Condition (first match) | Color |
| --- | --- |
| `lowPressure` | `#ef4444` |
| `RESERVOIR` | `#38bdf8` |
| `FAUCET` | `#fb923c` |
| pressure ≥ 30 m | `#22c55e` |
| pressure ≥ 18 m | `#facc15` |
| other | `#fb923c` |

- A label shows if the node is selected or flashed, or if the scale is above 0.036. **Show Details** adds `z=<elev> <u> · p=<pressure> <u>` (2 decimals, display unit).
- Edges (`:358-405`): open `rgba(56,189,248,0.45)` width 1.4 (selected `#7dd3fc` width 5). Closed `rgba(100,116,139,0.55)`. A valve has a 12 px square at mid-edge, green `#22c55e` open, red `#ef4444` closed. If the edge is selected or the scale is above 0.07, a direction arrow shows at mid-edge. **Show Details** or selection adds the label `<id> · <flow, 2 decimals> L/s`.
- Hit test: node radius 12 (reservoir), 10 (faucet), 9 (other). Edge distance < 7 px. Nodes win (`:541-566`).

### F22. Raw 2D Graph

- Same canvas with x/y = `sourceX`/`sourceY` (falls back to visual x/y) and no 3D controls (`FE App.tsx:1857-1870`, `FE features/network/NetworkCanvas.tsx:660-665, 783`).
- Header "Raw 2D network graph", "This view uses the original X/Y coordinates from the imported file."
- When all raw coordinates are equal: "All raw coordinates are identical. Nodes may overlap because the source file does not provide usable X/Y coordinates." (`:72-75, 837-841`).

### F23. Linear Graph (topology view)

- Header "Linear topology", "Source-to-terminal hierarchy with loop edges preserved.", counters "Sources N", "Nodes N", "Pipes N". Legend "Upstream path" and "Loop edge". Zoom −/Fit/+ (fit scale capped at 2.5) (`FE features/network/LinearNetworkView.tsx:531-617`).
- Layout (`FE features/network/networkTopology.ts:36-349`):
  1. Undirected adjacency. Neighbors sorted by edge id.
  2. Roots: `RESERVOIR`/`TANK` nodes. Else nodes without incoming edges. Else the first node.
  3. Multi-source breadth-first search sets depth, parent node and parent edge. Each unvisited node starts a new component.
  4. y: a leaf takes the next row (rows 74 apart from y = 80). A parent is centered between its first and last child. 110 between components.
  5. x = 90 + depth × 190.
  6. Tree edges: orthogonal elbow through the middle x. Other edges (loops): dashed, routed on a lane above or below (alternating), 44 + (i mod 5)·18 away, with 34 px horizontal stubs.
- Selection highlight: the path from the selection to its root in `#facc15` (width 4.2, 5.5 when selected). For an edge, the path starts at its downstream end. For a loop edge, at the end closer to a source (`:351-419`).
- Node colors as F21. Valve squares on the middle point. Focus zooms to at least 0.9.

### F24. Inspector (selection pop-up)

- A click on a node or an edge opens a floating inspector next to the click point, on the side with room (360 × 420 px estimate). × or a click on empty canvas closes it (`FE features/network/FloatingNetworkInspector.tsx:28-73`, `FE features/network/NetworkCanvas.tsx:632-656`).
- Node (`FE features/network/NetworkInpector.tsx:66-130`). The title is the label. Rows:

  | Row | Value |
  | --- | --- |
  | Type | node type |
  | X, Y | source coordinates, display unit, 2 decimals |
  | Visual X, Visual Y | 1 decimal, only for a node with `layoutGenerated` |
  | Elevation, Pressure | display unit, 2 decimals |
  | Status | "LOW PRESSURE" or "NORMAL" |

  Below the rows: a form with "Visual graph X", "Visual graph Y" and **Save position**.
- Edge (`:132-193`). The title is the edge id. Rows:

  | Row | Value |
  | --- | --- |
  | Type, Status | as stored |
  | Flow | "<2 decimals> L/s" |
  | Velocity | "<2 decimals> <u>/s" |
  | Length | display unit, 2 decimals |
  | Diameter ID, Nominal, Pipe type | text |
  | SDR, Roughness | 2 decimals |
  | Inner | display unit, not rounded |
  | Cost | 3 decimals |
  | pmax | 1 decimal |

  If flow and velocity are both 0, these two rows show "Not simulated". A missing value shows "-".
- Valves only: **Close valve** / **Open valve** ("Saving...").

### F25. Valves and node positions

- Open/close a valve: refused while a run is active: "Cancel the running simulation before changing network state." (`FE App.tsx:1053-1072`). The state is stored in the project graph. A closed edge gets flow 0 and velocity 0. An opened edge with flow 0 gets placeholder values 0.35 L/s and 0.20 m/s (`BE service/NetworkMutationService.java:24-40`, DEFECT: fake values). The visible results are cleared.
- The open/closed state of every edge is copied into each new run (`edgeOverrides`) and is applied by the solver (`BE service/SimulationRunService.java:106-112`, `PY services/simulation_service.py:43-126`).
- Move a node: only through the inspector form (no drag). Values must be finite numbers. The node's visual x/y are stored in the project graph (`FE features/network/NetworkInpector.tsx:39-52`, `BE service/NetworkMutationService.java:42-51`). The form edits visual coordinates even in the Raw 2D view (UNVERIFIED whether intended).
- Both states are lost on import or revision save (F8).

### F26. Node search

- Field "Search node..." and **Find** (`FE features/network/NetworkNodeSearch.tsx:15-89`):
  1. Empty: "Enter a node ID."
  2. Exact match (case-insensitive) on id or label wins.
  3. Else a unique substring match.
  4. Several matches: "<n> nodes match. Enter a more specific ID." None: "Node not found."
- Found node: the view centers on it (keeps the zoom in Visual/Raw. At least 0.9 in Linear) and flashes it pink `#f472b6` with glow for 2.5 s (`FE features/network/NetworkCanvas.tsx:178-213`).
- The Raw view searches the raw graph (`FE App.tsx:191`).

## 5. Domain tables (read-only)

### F27. Tables view

- Header: domain name (or "Imported NeatWork domain"), "Source: <format> · Imported: <date time>", **Edit input data** (F31) (`FE features/domain/DomainDataViewer.tsx:370-404`).
- Left: list of tables with "<n> rows". Right: the selected table (`:417-439`).
- Table groups, in this order. A table is found by name aliases (normalized: trimmed, lower case, spaces/hyphens → `_`). A non-empty table wins over an empty one (`FE features/domain/tableUtils.ts:33-70, 290-377`):

| Title | Description | Aliases |
| --- | --- | --- |
| Network nodes | "Network topology nodes with terminal pressure and NeatWork metadata." | nodes, node_list, nodelist |
| Network pipes | "Pipe links imported from the network topology file." | pipes, pipe_list, pipelist, arcs, arc_list, arclist, edges, edge_list, edgelist |
| Pipe diameter catalog | "Available pipe diameters imported from the DSG file or diameter database." | diameters, diameter_list, ... |
| Orifice diameter catalog | "Available orifice diameters imported from the DSG file or orifice database." | orifices, orifice_list, ... |
| Client behaviors | "Reusable demand behaviour definitions shared by client instances." | clientBehaviors, client_behaviors, ... |
| Client demand table | "Client instances attached to terminal outlets and linked to reusable demand behaviors." | clients, client_list, clientlist, generated_clients |
| Terminal outlets | "Faucet or terminal outlet nodes used as consumption points." | bornes, taps, ..., outlets |

- Empty `clients`/`bornes`/`taps`/`terminal_outlets` tables are hidden. Unknown tables follow with a prettified name and "Imported table: <key>".
- Terminal outlets is not shown alone. Its `pressure` and `N` values are added as the last columns of Network nodes, matched on the node id (`FE features/domain/DomainDataViewer.tsx:93-132, 293-313`).
- Columns are the union of row keys in first-seen order, plus a "#" row index column (`FE features/domain/tableUtils.ts:164-175`).
- Column labels (`FE features/domain/tableUtils.ts:72-130`): noeud "Node ID", haut "Elevation", xdata "X coordinate", ydata "Y coordinate", orif_ideal "Ideal orifice", orif_com "Commercial orifice", statut "Status", pred "From", succ "To", lon "Total pipe length", lon1 "Segment 1 length", ID1 "Segment 1 diameter ID", lon2 "Segment 2 length", ID2 "Segment 2 diameter ID", N "NeatWork flag", Nominal "Nominal size", diam "Inner diameter", cost "Cost", Pmax "Maximum pressure", Type "Pipe type code", client "Client", borne "Outlet", proba "Open probability", cible "Target flow", behaviorId "Behavior ID", name "Name", openProbability "Open probability", targetFlowLps "Target flow", profileId "Demand profile". Others: underscores → spaces, words capitalized.
- Unit suffix: length columns get "(<display unit>)" (F55), `cible`, target flow → "(L/s)". Probability columns → "(ratio)" (`:132-143, 190-206`).
- Header badges: rows count. Network nodes "Length: <u> · Orifice: <ou>". Orifice catalog "Unit: <ou>". Other tables "Unit: <u>" (`FE features/domain/DomainDataViewer.tsx:851-889`).

### F28. Node semantics and colors

- Network nodes shows a legend Source / Junction / Terminal and colors the Node ID cell by the status code (`FE features/domain/DomainDataViewer.tsx:852-866, 918-926`). The Status column itself is hidden (`:837-839`).
- Pipe From/To ids get the color of the node status found in Network nodes (`:943-948`).
- Colors (`FE styles/features/node-semantics.css:76-135`):

| Role | Text | Background | Dot |
| --- | --- | --- | --- |
| Source | `#7dd3fc` | `rgba(56,189,248,0.07)` | `#38bdf8` |
| Junction | `#fbbf24` | `rgba(245,158,11,0.07)` | `#f59e0b` |
| Terminal | `#86efac` | `rgba(34,197,94,0.07)` | `#22c55e` |

- Status mapping: 0/"SOURCE" → Source. 1/"JUNCTION"/"TRANSIT" → Junction. 2/"TERMINAL" → Terminal. Else no role ("—") (`FE features/network/nodeSemantics.tsx:36-106`).

### F29. Cell formatting in read-only tables

Rules, first match (`FE features/domain/DomainDataViewer.tsx:913-978`, `FE features/domain/tableUtils.ts:244-288`):

1. Empty value: "-".
2. NeatWork flag `N`: empty or "N" → "—", else the text.
3. Network nodes, orifice columns: "—" unless the node is a Terminal. Value in mm for metre projects, else in the display unit, 2 decimals (`:573-583`).
4. Network nodes, terminal `pressure` empty: "—".
5. Network pipes, ID1 (and ID2 when lon2 ≠ 0): "<id> (<inner diameter> <u>)" using the catalog. 3 decimals for m, 2 for ft and in (`:777-815`). Unknown id: the id alone.
6. Network pipes, lon2 = 0 (|x| < 1e-12): "—". ID2 then "—" too.
7. Orifice catalog `diameter_m`: mm with 2 decimals (m projects), ft with 4, in with 3 (`:597-608`). The `diameter_mm` column is hidden.
8. Client demand table `behaviorId`: the behavior name.
9. Identifier columns (noeud, node, id, pred, succ, id1, id2, client, borne, outlet, statut, status, source_index, n, nominal): shown as is.
10. Numbers and numeric strings: for a length column, converted to the display unit (F55). Then the F54 adaptive format.
11. Booleans "Yes"/"No". Objects as JSON.

### F30. Data the tables need

The full domain snapshot: every table row with its original keys, `params`, `warnings`, `name`, `sourceFormat`, and the creation time of the snapshot.

## 6. Domain editor

### F31. Edit input data

- **Edit input data** opens the editor on a deep copy of the active domain. Client rows are normalized first (F34) (`FE features/domain/DomainEditor.tsx:40-58`).
- Header "Editing input data", "Changes are stored only when a new revision is created.", **Cancel** (drops the draft) and **Save** (F9) (`:314-338`).
- Two tabs: **Input tables** and **Default properties** (`:340-355`).
- Input tables lists all tables of the draft (Terminal outlets is shown on its own here, not merged) (`:356-385`).
- Default properties: one field per key of `params`, except `seed`/`randomSeed`. The label is the raw key. Values use the same parser as cells (`:393-412`).

### F32. Editable table

- Header "<title>", "Edit input values.", status texts, selection summary ("<n> cells selected" and "<rows> × <cols>", or "No selection"), shortcut hints "Ctrl/Cmd+C Copy", "Ctrl/Cmd+V Paste", **Clear**, **+ Add row** (`FE features/domain/DomainEditor.tsx:946-998`).
- Columns: "#", one per key (labels as F27, with units), "Actions" with **Delete** per row (`:1038-1114`).
- Cell editor: a text field. Enter or leaving the field commits. Escape restores the value. An invalid value keeps the text, marks the cell red and shows the message as a tooltip and in the table header (`FE features/domain/ValidatedCellInput.tsx:15-63`).
- Shown value while editing: the canonical value converted to the display unit, 10 significant digits (`FE features/domain/tableUtils.ts:384-405`).
- Client demand table `behaviorId`: a drop-down "Select behavior" plus every behavior name (or id) (`:1074-1090`).
- Selection: mouse down selects a cell. Dragging extends a rectangle. Shift+click extends from the anchor. Escape clears (`:727-771`).
- Bulk editor: if the selection has 2 or more rows and 1 column, it shows. "<n> cells selected", the column label, a field "New value", **Apply to selected**, **Clear selection**. For a status column, buttons **Source** (0), **Junction** (1), **Terminal** (2). Success: "Updated <n> cells" (`:773-806, 999-1023`).
- Copy: the selection as tab/newline text in display units ("Copied <r> × <c> cells"). Normal text copy inside a field with selected text is kept (`:808-851`).
- Paste (`:853-944`):
  - Clipboard text is split on newlines (CRLF/CR → LF) and tabs. One trailing empty line is dropped.
  - One value and a selection of 2+ cells: the value fills the selection.
  - Otherwise the matrix is placed at the top-left cell of the selection. If it goes past the last row or column: "The pasted range does not fit inside this table." Rows are not added.
  - A single value into a single focused cell uses normal field paste.
  - All values are parsed first. The first error stops the paste: "Invalid pasted value at row <r>, <column label>: <message>". Nothing is changed on error.
  - Success: "Pasted 1 cell" / "Pasted <n> cells". The pasted rectangle becomes the selection.
- **+ Add row** appends a row with every known column set to "" (`:187-202`).
- **Delete** removes the row from the draft after the reference check (F33).

### F33. Validation rules

Column names are normalized by lower case and removal of spaces, `_` and `-` (`FE features/domain/domainEditorValidation.ts:16-57`).

| Columns | Rule | Message |
| --- | --- | --- |
| statut, status, statuscode, nodestatus, nodestatuscode | integer in {0,1,2} | "This value must be an integer." / "Allowed values are 0, 1 or 2." |
| proba, probability, openprobability, opentaps | 0 ≤ v ≤ 1 | "Value must be at least 0." / "Value must not exceed 1." |
| cible, targetflow, targetflowlps | v > 0 | "Value must be greater than 0." |
| lon, lon1, lon2, sdr, cost, pmax, roughness | v ≥ 0 | "Value must be at least 0." |
| any length column of F55 | finite number | |
| any column whose old value is a number | finite number | |
| other columns | text, kept exactly ("001" stays "001") | |

- Numeric input: trimmed. Empty → "A numeric value is required.". Not a finite number → "\"<text>\" is not a valid number." (`:85-136`). The parser is JavaScript `Number()`: it accepts `1e3`, `0x10`, ` 12 ` and refuses `1,5` (UNVERIFIED for the port: define the accepted syntax).
- Length values are converted from the display unit back to the stored unit (m, or mm for `diameter_mm`) (`FE features/domain/unitSystem.ts:189-198`).
- Bulk error: "Row <r>, <column label>: <message>" (`FE features/domain/DomainEditor.tsx:90-123`).
- At save, in order (`:224-240, 293-312`):
  1. Duplicate behavior ids: "Duplicate client behavior ID: <ids>".
  2. Every stored value of every table: "<table>, row <r>, <column label>: <message>" ("Expected a finite numeric value.", "Expected an integer.", ...).
- Server checks at save (`BE service/DomainCatalogIntegrityService.java`): pipe diameter references must exist ("Invalid pipe diameter reference: ...". Segment 2 only when lon2 ≠ 0). Behavior ids unique. Behavior probability and target flow valid. Client behavior references exist. Then the serializer writes the file and parses it again. Any parse or model error refuses the save (`PY neatwork/serialization/revision_serializer.py:345-391`).

### F34. Referential integrity and client behaviors

- If another row references the row, Delete is blocked (`FE features/domain/catalogIntegrity.ts:455-513`, `FE features/domain/DomainEditor.tsx:242-280`):
  - Node: by pipe start/end ("Pipe start node"/"Pipe end node"), client terminal ("Client terminal"), terminal outlet ("Terminal outlet").
  - Pipe diameter (id or nominal): by ID1 ("Segment 1 diameter") or active ID2 ("Segment 2 diameter").
  - Client behavior: by clients ("Client behavior reference").
  - Orifice diameter checks are commented out. Orifices can be deleted freely.
- Dialog "This item cannot be deleted" (eyebrow "Catalog item in use"), reason (for example "This node is still referenced by the network."), item label, "<n> reference(s)", "Replace these references before deleting the catalog item.", the first 20 references "<entity> <location>", "+<n> more references", **OK** (`FE features/domain/DomainEditor.tsx:462-541`).
- Ids compare trimmed and case-insensitive. "", "n", "-" are not references.
- Client behaviors (demand is owned by the Client demand table, `ClientBehavior` is the source):
  - A behavior has `behaviorId`, `name`, `openProbability`, `targetFlowLps`, `profileId`. A client row has `client`, `borne`, `behaviorId`.
  - At parse time legacy client rows (`proba`, `cible`) become behaviors with id `CB_` + first 10 upper-case hex characters of SHA-1 of `"<p>|<t>|<profile>"` (decimals normalized), name "Imported behavior <n>" (`PY neatwork/domain/client_behavior.py:6-98`).
  - The editor does the same for rows still in legacy form, with ids `CB_LEGACY_001`, ... (`FE features/domain/clientBehaviorUtils.ts:89-175`). DEFECT (UNVERIFIED): the existing behaviors are added twice to the list in that path, which then fails the duplicate check at save.
  - When written back to a DSG/TPO file, clients are written as legacy rows `client, borne, proba, cible`. Behavior names are restored after the re-parse (`PY neatwork/serialization/revision_serializer.py:91-167, 322-343`).
- File writing keeps unknown sections of the source file unchanged (for example `!loadfactor`) and replaces only known ones (`PY neatwork/serialization/revision_serializer.py:249-319`).

## 7. Simulation configuration and runs

### F35. Simulation bar

If a domain is active, the bar shows (`FE features/simulation/SimulationPanel.tsx:61-159`):

- **Simulation** opens the configuration dialog (F36).
- **Run Again** (when a previous run of this session finished in the foreground) starts a run with the same requested overrides, without the old demand fields `openTapsProbability`/`targetFlowLps` (`FE App.tsx:1013-1021, 2128-2135`).
- **Review** (when at least one completed run is known) opens F42 on the newest completed run (`FE App.tsx:2099-2108`).
- **Export** drop-down (with Review) (F40).
- **Cancel** ("Cancelling...") for the foreground run.
- Status "1 hydraulic simulation is running." / "<n> hydraulic simulations are running."
- Summary of the last foreground result: status, "Success <ok>/<total>", "Execution failures <n>", "Low P <n>", "Seed <n>", "Failure records <n>" when present.
- The **History** button is commented out (`:76-84`). NOT REACHABLE: the history dialog (F39) cannot be opened.

### F36. Configure simulation dialog

- Title "Configure simulation" (kicker "Simulation configuration"), text "Values from the active <DSG|TPO|network> input are prefilled. Run immediately to use them unchanged, or edit only the settings you want to override for this execution." Badge "Imported defaults" / "Custom overrides" with "No runtime overrides" / "<n> runtime override(s)" (`FE features/simulation/SimulationDialog.tsx:355-404`).
- Section "Hydraulic configuration": "Configure flow limitation and the terminal-orifice model independently.":
  - **Bounded flow** check box, "Constrain terminal flow according to the current bounded-flow model." Disabled (and on) when demand is not uniform.
  - **Orifice model** radio **Ideal** / **Commercial**, "Select which stored terminal-orifice model is associated with this execution."
  - Note when demand is not uniform: "This network contains heterogeneous client demand. Bounded flow is required by the current solver, while the orifice model is still stored independently."
- Section "Run setup": **Number of scenarios** (integer ≥ 1). **Use a specific seed** ("Enable this option when you want the simulation to be exactly reproducible."). Off: "A fresh random seed will be generated automatically when the simulation starts." On: **Random seed** (integer ≥ 0, starts at 42, hint "Default explicit seed: 42").
- Section "Client demand": "Demand values are read from the active Client demand table and cannot be overridden per simulation.": Source "Client demand table", Clients <n>, Demand profile "Uniform"/"Per client". When uniform also "Open faucets <p, 2 decimals> (<p×100, 0 decimals>%)" and "Target flow <t, 2 decimals> L/s". Note "To change demand values, edit the Client demand table and save a new input revision before running the simulation." Error when some clients cannot be resolved: "Some clients are missing an opening probability or target flow. Complete the Client demand table before simulation." (`:518-581`).
- Collapsible "Advanced scenario generation": "Safety limits for random scenario generation. Defaults suit most networks.": **Maximum generation batches** (integer ≥ 1, default 20), **Oversampling factor** (> 1, default 1.1), **Maximum draws per batch** (integer ≥ 1, default 100000) (`:583-616`).
- Footer: **Reset to imported defaults** (enabled when there are overrides), **Cancel**, and **Run with imported defaults** / **Run with overrides** ("Starting…"). If client demand is incomplete, Run is disabled (`:625-650`).
- Validation messages (`:77-104, 297-301`): "Number of scenarios must be a positive whole number.", "The current solver requires bounded flow for heterogeneous client demand.", "Maximum generation batches must be a positive whole number.", "Oversampling factor must be greater than 1.", "Maximum draws per batch must be a positive whole number.", "Random seed must be a non-negative whole number." Start failure: "The simulation could not be started. Review the message above and try again."
- Focus goes to the first field. Tab cycles inside the dialog. Escape and backdrop click close it, except while starting (`:142-190`).

### F37. Defaults and overrides

- Defaults come from the active domain. Parameter priority: `params` > `designParams` > `technicalParams`. A numeric string can use a comma as decimal separator (`FE features/simulation/simulationDefaults.ts:196-367`).

| Option | Domain keys | Default |
| --- | --- | --- |
| scenarioCount | nbsim, scenarioCount, scenario_count, numberOfScenarios, number_of_scenarios | 100 |
| orificeMode | orif_com, orificeCommercial, orifice_commercial, orificeMode, orifice_mode (IDEAL/0/false → Ideal; COMMERCIAL/1/true → Commercial) | COMMERCIAL |
| boundedFlow | bounded_flow, bounded-flow, boundedFlow, boundedFlowEnabled, bounded_flow_enabled; forced true when demand is not uniform | false |
| serviceQuality | servicequal, serviceequal, serviceQuality, service_quality | 0.65 |
| requiredPressureM | pressure, requiredPressureM, required_pressure_m; else the single distinct terminal pressure | 2 |
| orificeCoefficient | orifcoef, ... | 1 |
| faucetCoefficient | faucetcoef, ... | 1 |
| waterTemperatureC | watertemp, ... | 20 |
| commercialPipeLengthM | pipelength, ... | 6 |
| maxGenerationBatches / oversamplingFactor / maxDrawsPerBatch | camelCase or snake_case | 20 / 1.1 / 100000 |

- Uniform demand: the client table is complete (every client resolves to a probability and a target flow, through its behavior or legacy values) and all clients share one probability and one target flow (`FE features/simulation/simulationDefaults.ts:219-230`, `FE features/domain/clientBehaviorUtils.ts:14-87`).
- The dialog can change only scenario count, seed, bounded flow, orifice model and the three advanced limits. The other options are shown nowhere and are never sent (`FE features/simulation/SimulationDialog.tsx:199-216`).
- Only values that differ from the defaults are sent. An empty request means "Default" configuration mode. Any value means "Manual" (`FE features/simulation/SimulationDialog.tsx:228-278`, `BE service/SimulationRunService.java:126-129`).
- The engine reports resolved values and their source: "Modified by user" (USER_OVERRIDE), "Project default" (PROJECT_DEFAULT), "Generated" (seed), "Not applicable", and CLIENT_DEMAND_TABLE for demand values (`PY services/simulation_service.py:453-514`). The resolved set includes `uniformDemand`, `openTapsProbability` and `targetFlowLps` (only when uniform), `clientDemandCount`, `terminalRequiredPressureM` (per-terminal map) and `solverOptions`.

### F38. Run lifecycle

- A run takes the newest snapshot, the current edge states and the requested options. It gets `runNumber` = highest number of the project + 1 (numbers are never reused) (`BE service/SimulationRunService.java:95-149`).
- Statuses: PENDING → RUNNING → COMPLETED / COMPLETED_WITH_ERRORS / FAILED. PENDING/RUNNING → CANCEL_REQUESTED → CANCELLED. COMPLETED_WITH_ERRORS means at least one scenario had an execution failure (`:219-259`). Runs still active at backend start become FAILED "Simulation interrupted because the backend restarted." (`:343-356`).
- Several runs can be active at once. The newest started run is the foreground run: only its result updates the graph, the bar summary and **Run Again** (`FE App.tsx:850-1010`).
- The UI polls active jobs every 700 ms (1500 ms after an error) (`FE App.tsx:897-1010`). The port needs no polling (F57).
- Each completed run opens the review on its result, one at a time. A queue holds the others until the review closes (`FE App.tsx:240-330, 2054-2071`).
- Foreground failure shows the run error in the banner (`FE App.tsx:965-967`).
- Cancel is cooperative. A run that completes after a cancel request is marked CANCELLED (`BE service/SimulationRunService.java:219-228, 295-307`).

### F39. Simulation history (NOT REACHABLE, see F35)

- Dialog "Simulation History" (eyebrow "Project simulations"), "Running, completed,failed and cancelled executions.", search "Search simulation number, status or seed...", "<n> executions" (`FE features/simulation/SimulationHistoryDialog.tsx:54-239`).
- Newest first. Row: "#<runNumber> Simulation", status badge, created date-time, input name, "<file> · v<version> · <format>", badge "Default"/"Manual", "Scenarios <n>", "Seed <n>", error text. Buttons **Results** (completed runs), **Details** (F41), **Cancel** (PENDING/RUNNING), "Cancelling…" (CANCEL_REQUESTED), **Delete** (terminal statuses).
- Delete dialog "Delete Simulation #<n>?" (eyebrow "Permanent action"): "The simulation result will be permanently deleted.", "Its source project, DSG/TPO file and domain snapshot will not be deleted.", Status/Seed/Scenarios, **Delete Simulation** (`FE features/simulation/DeleteSimulationDialog.tsx:15-92`). Active runs: "Cancel the simulation before deleting it." Deleting clears the saved comparison workspace (`FE App.tsx:1700-1745, 2026-2043`).

### F40. Export

- **Export** menu items JSON, XLSX, DSG, INP. If a format is not available, its item is disabled (`FE features/export/ExportControl.tsx:17-106`, `FE features/export/types.ts:17-42`).

| Format | Content | Available when |
| --- | --- | --- |
| SIMULATION_JSON | statistics of the newest completed run, pretty JSON, `simulation_statistics.json` | a completed run exists |
| SIMULATION_XLSX | spreadsheet of the same statistics (`PY neatwork/simulation/spreadsheet_exporter.py`), `simulation_statistics.xlsx` | a completed run exists |
| DSG | newest DSG output file, else newest DSG input file | such a file exists |
| INP | newest EPANET INP output file | such a file exists |

- Generated files are versioned and kept in the project outputs, then downloaded (`BE service/ProjectExportService.java:37-123`).
- "Newest" means the latest `completedAt` over all revisions, not the run being reviewed (`BE service/SimulationRunService.java:48-52`).
- No UI produces DSG_OUTPUT or INP_OUTPUT files in this version (UNVERIFIED: the EPANET adapter exists in `PY neatwork/epanet/` but no route found).

### F41. Execution details

- Dialog "Simulation #<n>" (eyebrow "Execution details"), "Exact input and reproducibility parameters used by this execution." (`FE features/simulation/SimulationDetailsDialog.tsx:74-234`). Opened only from F39 (NOT REACHABLE).
- Sections: **Input data** (Network, File, File version, Format, Domain snapshot). **Execution** (Configuration, Status, Scenarios, Effective seed, Bounded flow, Orifice model). **Requested parameters** (table Parameter/Value, or "No runtime overrides were submitted. Project defaults were used."). **Resolved parameters** (Parameter/Value/Source, or "No resolved parameter set is available because this execution did not complete successfully."). Collapsible "Pipe state used by this simulation · <n> pipes" (id → OPEN/CLOSED). **Error**.
- `requiredPressureM` and `commercialPipeLengthM` show in the display unit with 2 decimals. `terminalRequiredPressureM` as JSON converted per entry.
- DEFECT: the source label has no case for CLIENT_DEMAND_TABLE and shows "—" (`:30-43`).

## 8. Simulation review

### F42. Review dialog

- Full dialog "Review simulations" (eyebrow "Hydraulic review"), "Inspect one persisted simulation or compare two simulations side by side." (`FE features/simulation/SimulationReviewDialog.tsx:88-201`).
- Tabs **Single simulation** and **Comparison**. Comparison is disabled with fewer than two completed runs: "At least two completed simulations are required to enable comparison."
- Escape closes it, then the next queued result opens (F38).

### F43. Single simulation mode

- Selectors **Simulation** ("Simulation #<runNumber>", all completed runs of the project, oldest first) and **Table** (the four tables of F45) (`:135-183`). Only the chosen table is shown.
- "No completed simulation is available yet." when empty. Runs without statistics: "This simulation does not contain result tables." (`FE App.tsx:254-288`).

### F44. Run summary and result header

- Card "Selected execution — Simulation #<n>", badge "Default"/"Manual", parameters (`FE features/simulation/SimulationReviewRunSummary.tsx:125-205`):
  - Bounded flow: "Enabled" / "Disabled" / "—".
  - Orifice model: "Ideal" / "Commercial".
  - Target flow: "Per client" when demand is not uniform, else "<2 decimals> L/s".
  - Open faucets: "Per client", else "<p, 2 decimals> (<p×100, 0 decimals>%)".
  - Values come from resolved options, then requested options.
- Collapsible "Run traceability": Input network, Source file, Version "v<n>", Format, Scenarios, Effective seed.
- Summary strip: Status, Scenarios, Successful, Execution failures, Success rate (ratio as %), Duration "<adaptive> s" (`FE features/simulation/SimulationResultsDialog.tsx:94-122`).
- "Execution failures" counts only scenarios that failed in preparation, solving or convergence. Water shortfalls are not failures. They are counted per terminal (F46).
- Section "Execution failure records": "Scenarios that failed during preparation solving or convergence.", table Scenario, Stage, Code, Message, "<n> records" (`:151-188`).
- Footer (stand-alone mode only): "Result values use adaptive display precision. Stored simulation values retain full numerical precision."

### F45. Result tables

Built from the statistics (`FE features/simulation/simulationResultTables.ts:109-338`). Every table has "#", a row count, and sortable headers (F47).

**Terminal Satisfaction**: "Percentage of requested terminal demand satisfied across successful scenarios."

| Column | Key | Format | Shown |
| --- | --- | --- | --- |
| Terminal | terminalId | Terminal color | always |
| Minimum (%) | minSat | ratio as % | always |
| Mean (%) | moySat | ratio as % | always |
| Maximum (%) | maxSat | ratio as % | always |
| CVaR 5% (%) | cVarSat5 | ratio as % | always |
| CVaR 20% (%) | cVarSat20 | ratio as % | always |
| Clients | clientCount | integer | details |
| Target flow / client (L/s) | one group: the value; several: "<n> groups" | raw text | details |
| Open probability / client | one group: the value; several: "<n> groups" | raw text | details |
| Active scenarios | activeScenarioCount, tooltip "Scenarios in which at least one client attached to the terminal was open." | integer | details |
| Water shortfalls | waterShortfallCount, tooltip "Successfully solved active scenarios in which delivered flow was below the target demand." | integer | details |
| Std deviation | stdSat | ratio as % | details |
| CV | cvSat (std/mean) | adaptive number | details |

- **Show details** / **Hide details** in the table header shows the "details" columns. If the user chooses another run, it resets (`FE features/simulation/SimulationResultsDialog.tsx:40-44, 124-149`). This is the "expandable terminal reliability and demand details".
- The per-group demand panel (`FE features/simulation/TerminalDemandDetails.tsx`) has its body commented out and is not used (NOT REACHABLE).

**Source Flow**: "Flow statistics at network sources.": Source (Source color), Minimum, Mean, Maximum, CVaR 5%, CVaR 95% (unit from the payload, default L/s).

**Pipe Velocity**: "Velocity statistics for network pipes.": Pipe, From and To (colored by the role of the node in the statistics), Minimum, Mean, Maximum (unit default m/s, converted to the display unit).

**Pressure**: "Pressure-head statistics at terminal and transit nodes.": Node (colored), Type (badge), Minimum, Mean, Maximum (unit default m, converted). Terminal rows first, then junction rows (`:85-105`). This is the "unified Pressure table" that replaces separate terminal and transit tables.

- Missing values show "—". Percent columns format the ratio × 100 with F54 and "%". Ratios above 1 show above 100% (unbounded flow can deliver more than the target) (`FE features/formatting/numericDisplay.ts:38-48`).
- Length and velocity columns convert from m and m/s to the display unit. Flows stay in L/s (`FE features/simulation/SimulationResultTable.tsx:64-84`).

### F46. Statistic definitions

From `PY neatwork/simulation/statistics.py`:

- Only successful scenarios count. If at least one attached client is open in a scenario, the terminal is active in that scenario.
- Satisfaction statistics of a terminal use the scenarios that are successful and active (`:187-246`):
  - min, mean, max. Std with ddof = 0. CV = std/mean, null when |mean| ≤ 1e-12. CVaR p% = mean of values ≤ the p-th percentile (p = 5, 20).
  - A terminal with no evaluated active scenario has all these values null (shown "—").
  - waterShortfallCount = evaluated active scenarios with satisfaction < 1 − 1e-9.
  - activeScenarioCount counts over all scenarios. EvaluatedActiveScenarioCount over successful ones.
- Source flow (L/s, converted from m³/s): min, mean, max, CVaR 5% (mean of values ≤ 5th percentile), CVaR 95% (mean of values ≥ 95th percentile) (`:248-278`).
- Pipe velocity: |flow| / (π d²/4) with d = diam1, for source and transit pipes only. Min, mean, max (`:280-319`).
- Pressure: min, mean, max per terminal and per transit node (`:321-365`).
- Demand profile per terminal: client count and groups of clients with the same effective behavior (behavior ids, names, client count, probability per client, target flow per client, profile) (`FE features/simulation/types.ts:122-145`, `PY services/simulation_service.py:376`).
- Schema versions: 1.0 (base), 1.1 (std, CV, active and shortfall counts), 1.2 (demand). Older stored results stay readable: missing fields show "—" (`FE features/simulation/types.ts:56-79, 147-161`).

### F47. Sorting in result tables

- A header click cycles ascending → descending → original order (`FE features/simulation/SimulationResultTable.tsx:219-237`). Icons ↕ / ↑ / ↓.
- Empty values always sort last. Numeric columns compare the raw numbers (not the formatted text). Text compares with natural order (numbers inside text compare by value), case-insensitive. Ties keep the original order (`:98-130, 179-217`).
- If the table changes or the column becomes hidden, sorting resets.

## 9. Comparison between revisions

### F48. Comparison mode

- Selectors **Left simulation**, **Right simulation** (an option chosen on one side is disabled on the other) and **Table** (`FE features/comparison/SimulationComparisonDialog.tsx:286-329`).
- Sources: all completed runs of the project, from every revision. Order: runs of the active snapshot first, then by `runNumber` descending (`FE App.tsx:352-437`). The labels are "Simulation #<runNumber>" (`FE features/comparison/comparisonDatasets.ts:442-455`).
- A "Default input" source (the domain tables) exists in code but is filtered out of the review (`FE App.tsx:434-437`). NOT REACHABLE.
- Initial choice: if the sources of the saved workspace still exist, use them. Else left = second-to-last and right = last source of the list (`FE features/comparison/SimulationComparisonDialog.tsx:29-54`). With the order above this picks the oldest runs, not the two newest that the comment describes (DEFECT, UNVERIFIED).
- **Table** lists only tables that both sides have (Terminal Satisfaction, Source Flow, Pipe Velocity, Pressure) (`FE features/comparison/comparisonTableCompatibility.ts:28-56`).
- The left/right/table choice is saved per project and active snapshot (`neatwork-comparison:<project>:<snapshot>` in browser storage) (`FE features/comparison/comparisonPersistence.ts:5-74`).
- States: "Loading comparison data...", "Could not load this comparison source.", "No common table": "These sources do not expose a compatible table structure. Choose two simulation results that can be compared directly."

### F49. Revision compatibility

Computed from the two snapshots (`BE service/DomainSnapshotCompatibilityService.java:214-489`):

1. Profile of a snapshot:
   - nodes = (id, role) from the nodes table (`noeud/node/nodeId/node_id/id`, role = `statut/status/...` as integer, −1 if missing);
   - edges = (id, from, to) from the pipes table (id from `edgeId/edge/pipe/id`). Rows without id are dropped;
   - clients = (client id, terminal id);
   - topology fingerprint = SHA-256 of sorted node and edge lines;
   - demand fingerprint = SHA-256 of sorted client lines;
   - effective input fingerprint = SHA-256 of a canonical form of `params`, `technicalParams`, `designParams` and all tables, without the x/y coordinate columns, with clients replaced by their resolved (client, terminal, probability, target flow, profile) rows. Numbers are compared without trailing zeros. Lists are sorted.
2. Level:
   - same snapshot id → SAME_SNAPSHOT (all entities matched);
   - same effective input → EQUIVALENT_INPUT;
   - same topology → SAME_TOPOLOGY;
   - else TOPOLOGY_CHANGED.
3. Warnings: SAME_TOPOLOGY → "Input revision differs. Metric changes may reflect network parameters as well as simulation settings.". Demand fingerprint differs → "Client-to-terminal allocation changed between revisions. Terminal statistics may reflect demand redistribution.". TOPOLOGY_CHANGED → "Network topology changed. Only matched entities should be compared directly."
4. Entity matching (topology-aware), separately for terminals (role 2), sources (0), transit nodes (1): a node is matched when it has the same id and role on both sides and the same sorted list of incident edges "id|from|to". Same id with different incident edges is "context changed" and counts as left-only and right-only. Pipes: matched when same id and same endpoints (`:866-971`).
5. Change summary: node counts, matched (same id and role), added, removed, changed roles. Pipe counts, matched, added, removed, changed endpoints. Terminal counts, matched, added, removed (`:548-675`).
6. `directMetricComparisonAllowed` = level ≠ TOPOLOGY_CHANGED. `partialMetricComparisonAllowed` = true.

DEFECT (UNVERIFIED): DSG/TPO pipe rows have no id column (`PY neatwork/parsers/dsg_parser.py:61`), so the edge set is empty. Then the topology ignores pipes, node context is always empty, and no pipe is ever "matched", so Pipe Velocity rows are never matched, even for the same snapshot. The port must use the edge id `A<n>` or the (from, to) pair.

Banners (`FE features/comparison/SimulationComparisonDialog.tsx:365-473`): "Checking revision compatibility...". Error text. SAME_SNAPSHOT "Exact same input snapshot — Both simulations were generated from exactly the same immutable network input.". EQUIVALENT_INPUT "Equivalent network input — The simulations belong to different revisions, but their effective hydraulic input is identical.". SAME_TOPOLOGY "Same topology — revised input — Node and pipe structure is unchanged, but hydraulic input values differ.". TOPOLOGY_CHANGED "Topology changed — Only entities that exist with the same identity in both revisions are compared directly.". Then the warnings.

For TOPOLOGY_CHANGED a summary shows "Nodes L → R, Matched m · +a · -r", the same for Pipes and Terminals, and a collapsible "View structural changes" with "Added nodes", "Removed nodes", "Changed roles", "Added pipes", "Removed pipes", "Changed endpoints" (`:475-686`).

### F50. Unified comparison table

- Two run summary cards LEFT and RIGHT: "Simulation #<n>", "<file> · v<version>", badge Default/Manual, Bounded flow, Orifice model, Target flow, Open faucets (as F44) (`FE features/comparison/ComparisonRunSummary.tsx:119-176`).
- Counters "Matched <n>", "Left only <n>", "Right only <n>" (`FE features/comparison/UnifiedComparisonTable.tsx:294-358`).
- Row alignment (`FE features/comparison/comparisonRowAlignment.ts:47-133`):
  1. Identity columns = those both tables declare: Terminal Satisfaction `terminalId`. Source Flow `sourceId`. Pipe Velocity `edgeId, startNodeId, endNodeId`. Pressure `nodeId` (`FE features/comparison/comparisonDatasets.ts:194-392`).
  2. Without a shared identity column, all rows are left-only then right-only. Rows are never paired by position.
  3. Identity = values of the identity columns joined with `::`. Rows are grouped by identity (duplicates allowed).
  4. Match policy from compatibility (`FE features/comparison/comparisonMatchPolicy.ts:8-50`): Pressure uses matched terminals + transit nodes. Terminal Satisfaction uses terminals. Source Flow uses sources. Pipe Velocity uses pipes. An identity whose first identity value is "context changed", or is not in the matched list, becomes left-only and right-only rows. Without compatibility (loading or error) there is no policy and all same-identity rows match.
  5. Otherwise the k-th left and k-th right row of a group pair up. Extra rows are left-only or right-only.
  6. Order: identities in first-seen order, left table first.
- Table: one scroll area for both sides. Columns: **Match** (identity with its role color and a status pill Matched / Left only / Right only), shared context columns once (Pipe Velocity From, To. Pressure Type), then LEFT value columns and RIGHT value columns. A shared value that differs between sides shows "<left> ↔ <right>" with a mismatch style (`FE features/comparison/UnifiedComparisonTable.tsx:116-168, 339-527`).
- Value columns: ratio → %, m and m/s converted to the display unit, L/s as is. Labels with units "(%)", "(<u>)", "(<u>/s)", "(L/s)" (`FE features/comparison/comparisonFormatting.ts:8-57`, `FE features/comparison/comparisonUnits.ts:14-89`).
- Missing side: "—".
- Colors: Matched `#86efac` on `rgba(34,197,94,0.10)`. Left only `#fbbf24` on `rgba(245,158,11,0.10)`, row tint `rgba(245,158,11,0.025)`. Right only `#93c5fd` on `rgba(59,130,246,0.10)`, row tint `rgba(59,130,246,0.025)` (`FE styles/features/comparison.css:1553-1577`). DEFECT: the left-only pill style uses the class `left_only` but the element has `left-only`, so it is unstyled.
- Sorting (`FE features/comparison/UnifiedComparisonTable.tsx:182-337`): click cycles ASC → DESC → none. Match column: by status (Matched, Left only, Right only), then natural order of the identity. Other columns: by the raw value of that side (shared: left value, else right). Missing values last in both directions. Numbers numerically, text in natural order. If a source or the table changes, sorting resets.

### F51. Numeric comparison panel

If compatibility allows direct or partial comparison (always true once loaded), the panel shows (`FE features/comparison/SimulationComparisonDialog.tsx:689-711`).

- Title "Numeric comparison", "Match hydraulic entities and compare selected numerical metrics." Selectors **Match rows by**, **Left metric**, **Right metric** (`FE features/comparison/ComparisonMetricPanel.tsx:24-184`).
- Strategies (`FE features/comparison/comparisonEngine.ts:29-299`): each identity column common to both tables, then aliases (both directions) when both tables have the keys: borne↔terminalId, outlet↔terminalId, noeud↔terminalId, noeud↔transitNodeId, noeud↔sourceId (pred,succ)↔(startNodeId,endNodeId), (sourceId,targetId)↔(startNodeId,endNodeId). None: "These tables do not expose a compatible entity identifier. Side-by-side inspection remains available above."
- Metrics: numeric columns marked comparable. Recommended pair: first left metric with the same key on the right and compatible units. Else aliases pressure↔meanPressureM, required_pressure_m↔meanPressureM, cible↔moySt (`:792-971`).
- Units are compatible if they are equal, or if both have no unit and the same key. Else warning "The selected metrics use different physical units. Difference, ratio and percentage change may not represent a meaningful hydraulic comparison."
- Matching ignores the compatibility policy. Rows with an empty identity part are skipped. Identities seen more than once on a side are skipped and counted ("Duplicate IDs skipped: <n>"). A row with a non-numeric value counts as left unmatched (`:431-786`).
- For each matched identity: difference = right − left. Ratio = right/left (null when left = 0). Change % = (right − left)/left × 100 (null when left = 0).
- Summary "Matched: n", "Left unmatched: n", "Right unmatched: n". Note "Numeric calculations use canonical hydraulic units. Values are converted to the project display unit only after the comparison is calculated."
- Result table (`FE features/comparison/ComparisonResultTable.tsx:25-120`): ID, Left (<u>), Right (<u>), Difference Δ (<u>, only when compatible, with a + sign for positive), Ratio, Change %. Positive values `#6ee7b7`, negative `#fca5a5`. Empty: "No matching numeric rows were found."
- DEFECT: Change % is multiplied by 100 twice and gets two "%" signs (a +10 % change shows "1000.00%%") (`FE features/comparison/comparisonEngine.ts:712-721` and `FE features/comparison/ComparisonResultTable.tsx:61-66`). The port must show "+10.00%".

## 10. Source File and JSON views

### F52. Text view

- Tabs **Source File** and **JSON** (`FE features/domain/DomainTextViewer.tsx:21-202`).
- **Source File**: the stored text of the active revision. Description "Original network file stored in this project.". Metadata "<file name>", "<DSG|TPO|JSON> · v<version>". If the source file exists, this is the default tab. If not, it is disabled.
- **JSON**: the active snapshot `domain`, pretty-printed with 2-space indent. Description "Parsed domain representation used by the application.". Metadata "Parsed JSON", "Domain snapshot".
- States: "Reading source file...". Error "Could not read source file — The original source file could not be read.". No file "Source file unavailable — This domain snapshot does not have an accessible source artifact. The parsed JSON representation is still available."
- The text is read-only and cached per file id (revisions are immutable).

## 11. Help menu and unit settings

### F53. Set global units

- Help > **Set global units** opens "Unit Settings" with the project name, "Choose the unit used to display hydraulic lengths, pressure head, coordinates and velocities throughout this project.", radio Meter (m), Feet (ft), Inches (in), note "Internal hydraulic calculations remain in canonical SI units. This setting changes display and user input only.", **Cancel**, **Apply** ("Applying...", enabled when the choice changed) (`FE features/settings/UnitSettingsModal.tsx:15-131`).
- The unit is stored per project (`BE service/ProjectService.java:96-106`). The server accepts m/meter/metre, ft/feet/foot, in/inch/inches (`BE enums/ProjectDisplayLengthUnit.java:31-46`).
- All views read it at once: tables, editor input, inspector, canvas details, results, comparison (`FE App.tsx:444-445`).

## 12. Formatting and units

### F54. Adaptive number format

`formatAdaptiveNumber` (`FE features/formatting/numericDisplay.ts:6-63`), tested in `FE features/simulation/resultNumberFormatting.test.ts:5-34`:

| Value | Output | Example |
| --- | --- | --- |
| not finite | "—" | |
| 0 or −0 | "0.00" | |
| \|v\| ≥ 1e-2 | 2 decimals | 12.3467 → "12.35" |
| 1e-4 ≤ \|v\| < 1e-2 | 4 decimals | 0.0047 → "0.0047", −0.0047 → "-0.0047" |
| \|v\| < 1e-4 | scientific, 2 decimals, no "+" in exponent | 0.000031 → "3.10e-5" |

- Ratio as percent: `adaptive(ratio × 100) + "%"`. 0.95 → "95.00%", 1 → "100.00%", 1.23 → "123.00%".
- Integer: truncated, locale grouping (for example "1,234").
- Signed: "+" before positive values.
- Rule: a non-zero value never shows as zero. Stored values keep full precision. Rounding is for display only.
- Some places use fixed decimals instead (inspector, dialogs, F29 rules 3, 5, 7). The port keeps these as listed.

### F55. Units

- Canonical units: lengths, elevations, coordinates, diameters, pressure head in m. Velocity in m/s. Flow in L/s for all inputs and results (the solver uses m³/s inside and converts to L/s for statistics: `PY neatwork/simulation/statistics.py:248-278`). Probability as ratio 0–1. Satisfaction as ratio (can exceed 1).
- Display length unit: m, ft (× 3.280839895013123), in (× 39.37007874015748) (`FE features/domain/unitSystem.ts:15-31, 112-175`). Velocity uses "<u>/s" with the same factor. Flow never changes with the display unit.
- Length columns (normalized keys) (`FE features/domain/unitSystem.ts:45-96`): haut, elevation, elevation_m, xdata, ydata, x, y, lon, lon1, lon2, length, length_m, pipe_length, pipe_length_m, diam, diameter, diameter_m, diam1, diam2, orif_ideal, orif_com, ideal_orifice_diameter_m, commercial_orifice_diameter_m, pressure, pressure_m, required_pressure_m, pipelength, commercial_pipe_length_m, commercialpipelengthm, requiredpressurem (all m). Diameter_mm (mm).
- Orifices: if the project unit is m, they show in mm. Else they show in the project unit.
- Edited values are converted back to the stored unit before storage (F33).

### F56. Color palette (dark theme)

From `FE styles/tokens.css:5-29`: background `#07111f`, soft `#0b1628`, text `#e5f6ff`, muted `#94a3b8`, accent `#38bdf8`, primary `#0ea5e9`, success `#22c55e`, warning `#facc15`, danger `#ef4444`. Canvas background `#08182d`. Run status badges: pending/running/cancel-requested, completed, completed-with-errors, failed/cancelled have their own colors in `FE styles/features/simulation.css:789-840`.

## 13. Observed defects and gaps

| # | Feature | Observation | Recommendation for the port |
| --- | --- | --- | --- |
| D1 | F5 | Opening a project does not restore the domain, graph, runs or export state. | Restore everything from the project folder. |
| D2 | F35, F39, F41 | History, Details and Delete-run dialogs cannot be opened (History button commented out). | Ask the product owner (Q3). |
| D3 | F49 | Pipe rows have no id, so compatibility has no pipes; Pipe Velocity never matches. | Use edge id `A<n>` or (from, to). |
| D4 | F51 | Change % shown ×100 twice with "%%". | Show `change%` once. |
| D5 | F48 | Initial comparison picks the oldest runs. | Pick the two newest runs. |
| D6 | F25 | Opening a valve sets fake flow 0.35 L/s and velocity 0.20 m/s. | Clear results instead. |
| D7 | F41 | Parameter source CLIENT_DEMAND_TABLE shows "—". | Label "Client demand table". |
| D8 | F50 | Left-only pill unstyled (CSS class mismatch). | Style it amber. |
| D9 | F34 | Legacy client normalization in the editor can duplicate existing behaviors. | Normalize once, at parse time. |
| D10 | F45 | Demand group detail panel is empty code. | Ask the product owner (Q6). |
| D11 | F8, F25 | Node positions and valve states are lost at each revision save or import. | Keep them per project, matched by node id / edge id. |
| D12 | F16 | JSON import exists in the parser but is not reachable. | Out of scope unless asked. |

## 14. Mapping to the Rust Slint app

### 14.1 What the port drops

These NW5 parts exist only because of the client-server design. A single-user desktop app with local files does not need them:

- PostgreSQL, Spring Boot, FastAPI, HTTP, CORS, ports, health checks ("Backend: UP", "Python Core: UP").
- Job polling (700 ms), job recovery after a backend restart, pessimistic locks.
- The cross-project artifact library with "Unknown project" owners. The file system and the file chooser replace it.
- Browser storage for the comparison workspace. It goes into the project file.
- Downloads through URLs. Export writes a file with a save dialog.

### 14.2 Project folder

The current Slint app keeps projects in `projects/` and catalogs in `db/` (docs/gui.md "Start"). Proposal: one folder per NW5 project, next to the existing flat files.

```
projects/<Project name>/
  project.json            # name, createdAt, sourceFormat (NONE|DSG|TPO), displayUnit (m|ft|in),
                          # activeRevision, comparison workspace, last view, last run options
  inputs/
    <base>_v1.dsg         # revision files, immutable once written
    <base>_v2.dsg
    revisions.json        # per file: version, visible name, parent, family, createdAt, archived,
                          # diameters/orifices file used (TPO)
  db/                     # copies of diameters.db / orifices.db used by a TPO import
  graph.json              # per revision family: node visual positions and edge open/closed states,
                          # keyed by node id and edge id (fixes D11)
  runs/
    0001.json             # run record: runNumber, revision file, status, configuration mode,
                          # requested options, resolved options and sources, edge states,
                          # statistics payload (schema 1.2), failures, timestamps, error
  exports/                # simulation_statistics_v<N>.json / .xlsx, DSG copies
```

- Revisions as files: a revision is a plain DSG/TPO file in `inputs/`. The domain snapshot is not stored. When the app opens a revision, it parses the file (the parse is fast and deterministic). An optional cache can keep the parsed JSON.
- Revision numbering and names follow F8 exactly (default `<base>_v<N>.<ext>`, custom name rules). No `_rev<N>_<uuid>` internal names are needed: the visible name is the file name. If the name exists, refuse the save with a message (open question Q2).
- Archive (F11, F17): set `archived: true` in `revisions.json`. Do not delete the file.
- Delete project (F6): remove the folder after the confirmation.
- Runs: one JSON file per run. `runNumber` = highest number in `runs/` + 1. A run file holds everything that F41–F51 need, so review and comparison work after a restart.
- Compatibility (F49): computed in Rust from the two revision files that the runs reference. No storage needed.

### 14.3 Feature map

| NW5 feature | Slint mapping | Needs a server? |
| --- | --- | --- |
| F1 start screen | optional splash | no |
| F2 menu | File: New Project, Open Project, Import Network, (Close, Exit); Database: Diameters, Orifices; Help: Set global units (+ existing Language, About) | no |
| F4–F6 projects | folder create/open/delete; open = restore state (D1) | no |
| F7 source lock | `sourceFormat` in `project.json` | no |
| F8–F10 revisions | write the new file with the serializer (port of `revision_serializer.py`: replace known sections, keep unknown ones, re-parse to validate) | no |
| F11 archive | flag in `revisions.json` | no |
| F12 import dialog | Slint drop area (Slint 1.19 supports file drops on the window, UNVERIFIED for a drop area inside a dialog) plus the native file chooser | no |
| F13 file library | list of files in `inputs/` of the project, plus "Browse..." for any file on disk (copied into `inputs/`) | no |
| F14–F15 DSG/TPO import | `neatwork-core` parsers; TPO database dialog with files from `db/` | no |
| F16 JSON import | out of scope (D12) | no |
| F17 database manager | list/add/archive `.db` files in `db/`; the existing Hardware database dialog can view them | no |
| F19–F26 graph views | extend `drawing.rs`: Visual (3D projection F21), Raw 2D (existing Coordinates layout), Linear (existing Tree layout, add loop edges and upstream path); inspector pop-up; search | no |
| F25 valves, positions | stored in `graph.json`; applied to the next run | no |
| F27–F30 tables | Slint table views with the F27 labels, F29 rules and F28 colors | no |
| F31–F34 editor | editable table with selection, bulk edit, clipboard (Slint clipboard access, UNVERIFIED for tab-separated multi-cell text), validation, reference checks | no |
| F35–F38 simulation | background thread with a cancel flag and a progress callback; one run at a time is enough (Q4) | no |
| F39, F41 history and details | a run list in the Simulation tab (Q3) | no |
| F40 export | JSON (serde), XLSX (needs a crate such as `rust_xlsxwriter`, UNVERIFIED), DSG copy | no |
| F42–F47 review | Slint dialog with run and table selectors, sortable table model | no |
| F48–F51 comparison | Rust port of alignment, compatibility and metric engine | no |
| F52 text views | read-only text area: revision file text and `serde_json` pretty output | no |
| F53 units | `displayUnit` in `project.json`; convert at display and input time | no |
| F54–F56 formatting | one Rust formatting module with unit tests copied from `resultNumberFormatting.test.ts` | no |

No NW5 feature needs a server.

### 14.4 Engine data the features need

| Feature | Data | Available in `neatwork-core` today? |
| --- | --- | --- |
| F14–F15, F27 | DSG/TPO parse with the NW5 tables, including `!Clients`, `!Borne`, client behaviors | Partly. The NW2024 format (docs/file-formats.md) has no `!Clients`/`!Borne`. A `!Borne` line (`borne, pressure, N`) has 3 fields, the same count as a load-factor line, so the field-count reader cannot tell them apart (UNVERIFIED). Needs a section-aware reader for NW5 files. |
| F19 | graph: types from status, generated layout when x = y = 0, edge ids `A<n>`, VALVE for TPO length ≤ 0 | Layouts exist in the Slint drawing; the rules of F19/F23 must be matched. |
| F33–F34 | serializer that rewrites DSG/TPO sections and keeps unknown ones | No. |
| F36–F38 | NW5 simulation: scenario generation (batches, oversampling, draws), bounded flow, ideal/commercial orifices, per-terminal required pressure, client behaviors, edge overrides, seed, cancel | No. The Rust engine is the NW2024 Monte-Carlo simulation (flows per tap, quantiles, speed, head per node). |
| F45–F46 | per-scenario arrays: satisfaction per terminal, open-client counts per terminal, flow per pipe, pressure per node, success mask, failure records (stage, code, message) | No. The NW2024 output tables do not carry CVaR, source flow, std/CV, shortfall counts or per-scenario failures. |
| F49 | node ids, roles, pipe endpoints, clients and their effective demand from two revisions | Yes, from the parsed files. |

This is the main cost of the integration: either port the NW5 Python simulation (`PY neatwork/simulation/`, about 5 000 lines) to Rust, or compute the NW5 statistics from the existing engine with extra per-run output. See Q1.

### 14.5 Suggested order

1. Project folder, revisions as files, source lock, units, formatting module (F4–F11, F53–F55).
2. NW5 DSG/TPO reader and writer with clients and behaviors (F14–F16, F33–F34).
3. Tables, editor, text views (F27–F34, F52).
4. Graph views and inspector (F19–F26).
5. Simulation engine decision (Q1), then runs, review and export (F35–F47).
6. Comparison (F48–F51).

## 15. Open questions for the product owner

1. Simulation engine: do you want the Slint app to run the NW5 stochastic model (bounded flow, scenario generation, CVaR satisfaction) through a Rust port of the Python core, or keep the NW2024 engine and show NW5-style tables from it? The two models give different numbers.
2. Revision file names: can a custom name overwrite an existing file? NW5 avoids the clash with a hidden internal name. With plain files the app must refuse or ask.
3. Simulation history, execution details and run deletion exist in NW5 but are hidden (D2). Do you want the port to show them?
4. Concurrent runs: NW5 allows several runs at once with a foreground run. Is one run at a time acceptable?
5. Do you want the user to choose an older revision as the active input, instead of re-importing it?
6. Do you want the per-terminal demand group panel (D10) under "Show details"?
7. Do you want node positions and valve states to survive revision saves (D11)?
8. Are NW5 DSG files (with `!Clients`/`!Borne`) also expected to open in the NW2024-style Design tabs, or only in the NW5 workspace?
9. Is the "Default input" comparison source (domain tables against results) wanted? It is coded but hidden.
10. Which export formats are required: JSON, XLSX, DSG, INP? INP has no producer in NW5.
