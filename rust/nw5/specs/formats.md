# NeatWork 5 file formats and internal tables

Status: specification for phase 3 of [../PLAN.md](../PLAN.md) (data model and formats). Written 2026-11-20 from the NW5 sources at commit `a129c37a9ab04eb062ba87c184f7c75ff03eaa7c` (`reference/neatwork5/SOURCE_COMMIT`).

This document tells a Rust developer how to read and write every NeatWork 5 (NW5) file. It also tells how NW5 builds its internal tables. NW4 formats (Java, `.tpo`/`.dsg`/`.sim`) are in [../../../docs/file-formats.md](../../../docs/file-formats.md). This document only repeats NW4 facts where NW5 differs or depends on them.

## 0. Conventions

### 0.1 Citation prefixes

| Prefix | Path |
| --- | --- |
| `M:` | `reference/neatwork5/matlab/` |
| `PY:` | `reference/neatwork5/python/` |
| `APP:` | `reference/neatwork5/application/python-core/app/` |
| `APPT:` | `reference/neatwork5/application/python-core/tests/` |
| `BE:` | `reference/neatwork5/application/backend/src/main/java/com/neatwork/backend/` |
| `FE:` | `reference/neatwork5/application/frontend/src/` |
| `EX:` | `reference/neatwork5/examples/` |
| `T:` | `reference/neatwork5/tests/matlab/` |
| `DOC:` | `reference/neatwork5/docs/design/` |

A citation is `prefix:file:line`. Line numbers are those of the file at the commit above.

### 0.2 Markers

- **UNVERIFIED**: read from the code but not executed (MATLAB is not available), or inferred. A Rust test must check it before the port relies on it.
- **DEFECT**: the source code does something that is very likely not intended. The port must decide whether to copy it (see sections 12.3 and 15).
- **PROPOSAL**: no NW5 code exists. The text is a design for the port, not a description of NW5.

Facts marked "verified" were verified on 2026-11-20 by running the python-core parsers on every example file (scratch script, python 3, pydantic 2.14).

### 0.3 The three NW5 code bases

NW5 has three independent implementations that read the same files. They do not agree on every detail. Each section says which one it describes.

| Code base | Role | Entry points |
| --- | --- | --- |
| MATLAB (`M:`) | Reference design engine (`makeReinfDsg`) and simulation (`runSimul`). Writes `.dsg` and NW5 JSON. | `M:InOut/parseTpoToTables.m`, `M:InOut/readDesign.m`, `M:InOut/data4DsgFile.m` |
| Python research code (`PY:`) | Early ports of the design reader and simulation. | `PY:design/parserTPO.py`, `PY:simulation/data.py` |
| Application python-core (`APP:`) | FastAPI service behind the Java backend and the Electron frontend. Imports `.tpo`/`.dsg`, writes revisions, simulates, reads EPANET `.inp`. | `APP:services/import_service.py`, `APP:neatwork/serialization/revision_serializer.py` |

The Java backend (`BE:`) stores the parsed domain as a JSON snapshot and compares snapshots.

## 1. Common lexical rules of NeatWork text files (`.tpo`, `.dsg`, `.db`)

### 1.1 Lines and fields

- Encoding: UTF-8 in practice. Comment sections contain accented French text. One file contains invalid UTF-8 replaced by `?` (`EX:dsg_examples/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg:2`, "modle"). The reader must accept invalid bytes in `!Commentaires` (decode lossily or keep raw bytes).
- Line end: LF in all examples. No example has CRLF or a BOM (verified). The app strips a UTF-8 BOM (`APP:neatwork/parsers/parsing_utils.py:5`). The Rust reader must strip a BOM and accept CRLF.
- The last line can end without a newline (`Toy_bbd.tpo`, `Esperenza2S.dsg`, `CruscadeFull.tpo` have none).
- Fields are separated by `,`. Surrounding spaces are trimmed by the app (`APP:neatwork/parsers/parsing_utils.py:41`) and by the Python research code. MATLAB `textscan` with `'Delimiter', ','` keeps leading spaces inside `%s` fields UNVERIFIED.
- There is no CSV quoting. A double quote is an ordinary character. The nominal pipe size uses `"` for inches, and some files contain spreadsheet artefacts like `"1/2"""` (`EX:tpo_examples/Toy/Toy_bbd.tpo:30`). All three readers keep the field text unchanged, quotes included (verified for the app: `Nominal` = `"1/2"""`). The Rust reader must keep the raw text and must never split on a comma inside quotes (no example has one).
- Blank lines: the app drops them (`APP:neatwork/parsers/parsing_utils.py:7`). The Python research readers keep them and then fail on a short row. MATLAB `textscan` treats an empty line as no data UNVERIFIED. Rust: ignore blank lines.
- `#` comments: only the app drops lines that start with `#` (`APP:neatwork/parsers/parsing_utils.py:7`). No example uses `#`. Rust: accept and drop them on read. Never write them.

### 1.2 Section headers

A line whose trimmed text starts with `!` starts a section. The section ends at the next header or at the end of the file.

The implementations match headers differently:

| Code | Rule | Source |
| --- | --- | --- |
| MATLAB | First line whose trimmed text **starts with** `!` + name (case-sensitive prefix). `!Client` also matches `!Clients`, `!Borne` also matches `!Bornes`, `!Nodes` matches `!Nodes,`. The block ends at the next line that starts with `!`. | `M:InOut/getSectionBlock.m:4`, `:12` |
| MATLAB TPO | `!Nodes`, `!Pipes`, `!Default properties` must all exist **and** appear in this order, else error. Uses `startsWith` on untrimmed lines. | `M:InOut/parseTpoToTables.m:41` to `:49` |
| App | Name after `!` is trimmed, BOM-stripped, lower-cased, `_` and `-` become spaces, then compared **for equality**. `!Nodes,` does not match `Nodes` (verified: `Cruscade.tpo` gives 0 nodes). A second header with the same name restarts the block. | `APP:neatwork/parsers/parsing_utils.py:10` to `:39` |
| Python research and `SimulationData` file loader | `line.startswith("!" + name)`, case-sensitive, on stripped lines. A second identical header restarts the block. A section that reaches EOF is kept. A header on the last line gives "missing". | `PY:design/utilsDesign.py:3`, `APP:neatwork/simulation/numerical_utils.py:12` |

Observed header variants in the examples:

- Trailing commas from a spreadsheet: `!Nodes,`, `!Nodes,,`, `!Pipes,`, `!Default properties,` (`EX:tpo_examples/Cruscade/Cruscade.tpo:1`, `CruscadeFull.tpo:1`).
- The same header twice on consecutive lines: `!Default properties` / `!Default properties` and `!loadfactor` / `!loadfactor` in `EX:tpo_examples/Red 2/Red 2_Secu1.00_fusion0_split0.dsg`, `!loadfactor` twice in `EX:dsg_examples/Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg:817`, `!Clients` twice in `Cruscade.tpo`, `CruscadeFull.tpo` and `Red 3/Red 3b.tpo`. With MATLAB `getSectionBlock`, the first header's block is then **empty** (the next header follows at once) DEFECT. The app and the Python code take the second block.
- `!Borne` (written by MATLAB and the app) versus `!Bornes` (named in `EX:tpo_examples/Toy/README.md` and searched by `APP:neatwork/simulation/data.py:171`). No example contains `!Bornes`.
- `!Commentaires` (free text, NW5 only).

**Rust reading rule (normative for the port):** a header is a trimmed line that starts with `!`. Its name is the text after `!`, with trailing commas and spaces removed, then compared case-insensitively after mapping `_` and `-` to spaces. Aliases: `Bornes` = `Borne`, `Client` = `Clients`, `Default Properties` = `Default properties`, `loadFactor` = `loadfactor`. Consecutive identical headers (only blank lines between them) count as one. If a section name appears twice with data, the first block wins and a warning is issued (no example has this). Unknown sections are kept verbatim for round-trip (section 12). Header order is free (MATLAB TPO order check is not reproduced, all examples respect `Nodes < Pipes < Default properties` anyway).

### 1.3 Numbers

- Readers accept any float syntax: `200`, `200.0`, `1.0E9`, `1.00E+09`, `1.87E-08`, `1e-06`, `0.0050`. MATLAB `%f` and Python `float()` also accept `Inf`, `inf`, `NaN`. Rust: parse with `f64::from_str` after trimming. Accept `Inf`/`inf`/`-Inf`/`NaN` case-insensitively.
- The app converts every field that parses as a float to a number. An integral value becomes an integer (`APP:neatwork/parsers/parsing_utils.py:44`). This includes **identifiers**: a node named `150` becomes the JSON number `150`. A name like `007` or `1.50` becomes `7` or `1.5` DEFECT. The Rust model must store identifiers as strings and never parse them.
- MATLAB writes numbers with `num2str(x, '%.15g')` (`M:InOut/export_neatwork_csv.m:81`): `200`, `0.0182`, `1.87e-08`, `1e-06`, `5.5737646484375`, `Inf`, `NaN`. The app writes Python `str()` of the parsed value, with integral floats printed as integers (`APP:neatwork/serialization/revision_serializer.py:19`): `200`, `1.87e-08`, `inf`.
- NW4 (Java) writes `Double.toString` (`200.0`, `1.0E9`). NW5 readers accept both.

### 1.4 Placeholder `N`

NW4 pads some rows with the literal `N` ("not used"). NW5 keeps the padding in `.dsg` files so that NW4 can read them. The padded fields are: field 9 of `!Diameters`, fields 2-6 of `!Orifices`, field 3 of `!Borne`. Field 8 of `!Pipes` was padding in NW4 and is now a status (section 4.3). Readers ignore `N` padding. `BE:service/DomainCatalogIntegrityService.java:79` treats `N`, `-` and blank as "no reference".

## 2. Internal tables

NW5 MATLAB stores a network in MATLAB tables (one row per entity) plus parameter structures. The app uses lists of JSON objects with the same column names ("legacy names") or camelCase aliases (section 7.3). The Rust data model must mirror the persistent columns and compute derived columns on demand.

Column types: `str` (identifier or text), `f64`, `i32` (small integer code), `bool`. Units: m (metre), m of water (head or pressure), L/s (litre per second), m3/s.

Node status code `statut`: `0` source (reservoir), `1` intermediate (transit, junction), `2` terminal (borne, tap point) (`APP:schemas/simulation_domain.py:55`, `APP:neatwork/parsers/parsing_utils.py:83`).

### 2.1 TN: nodes

| Column | Type | Unit | Persistent | Meaning | Created by |
| --- | --- | --- | --- | --- | --- |
| `noeud` | str | | yes | Node identifier. Unique (`M:InOut/validate_and_align_Tables.m:28`). MATLAB stores it as `categorical`. | `.tpo`/`.dsg` `!Nodes` field 1 |
| `haut` | f64 | m | yes | Physical elevation ("hauteur"). Can be negative and need not be 0 at the source (`Red 1b.tpo`: source at -1). Design and export keep it unchanged (`T:test_physical_elevations.m`). | field 2 |
| `xdata` | f64 | free (often m, UTM) | yes | X coordinate, for drawing only. | field 3 |
| `ydata` | f64 | free | yes | Y coordinate. | field 4 |
| `nbrob` | f64 (integer value) | count | `.tpo` only | Number of taps (robinets) of a terminal node. After import it is **recomputed from TC** (number of clients per node, 0 for non-terminals) and the file value is discarded (`M:InOut/apply_client_data.m:15`). | `.tpo` field 5 |
| `orif_ideal` | f64 | m | `.dsg` only | Ideal (continuous) orifice inner diameter at a terminal. `1` = no orifice in NW5 MATLAB output. `10000` = no orifice in NW4 output. `0` = unset (TPO-derived or EPANET). | `.dsg` field 5 |
| `orif_com` | f64 | m | `.dsg` only | Commercial orifice diameter chosen from TO (or the sentinels above). | `.dsg` field 6 |
| `statut` | i32 | code | yes | 0/1/2, see above. Last field of the row in both formats. | `.tpo` field 6, `.dsg` field 7 |
| `degree` | f64 | count | no | Number of incident arcs. | `M:CreateDsgDev/enrichTablesReinf.m` (`TN.degree = deg`), `M:CreateDsgDev/processSol.m:366` |
| `degre` | f64 | count | no | Same, simulation side. | `M:Simul/incidence_intermediaire.m:15` |
| `demande` | f64 | m3/s | Txt only | Demand column of the Cruscade Txt node file. Dropped on read. | `M:InOut/readDesign.m:117` |

Row order: MATLAB and the Python readers sort TN by `(statut, noeud)` with plain lexicographic string order (`M:InOut/parseTpoToTables.m:59`, `M:InOut/data4DsgFile.m:81`). The app keeps file order. Writers that copy MATLAB must sort. `Source,...,0` therefore comes first, terminals last.

### 2.2 TA: arcs (links, pipes)

Persistent columns (in `.dsg` `!Pipes` order):

| Column | Type | Unit | Meaning |
| --- | --- | --- | --- |
| `pred` | str | | Upstream node. After design, arcs are oriented from the source (`M:InOut/peel_orient_tree.m`). |
| `succ` | str | | Downstream node. |
| `lon` | f64 | m | Total arc length. TPO readers raise lengths below 0.01 m to 0.01 m (`M:InOut/parseTpoToTables.m:73`, `PY:design/parserTPO.py:78`). The app keeps the raw length in the table and uses `max(0.01, L)` only for the drawing (`APP:neatwork/parsers/tpo_parser.py:177`). |
| `lon1` | f64 | m | Length of segment 1 (larger diameter after design, `M:CreateDsgDev/processSol.m:202`). |
| `ID1` | str | | Diameter ID (TD.ID) of segment 1. |
| `lon2` | f64 | m | Length of segment 2. An arc with one diameter has `0`. |
| `ID2` | str | | Diameter ID of segment 2. For `lon2 = 0`: `fictif` in current MATLAB output, `0` in older NW5 output and NW4 output. |
| `N` | str | | Arc status: `N` (new/normal), `B` (blocked: existing pipe that must not be reinforced), `R` (reinforcement pipe added by the design). (`M:CreateDsgDev/prepareReinforcementTables.m:129`, `M:CreateDsgDev/processSol.m:76`). NW4 always writes `N`. |

Invariants (`M:CreateDsgDev/prepareReinforcementTables.m:92` to `:141`, `APP:neatwork/simulation/validation.py:305`):

- `lon1 >= 0`, `lon2 >= 0`, finite.
- A segment with length > 0 has a real ID (present in TD, not `fictif`). A segment with length 0 has ID `fictif` (MATLAB) or an empty/`0`/`None` ID (app: `None`, `""`, `0`, `"0"` all mean "no diameter", `APP:schemas/simulation_domain.py:191`).
- For an equipped arc, `lon1 + lon2 = lon` (app tolerance `rtol 1e-6, atol 1e-9`, `APP:neatwork/simulation/validation.py:469`, MATLAB `1e-9*max(1,lon)` in `M:CreateDsgDev/processSol.m:489`, `1e-12*max(1,|lon|)` in `M:Simul/completeTables.m:65`).
- An arc with `N = B` must be equipped.
- **DEFECT**: `M:CreateDsgDev/prepareReinforcementTables.m:135` asserts `abs(lon1+lon2-lon) > 1e-7` (inverted test) for equipped arcs. Any `.dsg` with equipped arcs therefore fails in `makeReinfDsg` UNVERIFIED. A TPO input has no equipped arcs, so the assertion is vacuous there.

Derived columns (not written to `.dsg`, listed so the port can name them consistently):

| Column | Unit | Meaning | Source |
| --- | --- | --- | --- |
| `equip` | bool | `lon1 > 0 or lon2 > 0` | `M:CreateDsgDev/prepareReinforcementTables.m:143` |
| `pmax` | m | Maximum hydrostatic pressure: `Hsource - min(haut(pred), haut(succ))`, `Hsource = haut(source) + pump_pressure` | `M:CreateDsgDev/enrichTablesReinf.m:36` |
| `haut` | m | `haut(succ) - Hsource` (negative below the source head) | `M:CreateDsgDev/enrichTablesReinf.m:49` |
| `statut` | code | Status of `succ` (design). Simulation: 0 if `pred` is a source, 2 if `succ` is a terminal, else 1 | `enrichTablesReinf.m`, `M:Simul/completeTables.m:105` |
| `nbrob` | count | Taps at `succ` | `enrichTablesReinf.m` |
| `pressure` | m | Required pressure at `succ` (from TB). It is 0 for an arc that does not end at a borne | `enrichTablesReinf.m` |
| `diam1`, `diam2` | m | Inner diameters of the segments (0 for `fictif`). Cruscade Txt gives them instead of IDs. | `M:CreateDsgDev/processSol.m:41`, `M:Simul/completeTables.m:44` |
| `nature` | str | `equip` (existing pipe) or `reinf` (new pipe) | `M:CreateDsgDev/processSol.m:79` |
| `nbdep` | count | Number of downstream taps (`M_path * nbrob`) | `M:CreateDsgDev/build_incidence_loads.m:215` |
| `loadFactor` | ratio | Load factor of the NW4 heuristic | `M:CreateDsgDev/build_incidence_loads.m:11` |
| `Qmax` | m3/s | Design flow used in the LP | `M:CreateDsgDev/build_reinf_model.m:231` |
| `cost1`, `cost2`, `totcost` | currency | Segment costs (`lon_i * TD.cost`) | `M:Simul/completeTables.m:44` to `:99` |
| `gravit` | m | Simulation: `-haut(source)` on source arcs, `haut(borne)` on terminal arcs, else 0 | `M:Simul/completeTables.m:12` |
| `beta` | | `bescal * (lon1/diam1^nu + lon2/diam2^nu)` (head loss = `beta * Q^lambda`, Q in m3/s) | `M:Simul/completeTables.m:71` |
| `orif_ideal`, `orif_com` | m | Orifice of `succ` copied to the arc | `M:Simul/completeTables.m:141` |
| `alpha_ideal`, `alpha_com`, `alpha` | | Terminal loss coefficient: `(orifcoef/orif)^4 + 1/faucetcoef` (loss in m = `alpha * q^2`, q in m3/s). MATLAB uses `alpha = alpha_ideal`. The app uses `alpha_com` for `orif_com = 1` | `M:Simul/completeTables.m:136` to `:155` |
| `ecart` | m | Optional: if present, `alpha = ecart / (targetflow/1000)^2` | `M:Simul/completeTables.m:152` |
| `external_edge_id` | str | App only: stable edge ID, `A1`, `A2`, ... in file order | `APP:neatwork/simulation/domain_adapter.py:168` |
| `is_open` | bool | App only: hydraulic open/closed flag (`enabled`, `operational_status`) | `APP:schemas/simulation_domain.py:166` |

Row order: TPO readers sort by `(succ, pred)` (`M:InOut/parseTpoToTables.m`, `PY:design/parserTPO.py`). Design sorts by `(statut, succ)`. The `.dsg` writer writes TA in post-processing order: existing equipped arcs first, then reinforcement arcs (`M:CreateDsgDev/processSol.m:345`).

Parallel arcs: after a reinforcement, one link can appear twice (existing pipe + `R` pipe, same `pred`/`succ`). This is how NW5 represents parallel pipes. The design itself refuses parallel arcs, cycles, several sources and a source degree other than 1 (`M:CreateDsgDev/makeReinfDsg.m:104`).

### 2.3 TC: clients (users, demand points)

| Column | Type | Unit | Meaning |
| --- | --- | --- | --- |
| `client` | str | | Client identifier, unique. Generated names: `C<k><borne>`, k = 1..nbrob (MATLAB, `M:InOut/parseTpoToTables.m:172`), for example `C4P2`. |
| `borne` | str | | Terminal node (statut 2) that serves the client. Several clients can share a borne. |
| `proba` | f64 | ratio 0-1 | Opening probability (`opentaps` for uniform demand). |
| `cible` | f64 | L/s | Target flow (`targetflow` for uniform demand). Must be > 0 (`APP:neatwork/domain/client_demand.py:19`). |
| `ecart` | f64 | m | Optional 5th column read by the unused `M:InOut/read_or_build_TC.m:17`. |
| `pressure` | f64 | m | Optional column built by `read_or_build_TC.m:44` (unused). |

Row order: MATLAB sorts by `(borne, client)` lexicographically (`M:InOut/parseTpoToTables.m:179`), so `C10P2` comes before `C1P2` (see `EX:dsg_examples/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg:66`).

TC is authoritative ("clients override counts", `T:test_client_data_precedence.m`):

1. `TN.nbrob` = number of TC rows per node (`M:InOut/apply_client_data.m:15`). Every `TC.borne` must be a node, else error `NW5:ClientNode` "Client terminals are absent from TN: ..." (`:9`).
2. `uniform_dem = 1` iff all `proba` are equal and all `cible` are equal (`:19`). Then `opentaps := proba(1)` and `targetflow := cible(1)`. A stored `uniform_dem` is ignored.
3. If `uniform_dem = 0` and some borne has more than one client, `bounded_flow := 1`, with warning `NW5:BoundedFlowRequired` if it was 0 (`:26`).
4. After the parameter review of the design, a uniform demand rewrites every `proba`/`cible` with the reviewed `opentaps`/`targetflow` (`M:CreateDsgDev/makeReinfDsg.m:119`).

The app applies the same rules in `resolve_client_demand_profile` and `synchronize_client_demand_params` (`APP:neatwork/domain/client_demand.py:19`, `:55`). For any non-uniform demand, `build_simulation_data` forces `bounded_flow = 1`, even with one client per borne (`APP:neatwork/simulation/domain_adapter.py:289`). The MATLAB Txt reader does the same (`M:InOut/LireTablesDsgTxt.m:145`).

### 2.4 TB: bornes (terminals with a required pressure)

| Column | Type | Unit | Meaning |
| --- | --- | --- | --- |
| `borne` | str | | Terminal node ID. The set of bornes must equal the set of statut-2 nodes ("Les bornes ne correspondent pas aux noeuds terminaux", `M:InOut/parseTpoToTables.m:199`, `M:InOut/parseDsgToTables.m:113`). |
| `pressure` | f64 | m of water | Required (contractual) pressure at the borne. Only bounded flow (`bounded_flow = 1`) uses it. ≥ 0. |
| `N` | str | | Placeholder `N` (written for NW4). Built TB has no `N` column. The writer adds it (`M:InOut/data4DsgFile.m:115`). For a row without `N`, the app writes `N`. For a TB row that it generated from a TPO, it writes an empty string (`APP:neatwork/parsers/tpo_parser.py:306`). |

If `uniform_dem = 1`, the design overwrites every TB pressure with `pardsg.pressure` (`M:CreateDsgDev/enrichTablesReinf.m:55`). If `uniform_dem = 1` and `bounded_flow = 1`, the MATLAB simulation does the same (`M:Simul/runSimul.m:36`). Per-borne pressures therefore matter only for non-uniform demand. Example with different pressures: `EX:dsg_examples/Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg` (P4 at 3 m).

### 2.5 TD: pipe catalogue (diameters)

| Column | Type | Unit | Meaning |
| --- | --- | --- | --- |
| `ID` | str | | Catalogue ID. Generated IDs are `D001`, `D002`, ... (3 digits, 1-based). |
| `Nominal` | str | | Nominal size, raw text (`1/2"`, `"1/2"""`, `75`). The app TPO reader calls it `size`. |
| `SDR` | f64 or str | | Standard dimension ratio. **Can be text**: `PEHD`, `PVC-BO`, `PVC BO`, `Fonte` in `EX:tpo_examples/Cruscade/DiamBaseCruscade.db`. Store as text, parse on demand. |
| `diam` | f64 | m | Inner diameter. > 0 for real pipes. |
| `cost` | f64 | currency per m | Unit cost. ≥ 0. |
| `pressure` / `Pmax` | f64 | m of water | Maximum allowed pressure (pressure class). Compared with `TA.pmax` in `M:CreateDsgDev/validate_existing_pressure.m`. Name `pressure` in TPO/db tables, `Pmax` in `.dsg` tables, renamed to `pressure` before design (`M:CreateDsgDev/makeReinfDsg.m:92`). Scale UNVERIFIED: NW4-derived catalogues hold values 10 times larger than NW5 catalogues. Examples: `4260`, `2000`, `1700` in `DiamBaseCeiba.db` and `Toy2b_NW4.dsg`, against `426`, `200`, `170`. |
| `type` / `Type` | i32 or str | code | Material code: `1` PVC, `2` iron (NW4). `EPANET` for catalogue rows built by the EPANET import. |
| `Roughness` / `rugo` | f64 | mm (NW4) | Roughness. Not used by the NW5 head-loss law. `Esperenza2S.dsg` uses `2`. |
| `resist` | f64 | | Derived: linear resistance `bescal * diam^-nu` (`M:CreateDsgDev/prepareReinforcementTables.m:46`). |
| `cond` | f64 | | Derived: `resist^(-1/lambda)`. |

The fictitious pipe: before design, MATLAB removes any row with `ID = fictif` and appends a new one (`M:CreateDsgDev/prepareReinforcementTables.m:54` to `:65`). The new row has `ID = Nominal = SDR = "fictif"`, `diam = 0`, `cost = 0`, `pressure = 1000`, `resist = Inf`, `cond = 0`, `Roughness = 0`. Its `type` is a copy of row 1. MATLAB then sorts TD by `diam`, so `fictif` is first. **The writer keeps this row**: `data4DsgFile` writes `TD(:,1:8)` (`M:InOut/data4DsgFile.m:104`). Thus a current MATLAB `.dsg` and JSON contain a `fictif,fictif,fictif,0,0,1000,<type>,0,N` row UNVERIFIED. No example file has it, because the examples are older than this code. The app rejects that row on simulation (`INVALID_DIAMETER_VALUE`, `diam` must be > 0). The Rust reader must drop a `fictif` catalogue row and the writer must not write it (section 12.3).

Row order and IDs for a catalogue from a `.db` file: MATLAB sorts by `diam` then numbers `D001...` (`M:InOut/parseTpoToTables.m:110` to `:118`). `PY:design/parserTPO.py:176` does the same. The app numbers in **file order** without sorting (`APP:neatwork/parsers/tpo_parser.py:105`). All example `.db` files are already sorted by `diam` with no duplicate (verified), so the IDs agree on the examples. Rust: sort stably by `diam`, then number.

### 2.6 TO: orifice catalogue

| Column | Type | Unit | Meaning |
| --- | --- | --- | --- |
| `in_diam` | f64 | m | Commercial orifice inner diameter. |

The app stores each row as `{source_index (1-based), diameter_m, diameter_mm}` (`APP:neatwork/parsers/db_parser.py:13`, `APP:neatwork/parsers/dsg_parser.py:210`). A row from a TPO `!Orifices` section is `{in_diam, N}` (`APP:neatwork/parsers/tpo_parser.py:109`).

NW4 adds the bounds `1.0E-6` and `1.0` to the list it writes into `.dsg` (see the NW4 doc). Some NW5 files keep them (`Ceiba/orifices.db`, `CeibaNW4*.dsg`, `Toy2b_NW4.dsg`). NW5 design appends `1` (= no orifice) to the candidate list itself (`M:CreateDsgDev/processSol.m:322`). Rust: on read, keep the list as is. For design candidates, remove values ≤ 1e-6 and ≥ 1 and then append `1`.

### 2.7 TP, param, pardsg, partec: parameters

MATLAB reads the `!Default properties` rows `name,value` into TP (`property`, `value`). It copies TP into the structure `param`, one field per name (`M:InOut/parseTpoToTables.m:85`). Then it splits `param` into `pardsg` (design) and `partec` (technical), with defaults (`M:CreateDsgDev/extract_parameters.m`). A name that is not a valid MATLAB field name (for example `bounded-flow`) makes `cell2struct` fail UNVERIFIED.

Defaults differ by code base. "none" means the code base has no default (the key is absent).

| Name | Meaning | Unit | MATLAB `extract_parameters` | App TPO | App DSG | App simulation | `PY:design` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `bescal` | Friction scale: head loss per m = `bescal * Q^lambda / D^nu` (Q m3/s, D m) | | 8.9396e-4 | 8.9396e-4 | 8.9396e-4 | 8.9396e-4 | 8.9396e-4 |
| `lambda` | Flow exponent of the head-loss law | | 1.781 | 1.781 | **0** DEFECT | 1.781 | 1.781 |
| `nu` | Diameter exponent | | 4.781 | 4.781 | 4.781 | 4.781 | 4.781 |
| `orifcoef` | Orifice discharge coefficient | | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 |
| `faucetcoef` | Faucet loss coefficient (loss = q²/faucetcoef, q m3/s) | | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 |
| `targetflow` | Uniform target flow per client | L/s | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `opentaps` | Uniform opening probability | ratio | 1 | 1 | 1 | 1 | 1 |
| `pressure` | Required pressure at bornes (bounded flow) | m | 2 (for a missing key, `parseDsgToTables` adds 2, `M:InOut/parseDsgToTables.m:90`) | 2 | 2 | 2 | 2 |
| `pump_pressure` | Head gain at the source | m | 0 | none | none | none | none |
| `secu` | Safety factor on available head of terminal arcs, ≥ 1 | | 1.0 | 1.0 | 1.0 | 1.0 | 1.0 |
| `vmax` | Maximum velocity | m/s | 1.5 | 1.5 | 1.5 | 1.5 | 1.5 |
| `vmax_secu` | Quantile for the velocity load | ratio | 0.95 | 0.95 | 0.95 | 0.95 | 0.95 |
| `servicequal` | Quantile for the design load | ratio | 0.65 | 0.65 | `serviceequal` 0.65 DEFECT (alias fixed in `APP:neatwork/simulation/constants.py:65`) | 0.65 | 0.65 |
| `fusion` | Merge degree-2 nodes (design) | 0/1 | 0 | 0 | 0 | 0 | 0 |
| `split` | Create a node at a diameter change (design) | 0/1 | 0 | 0 | 0 | 0 | 0 |
| `bounded_flow` | 1: flow capped at demand above `pressure` | 0/1 | 0 | 0 | `bounded-flow` 0 DEFECT | 0 | 0 |
| `uniform_dem` | 1: all clients identical (derived from TC) | 0/1 | 1 | 1 | 1 | 1 | 1 |
| `pipelength` | Commercial pipe length for rounding | m | 6.0 | 6.0 | 6.0 | 6.0 | 6.0 |
| `seuil` | NW4 quick-check threshold (unused) | | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| `limitbudget` | NW4 budget limit (unused) | | Inf | inf | inf | inf | inf |
| `watertemp` | Water temperature (unused) | °C | 20.0 | 20.0 | 20.0 | 20.0 | 20.0 |
| `use_efficient_pipes` | Design option | 0/1 | 0 | 0 | 0 | 0 | 0 |
| `nScenLoad` | Monte Carlo samples for load factors | count | 200000 | none | none | none | none |
| `use_cvar` | Empirical/binomial CVaR loads | 0/1 | 0 | none | none | none | none |
| `nbsim` | Number of simulated scenarios | count | 100 (set by `M:Simul/review_simul_parmeters.m:12`) | none | 100 | 100 | 100 |
| `orif_com` | Simulation uses commercial (1) or ideal (0) orifices | 0/1 | 0 (review) | none | 1 | 1 | 1 |

Citations: `M:CreateDsgDev/extract_parameters.m:7` to `:43`, `APP:neatwork/parsers/tpo_parser.py:26` to `:49`, `APP:neatwork/parsers/dsg_parser.py:23` to `:47`, `APP:neatwork/simulation/constants.py:22` to `:46`, `PY:design/parserTPO.py:83`.

Typical NW5 values in examples: `faucetcoef 1.87E-08`, `orifcoef 0.59`, `opentaps 0.28` to `0.6`, `targetflow 0.12` to `2`. `DOC:types de clients.md` gives the meaning of each parameter (French).

Precedence:

- MATLAB: file value, else default. `pardsg` holds design values, `partec` holds `orifcoef, faucetcoef, lambda, nu, bescal`. Both are sorted alphabetically by `orderfields` (ASCII). The `.dsg` writer writes `pardsg` fields then `partec` fields (`M:InOut/data4DsgFile.m:95`), which gives the order seen in `Toy_bbd_Secu1.00_fusion0_split0.dsg:24` to `:46`.
- App: `params` = defaults overlaid with file values. For TPO, the app keeps **only the keys of the default tables**. It drops all other file parameters, for example `nScenLoad`, `nbsim`, `pump_pressure` DEFECT. For DSG it keeps every file key plus the defaults, including the misspelled default keys.
- App simulation: `defaults < technicalParams < designParams < params`. Each key is converted to snake_case, then aliased (`APP:neatwork/simulation/domain_adapter.py:77`, `APP:neatwork/simulation/constants.py:62`). Booleans become 0/1. `bounded_flow, fusion, nbsim, orif_com, split, uniform_dem, use_efficient_pipes` become integers, all others floats.
- Seeds (`seed`, `random_seed`, any spelling) are execution metadata and are never written to a revision (`APP:neatwork/serialization/revision_serializer.py:47`).

### 2.8 dat: run metadata (MATLAB only, not persisted)

`dat` is a MATLAB structure. It is not written to `.dsg`, and `save_dsg_json` does not write it (the line is commented out, `M:InOut/save_dsg_json.m:27`). File fields: `nom` (network name), `version` (suffix), `nomv` (`nom` + `version`), `secu`, `fusion`, `split`. Type fields: `type` (`tpo`/`dsg`/`json`/`mat`/`txt`), `output_type` (`dsg`/`json`). Path fields: `adresse2fiche` (file path), `adresse2dossier`, `nom2fiche` (file name without extension). Physical field: `source_height`. Graph matrices: `M_incid` (|TN|-1 × |TA| reduced incidence), `M_path` (|TA| × |TA| path matrix), `Path2Term`. Index masks: `idNsrc/idNtra/idNter`, `idAsrc/idAtra/idAter`, sets `setNsrc...`. Counts: `nbA, nbAsrc, nbAter, nbAtra, nbNter, nbNtra`. Other: `idxPred, idxSucc, parentArc`, `totcost`, `options` (IPM options). The Rust port needs the graph fields as engine state only.

### 2.9 Design result tables

| Table | Columns | Meaning | Source |
| --- | --- | --- | --- |
| TL | `ID` (= TA.succ), one column per borne | Design flow (L/s) on each arc for the demand of each borne (NW4 heuristic, VaR/CVaR) | `M:CreateDsgDev/build_incidence_loads.m:277` |
| TR | `ID`, `rexist`, one column per real pipe ID | Linear resistance of the existing equipment and of each candidate | `M:CreateDsgDev/equivResist.m` |
| TAH ("hydraulics") | `pred, succ, equivResist, designFlow, pathLoss, margin` | Equivalent linear resistance after reinforcement. Design flow `max(TL)` (L/s). Head loss from source to `succ` (m). Head margin (m) | `M:CreateDsgDev/processSol.m:233` to `:311` |
| TCP ("complement TA") | `pred, succ, nature, diam1, diam2` | `nature` = `old` (existing) or `new` (reinforcement). Diameters in m | `M:CreateDsgDev/processSol.m:347` |
| TS ("constraint satisfaction") | `noeud, constraint_type, head_required, head_available, head_margin, head_deficit` | One row per arc (by `succ`). `constraint_type` = `demand_satisfaction` (terminal) or `intrusion_prevention`. Heads in m. `head_deficit = max(0, -head_margin)` | `M:CreateDsgDev/processSol.m:289` |
| TFact | `succ, designFlow, margin` | Subset of TAH written as `!loadfactor` | `M:InOut/data4DsgFile.m:101` |
| reinfcost | `costOptim, costInter, costFinal, costFixed` | Costs before/after rounding, fixed reinforcement cost | `M:CreateDsgDev/processSol.m:353` |
| TableFusion | `noeud_supprime, extremite1, long1, extremite2, long2, lon_total` | Trace of fusions. Never written to disk by the current code (only named in the run meta) | `M:CreateDsgDev/buildContext.m:28` |

## 3. `.tpo` (topography) in NW5

### 3.1 Sections

| Section | Fields | Columns | NW4 | Required |
| --- | --- | --- | --- | --- |
| `!Commentaires` | free | Free text lines (commas allowed). Ignored by readers. Kept by the app (`comments`) | ignored | no |
| `!Nodes` | 6 | `noeud, haut, xdata, ydata, nbrob, statut` | same | yes |
| `!Pipes` | 3 | `pred, succ, lon` (app name `length`) | same | yes |
| `!Default properties` | 2 | `name, value` (section 2.7) | same, fewer keys | yes |
| `!Diameters` | 7 or 8 | See 3.2 | no | no (else `.db`) |
| `!Orifices` | 2 | `in_diam, N` | no | no (else `orifices.db`) |
| `!Clients` | 4 | `client, borne, proba, cible` | no | no |
| `!Borne` | 3 | `borne, pressure, N` | no | no |

NW4 ignores unknown sections, so an NW5 `.tpo` stays readable by NW4 (`DOC:FormatPourNW5.md`, `EX:tpo_examples/Toy/README.md`). NW4 reads `!Nodes` field 5 as tap count. If `!Clients` exists, NW5 uses it instead.

Example (`EX:tpo_examples/Toy/Toy_bbd.tpo`, shortened):

```
!Commentaires
bbd = modele de base (b) + debits bornes (b) + clients differencies (d)
!Nodes
Source,0,0,0,0,0
A,-10,0,0,0,1
P2,-9,0,0,3,2
!Pipes
Source,A,200
A,P2,180
!Default properties
orifcoef,0.59
opentaps,0.6
faucetcoef,1.87E-08
targetflow,0.12
!Diameters
"1/2""",13.5,0.0182,0.2799,200,1,0.0015
!Orifices
0.0017,N
!Clients
C1P2,P2,0.6,0.03
C2P2,P2,0.6,0.03
```

### 3.2 `!Diameters` in a `.tpo`: two incompatible layouts

- 7 fields, no ID: `Nominal, SDR, diam, cost, pressure, type, Roughness` (same as a `.db` row). Every example uses it (`Toy_bbd.tpo`, `Toy_bbu.tpo`, `Toy_blu.tpo`). So do the app reader (`APP:neatwork/parsers/tpo_parser.py:96`), the app writer (`APP:neatwork/serialization/revision_serializer.py:201`) and `PY:design/parserTPO.py:167`. IDs are then generated (`D001...`).
- 8 fields, ID first: `ID, Nominal, SDR, diam, cost, pressure, type, Roughness`. Read by MATLAB `parseTpoToTables` (`M:InOut/parseTpoToTables.m:98`) and used by its test fixture (`T:test_client_data_precedence.m`, `D1,P50,11,0.05,2,1000,1,0`). MATLAB reading a 7-field row misaligns all values DEFECT UNVERIFIED.

**Rust rule:** decide per row by field count: 8 fields → first is the ID. 7 fields → generate the ID after sorting. If the catalogue has IDs that are not generated, write 8 fields. Else write 7 fields, as the app and the examples do (PROPOSAL). A writer for MATLAB must write 8.

### 3.3 Defaults when a section is missing

| Missing | MATLAB (`parseTpoToTables`) | App (`parse_tpo`) |
| --- | --- | --- |
| `!Nodes`, `!Pipes`, `!Default properties` | Error "Sections manquantes dans le .tpo (attendu: !Nodes, !Pipes, !Default properties)." (`:46`). Wrong order: "Ordre des sections inattendu dans le .tpo." (`:49`) | Empty tables, no error (Cruscade case) |
| `!Diameters` | Read `examples/tpo_examples/<nom>/DiamBase<nom>.db` (`:103`). A missing file gives the error "Fichier introuvable : ..." | Use the uploaded diameters DB, else `ValueError("TPO import failed: missing !Diameters and diameters.db.")` (`:101`). The backend refuses earlier: "TPO import requires a diameters database. ..." (`BE:service/ProjectDomainImportService.java:84`) |
| `!Orifices` | Read `examples/tpo_examples/<nom>/orifices.db`. A missing file is an error | Use the uploaded orifices DB, else warning "TPO has no !Orifices and no orifices.db was provided." (`:116`) |
| `!Clients` | Build one client per tap: for each node with `statut = 2` and `nbrob > 0`, clients `C1<node>` .. `C<nbrob><node>` with `proba = opentaps`, `cible = targetflow` (`:146` to `:177`) | Build clients `C1, C2, ...` with a **global** counter over terminal nodes in file order, same proba/cible. Warning "TPO has no !Clients section. N clients were generated from terminal nodes where statut = 2 using nbrob." (`:120`, `:292`) |
| `!Borne` | One row per statut-2 node, `pressure = pardsg.pressure` (`:206`) | Same, with `N = ""`, warning "TPO has no !Borne section. N bornes were generated ..." (`:125`) |

`PY:design/parserTPO.py:140` generates `C<k><borne>` with k from **0** and stores the ID under a wrong key (`df_client`) DEFECT.

**Rust rule:** generate `C<k><borne>`, k = 1..nbrob, like MATLAB (this matches all example `.dsg` files). Terminals with `nbrob = 0` get no client. The simulation then reports `TERMINAL_WITHOUT_CLIENT`.

### 3.4 Validation done on TPO import

- MATLAB `validate_and_align_Tables` (`M:InOut/validate_and_align_Tables.m`): duplicate node IDs (`NW:DuplicateNodeIds`), arc nodes not in TN ("Noeuds inconnus dans TA: pred={...} succ={...}"), TN nodes not used by any arc ("Noeuds présents dans TN mais absents de TA: {...}"), bornes not in TN or TA ("Bornes de TB absentes de TN={...} ou de TA (pred/succ)={...}"), self-loop warning.
- MATLAB `peel_orient_tree`: exactly one source ("Il doit y avoir exactement une source avec TN.statut == 0 (trouvé: %d)."), then the topology report. The design stops with `NW5:InvalidDesignTopology` "Conception interrompue : <reason>" unless the network is a tree with no parallel arc and a source of degree 1. The reason text suggests "ajouter une source fictive reliée à l'ancienne par un arc de longueur nulle" (`M:InOut/peel_orient_tree.m:154`).
- App `_validate_graph`: "TPO import failed: unknown pipe nodes [...]", "TPO import failed: unknown bornes [...]", "TPO import failed: self-loop detected on node X" (`APP:neatwork/parsers/tpo_parser.py:320` to `:328`).

## 4. `.dsg` (design) in NW5

### 4.1 Sections

| Section | Fields | Columns | Written by NW4 | Written by MATLAB NW5 now | Read by |
| --- | --- | --- | --- | --- | --- |
| `!Commentaires` | free | Free text | no | no (kept by the app) | app |
| `!Nodes` | 7 | `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut` | yes | yes (1st) | all |
| `!Pipes` | 8 | `pred, succ, lon, lon1, ID1, lon2, ID2, N` | yes | yes (2nd) | all |
| `!Default properties` | 2 | `name, value` | yes | yes (3rd) | all |
| `!loadfactor` | 3 | three variants, 4.4 | yes | yes (4th) | NW4. MATLAB `maj_NW4_NW5`. App keeps verbatim |
| `!Diameters` | 9 (8 tolerated) | `ID, Nominal, SDR, diam, cost, Pmax, Type, Roughness, N` | yes | yes (5th) | all |
| `!Orifices` | 6 | `in_diam, N, N, N, N, N` | yes | yes (6th) | all |
| `!Clients` | 4 | `client, borne, proba, cible` | no | yes (7th) | all. **required** by MATLAB |
| `!Borne` | 3 | `borne, pressure, N` | no | yes (8th) | all |
| `!Hydraulics` | 6 | TAH: `pred, succ, equivResist, designFlow, pathLoss, margin` | no | yes (9th) | none (app keeps verbatim) |
| `!ComplementTA` | 5 | TCP: `pred, succ, nature, diam1, diam2` | no | yes (10th) | none |
| `!Costs` | 2 | `name, value`: `costOptim, costInter, costFinal, costFixed` | no | yes (11th) | none |
| `!ConstraintSatisfaction` | 6 | TS columns (2.9) | no | yes if TS is not empty (12th) | none |

Writer: `M:InOut/export_neatwork_csv.m:7` to `:41`. Each row is the table row joined by `,` (`:66`, `:81`). There is no header row and no `N` after the table columns. Lines end with `\n`. Numbers use `%.15g`. Strings and categoricals are written as they are. Logical columns print as `true`/`false` (none is written today). The writer is called by `data4DsgFile` (`M:InOut/data4DsgFile.m:128`), which prepares:

- `!Nodes`: TN columns in the order above, sorted by `(statut, noeud)`.
- `!Pipes`: TA columns `pred..N`. For a TA without `N`, it adds `N = "N"`.
- `!Default properties`: `name,value` rows, `pardsg` (alphabetical) then `partec` (alphabetical). If one value is not numeric, the whole value column becomes text (`M:InOut/data4DsgFile.m:139`).
- `!loadfactor`: TFact (`succ, designFlow, margin`).
- `!Diameters`: TD first 8 columns + `N`.
- `!Orifices`: TO + 5 `N` columns.
- `!Borne`: TB, plus an `N` column for a TB without one.
- File name: dialog box with default `<nom><version>_Secu<secu %1.2f>_fusion<%1.0f>_split<%d>` + `.dsg` in `out/<nom>/` (`M:InOut/data4DsgFile.m:50`). Example: `Toy_bbd_Secu1.00_fusion0_split0.dsg`.

The order of the first 8 sections matches every MATLAB-written example. Older NW5 writers used other orders (`Red 2_Secu...dsg`: Nodes, Pipes, Clients, Default properties, loadfactor, Diameters, Orifices, `Esperenza2S.dsg`: ..., Borne before Clients). Readers must not depend on order.

### 4.2 Readers

MATLAB `parseDsgToTables` (`M:InOut/parseDsgToTables.m`):

- `!Clients` is required: error "Le fichier ne contient pas de section Clients. Mettre à jour le fichier NW4 avec l'utilitaire." (`:41` to `:44`). `T:test_client_data_precedence.m` checks the substring `ne contient pas de section Clients`.
- `!Pipes` `'%s %s %f %f %s %f %s %s'`, `!Nodes` `'%s %f %f %f %f %f %d'`, `!Diameters` `'%s %s %s %f %f %f %d %f %s'` (SDR read as text), `!Orifices` `'%f %s %s %s %s %s'` (only `in_diam` kept), `!Default properties` `'%s %f'`.
- For a missing `pressure`, it adds `pressure = 2` (`:90`). For a missing `!Borne`, it builds TB from the terminals (`:116` to `:120`). Else it checks that bornes = terminals.
- `!loadfactor`, `!Hydraulics`, `!ComplementTA`, `!Costs`, `!ConstraintSatisfaction`, `!Commentaires` are ignored.
- Then `apply_client_data` (2.3). `readDesign` also trims all IDs with `normalize_id_column` (`M:InOut/readDesign.m:134`). An empty ID gives "IDs contiennent des valeurs manquantes (%d).". For a missing `bounded_flow` key, `readDesign` sets `bounded_flow = 1` (`:155`). So a `.dsg` without `bounded_flow` is read as bounded-flow in MATLAB, but as `bounded_flow = 0` by the app and by `extract_parameters` DEFECT (inconsistent defaults).

App `parse_dsg` (`APP:neatwork/parsers/dsg_parser.py`):

- Requires `!Nodes` and `!Pipes`: "DSG import failed: missing !Nodes section" / "... !Pipes section" (`:90`, `:92`).
- Missing `!Clients` → warning "DSG has no !Clients section. Simulation requires client demand data before running." (`:94`).
- Missing `!Borne` → builds "outlet" rows `{outlet, elevation_m, ideal_orifice_diameter_m, commercial_orifice_diameter_m}` from terminals (`:202`). These rows have no `borne`/`pressure`, so the typed simulation domain rejects them (verified on `ToyXlx.dsg`: pydantic "Field required" on `terminal_id` and `required_pressure_m`) DEFECT.
- Rows with fewer fields than the layout are **dropped silently**. Extra fields are ignored (`APP:neatwork/parsers/parsing_utils.py:63`). An 8-field `!Diameters` (no trailing `N`, as in `EX:dsg_examples/Toy/ToyDevReinf.dsg`) therefore gives an empty catalogue (verified) DEFECT. Rust must accept 8 or 9 fields.
- Orifices are read as `{source_index, diameter_m, diameter_mm}`.
- `params` = `DSG_DEFAULT_PARAMS` + file values. `technicalParams = {}`. `designParams` absent (`{}`).

### 4.3 Pipe rows and the `N` status

`pred,succ,lon,lon1,ID1,lon2,ID2,N`:

- One diameter: `lon1 = lon`, `lon2 = 0`, `ID2` = `fictif` (current MATLAB), `0` (NW4 and older NW5). A bare arc (TPO before design, or an arc that the design left to a parallel pipe) has `lon1 = lon2 = 0`, `ID1 = ID2 = fictif` (`ToyDevReinf.dsg`: `C,P3,90,0,fictif,0,fictif,N`).
- Two diameters: `lon1 + lon2 = lon`. `ID1` is the larger diameter in NW5 output. If `pipelength > 0.1`, segment 1 has a length that is a multiple of `pipelength` (`M:CreateDsgDev/processSol.m:211`).
- `N` column: `N`, `B` or `R` (2.2). Current MATLAB output from a TPO marks every designed pipe `R` (all arcs are "reinforcements" of an empty network) UNVERIFIED. Examples written by older code show `N`.
- Parallel pipes: two rows with the same `pred,succ` (existing `N`/`B` + new `R`). For a link that ends at a terminal, the app simulation then fails with `INVALID_TERMINAL_IN_DEGREE`. Other parallel links are accepted by validation UNVERIFIED for the solver.

Reading rule for `ID1`/`ID2` (Rust): for a segment of length 0, treat `fictif`, `0`, `N`, `-` and empty as "no pipe". For a segment of length > 0, one of these IDs or an ID that is not in TD is an error.

### 4.4 `!loadfactor`: three variants

| Variant | Row | Meaning | Where |
| --- | --- | --- | --- |
| NW4 | `node, loadfactor, 0` | Load factor of the arc that ends at `node` (ratio of design flow to `targetflow`) | NW4 files, `Toy2b_NW4.dsg`, `ToyXlx.dsg`, `CeibaNW4.dsg`, also `ToyDevReinf.dsg` |
| Older NW5 | `pred, succ, loadfactor` | Same ratio, keyed by arc | `Toy_*_Secu1.00_fusion0_split0.dsg`, `Red 2_Secu...dsg`, `CeibaNW4_Secu...dsg` |
| Current NW5 | `succ, designFlow, margin` | Design flow (L/s) and head margin (m), **not** a load factor | `M:InOut/data4DsgFile.m:101` |

The three share the field count 3. Distinguish them: if field 3 is `0` on all rows and field 1 is a node → NW4. If fields 1 and 2 are both node IDs → older NW5. Else current NW5 (field 2 > 0 in L/s, field 3 can be negative). NW4 misreads the current variant (a flow read as a load factor) DEFECT. The simulation never needs this section. Rust: keep the raw rows for round-trip. Expose a typed view only for a detected variant.

### 4.5 Orifice sentinels in `!Nodes`

| Value | Meaning | Producer |
| --- | --- | --- |
| `1` (`orif_ideal` and `orif_com`) | No orifice (1 m diameter, negligible loss) | NW5 MATLAB for non-terminals and terminals with margin < 0.1 m or bounded flow (`M:CreateDsgDev/processSol.m:316` to `:333`) |
| `10000` / `10000.0` | No orifice | NW4 (`Toy2b_NW4.dsg`, `ToyXlx.dsg`) |
| `0` | Not set | NW4 non-terminals, Esperenza non-terminals, app TPO→domain, EPANET import |
| `10` (`orif_ideal`) | No orifice (Txt variant) | `EX:tpo_examples/ToyTxt/Toy2bDsgNodes.txt` |
| other | Diameter in m | |

If `bounded_flow = 0`, the app simulation needs a positive orifice on every terminal (`INVALID_TERMINAL_ORIFICE`). It reads `orif_com`, or `orif_ideal` for `orif_com = 0`. The MATLAB simulation always uses `orif_ideal`, whatever `orif_com` says DEFECT (simulation spec).

### 4.6 Example (current layout, `EX:dsg_examples/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg`, shortened)

```
!Nodes
Source,0,0,0,1,1,0
A,-10,0,0,1,1,1
P2,-9,0,0,1,1,2
!Pipes
Source,A,200,200,D009,0,0,N
A,B,500,38,D009,462,D004,N
!Default properties
bounded_flow,1
...
watertemp,20
bescal,0.00089396
faucetcoef,1.87e-08
lambda,1.781
nu,4.781
orifcoef,0.59
!loadfactor
Source,A,5.5737646484375
!Diameters
D001,"1/2""",13.5,0.0182,0.2799,200,1,0.0015,N
!Orifices
0.0017,N,N,N,N,N
!Clients
C1P1,P1,0.6,0.12
C10P2,P2,0.6,0.03
!Borne
P1,2,N
```

## 5. NW4 → NW5 upgrade

### 5.1 `.tpo`

No conversion is needed: an NW4 `.tpo` is a valid NW5 `.tpo`. Missing NW5 sections are built (3.3). `nbrob > 1` at a terminal becomes `nbrob` clients at that borne. NW4 parameters missing in NW5 take the defaults of 2.7. Note that NW4 (Java) itself expands a multi-tap node into fictitious faucet nodes `<node>_a`, `<node>_b`, ... joined by 1 m pipes. NW5 does not.

### 5.2 `.dsg`: `maj_NW4_NW5.m`

`M:InOut/maj_NW4_NW5.m` is a script (not a function) with the file hard-coded (`nom = 'Toy'`, `version = '2b_NW4'`, `:36`), reading `examples/dsg_examples/<nom>/<nom><version>.dsg` and writing `out/<nom>/<nom><version>_NW5.dsg`. Algorithm:

1. If a `!Clients` header exists: print "Ce fichier contient unes section Clients. Ce n'est pas un fichier NW4." and stop (`:49`).
2. Read `!Nodes` (7 fields). Strip the suffix `_...` from each name (`regexprep(name, '_.*$', '')`, `:77`). Group rows by base name in order of appearance. For a group with suffixed rows (the fictitious faucets `P2_a`, `P2_b`, ...), keep one row: base name + the fields of the **first suffixed row** (its elevation, coordinates, orifices and statut 2) (`:87`). So the parent `P2` (statut 1 in NW4) becomes terminal with the faucet's orifices.
3. For a group of more than one row, `nbrob` = rows in group − 1. Else `nbrob` = 1 (`:179`).
4. Delete every `!Pipes` row whose `succ` contains `_` (`:113`). Delete every `!loadfactor` row whose node contains `_` (`:123`).
5. Read `!Default properties`. For each missing key, add `pressure = 0`, `bounded_flow = 0`, `uniform_dem = 1` (`:152` to `:160`). Note: `pressure` default 0 here, 2 elsewhere.
6. Build TC: for each terminal, clients `C<k><node>`, k = 1..nbrob, `proba = opentaps`, `cible = targetflow`. Checks (`:216` to `:229`): unique IDs (`Doublons dans TC.ID (chaque client doit être unique).`), non-empty borne (`Chaque client doit avoir exactement une 'borne' non vide.`), borne is terminal (`Borne(s) non terminales ou inconnues: %s`). Sort TC by `(borne, client)`.
7. Write the sections with `writetable` (comma, no header) (`:244` to `:250`). The order is `!Nodes`, `!Pipes`, `!Clients`, `!Default properties`, `!loadfactor`, `!Diameters`, `!Orifices`. `!Diameters` has 8 columns (no `N`). `!Orifices` has 1 column. There is no `!Borne` (readers build it).

Defects (all UNVERIFIED, from reading the code):

- Any real node whose name contains `_` is treated as a fictitious faucet and its incoming pipe is deleted.
- `!loadfactor` is parsed with the 8-field `!Pipes` format string (`:120`). On 3-field rows `textscan` reads across lines and the result is garbage. The variable name is also misspelled (`lodfactor`).
- `writetable` number formatting is MATLAB's default (up to 15 significant digits), not `%.15g` exactly.

**Rust rule (PROPOSAL):** implement the upgrade as a function `upgrade_nw4_dsg(&Design) -> Design` that detects fictitious faucets by structure, not by name alone. A fictitious faucet is a statut-2 node `X` with these properties: its only incoming pipe comes from a node `P` and has length 1 m, and its name is `P_<suffix>`. The `<suffix>` is lower-case letters (`_a`..`_z`, then `_aa`..., as NW4 generates them). Merge the faucets into `P` as above. Keep the NW4 load factors of the remaining nodes. Write the current NW5 layout (section 4.1), with `!Borne` rows at `pressure` (default 2, see open question 3 in section 15).

## 6. Clients, multi-tap nodes, sources and meshed networks

- **Multi-tap node (NW5):** one terminal node (statut 2, in TB) and several TC rows with `borne = node`. `nbrob` = number of those rows. Clients at one borne can differ (`Toy_bbd`: P2 has 12 clients at 0.03 L/s, P3 has 2 at 0.24 L/s with p = 0.3).
- **Multi-tap node (NW4):** parent node statut 1 + fictitious terminals `<node>_a...` with 1 m pipes (`Toy2b_NW4.dsg`). Upgrade per section 5.
- **Behaviour cases** (`DOC:types de clients.md`, `M:CreateDsgDev/makeReinfDsg.m` header): A = NW4 (`uniform_dem 1, bounded_flow 0`), B = NW4+ (`1, 1`), C = irrigation (`0, 1`), D = (`0, 0`) not handled: app validation `UNSUPPORTED_SIMULATION_CASE` "The combination uniform_dem=0 and bounded_flow=0 is not supported.", MATLAB processSol "Demandes heterogenes non bornees non traitees." (`NW5:Orifices`).
- **Source:** statut 0. Design: exactly one source, degree 1. A source connected to several arcs needs a fictitious source joined by a zero-length arc (zero length becomes 0.01 m) (`M:InOut/peel_orient_tree.m:154`). Examples use a 0.1 m pipe (`Red 2`: `Source,724,0.1`). Simulation (app): one or more sources. A source must have no incoming pipe and at least one outgoing pipe. A source must not feed a terminal directly (`DIRECT_SOURCE_TO_TERMINAL_PIPE`). Multi-source example: `Esperenza2S.dsg` (S1, S2).
- **Pump:** no pump entity. `pump_pressure` (m) is a head gain added to the source elevation in design (`Hsource = haut(source) + pump_pressure`, `M:CreateDsgDev/enrichTablesReinf.m:36`, `M:CreateDsgDev/apply_pump_pressure.m:17`). It is written in `!Default properties` by the current MATLAB writer (pardsg). The app TPO reader drops it (2.7).
- **Valves:** the app marks a TPO arc with length ≤ 0 as `VALVE` in the drawing (`APP:neatwork/parsers/tpo_parser.py:173`). A pipe can be closed by `enabled = false` or `operational_status` in `{CLOSED, DISABLED, INACTIVE, OFF, OUT_OF_SERVICE}` (`APP:schemas/simulation_domain.py:185`). No file format stores this today except the app snapshot JSON.
- **Meshed networks:** the simulation accepts them. App topology validation checks reachability from a source and terminal in-degree = 1, not cycles. `Esperenza2S.dsg` has the cycles J3-J4-J7-J6 and J3-J5-J8-J6. Refused by the design ("Le graphe simplifié contient un cycle."). File formats need nothing special: a meshed network is just more `!Pipes` rows. Arc orientation in a meshed network is the file orientation (no peeling).

## 7. JSON formats

NW5 has four unrelated JSON documents.

### 7.1 MATLAB design JSON (`NW-json-v1`)

Writer `M:InOut/save_dsg_json.m`. For `output_type = 'json'`, `data4DsgFile` calls it. File name as for `.dsg` with extension `.json`. Encoded by `jsonencode(P, 'PrettyPrint', true)`.

Top-level object, keys in this order:

| Key | JSON type | Content |
| --- | --- | --- |
| `schema` | string | `"NW-json-v1"` (`:5`) |
| `TN` | array of objects (see note) | TN with columns `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut`, in processing order (not sorted) |
| `TA` | array | Full TA after design: `pred, succ, lon, lon1, ID1, lon2, ID2, N, equip (bool), diam1, diam2, nature` UNVERIFIED (column set inferred from `M:CreateDsgDev/processSol.m:71` to `:80`) |
| `TB` | array | `borne, pressure`, plus `N` for a TB read from a file |
| `TC` | array | `client, borne, proba, cible` |
| `TD` | array | First 8 columns of TD: `ID, Nominal, SDR, diam, cost, pressure, type, Roughness`, including the `fictif` row UNVERIFIED |
| `TO` | array | `in_diam` |
| `TFact` | array | `succ, designFlow, margin` |
| `TAH` | array | `pred, succ, equivResist, designFlow, pathLoss, margin` |
| `TCP` | array | `pred, succ, nature, diam1, diam2` |
| `TS` | array | TS columns (2.9). An absent TS gives `[]` |
| `TPar` | array | `{ "name": <string>, "value": <number or string> }`, pardsg then partec |
| `pardsg` | object | All design parameters (2.7), alphabetical |
| `partec` | object | `bescal, faucetcoef, lambda, nu, orifcoef` |
| `reinfcost` | object | `costOptim, costInter, costFinal, costFixed` |

Encoding details (MATLAB `jsonencode`, UNVERIFIED for the exact text):

- A table with one row is encoded as a single **object**, not an array (MATLAB `table2struct` of one row is a 1×1 struct). Readers must accept an object or an array for every table key.
- An empty table is `[]`.
- `Inf` and `NaN` become `null` (for example `limitbudget`).
- Categorical and string values become strings. Logicals become `true`/`false`.
- `dat` is not written. There is no `meta` key.

Readers:

- `M:InOut/read_Dsg_json.m`: accepts table keys `TNodes, TArcs, TClients, TPar, TFact, TDn, TOn, TA, TN, TB, TC, TD, TO` and struct keys `pardsg, partec, dat, meta` (`:37`, `:41`). It ignores `schema`, `TAH`, `TCP`, `TS`, `reinfcost`. It converts `pred, succ, noeud, ID, ID1, ID2` to categorical and renames `loadfactor` to `loadFactor` in `TA`/`TFact`. Nested struct columns are flattened as `<col>_<sub>`. Error text: "Format JSON inattendu pour table (%s).". Warning "Impossible de convertir la clé %s en table. Stockée brute.".
- `M:InOut/readDesign.m` (`type = 'json'`) needs `TA, TN, TC, TB, TD, TO, TPar, pardsg, partec`. It rebuilds `param` from `TPar`. For a `param` that differs from `pardsg ∪ partec`, it prints "param et parami ne sont pas identiques" (`:104`).
- `M:InOut/load_dsg_json.m` (older): needs `TN, TA, TC, TD, TO, TPar, pardsg, partec, dat` and fails on files from `save_dsg_json` (no `dat`) DEFECT.

Versioning: only the tag `"NW-json-v1"`. No reader checks it. Rust: write `"NW-json-v1"`. On read, accept a missing or equal tag, warn on another value.

### 7.2 MATLAB run metadata (`NW_design_meta_v1`)

`M:CreateDsgDev/makeReinfDsg.m:186` to `:227`, `M:CreateDsgDev/write_meta.m`: file `out/<nom>/runs/meta_<nom>_<datetime>.json`, pretty-printed:

| Key | Type | Meaning |
| --- | --- | --- |
| `schemaVersion` | string | `"NW_design_meta_v1"` |
| `network` | string | `dat.nom` |
| `version` | string | `dat.version` |
| `date` | string | ISO 8601 `yyyy-MM-ddTHH:mm:ssZ` (local time labelled `Z` DEFECT) |
| `useFusion`, `useSplit` | bool | Options used |
| `nFusions`, `nbsplit` | number | Counts |
| `vmax, secu, uniform_dem, bounded_flow, pressure` | number | Copied from pardsg |
| `solver` | object | `{message, fval}` |
| `traceFile` | string | `<nomv>_TableFusion.csv`, only after fusions (the file is not written) |

### 7.3 Application domain JSON (snapshot `domainJson`)

Produced by the python-core parsers (`NeatworkDomainPayload`, `APP:neatwork/schemas.py:66`) and stored by the backend in `project_domain_snapshots.domain_json` (jsonb, `BE:model/entity/ProjectDomainSnapshotEntity.java`). It is the edit model of the GUI. A snapshot is immutable. An edit creates a new revision file and a new snapshot (`BE:service/ProjectDomainRevisionService.java`).

```json
{
  "name": "Toy_bbd",
  "sourceFormat": "DSG",
  "comments": ["bbd = ...", "..."],
  "params": { "bescal": 0.00089396, "bounded_flow": 1, "...": "..." },
  "technicalParams": {},
  "designParams": {},
  "tables": {
    "nodes": [ { "noeud": "Source", "haut": 0, "xdata": 0, "ydata": 0, "orif_ideal": 1, "orif_com": 1, "statut": 0 } ],
    "pipes": [ { "pred": "Source", "succ": "A", "lon": 200, "lon1": 200, "ID1": "D009", "lon2": 0, "ID2": 0, "N": "N" } ],
    "edges": [ "same list as pipes" ],
    "clients": [ { "client": "C1P1", "borne": "P1", "behaviorId": "CB_BAE72D2A3E" } ],
    "clientBehaviors": [ { "behaviorId": "CB_BAE72D2A3E", "name": "Imported behavior 1", "openProbability": 0.6, "targetFlowLps": 0.12, "profileId": null } ],
    "bornes": [ { "borne": "P1", "pressure": 2, "N": "N" } ],
    "diameters": [ { "ID": "D001", "Nominal": "\"1/2\"\"\"", "SDR": 13.5, "diam": 0.0182, "cost": 0.2799, "Pmax": 200, "Type": 1, "Roughness": 0.0015, "N": "N" } ],
    "orifices": [ { "source_index": 1, "diameter_m": 0.0017, "diameter_mm": 1.7 } ]
  },
  "warnings": []
}
```

Fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | string | Network name (request `name`, after a revision, the new file name without extension, `BE:service/ProjectDomainRevisionService.java:115`) |
| `sourceFormat` | `"TPO"` \| `"DSG"` \| `"JSON"` | Origin format. The EPANET import builds a `SimulationDomainInput` with `"EPANET"`, the reference test uses `"EPANET_REFERENCE"` |
| `comments` | array of strings \| null | `!Commentaires` lines (DSG). `null` for TPO DEFECT (the typed domain then rejects it: `Input should be a valid list`, verified) |
| `params` | object | All parameters (2.7). Values: numbers, or strings for non-numeric file values. `inf` as a float (not valid JSON, UNVERIFIED how FastAPI serialises it, Starlette's default `JSONResponse` refuses NaN/Inf) |
| `technicalParams` | object | TPO: the 5 technical parameters. DSG: `{}`. EPANET: `{ "epanet": { units, headloss, options, patterns, emitters, preservedSections } }` |
| `designParams` | object | TPO: design parameters. DSG: `{}` |
| `tables.nodes` | array | TPO rows `{noeud, haut, xdata, ydata, nbrob, statut}`. DSG rows `{noeud, haut, xdata, ydata, orif_ideal, orif_com, statut}`. Numeric-looking IDs are numbers (1.3) |
| `tables.pipes` | array | TPO rows `{pred, succ, length}`. DSG rows `{pred, succ, lon, lon1, ID1, lon2, ID2, N}` |
| `tables.edges` | array | Same list as `pipes` (duplicate) |
| `tables.clients` | array | `{client, borne, behaviorId}` after normalisation. Legacy rows `{client, borne, proba, cible}` are accepted |
| `tables.clientBehaviors` | array | `{behaviorId, name, openProbability, targetFlowLps, profileId}` |
| `tables.bornes` | array | `{borne, pressure, N}`. Generated TPO rows have `N: ""`. DSG without `!Borne` gives outlet rows (4.2) |
| `tables.diameters` | array | TPO rows `{size, SDR, diam, cost, pressure, type, rugo, ID}`. DSG rows `{ID, Nominal, SDR, diam, cost, Pmax, Type, Roughness, N}` |
| `tables.orifices` | array | `{source_index, diameter_m, diameter_mm}` or (TPO section) `{in_diam, N}` |
| `warnings` | array of strings | Parser warnings (3.3, 4.2) |

Client behaviours (`APP:neatwork/domain/client_behavior.py`):

- On import, each legacy client row becomes `{client, borne, behaviorId}`. Identical `(proba, cible, profile)` share one behaviour (`:31`). A row without `behaviorId` must have `proba` and `cible`. Else the error is `Client 'X' has neither behaviorId nor complete legacy demand values.` (`:68`).
- `behaviorId = "CB_" + upper(hex(sha1(canonical))[0:10])`, `canonical = "<p>|<t>|<profileId or empty>"`, each number printed as `format(Decimal(str(v)).normalize(), 'f')` (`:12`). Test vectors (computed 2026-11-20): `0.6|0.12|` → `CB_BAE72D2A3E`. `0.5|2|` → `CB_89C66A034C`. `0.28|0.12|` → `CB_3090BA5D71`. `1|10|` → `CB_5DDB4706FD`. Rust: print with the shortest round-trip decimal (`ryu`-style `{}` of f64), strip a trailing `.0`, never use exponent notation.
- `name` = `"Imported behavior <n>"`, n = 1-based count at creation (`:83`). Names are not stored in `.dsg`/`.tpo`. A revision restores them from the edited domain (`APP:neatwork/serialization/revision_serializer.py:322`).
- On write, each client is materialised back to `client, borne, proba, cible` (`:153`). An unknown behaviour is "Unknown client behavior 'X'.".

Visual graph (`VisualGraph`, `APP:neatwork/schemas.py:11` to `:47`), returned with the domain and stored by the backend as network nodes/edges:

- Node: `id, label, type` (`RESERVOIR` for statut 0, `FAUCET` for 2, else `JUNCTION`), `x, y` (layout), `elevationM` (DSG: `haut`), `pressureM` (TPO: `haut` DEFECT, DSG: 0), `lowPressure`, `sourceX, sourceY` (file coordinates), `layoutGenerated`.
- Edge: `id` (`A1`.. in file order), `sourceId, targetId, type` (`PIPE`, or `VALVE` for TPO length ≤ 0), `status` (`OPEN`), `flowRateLps, velocityMps, lengthM, diameterMm, nominalSize, sdr, innerDiameter, cost, pmax, pipeTypeCode, roughness`. (The DSG parser passes `pipeTypecode`, a misspelling that pydantic ignores, so `pipeTypeCode` stays null DEFECT.)
- Layout: when every node has `xdata = ydata = 0`, a tree layout is generated: x = depth × 160, y = leaf index × 80, parent y = mean of children (`APP:neatwork/parsers/dsg_parser.py:238`).

### 7.4 Application JSON import (`sourceFormat = JSON`)

`APP:neatwork/parsers/json_parser.py`: either `{ "graph": VisualGraph, "domain": { "params": {...}, "tables": { nodes, pipes, clients, bornes, diameters, orifices } } }` or a bare `VisualGraph`. `technicalParams`, `designParams`, `comments`, `clientBehaviors` and `edges` are not read, and client behaviours are not normalised. The backend refuses JSON as a primary import (`BE:service/ProjectDomainImportService.java:52`). The artifact type `JSON_INPUT` exists all the same. MATLAB `NW-json-v1` files are not accepted by this parser.

### 7.5 Typed simulation domain (`SimulationDomainInput`)

`APP:schemas/simulation_domain.py` is the canonical typed form used to simulate and to check revisions. Unknown fields are kept (`extra="allow"`), strings are trimmed. Each field accepts several names:

| Entity | Field | Accepted names | Type, default |
| --- | --- | --- | --- |
| node | `node_id` | `node_id, nodeId, noeud, id` | str, required, non-empty |
| | `elevation_m` | `elevation_m, elevationM, haut` | f64, required |
| | `x`, `y` | `x/xdata`, `y/ydata` | f64, 0 |
| | `ideal_orifice_m` | `ideal_orifice_m, idealOrificeM, orif_ideal` | f64, 0 |
| | `commercial_orifice_m` | `commercial_orifice_m, commercialOrificeM, orif_com` | f64, 0 |
| | `status` | `status, statut` | int, required |
| pipe | `edge_id` | `edge_id, edgeId, external_edge_id, externalEdgeId, id` | str or null |
| | `predecessor_id` / `successor_id` | `pred, predecessorId, predecessor_id, sourceId` / `succ, successorId, successor_id, targetId` | str, required |
| | `length_m` | `length_m, lengthM, length, lon` | f64 or null |
| | `first_length_m` | `first_length_m, firstLengthM, lon1` | f64 or null (null → `length_m`) |
| | `first_diameter_id` | `first_diameter_id, firstDiameterId, ID1` | str or null (`""`, `0`, `"0"` → null) |
| | `second_length_m` | `second_length_m, secondLengthM, lon2` | f64, 0 |
| | `second_diameter_id` | `second_diameter_id, secondDiameterId, ID2` | str or null |
| | `metadata_n` | `metadata_n, metadataN, N` | any |
| | `enabled` | `enabled, is_open, isOpen` | bool, true |
| | `operational_status` | `operational_status, operationalStatus, status, edgeStatus` | str or null |
| client | `client_id` / `terminal_id` | `client, clientId, client_id` / `borne, terminalId, terminal_id` | str |
| | `behavior_id` | `behavior_id, behaviorId` | str or null |
| | `open_probability` / `target_flow_lps` | `proba, openProbability, open_probability` / `cible, targetFlowLps, target_flow_lps` | f64 or null (legacy) |
| borne | `terminal_id`, `required_pressure_m`, `metadata_n` | `borne/terminalId/terminal_id`, `pressure/requiredPressureM/required_pressure_m`, `N` | |
| diameter | `diameter_id` | `ID, id, diameterId, diameter_id` | str |
| | `nominal_size` | `Nominal, size, nominalSize, nominal_size` | str, required |
| | `sdr` | `SDR, sdr` | f64, 0 (text SDR such as `PEHD` fails validation) |
| | `inner_diameter` | `diam, innerDiameter, inner_diameter` | f64, required |
| | `cost` | `cost` | f64, 0 |
| | `maximum_pressure` | `Pmax, pressure, maximumPressure, maximum_pressure` | f64, 0 |
| | `type_code` | `type, Type, typeCode, type_code` | str, `"0"`. `1.0` → `"1"` |
| | `roughness` | `Roughness, rugo, rugosité, roughness` | f64, 0 |
| behaviour | `behavior_id, name, open_probability (0..1), target_flow_lps (≥ 0), profile_id` | camelCase or snake_case | |
| domain | `name, version (""), source_format ("DSG", upper-cased), params, technical_params, design_params, comments ([]), tables{nodes, pipes, clients, client_behaviors, bornes, diameters, orifices}` | `sourceFormat, technicalParams, designParams, clientBehaviors` | |

`edges` is not part of the typed tables (extra key, kept and ignored).

### 7.6 Simulation statistics JSON (schema `1.1`)

`APP:neatwork/simulation/json_exporter.py:98`. Exported by `/api/simulation/export` and stored by the backend as `simulation_statistics.json`. UTF-8, `allow_nan=False` (NaN/Inf → `null`):

```json
{
  "schemaVersion": "1.1",
  "network": { "name": "Toy_bbd_Secu1.00_fusion0_split0", "version": "" },
  "demand": { "terminals": [ { "terminalId": "P2", "clientCount": 12, "groups": [ { "groupId": "CB_...", "behaviorIds": [], "behaviorNames": [], "clientCount": 12, "openProbability": 0.6, "targetFlowLps": 0.03, "profileId": null } ] } ] },
  "units": { "sourceFlow": "L/s", "pipeVelocity": "m/s", "terminalPressure": "m", "transitPressure": "m", "duration": "s", "satisfaction": "ratio" },
  "summary": { "scenarioCount": 100, "successfulScenarios": 100, "failedScenarios": 0, "successRate": 1.0, "seed": 42, "durationSeconds": 0.1, "minIterations": 8, "meanIterations": 12.46, "maxIterations": 22, "failureCountByCode": {} },
  "scenarioSelection": { "successfulMask": [true], "includedScenarioIndices": [0], "excludedScenarioIndices": [] },
  "statistics": {
    "terminalSatisfaction": [ { "terminalId": "P1", "minSat": 0.99, "moySat": 0.99, "maxSat": 1.0, "cVarSat5": 0.99, "cVarSat20": 0.99, "stdSat": 0.0, "cvSat": 0.0, "activeScenarioCount": 60, "evaluatedActiveScenarioCount": 60, "waterShortfallCount": 0 } ],
    "sourceFlow": [ { "sourceId": "Source", "minSt": 0.12, "moySt": 0.47, "maxSt": 0.74, "cVarSt5": 0.18, "cVarSt95": 0.71 } ],
    "pipeVelocity": [ { "edgeId": "A1", "startNodeId": "Source", "endNodeId": "A", "minVelocityMps": 0.07, "meanVelocityMps": 0.29, "maxVelocityMps": 0.46 } ],
    "terminalPressure": [ { "terminalId": "P1", "minPressureM": 1.1, "meanPressureM": 2.07, "maxPressureM": 4.76 } ],
    "transitPressure": [ { "transitNodeId": "A", "minPressureM": 7.99, "meanPressureM": 9.13, "maxPressureM": 9.63 } ]
  },
  "failures": [ { "scenarioIndex": 3, "stage": "...", "code": "...", "message": "...", "details": {} } ]
}
```

The fixtures `APPT:fixtures/reference/toy_*_seed42.json` are an older form (no `demand`, no `durationSeconds`, no `stdSat`/`cvSat`/counts) UNVERIFIED version label. The statistics themselves belong to the simulation spec. This document fixes only the envelope.

### 7.7 Versioning and back-compatibility rules

- `NW-json-v1` (MATLAB), `NW_design_meta_v1` (run meta) and statistics `1.1` are the only version tags. The app domain JSON has none. Compatibility is by field aliases (7.5) and by keeping unknown fields.
- A project has one permanent source format, set by its first successful import (`BE:service/ProjectSourcePolicy.java`): "This project already contains an input file of the opposite format.", "This project uses X. Importing Y is not allowed.", "This legacy project requires a source format review before importing.".
- Revisions: "Saves edited input data as a completely new DSG/TPO revision. The original artifact and original snapshot remain immutable." A revised TPO embeds its catalogues, so the new snapshot has no DB artifacts.
- Snapshot comparison (`BE:service/DomainSnapshotCompatibilityService.java`), levels `SAME_SNAPSHOT`, `EQUIVALENT_INPUT`, `SAME_TOPOLOGY`, `TOPOLOGY_CHANGED` (`UNKNOWN` unused):
  - Keys match case-insensitively with spaces, `_` and `-` removed (`:73`).
  - Node identity = `(id, statut)` with id from `noeud|node|nodeId|node_id|id`, role from `statut|status|statusCode|nodeStatus` (−1 for a missing role).
  - Edge identity = `(id, from, to)`, id from `edgeId|edge|pipe|id` (`:132`). DSG/TPO pipe rows have no such key, so **edges are always empty** for imported snapshots and the topology fingerprint covers nodes only DEFECT.
  - Client identity uses `terminalId|terminal_id|terminal|nodeId` (`:200`), not `borne`, so the demand fingerprint ignores terminals for imported snapshots DEFECT. The effective-client rows do read `borne` (`:764`).
  - Topology fingerprint = SHA-256 of sorted `id|role` lines, `\n---EDGES---\n`, sorted `id|from|to` lines (`:299`).
  - Effective-input fingerprint = SHA-256 of a canonical string of `{params, technicalParams, designParams, tables}` (`:337`). Before hashing, the code removes coordinates (`x, y, xdata, ydata`) and sorts rows by their canonical string. It replaces clients and behaviours by resolved rows `{client, terminal, probability, targetFlow, profile}`. Canonical form: numbers via `BigDecimal.stripTrailingZeros().toPlainString()`, booleans `true/false`, maps as sorted `key=value` joined with `Collectors.joining("{", ",", "}")` (delimiter `{`, prefix `,`, suffix `}`: an unusual but stable form), lists sorted and joined with delimiter `[` (`:265` to `:283`). The backend computes the fingerprints again for each comparison and does not store them. So a Rust port can use any stable form (PROPOSAL: keep the Java form).
  - Warnings: `Input revision differs. Metric changes may reflect network parameters as well as simulation settings.`, `Client-to-terminal allocation changed between revisions. Terminal statistics may reflect demand redistribution.`, `Network topology changed. Only matched entities should be compared directly.`

## 8. EPANET `.inp`

### 8.1 What NW5 does today

- **No NW5 code writes an `.inp` file.** The backend export `INP` only returns the latest stored artifact of type `INP_OUTPUT` (`BE:service/ProjectExportService.java:81`). Nothing creates such an artifact. So the option always shows `No EPANET INP artifact exists for this project.` The frontend lists the format (`FE:features/export/types.ts:1`). `EX:README.md` mentions `xxx.inp` exports "non versionnés" that no code produces.
- python-core contains an **EPANET 2.2 import prototype**: `APP:neatwork/epanet/` (`parser.py` syntax, `semantic.py` typed model, `units.py`, `adapter.py` mapping to `SimulationDomainInput`, `prototype.py` pipeline). It is not wired to any HTTP endpoint. Its test fixtures (`APPT:fixtures/epanet/*.inp`) are missing from the copy, so its tests cannot run.

Section 8.2 specifies the import (implemented). Section 8.3 specifies an export (PROPOSAL), built as the inverse of the import and of the reference test `APPT:reference_validation/neatwork_fixture.py`.

### 8.2 Import (implemented in python-core)

Lexical (`APP:neatwork/epanet/parser.py`): lines split on `\n`/`\r\n`. BOM stripped. `;` starts a comment. A header is `[NAME]` (after removing the comment), name upper-cased. `[END]` stops parsing. Lines before the first header form the preamble. Tokens are whitespace-separated (no quoted tokens). Repeated sections are merged. Raw lines are kept.

Typed sections (`APP:neatwork/epanet/semantic.py:16`): `TITLE, JUNCTIONS, RESERVOIRS, PIPES, DEMANDS, EMITTERS, COORDINATES, PATTERNS, OPTIONS`. All other sections are preserved raw (`preserved_sections`).

| Section | Row | Parsed |
| --- | --- | --- |
| `[OPTIONS]` | `KEY value...` | Multi-word keys `MINIMUM PRESSURE, REQUIRED PRESSURE, PRESSURE EXPONENT, DEMAND MULTIPLIER, EMITTER EXPONENT, DEMAND MODEL, SPECIFIC GRAVITY, UNITS, HEADLOSS, PATTERN`. Other keys = first token. Defaults `UNITS GPM`, `HEADLOSS H-W` |
| `[JUNCTIONS]` | `ID Elev [Demand [Pattern]]` | ≥ 2 tokens, else `INVALID_JUNCTION_ROW` |
| `[RESERVOIRS]` | `ID Head [Pattern]` | |
| `[PIPES]` | `ID Node1 Node2 Length Diameter Roughness [MinorLoss [Status]]` | ≥ 6 tokens, else `INVALID_PIPE_ROW`. Status default `OPEN` |
| `[DEMANDS]` | `Junction Demand [Pattern [Category...]]` | |
| `[EMITTERS]` | `Junction Coefficient` | |
| `[COORDINATES]` | `Node X Y` | |
| `[PATTERNS]` | `ID mult mult ...` | Several lines append |

Unit conversion (`APP:neatwork/epanet/units.py`): the units are US units for `UNITS` ∈ {CFS, GPM, MGD, IMGD, AFD}. Length/elevation/head: ft × 0.3048. Pipe diameter: in × 0.0254 (US) or mm / 1000 (SI). Flow to L/s: CFS 28.316846592, GPM 0.0630901964, MGD 43.81263638888889, IMGD 52.61678240740741, AFD 14.2764101568, LPS 1, LPM 1/60, MLD 11.574074074074074, CMH 1/3.6, CMD 1/86.4. Pressure: psi × 0.703249614902 (US) or m. D-W roughness: millifeet × 0.0003048 or mm / 1000. Unknown `UNITS` → `ValueError("Unsupported EPANET flow units: X")`.

Mapping (`APP:neatwork/epanet/adapter.py`):

| EPANET | NeatWork | Notes |
| --- | --- | --- |
| Junction | node, statut 1, `elevation_m` converted, orifices 0. Extra `epanetBaseDemand`, `epanetPatternId` | A junction that receives a client becomes statut 2 |
| Reservoir | node, statut 0, `elevation_m = head` | Pattern → `TIME_VARYING_RESERVOIR_HEAD` |
| Coordinates | `x, y` (not converted) | Missing → (0, 0) |
| Pipe | pipe `edge_id = ID`, `pred = Node1`, `succ = Node2`, `lon = lon1 = Length` (m), `ID1` = catalogue ID, `lon2 = 0`. `enabled = status != CLOSED`. Extra `epanetMinorLoss` | `CV` → `CHECK_VALVE_NOT_SUPPORTED`. Other status → `UNKNOWN_PIPE_STATUS` |
| Diameter + roughness | catalogue row `EPD_<sha1("%.12g(diam_m)|%.12g(roughness)|HEADLOSS")[0:10] upper>`, `nominal_size = "<d> mm"` or `"<d> in"`, `sdr 0, cost 0, maximum_pressure 0, type_code "EPANET"`, `roughness` raw | for example `0.063|100|H-W` → `EPD_34F2A471AF` |
| Demand / junction base demand | Policy `PRESERVE_UNRESOLVED` (default): no client, issue `INTERMITTENT_PROBABILITY_UNRESOLVED`. Policy `DETERMINISTIC_ALWAYS_OPEN`: one client `EP_<node>_<nn>` (nn = 2-digit index per node) with behaviour `openProbability 1`, `targetFlowLps = demand × DEMAND MULTIPLIER` converted, `profileId = pattern` | Negative demand → `NEGATIVE_DEMAND_NOT_CLIENT`. Pattern → `TIME_PATTERN_NOT_MAPPED` |
| `REQUIRED PRESSURE` | borne `required_pressure_m` (converted) for every terminal | Missing → adapter default, else `TERMINAL_PRESSURE_UNRESOLVED` |
| Emitters | not mapped (`EMITTER_ORIFICE_MAPPING_UNRESOLVED`) | Kept in `technicalParams.epanet.emitters` |
| `[TANKS]`, `[PUMPS]`, `[VALVES]` with data | `UNSUPPORTED_TANKS/PUMPS/VALVES` | |
| H-W with one C, no minor loss | Optional (`enable_hw_param_mapping`, off by default): `lambda 1.852, nu 4.871, bescal = 10.667 × C^-1.852` | Several C → `VARIABLE_HW_ROUGHNESS_UNSUPPORTED` |
| `HEADLOSS D-W` / `C-M` / other | `DW_HEADLOSS_MODEL_NOT_VALIDATED`, `CM_HEADLOSS_MODEL_NOT_VALIDATED`, `UNKNOWN_HEADLOSS_MODEL` | |
| `DEMAND MODEL` | PDA/PDD needs `MINIMUM PRESSURE 0` (`PDA_MINIMUM_PRESSURE_UNSUPPORTED`) and `PRESSURE EXPONENT 0.5` (`PDA_PRESSURE_EXPONENT_UNSUPPORTED`). DDA (default) → `DDA_NOT_SIMULATION_EQUIVALENT`. Other → `UNKNOWN_DEMAND_MODEL` | |
| `[TITLE]` | `comments = [title]` | |
| everything | `sourceFormat "EPANET"`, `technicalParams.epanet = {units, headloss, options, patterns, emitters, preservedSections}` | |

Every issue has `code, severity (INFO|WARNING|BLOCKING), message, section, line_number`. `simulation_ready` is true only without a BLOCKING issue. Numeric parse errors give `INVALID_NUMBER` "Invalid numeric value for <field>: '<text>'". Note: every default EPANET file (DDA) is therefore "not simulation-ready".

### 8.3 Export (PROPOSAL for the Rust port)

Target: EPANET 2.2 `.inp`, SI units, readable by EPANET 2.2 and WNTR. The export is a deterministic snapshot of one design. It cannot carry NW5's stochastic demand. Rules:

1. `[TITLE]`: network name, then the `!Commentaires` lines.
2. `[JUNCTIONS]` `ID Elev Demand Pattern`: every statut-1 and statut-2 node, `Elev = haut` (m). Demand 0 here. Demands go to `[DEMANDS]`.
3. `[RESERVOIRS]` `ID Head`: every statut-0 node, `Head = haut + pump_pressure` (m).
4. `[PIPES]` `ID Node1 Node2 Length Diameter Roughness MinorLoss Status`: one pipe per segment of length > 0. A one-diameter arc gives the pipe `<pred>-<succ>`. For a name that is not a valid EPANET ID, use `A<n>`. An EPANET ID has at most 31 characters, no spaces and no `;`. A two-diameter arc gives two pipes and one intermediate junction `<pred>_<succ>_s`. Pipe `<id>#1` goes from pred to the junction (`lon1`, `diam(ID1)`). Pipe `<id>#2` goes from the junction to succ (`lon2`, `diam(ID2)`). The junction has demand 0 and an elevation interpolated linearly along the length. Diameter in mm (`diam × 1000`). Roughness = H-W C (rule 7). MinorLoss 0. Status `OPEN`, or `CLOSED` for an app pipe that is not open. Parallel pipes keep their own IDs (`<id>#R` for the reinforcement).
5. `[DEMANDS]` `Junction Demand Pattern ;Category`: one line per client, `Demand = cible` (L/s), category = client ID. For an always-open snapshot this sums to the borne demand. `proba` cannot be expressed. Write it as a comment `; p=<proba>`.
6. `[EMITTERS]`: only for `bounded_flow = 0`. The NW5 terminal loss is `h = alpha q²`, with h in m and q in m3/s. Here `alpha = (orifcoef/orif)^4 + 1/faucetcoef`, and the orifice is `orif_com` or `orif_ideal` per the `orif_com` parameter. This loss equals an EPANET emitter `q = K h^0.5` with `K = 1000 / sqrt(alpha)` in L/s/m^0.5 per open client. With n clients open at the borne, `K = n × 1000 / sqrt(alpha)`. Write `EMITTER EXPONENT 0.5` and no demand for these terminals (the emitter alone sets the flow). Example: `orif_com = 0.0044`, `orifcoef 0.59`, `faucetcoef 1.87e-8` → `alpha = 3.7677e8`, `K = 0.0515 L/s/m^0.5`.
7. `[OPTIONS]`: `UNITS LPS`, `HEADLOSS H-W`. NW5's law is `h = bescal L Q^lambda / D^nu` with `lambda = 1.781`, `nu = 4.781`. H-W uses 1.852 and 4.871. So the export is **approximate**, except for `lambda = 1.852` and `nu = 4.871`. For these exponents, `C = (10.667 / bescal)^(1/1.852)` is exact (inverse of the adapter rule). For the default `bescal = 8.9396e-4` this gives `C ≈ 158.95`. Write a `;` comment with the original `bescal, lambda, nu`.
8. Bounded flow (`bounded_flow = 1`): `DEMAND MODEL PDA`, `MINIMUM PRESSURE 0`, `REQUIRED PRESSURE <p>`, `PRESSURE EXPONENT 0.5`, which the import adapter and `APPT:reference_validation` treat as equivalent to NW5 bounded flow. EPANET 2.2 has one global required pressure: if the bornes have different pressures, write the maximum and add a warning. Unbounded flow: `DEMAND MODEL DDA` is wrong for emitter-only terminals. Keep DDA with zero demands at those terminals.
9. `[COORDINATES]`: `xdata, ydata` of every node. Intermediate nodes interpolated.
10. `[TIMES]` `DURATION 0`. `[REPORT]` `STATUS NO`. `[END]`.
11. Number format: shortest round-trip decimal, columns separated by 4 spaces or a tab. Sections in the order TITLE, JUNCTIONS, RESERVOIRS, PIPES, EMITTERS, DEMANDS, OPTIONS, TIMES, REPORT, COORDINATES, END.

Example: the reference case of `APPT:reference_validation/neatwork_fixture.py` (bounded flow, one H-W C = 140, 1 L/s at P1, 20 m required). Its original `reference_pda_hw.inp` is missing. This text is a reconstruction (UNVERIFIED against the original file):

```
[TITLE]
reference_pda_hw

[JUNCTIONS]
;ID   Elev   Demand   Pattern
J1    0      0
P1    0      0

[RESERVOIRS]
;ID   Head
S1    12

[PIPES]
;ID   Node1   Node2   Length   Diameter   Roughness   MinorLoss   Status
E1    S1      J1      100      63         140         0           Open
E2    J1      P1      100      63         140         0           Open

[DEMANDS]
;Junction   Demand   Pattern   ;Category
P1          1                  ;C1 p=1

[OPTIONS]
UNITS               LPS
HEADLOSS            H-W
DEMAND MODEL        PDA
MINIMUM PRESSURE    0
REQUIRED PRESSURE   20
PRESSURE EXPONENT   0.5

[TIMES]
DURATION 0

[COORDINATES]
S1   0   0
J1   1   0
P1   2   0

[END]
```

The matching NeatWork domain: `lambda 1.852, nu 4.871, bescal = 10.667 × 140^-1.852 = 1.13087e-3, bounded_flow 1, uniform_dem 1, opentaps 1, targetflow 1, nbsim 1`. Node S1 statut 0 elevation 12, J1 statut 1, P1 statut 2. Pipes E1, E2 of 100 m on D63 (`innerDiameter 0.063`). Client C1 at P1 (p = 1, 1 L/s). Borne P1 at 20 m.

Example of a two-diameter arc of `Toy_bbd`: `A,B,500,38,D009,462,D004,N`. D009 = 0.0453 m and D004 = 0.0304 m. A is at −10 m and B at −15 m. The export is:

```
[JUNCTIONS]
A_B_s   -10.38   0
[PIPES]
A-B#1   A       A_B_s   38    45.3   158.95   0   Open
A-B#2   A_B_s   B       462   30.4   158.95   0   Open
```

## 9. Cruscade Txt format (`*Dsg*.txt`)

A design split into five text files, used to exchange designs with the AMPL stochastic-design work. "Ce format n'est pas destiné à perdurer" (`M:InOut/LireTablesDsgTxt.m:2`). Reader: `LireTablesDsgTxt(nom, version)`, called by `readDesign(..., type = 'txt')` (`M:InOut/readDesign.m:107`). No writer exists.

Files: folder `<root>/examples/<nom>Txt/` (`:22`), files `<nom><version><base>.txt` with `base` ∈ `DsgArcs, DsgClients, DsgNodes, DsgDbase, DsgParam`. The examples are actually under `examples/tpo_examples/<nom>Txt/`, so the MATLAB path does not find them DEFECT (UNVERIFIED). Rust: take the five paths, or a folder and a prefix.

Lexical: first line is a header with column names. Delimiter per file: tab if the first line has at least as many tabs as commas, else comma. Tab if it has neither (`:195`). Read with `detectImportOptions`/`readtable` (`TextType string`), so quoted fields follow `readtable` rules UNVERIFIED (`"1/2"""` probably becomes `1/2"`). Columns are matched **by name**, order free. Extra columns allowed. A column named `N` is dropped (`:119`).

| File | Required columns | Text columns | Example columns | Units |
| --- | --- | --- | --- | --- |
| `DsgArcs` | `pred, succ, lon, lon1, lon2` and either `diam1, diam2` or `ID1, ID2` ("DsgArcs: il faut (diam1,diam2) ou (ID1,ID2) dans %s.", "DsgArcs: colonnes requises manquantes dans %s: %s") | `pred, succ, ID1, ID2` | Cruscade: `pred succ lon lon1 diam1 lon2 diam2`. Toy: `pred succ lon lon1 ID1 lon2 ID2 N` | m. `diam*` in m, `0` = no segment |
| `DsgClients` | `borne, client, proba, cible` | `borne, client` | Cruscade: `client borne cible pression proba`. Toy: `client,borne,proba,cible` | `cible`: m3/s in Cruscade (0.001045093), L/s in Toy (0.12) DEFECT, see below. `pression` m (ignored) |
| `DsgNodes` | `noeud, xdata, ydata, orif_ideal, orif_com, statut` (and `haut`, used later) | `noeud` | `noeud haut xdata ydata orif_ideal orif_com statut demande` | m. `demande` m3/s, dropped |
| `DsgDbase` | `ID, diam, cost` | `ID` | Cruscade: `ID materiau diam cost`. Toy: `ID Nominal SDR diam cost Pmax Type Roughness` | m, currency/m |
| `DsgParam` | `property, value` | `property` | `property,value` (comma) | as 2.7 |

Error text for a missing column: "%s: colonnes requises manquantes dans %s: %s" (`:113`, the first `%s` receives the list of required names, a formatting DEFECT).

Node status codes are **not** 0/1/2. The file uses three distinct codes (examples: source 1000, intermediate 0, terminal 1 or 3). Rule (`:157` to `:168`): fewer than 3 distinct values → error "DsgNodes.statut doit contenir au moins 3 catégories : source, nœuds intermédiaires, nœuds terminaux. Trouvé: [...]". If the set is not exactly {0, 1, 2}: the minimum becomes 1 (intermediate), values strictly between min and max become 2 (terminal), the maximum becomes 0 (source).

After reading (`M:InOut/readDesign.m:111` to `:120`): `TA = DsgArcs`, `TN = DsgNodes` without `demande`, renamed to `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut` **by position**, `TC = DsgClients`, `TD = DsgDbase`, `TP = DsgParam`. `param` = TP values as given (number or string). `uniform_dem` and `bounded_flow` derived from TC (2.3). There is no TB. The simulation builds one from `param.pressure` UNVERIFIED. When arcs give `diam1/diam2`, the simulation uses them directly and takes the cheapest catalogue cost per diameter (`M:Simul/completeTables.m:79` to `:93`).

Units defect: the Cruscade client `cible` is in m3/s while every other NW5 table uses L/s. Nothing converts it, so the MATLAB simulation of `CruscadeDet2406` uses targets 1000 times too small UNVERIFIED. Rust: detect by magnitude is unsafe. Take an explicit unit option, default L/s, and document that Cruscade Txt needs m3/s.

Cruscade in `.tpo` form (`EX:tpo_examples/Cruscade/Cruscade.tpo`, `CruscadeFull.tpo`): headers with trailing commas, numeric node names, `pipelength 0.1`, `fusion 1`. Its `!Clients` rows are `client, borne, <pressure>, <proba>` (`c1B1,1,40,0.8`, compare `CruscadeDet2406DsgClients.txt`: `cible 0.001045093, pression 40, proba 0.8`), so they do **not** follow the `client, borne, proba, cible` layout: read normally, `proba = 40` is invalid DEFECT (data error in the example). The app reads 2 behaviours with `openProbability 40` (verified).

## 10. Spreadsheets and CSV

### 10.1 xlsx inputs

No NW5 code reads an xlsx file, and no example input is an xlsx. (`ToyXlx` is a `.tpo`/`.dsg` example. Only its name recalls an Excel origin.) Rust: no xlsx reader.

### 10.2 xlsx outputs (simulation statistics)

| Producer | File | Sheets |
| --- | --- | --- |
| MATLAB `stat_processing` | `simul_debit.xlsx` in the current folder (`M:Simul/stat_processing.m:200`) | `Sheet1`: TableSat at B2 with first column `Bornes` (`minSat, moySat, maxSat, cVarSat5, cVarSat20`). `Sheet2`: TableSt (`minSt, moySt, maxSt, cVarSt5, cVarSt95`, row = source). `Sheet3`: TableResV (`debut, fin, minV, moyV, maxV`). `Sheet4`: TablePress (`minP, moyP, maxP`, row = borne). `Sheet5`: TablePrB (`minPrB, moyPrB, maxPrB`, row = transit node). Table at B2 (row 1 and column A empty) |
| `PY:simulation/simulation.py:443` | `simul_debit_<name>_<version>.xlsx` | Same five sheets (verified on `PY:test_examples/simul_debit_Toy_bbd.xlsx`) |
| App `SimulationSpreadsheetExporter` | `simulation_statistics.xlsx` | `Summary` (Property/Value rows: `Schema version, Network, Network version, Scenario count, Successful scenarios, Execution failures, Success rate, Seed, Duration (s), Mean iterations`), `Terminal Satisfaction`, `Source Flow`, `Pipe Velocity`, `Terminal Pressure`, `Transit Pressure`, `Failures`. Each sheet = JSON record keys as header row, or a single cell `No data`. A typo makes it fail on every record (`isinstance(value(dict,list,tuple))`, `APP:neatwork/simulation/spreadsheet_exporter.py:87`) DEFECT |

### 10.3 CSV

`export_neatwork_csv` despite its name writes the `.dsg` file (section 4). The only CSV named in NW5 is `<nomv>_TableFusion.csv` (columns of TableFusion, 2.9), referenced by the run meta but never written. Rust: no CSV format to support. A `--fusion-trace` CSV can be added later with the TableFusion columns (PROPOSAL).

## 11. Catalogue files (`.db`)

### 11.1 Diameters

One pipe per line, 7 fields, no ID: `Nominal, SDR, diam, cost, pressure, type, Roughness` (same columns as NW4 `diameters.db`).

Files in the examples (verified):

| File | Rows | Particularities |
| --- | --- | --- |
| `Ceiba/DiamBaseCeiba.db` | 34 | Pressure ×10 (`4260`), no final newline |
| `Cruscade/DiamBaseCruscade.db` | 13 | SDR is text (`PEHD`, `PVC-BO`, `PVC BO`, `Fonte`), nominal in mm (`75`) |
| `Red 1/DiamBaseRed 1.db` | 35 | **Blank first line** |
| `Red 1/diameters.db`, `Red 2/diametersBof.db` | 8 | Identical. Nominal `A`..`H` |
| `Red 2/DiamBaseRed 2.db`, `Red 3/DiamBaseRed 3.db` | 36, 37 | 2018 costs |
| `Toy/DiamBaseToy.db` | 15 | Used by the Toy `.tpo` examples |
| `Toy/diameters.db`, `ToyXlx/diameters.db` | 36 | Identical. First line `!Database new costs 2018` (a header-like title line) |
| `ToyXlx/DiamBaseToyXlx.db` | 7 | Leading space in ` 1/2"` |

Rules:

- NW5 looks for `DiamBase<nom>.db` (MATLAB, Python research) in the network folder. The app takes any uploaded file. `diameters.db`/`diametersBof.db` are not read by any NW5 code (left over from NW4 projects).
- A line starting with `!` is a title, not data (the app drops it because it has fewer than 7 fields, MATLAB misreads it UNVERIFIED). Blank lines are skipped.
- Nominal keeps leading/trailing spaces in NW4 files (` 1/2"`, `1    "`). The app trims them. Rust: trim for display, keep raw for writing back to `.db`.
- Sorting and IDs: section 2.5.

Differences with NW4 `diameters.db` (`crates/neatwork-core/resources/diameters.db`): same 7-field layout and units. NW4 ships one global catalogue (2024 costs, for example `1/2",13.5,0.0182,1.05,176.8,...`). NW5 uses one catalogue per network and can embed it in the `.tpo`/`.dsg`. It adds IDs (`D001`) in `.dsg` and accepts text in the SDR column.

### 11.2 Orifices

One diameter (m) per line, one field. `Toy`, `Red *`, `Cruscade`, `ToyXlx`: 11 values `0.0017 ... 0.0056`. `Ceiba/orifices.db`: 13 values including the NW4 bounds `1.0E-6` and `1.0`. In a `.tpo` `!Orifices` section a row is `diameter,N`. In a `.dsg` it is `diameter,N,N,N,N,N`. NW4's bundled `orifices.db` (`0.001, 0.002, ...`) is a different list. Rust: section 2.6 rules.

## 12. Round-trip and compatibility rules

### 12.1 What each writer preserves

| Writer | Preserves | Changes |
| --- | --- | --- |
| MATLAB `data4DsgFile` → `.dsg` | Elevations and coordinates unchanged (`T:test_physical_elevations.m`) | Regenerates every section from tables. Drops `!Commentaires` and unknown sections. Sorts nodes. Writes `fictif` IDs and the `fictif` catalogue row. `!loadfactor` = flows and margins |
| App revision serializer (`APP:neatwork/serialization/revision_serializer.py:249`) | Preamble, unknown sections (for example `!loadfactor`, `!Hydraulics`), section order and original header text of unknown sections | Owned sections (`Commentaires, Nodes, Pipes, Default properties, Diameters, Orifices, Clients, Borne`) are rewritten under canonical headers. A missing owned section that has data is appended at the end. Trailing spaces of comment lines removed. Output ends with one `\n` |
| `maj_NW4_NW5` | Most NW4 data | See 5.2 |

App DSG round-trip of `Toy_bbd_Secu1.00_fusion0_split0.dsg` (verified): `!Default properties` gains the default keys `bounded-flow,0`, `serviceequal,0.65`, `nbsim,100`, `orif_com,1`. A source without `lambda` or `limitbudget` also gains `lambda,0` and `limitbudget,inf`. The default keys come first. MATLAB then fails on the field name `bounded-flow` UNVERIFIED, and NW4 sees unknown keys. App TPO revisions write 7-field `!Diameters` and `in_diam,N` orifices, and drop non-default parameters. They also fail their own validation: `comments = null` (verified), and `MISSING_PIPE_DIAMETER` for every bare TPO pipe UNVERIFIED. So a TPO revision cannot be saved today DEFECT.

### 12.2 Compatibility matrix

| File | NW4 Java | MATLAB NW5 | App |
| --- | --- | --- | --- |
| NW4 `.tpo` | yes | yes (DB files from the folder) | yes (DB upload required) |
| NW5 `.tpo` (`!Clients`, `!Borne`, embedded catalogues) | yes, ignores new sections. Uses `nbrob` | yes (8-field `!Diameters`) | yes (7-field `!Diameters`) |
| NW4 `.dsg` | yes | no ("ne contient pas de section Clients") until `maj_NW4_NW5` | yes, with warning. Not simulatable (no clients, outlet bornes, `lambda 0`) |
| NW5 `.dsg` | yes, but sees fictitious-free multi-client terminals as single taps, `!loadfactor` can hold flows, `ID2 fictif` unknown to NW4 UNVERIFIED | yes | yes (9-field `!Diameters` only) |
| `NW-json-v1` | no | yes | no |
| Txt (Cruscade) | no | yes (path defect) | no |
| `.inp` | no | no | import prototype only |

### 12.3 Normative Rust policy

Reading (all formats): be lenient as described in 1.2-1.4, 3.2, 4.2-4.4. Keep raw text of unknown sections and of every original line for round-trip. Derive `nbrob`, `uniform_dem`, `opentaps`, `targetflow`, `bounded_flow` from TC exactly as `apply_client_data`. Drop a `fictif` catalogue row. Map `ID = 0/N/-/empty` with zero length to "no pipe".

Writing `.dsg` (NW5 layout, PROPOSAL fixed here):

1. Section order and columns of 4.1: `!Nodes` … `!Borne`. After a design, add `!Hydraulics`, `!ComplementTA`, `!Costs`, `!ConstraintSatisfaction`. For a network with comments, write `!Commentaires` first.
2. Numbers with the semantics of MATLAB `num2str(x,'%.15g')`: 15 significant digits, no trailing zeros. The exponent form (`e-08`, `e+15`) is used for an exponent < −4 or ≥ 15. `Inf` → `Inf`.
3. `!Pipes` 8 fields. For `lon2 = 0`, write `ID2 = fictif`, the current NW5 MATLAB form. NW4 compatibility of `fictif` is UNVERIFIED (open question).
4. `!Diameters` 9 fields with trailing `N`, real pipes only, IDs kept.
5. `!Orifices` 6 fields.
6. `!Default properties`: the network's parameters with canonical names only (never `bounded-flow`, `serviceequal`), design parameters alphabetically then technical ones, as MATLAB.
7. `!Clients` sorted by `(borne, client)`. `!Borne` with `N`.
8. LF line ends, final newline.

Round-trip test (Rust): for every example `.dsg` and `.tpo`, `read → write → read` must give equal tables. IDs must be equal strings and numbers equal within 1e-12 relative. Unknown sections must come back byte-identical.

## 13. Error and validation messages

### 13.1 MATLAB

| Identifier / text | Where | Condition |
| --- | --- | --- |
| `Fichier introuvable : %s` / `Introuvable: %s` / `Impossible d'ouvrir : %s` / `Ouverture impossible : %s` | readers | Missing or unreadable file |
| `Sections manquantes dans le .tpo (attendu: !Nodes, !Pipes, !Default properties).` | `parseTpoToTables.m:46` | |
| `Ordre des sections inattendu dans le .tpo.` | `:49` | |
| `Les bornes ne correspondent pas aux noeuds terminaux` | `parseTpoToTables.m:199`, `parseDsgToTables.m:113` | `!Borne` set ≠ terminal set |
| `Le fichier ne contient pas de section Clients. Mettre à jour le fichier NW4 avec l'utilitaire.` | `parseDsgToTables.m:43` | `.dsg` without `!Clients` |
| `NW5:ClientNode` `Client terminals are absent from TN: %s` | `apply_client_data.m:9` | |
| warning `NW5:BoundedFlowRequired` | `apply_client_data.m:26` | |
| `this type of file is not taken into consideration` | `readDesign.m:123` | type not in mat/dsg/json/txt |
| `IDs contiennent des valeurs manquantes (%d).` | `utils/normalize_id_column.m` | empty ID |
| `NW:DuplicateNodeIds`, `Noeuds inconnus dans TA: ...`, `Noeuds présents dans TN mais absents de TA: ...`, `Bornes de TB absentes de TN=... ou de TA (pred/succ)=...`, warning `Arc boucle (self-loop) détecté sur %s (ligne %d).` | `validate_and_align_Tables.m` | |
| `Il doit y avoir exactement une source avec TN.statut == 0 (trouvé: %d).` | `peel_orient_tree.m:45` | |
| `NW5:InvalidDesignTopology` `Conception interrompue : %s` | `makeReinfDsg.m:104` | not a tree, parallel arcs or source degree ≠ 1 |
| `NW5:Source` `Une source unique est requise.` | `makeReinfDsg.m`, `apply_pump_pressure.m` | |
| `NW5:Catalogue` (`TD contient a la fois Pmax et pressure.`, `Le catalogue TD ne contient pas toutes les variables requises.`, `Le catalogue TD est vide.`, `TD contient un ID vide ou manquant.`, `ID de tuyaux non uniques.`, `Les tuyaux réels doivent avoir un diamètre strictement positif et fini.`) | `prepareReinforcementTables.m` | |
| `NW5:MissingEquipment`, `NW5:EquipmentLength`, `NW5:EquipmentID`, `NW5:UnknownPipe`, `NW5:EquipmentStatus` (`TA.N doit valoir "N" (nouveau), "B" (bloque) ou "R" (renforce).`, `Un arc sans conduite reelle ne peut etre bloque.`) | `prepareReinforcementTables.m` | |
| `NW5:ExistingPressure` `Dimensionnement interrompu : pression hydrostatique incompatible avec l'existant.` + per-section detail | `validate_existing_pressure.m` | |
| `NW5:OutputFormat` `Only DSG and JSON exports are supported.` | `data4DsgFile.m:35` | |
| `NW5:OutputDirectory` | `data4DsgFile.m` | mkdir failed |
| Txt: `DsgArcs: il faut (diam1,diam2) ou (ID1,ID2) dans %s.`, `DsgArcs: colonnes requises manquantes dans %s: %s`, `DsgNodes.statut doit contenir au moins 3 catégories ...` | `LireTablesDsgTxt.m` | |
| `maj_NW4_NW5`: `Doublons dans TC.ID ...`, `Chaque client doit avoir exactement une 'borne' non vide.`, `Borne(s) non terminales ou inconnues: %s`, `Bloc NW4 invalide: aucune colonne après le nom (pas de virgule ?)` | `maj_NW4_NW5.m` | |

### 13.2 App parsers and backend

| Text | Where |
| --- | --- |
| `TPO import failed: missing !Diameters and diameters.db.` / `unknown pipe nodes [...]` / `unknown bornes [...]` / `self-loop detected on node X` | `APP:neatwork/parsers/tpo_parser.py` (HTTP 400) |
| warnings `TPO has no !Orifices and no orifices.db was provided.`, `TPO has no !Clients section. ...`, `TPO has no !Borne section. ...` | same |
| `DSG import failed: missing !Nodes section` / `... !Pipes section`. Warning `DSG has no !Clients section. Simulation requires client demand data before running.` | `APP:neatwork/parsers/dsg_parser.py` |
| `Unsupported format: X`, `Only DSG and TPO revisions are supported.` | import service, serializer |
| `Client 'X' has neither behaviorId nor complete legacy demand values.`, `Unknown client behavior 'X'.`, `Client 'X' references unknown behavior 'Y'.`, `Client 'X' has no usable demand behaviour.`, `Client demand table is empty.`, `Client opening probability must be between 0 and 1.`, `Client target flow must be strictly positive.` | `APP:neatwork/domain/` |
| `Open probability must be between 0 and 1.`, `Target flow must not be negative.`, `<Node/Pipe endpoint/Client/Borne endpoint> identifier cannot be None./empty.` | `APP:schemas/simulation_domain.py` |
| Backend: `Unsupported network input format.`, `Only DSG and TPO can be imported as primary network sources.`, `The selected artifact does not match the requested input format.`, `The selected primary artifact is empty`, `TPO import requires a diameters database. The selected TPO file has no !Diameters section. Upload diameters.db as DB_DIAMETERS_INPUT and select it in Domain import.`, `Invalid pipe diameter reference: Segment 1 of A → B uses 'D9', but that diameter does not exist in the pipe diameter catalog.`, `Duplicate client behavior ID: X.`, `Client behavior X has an invalid opening probability./target flow.`, `This legacy snapshot has no source artifact. Re-import the network before editing it.`, `Revision format does not match project format.` | `BE:service/` |

The backend `hasSection` test for `!Diameters` is an exact, case-insensitive match of the trimmed line (`BE:service/ProjectDomainImportService.java:171`), so `!Diameters,` is not found.

### 13.3 App simulation validation codes

`APP:neatwork/simulation/validation.py` collects all issues, then raises `SimulationValidationError` `Simulation validation failed with N issue(s): ...` (HTTP 422, `detail = {code, message, issues...}`). Each issue: `code, message, entity_type, entity_id, field_name, details`.

| Code | Rule |
| --- | --- |
| `MISSING_TABLE`, `INVALID_TABLE_TYPE`, `MISSING_TABLE_COLUMN` | Required columns: nodes `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut`. Pipes `external_edge_id, pred, succ, lon, lon1, ID1, lon2, ID2`. Clients `client, borne, proba, cible`. Bornes `borne, pressure`. Diameters `ID, diam, cost, Pmax, Type, Roughness` |
| `EMPTY_NODE_TABLE`, `EMPTY_NODE_ID`, `DUPLICATE_NODE_ID`, `INVALID_NODE_NUMBER` (non-finite haut/x/y/orifices), `INVALID_NODE_STATUS`, `UNSUPPORTED_NODE_STATUS` (not 0/1/2), `MISSING_SOURCE_NODE`, `MISSING_TERMINAL_NODE` | nodes |
| `EMPTY_PIPE_TABLE`, `DUPLICATE_PIPE_ID`, `EMPTY_PIPE_ID`, `UNKNOWN_PIPE_NODE`, `PIPE_SELF_LOOP` | pipes: identity and endpoints |
| `INVALID_PIPE_LENGTH`, `NEGATIVE_PIPE_LENGTH`, `ZERO_PIPE_LENGTH` (`lon1 + lon2 ≤ 0`), `PIPE_LENGTH_MISMATCH` | pipes: lengths |
| `MISSING_PIPE_DIAMETER` (`length > 0` without ID), `UNKNOWN_PIPE_DIAMETER` | pipes: diameters |
| `EMPTY_CLIENT_TABLE`, `DUPLICATE_CLIENT_ID`, `EMPTY_CLIENT_ID`, `UNKNOWN_CLIENT_TERMINAL`, `INVALID_CLIENT_PROBABILITY`, `CLIENT_PROBABILITY_OUT_OF_RANGE`, `INVALID_CLIENT_TARGET_FLOW` (≤ 0), `TERMINAL_WITHOUT_CLIENT`, `NO_ACTIVE_CLIENT_PROBABILITY` | clients |
| `EMPTY_BORNE_TABLE`, `DUPLICATE_BORNE_ID`, `MISSING_TERMINAL_BORNE`, `UNKNOWN_BORNE_TERMINAL`, `INVALID_BORNE_PRESSURE` (< 0 or non-finite) | bornes |
| `EMPTY_DIAMETER_TABLE`, `DUPLICATE_DIAMETER_ID`, `INVALID_DIAMETER_VALUE` (`diam > 0`, `cost ≥ 0`, `Pmax ≥ 0`, `Roughness ≥ 0`, all finite) | diameters |
| `MISSING_SIMULATION_PARAMETER` | parameters: `bescal, bounded_flow, faucetcoef, lambda, nbsim, nu, opentaps, orif_com, orifcoef, pressure, servicequal, targetflow, uniform_dem, vmax, vmax_secu` must exist |
| `INVALID_BINARY_PARAMETER`, `INVALID_SCENARIO_COUNT` | parameters: flags are 0/1, `nbsim` is a positive integer |
| `INVALID_POSITIVE_PARAMETER`, `INVALID_NON_NEGATIVE_PARAMETER`, `PARAMETER_OUT_OF_RANGE` | parameters: `bescal, lambda, nu, orifcoef, vmax, vmax_secu` > 0. `faucetcoef, pressure` ≥ 0. `opentaps, servicequal` in [0, 1] |
| `UNSUPPORTED_SIMULATION_CASE`, `INVALID_TARGET_FLOW_PARAMETER`, `INVALID_TERMINAL_ORIFICE` | parameters: case D refused. Unbounded flow needs `targetflow > 0` and terminal orifices |
| `DIRECT_SOURCE_TO_TERMINAL_PIPE`, `SOURCE_HAS_INCOMING_PIPE`, `SOURCE_WITHOUT_OUTGOING_PIPE`, `TERMINAL_HAS_OUTGOING_PIPE`, `INVALID_TERMINAL_IN_DEGREE` (≠ 1), `NODE_UNREACHABLE_FROM_SOURCE` | topology |
| `INVALID_PARAMETER_CONTAINER` | params not a dict |

### 13.4 EPANET import codes

See 8.2: `INVALID_NUMBER, INVALID_JUNCTION_ROW, INVALID_PIPE_ROW, VARIABLE_HW_ROUGHNESS_UNSUPPORTED, DW_HEADLOSS_MODEL_NOT_VALIDATED, CM_HEADLOSS_MODEL_NOT_VALIDATED, UNKNOWN_HEADLOSS_MODEL, PDA_MINIMUM_PRESSURE_UNSUPPORTED, PDA_PRESSURE_EXPONENT_UNSUPPORTED, DDA_NOT_SIMULATION_EQUIVALENT, UNKNOWN_DEMAND_MODEL, TIME_VARYING_RESERVOIR_HEAD, UNKNOWN_PIPE_STATUS, CHECK_VALVE_NOT_SUPPORTED, EPANET_MINOR_LOSS_UNSUPPORTED, INTERMITTENT_PROBABILITY_UNRESOLVED, NEGATIVE_DEMAND_NOT_CLIENT, TIME_PATTERN_NOT_MAPPED, TERMINAL_PRESSURE_UNRESOLVED, EMITTER_ORIFICE_MAPPING_UNRESOLVED, UNSUPPORTED_TANKS, UNSUPPORTED_PUMPS, UNSUPPORTED_VALVES` (all BLOCKING).

## 14. Example files (`EX:`)

Parse results: "app" = python-core parser outcome (verified 2026-11-20). "simulatable" = app typed validation outcome where tested.

### 14.1 `tpo_examples/`

| File | Content and what it exercises |
| --- | --- |
| `Toy/Toy_bbd.tpo` | 8 nodes, 7 pipes. `!Commentaires`, embedded 7-field `!Diameters` with `"1/2"""` quoting, 2-field `!Orifices`, `!Clients` with non-uniform demand (16 clients, P2 has `nbrob 3` but 12 clients: TC overrides counts). No `bounded_flow` key. No final newline. App: 3 behaviours |
| `Toy/Toy_bbu.tpo`, `Toy/Toy_blu.tpo` | Same network, uniform clients built from `nbrob` (7 clients). Identical except comments: the "b"/"l" (bounded/free) choice is not in the file (chosen at review) |
| `Toy/DiamBaseToy.db` | 15 pipes, Toy catalogue |
| `Toy/diameters.db` | 36 pipes with title line `!Database new costs 2018` (unused by NW5) |
| `Toy/orifices.db` | 11 orifices |
| `Toy/README.md` | Naming scheme `Toy_{x1}{x2}{x3}` |
| `ToyXlx/ToyXlx.tpo`, `ToyXlx/ToyXlxV6b.tpo` | NW4 `.tpo`, 8 nodes. Differ by `nbrob` (P2 1→2, P3 1→3) and parameter order. Catalogue from `DiamBaseToyXlx.db` (7 rows, a space before the nominal) |
| `ToyXlx/diameters.db`, `ToyXlx/orifices.db` | Copies of Toy files |
| `Red 1/Red 1b.tpo` | 18 nodes, real coordinates, **source elevation −1**, `!Clients` (8) |
| `Red 1/DiamBaseRed 1.db` | Blank first line |
| `Red 1/diameters.db`, `Red 1/orifices.db` | Short catalogue `A`..`H` (unused), standard orifices |
| `Red 2/Red 2.tpo` | 31 nodes, NW4 `.tpo`, 0.1 m source pipe |
| `Red 2/Red 2b.tpo` | Same + `!Clients` (10), one client at p = 0.40001 (2 behaviours: tests exact equality of probabilities) |
| `Red 2/Red 2_Secu1.00_fusion0_split0.dsg` | Older NW5 `.dsg` (Clients before Default properties, duplicated `!Default properties` and `!loadfactor` headers, older `!loadfactor`, no `!Borne`) |
| `Red 2/DiamBaseRed 2.db`, `diametersBof.db`, `orifices.db` | Catalogues |
| `Red 3/Red 3.tpo`, `Red 3/Red 3b.tpo` | 430 nodes, 176 terminals, source named `272A`. `3b` adds `!Clients` (duplicated header) with p = 0.1, 0.4 L/s overriding the defaults 0.2/0.2 |
| `Red 3/DiamBaseRed 3.db`, `orifices.db` | Catalogues |
| `Ceiba/Ceiba.tpo`, `Ceiba/Ceiba BUll.tpo` | 397 nodes, numeric node names (`150`), 222 terminals. The two files are identical. File name with a space |
| `Ceiba/DiamBaseCeiba.db`, `Ceiba/orifices.db` | Pressure ×10. Orifices with NW4 bounds `1.0E-6`, `1.0` |
| `Cruscade/Cruscade.tpo` | 123 nodes, headers with trailing commas (app finds no node), duplicated `!Clients`, clients in `client, borne, pressure, proba` order, `fusion 1`, `pipelength 0.1`, numeric node names |
| `Cruscade/CruscadeFull.tpo` | Same + `!Borne` (55 rows, 40 m). App error "unknown bornes" because its nodes were not found |
| `Cruscade/DiamBaseCruscade.db`, `orifices.db` | Text SDR (material) |
| `CruscadeTxt/CruscadeDet2406Dsg{Arcs,Clients,Dbase,Nodes,Param}.txt` | Txt design: tab-separated (Param comma), `diam1/diam2` instead of IDs, status codes 1000/0/3, `cible` in m3/s, 358 clients with pressure column |
| `ToyTxt/Toy2bDsg*.txt`, `ToyTxt/Toy3bDsg*.txt` | Txt design of Toy (2000 m pipes): ID-based arcs with `N`, status codes 1000/0/1, `orif_ideal 10` sentinel, quoted nominal, Dbase with full NW columns. `3b` has a non-uniform client (C1P1 p = 0.4, 0.15 L/s) |
| `README.md` | Mentions `.inp` exports that no code produces |

### 14.2 `dsg_examples/`

| File | Content and what it exercises |
| --- | --- |
| `Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg` | Current-style NW5 output, non-uniform clients, bounded flow, `ID2 = 0`, older `!loadfactor`, comments with invalid UTF-8. App simulatable (verified). Reference for `APPT:fixtures/reference/toy_bbd_seed42.json` |
| `Toy/Toy_bbu_Secu1.00_fusion0_split0.dsg` | Uniform clients, bounded flow. Reference `toy_bbu_seed42.json` |
| `Toy/Toy_blu_Secu1.00_fusion0_split0.dsg` | Uniform clients, free flow (`bounded_flow 0`), real orifices only at P1, `1` sentinels elsewhere. Reference `toy_blu_seed42.json` |
| `Toy/Toy_Secu1.00_fusion0_split0.dsg` | Uniform design (7 clients) |
| `Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg` | Same network with 4 clients and per-borne pressure (P4 = 3 m) |
| `Toy/ToyDevReinf.dsg` | Reinforcement input: `fictif` IDs, bare arc (`lon1 = lon2 = 0`), blocked arc `B`, NW4 `!loadfactor`, 8-field `!Diameters` (app drops them), parameters without `lambda` (app: `lambda 0`). App validation fails (verified) |
| `Toy/Toy2b_NW4.dsg` | NW4 output with fictitious faucets `P2_a/b/c`, `P3_a/b`, `10000` sentinel, NW4 orifice bounds, unsorted `!Diameters`. Input of `maj_NW4_NW5` |
| `Toy/ToyXlx.dsg` | NW4 output, empty `!Orifices` section, no clients, `10000` sentinels. App builds outlet bornes (typed validation fails, verified) |
| `Ceiba/CeibaNW4.dsg` | Large NW4 output (397 nodes) |
| `Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg` | Its NW5 form: duplicated `!loadfactor` header, 222 clients and bornes, `1e-06` printed by `%.15g`. App simulatable (verified) |
| `Esperenza/Esperenza2S.dsg` | **Meshed, two sources** (S1, S2). 5 bornes at 3 m, 20 clients at 2 L/s, roughness 2, all orifices 0.005. No `!loadfactor`, Borne before Clients, no final newline. App simulatable (verified) |
| `Esperenza/Esperenza2S copy.dsg` | Same but `orif_ideal = 1` on terminals and P3 `orif_com = 1` (tests ideal vs commercial orifice choice) |
| `Red 1/Red 1.mat` | MATLAB v7.3-style MAT file (tables as opaque MCOS objects, only `pardsg`/`partec` structs are readable without MATLAB). Out of scope for Rust (MAT export was removed, `M:InOut/README.md`) |
| `README.md` | Describes `.dsg`, `.mat`, `.json` (no JSON example exists) |

## 15. Open questions and risks for the port

1. **Three readers, three behaviours.** The Rust reader follows the lenient union (1.2) and MATLAB semantics for derived values. Decide whether to emit warnings where MATLAB fails (section-order check, 8-field TPO diameters).
2. **`ID2 = fictif` vs `0`.** Current MATLAB writes `fictif`. Examples and NW4 use `0`. Writing `fictif` can break NW4 reading of NW5 files UNVERIFIED. Check `crates/neatwork-core/src/project/design.rs`: the NW4 port probably looks up the ID only for a length > 0.
3. **Default `pressure`** differs: 2 (MATLAB readers, app), 0 (`maj_NW4_NW5`). Default `bounded_flow` for a `.dsg` without the key: 1 (`readDesign`) vs 0 (app, `extract_parameters`). The port must choose one per format and record it in an ADR.
4. **`!loadfactor` semantics changed** in the current MATLAB writer (flows, margins). NW4 will misread new files.
5. **`fictif` catalogue row** in current MATLAB output breaks the app simulation. Rust drops it on read and never writes it.
6. **App defaults with typos** (`bounded-flow`, `serviceequal`, `lambda 0`) leak into revisions and break MATLAB and NW4-derived files. Rust must not reproduce them.
7. **Identifier typing**: the app turns numeric names into numbers. Ceiba and Cruscade have numeric names. Rust keeps strings.
8. **Snapshot fingerprints** ignore edges and terminals for imported data (7.7). Porting the comparison faithfully keeps these weaknesses.
9. **EPANET export** does not exist in NW5. 8.3 is a design whose exactness holds only for H-W exponents and PDA bounded flow.
10. **Cruscade data errors**: the unit of `cible` in Txt, the column order of `!Clients` in `.tpo`, and the header commas. Until somebody corrects them, or the reader gets explicit unit and layout options, the Cruscade examples are not usable as regression references.
11. MATLAB `jsonencode` and `writetable` number formatting, `textscan` behaviour on blank lines and misaligned rows are UNVERIFIED without MATLAB.
