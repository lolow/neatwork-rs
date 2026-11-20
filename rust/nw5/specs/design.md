# NW5 design (`makeReinfDsg`): porting specification

Status: draft for phase 5 of [../PLAN.md](../PLAN.md), written 2026-11-20.

Source: `reference/neatwork5` (called **NW5** below), commit in `reference/neatwork5/SOURCE_COMMIT`
(`APLV-F/neatwork5-mat-2 a129c37a`). All paths below are relative to `reference/neatwork5/`.
Citations are `file:line` in that snapshot.

MATLAB is not available here. Statements marked **UNVERIFIED** come from reading the code or from
general knowledge of MATLAB and were not executed. Statements marked **CHECKED** were checked by
an independent Python/HiGHS re-implementation written for this specification (not part of the repo).

---

## 0. Summary for the Rust developer

* The NW5 design is a **reinforcement** design. It takes a tree network, possibly already equipped
  with pipes, and chooses for each link (arc) an optional new pipe laid **in parallel** to the
  existing one. A bare `.tpo` network is a special case: no arc has a pipe. Every arc then gets a
  "reinforcement", which is its only pipe.
* Pipeline, in order:
  1. read `.tpo`/`.dsg` into tables.
  2. checks and tree orientation, then parameter review.
  3. derived arc data (`pmax`, `haut`, terminal pressure) and the existing-pressure check.
  4. equivalent resistances `TR` and the path matrix.
  5. design loads `TL` (service quality) and `TLv` (velocity).
  6. LP/MILP and solution clean-up.
  7. post-processing (mixture reduction, junction removal, commercial rounding, margins, orifices).
  8. `.dsg` or `.json` export, then run metadata JSON.
* Options `fusion`, `split`, `use_cvar`, `use_efficient_pipes` exist in the parameter set, but the
  driver forces them to 0 (`review_design_parameters_reinf.m:60-63`). Fusion of degree-2 nodes is
  dead code. Splitting into new nodes is not implemented. Nothing "undoes" a fusion. See §7.
* The snapshot **cannot run as committed** because of two regressions (§7.1, B1 and B2). The Rust
  port must implement the intended behaviour, which the MATLAB tests describe.
* The LP of the current code reproduces the pre-rounding solutions of four example `.dsg` files
  (CHECKED, §5.3). No example `.dsg` is a byte-for-byte output of the current code: they were
  produced by older versions with a different commercial-length rounding and an older `.dsg` layout.

---

## 1. Call flow and parameters

### 1.1 Driver `matlab/CreateDsgDev/makeReinfDsg.m`

`makeReinfDsg` is a script, not a function (`clear` at line 37). Its inputs are hard-coded:

| Variable | Line | Value in snapshot | Meaning |
| --- | --- | --- | --- |
| `nom` | 42 | `'Toy'` | network name, also folder name |
| `version` | 43 | `"2b"` | suffix, file is `<nom><version>.tpo` |
| `input_type` | 44 | `'tpo'` | `tpo` or `dsg` (lines 45-48 reject others) |
| `output_type` | 49 | `'json'` | `dsg` or `json` |
| `secu`, `fusion`, `split` | 53-55 | `[]` | only for `dsg` input: parts of the input file name |
| `lieu` | 57 | `'examples/dsg_examples'` | folder of the `.dsg` input |

Note: `examples/tpo_examples/Toy/Toy2b.tpo` does not exist. The default run fails with
"Fichier introuvable" (`parseTpoToTables.m:29-31`). The Rust port takes these values as arguments.

Steps, in order (line numbers in `makeReinfDsg.m`):

| # | Line | Call | Effect |
| --- | --- | --- | --- |
| 1 | 61-72 | `parseTpoToTables(dat)` | `.tpo` → `TN, TA, TC, TB, TP, TD, TO, partec, pardsg` (§1.3) |
| 1' | 74-80 | `readDesign(nom,version,secu,fusion,split,input_type,lieu)` | `.dsg` → same tables (§1.4) |
| 2 | 84-86 | none | exactly one node with `TN.statut==0`, `dat.source_height = TN.haut(source)` (physical) |
| 3 | 89 | `prepareReinforcementTables(TA,TD,partec)` | catalogue + fictitious pipe + existing equipment (§2.2) |
| 4 | 92-94 | none | rename `TD.Pmax` → `TD.pressure` if present |
| 5 | 98 | `validate_and_align_Tables(TN,TA,TB)` | node-set checks (§2.3) |
| 6 | 101-105 | `peel_orient_tree(TN,TA)` | orient from source, error `NW5:InvalidDesignTopology` unless tree, acyclic, no parallel arcs, source degree 1 |
| 7 | 115 | `review_design_parameters_reinf(pardsg,TC)` | interactive dialog + forced values (§1.2) |
| 8 | 118-121 | none | if `uniform_dem==1`: `TC.proba = opentaps`, `TC.cible = targetflow` for all clients |
| 9 | 129 | `enrichTablesReinf(TA,TB,TN,TD,pardsg,dat.source_height)` | `TA.pmax, TA.haut, TA.statut, TA.nbrob, TA.pressure`, `TN.degree`, sort `TA` (§2.4) |
| 10 | 132 | `validate_existing_pressure(TA,TD)` | each existing segment must hold `pmax` (§2.5) |
| 11 | 135 | `equivResist(TA,TN,TD,partec)` | table `TR` (§2.6) |
| 12 | 139 | `buildContext(pardsg)` | `ctx` (fusion flags, §7.3) |
| 13 | 145 | (commented) `fusion_noeuds_degre_2` | **not called** |
| 14 | 155 | `build_graph_artifacts(TA,TN,dat)` | `dat.M_incid`, `dat.M_path`, `dat.idArcT`, `dat.Path2Term` (§2.7) |
| 15 | 159 | `build_incidence_loads(TA,TN,TC,pardsg,dat)` | `TL` at `servicequal`, `TA.nbdep` (§3) |
| 16 | 161-163 | same with `servicequal := vmax_secu` | `TLv` (new random draw in Monte Carlo) |
| 17 | 165 | none | `TLv{:,2:end} = max(TLv{:,2:end}, TL{:,2:end})` elementwise |
| 18 | 169 | `build_reinf_model(TA,TD,TL,TLv,TR,pardsg,partec,dat)` | LP/MILP (§4) |
| 19 | 172-173 | `solveModel(model,"final")` | `linprog` or `intlinprog`, clean-up (§4.6) |
| 20 | 176 | `processSol(...)` | physical network, margins, orifices, costs (§5) |
| 21 | 179-183 | `data4DsgFile(...)` | export `.dsg` or `.json`, empty name → stop (§6) |
| 22 | 186-220 | `write_run_meta` → `write_meta` | `out/<nom>/runs/meta_<nom>_<datetime>.json` (§6.5) |

`apply_pump_pressure.m` is **not called** any more (diagnostic §1.4, `docs/design/Diagnostic_makeReinfDsg.md:70-74`).
The pump gain enters through `enrichTablesReinf` (§2.4).

### 1.2 Parameters

Parameters come from `!Default properties` (name,value lines), become `param`, then are split by
`extract_parameters(param)` (`matlab/CreateDsgDev/extract_parameters.m`) into `partec` (technical) and
`pardsg` (design). `mustfield(s,f,d)` (`utils/mustfield.m:4-8`) returns `d` when the field is absent
**or empty**. Both structures are sorted by field name (`orderfields`, ASCII order).

| Name | Struct | Default (`extract_parameters.m`) | Unit | Used in | Meaning / remarks |
| --- | --- | --- | --- | --- | --- |
| `orifcoef` | partec | 1.0 (l.7) | none | `processSol.m:320` | orifice head-loss coefficient. NW4 files carry 0.59 or 0.62 |
| `faucetcoef` | partec | 1.0 (l.8) | m⁻¹·(m³/s)² | `build_reinf_model.m:212,216`, `processSol.m:280` | faucet loss `q²/faucetcoef` |
| `lambda` | partec | 1.7810 (l.9) | none | resistances, losses | flow exponent λ |
| `nu` | partec | 4.7810 (l.10) | none | `prepareReinforcementTables.m:46` | diameter exponent ν |
| `bescal` | partec | 8.9396e-4 (l.11) | none | `prepareReinforcementTables.m:46` | friction coefficient β |
| `targetflow` | pardsg | 0.0 (l.24) | L/s | loads, faucet loss, orifices | demand of one open client. Replaced by the common `TC.cible` when demand is uniform (`apply_client_data.m:21-23`) |
| `opentaps` | pardsg | 1 (l.25) | none | binomial loads | opening probability, must be in (0,1] for uniform demand (`build_incidence_loads.m:218-222`, error `NW5:LoadProbability`) |
| `pump_pressure` | pardsg | 0 (l.26) | m | `enrichTablesReinf.m:24` | head gain at the source, finite, ≥ 0 (`review_design_parameters_reinf.m:76-77`) |
| `pressure` | pardsg | 2 (l.27) | m | `enrichTablesReinf.m:54-56`, `parseTpoToTables.m:206` | required pressure at terminals when `bounded_flow==1`, copied to every `TB.pressure` when `uniform_dem==1` |
| `secu` | pardsg | 1.0 (l.28) | none | `build_reinf_model.m:201`, `processSol.m:267` | safety factor ≥ 1 (asserted `review_design_parameters_reinf.m:70-74`, `build_reinf_model.m:75-79`), divides the static head of **terminal** nodes only |
| `vmax` | pardsg | 1.5 (l.29) | m/s | `build_reinf_model.m:102` | maximum velocity |
| `vmax_secu` | pardsg | 0.95 (l.30) | none | `makeReinfDsg.m:162` | quantile level for the velocity load `TLv` |
| `servicequal` | pardsg | 0.65 (l.31) | none | loads | quantile level for the design load `TL`, must be in (0,1] (`build_incidence_loads.m:99-101`) |
| `fusion` | pardsg | 0 (l.32) | 0/1 | `buildContext.m`, output name | forced to 0 by review (l.60) |
| `split` | pardsg | 0 (l.33) | 0/1 | `buildContext.m`, output name | forced to 0 by review (l.61) |
| `bounded_flow` | pardsg | 0 (l.34-35) | 0/1 | model, margins, orifices | 1 = flow capped at demand, pressure `TB.pressure` required at terminal. Default 1 when read from `.dsg`/`.json` without the field (`readDesign.m:153-157`). Forced to 1 when `uniform_dem==0` and some terminal has several clients (`apply_client_data.m:24-32`, `review_design_parameters_reinf.m:22-23,49-51`) |
| `pipelength` | pardsg | 6.0 (l.36) | m | `processSol.m:210-211` | commercial pipe length, review forces a value ≥ 0.01 (`review_design_parameters_reinf.m:78-81,85-110`), rounding applies only if `pipelength > 0.1` |
| `seuil` | pardsg | 0.0 (l.37) | none | none | NW4 legacy, unused |
| `limitbudget` | pardsg | inf (l.38) | none | none | NW4 legacy, unused, review sets it to **1e8** (l.64), so the exported value is `100000000` |
| `watertemp` | pardsg | 20.0 (l.39) | °C | none | unused, review sets 20 (l.65) |
| `uniform_dem` | pardsg | 1 (l.40) | 0/1 | everywhere | **derived from the clients**: 1 iff all `TC.proba` are equal and all `TC.cible` are equal (`apply_client_data.m:19-20`), never shown in the dialog |
| `use_efficient_pipes` | pardsg | 0 (l.41) | none | none | unused, forced 0 (l.63) |
| `nScenLoad` | pardsg | 200000 (l.42) | none | Monte Carlo | number of scenarios, fallback 20000 if the field is missing at load time (`build_incidence_loads.m:105-107`) |
| `use_cvar` | pardsg | 0 (l.43) | 0/1 | loads | CVaR instead of VaR, **forced 0** by review (l.62), so only the unit tests use CVaR |

Review step (`review_design_parameters_reinf.m`):

1. Add `pump_pressure=0` if missing (l.18-20).
2. `requireBounded = (uniform_dem==0) && (number of distinct TC.borne < number of clients)` (l.22-23).
3. Dialog fields = `pardsg` minus `uniform_dem, fusion, limitbudget, split, use_cvar,
   use_efficient_pipes, watertemp` (l.27-29), minus `bounded_flow` when `requireBounded` (l.30-38).
   `promptParamDialog.m` parses numbers with `str2double`. An unparsable entry keeps the old value.
4. `pardsg := dialog values + uniform_dem (+ bounded_flow=1 if requireBounded)`, then
   `fusion=0, split=0, use_cvar=0, use_efficient_pipes=0, limitbudget=1e8, watertemp=20`
   (l.54-65), `orderfields` (l.67).
5. Assert `secu` finite real scalar ≥ 1 (l.70-74), `pump_pressure` finite ≥ 0 (l.76-77),
   `pipelength` ≥ 0.01 else second dialog, fallback 0.01 (l.78-81, 85-110).

Rust: replace both dialogs by a non-interactive parameter set (the diagnostic recommends this:
`Diagnostic_makeReinfDsg.md:151`). Keep the forced values and checks of step 4-5. Add the checks that
the diagnostic lists as missing (§7.2, B9): `vmax > 0`, `nScenLoad` positive integer, flags in {0,1},
probabilities in [0,1], demands ≥ 0.

### 1.3 Reading a `.tpo` (`matlab/InOut/parseTpoToTables.m`)

* File `examples/tpo_examples/<nom>/<nom><version>.tpo` (l.27). Lines split on `\n`.
* Sections are found by `getSectionBlock` (`getSectionBlock.m:1-28`): first line whose trimmed
  text **starts with** `"!"+name`. Data run to the next line starting with `!`. So `'Client'`
  matches `!Clients`, `'Borne'` matches `!Borne` and `!Bornes`. `!Nodes`, `!Pipes`,
  `!Default properties` are mandatory and must appear in this order (l.41-50).
* `!Nodes` (l.53-59): `noeud,haut,xdata,ydata,nbrob,statut` (`%s %f %f %f %f %d`), sorted by
  `(statut, noeud)`. `statut`: 0 source, 1 intermediate, 2 terminal (borne).
* `!Pipes` (l.62-74): `pred,succ,lon`, sorted by `(succ,pred)`. `lon = max(lon, 0.01)` (l.73-74).
* `!Default properties` (l.79-89): `name,value`, sorted by name → `param` → `extract_parameters`.
* Catalogue `TD` (l.93-120):
  * with a `!Diameters` section: 8 fields `ID,Nominal,SDR,diam,cost,pressure,type,Roughness`
    (`%s %s %f %f %f %f %d %f`).
  * otherwise `DiamBase<nom>.db`: 7 fields `Nominal,SDR,diam,cost,pressure,type,Roughness`, sorted
    by `diam`, IDs `D001, D002, …` (`"D"+%03d`) in that order.
  * **Defect**: the `!Diameters` sections of `Toy_bbd.tpo`, `Toy_bbu.tpo`, `Toy_blu.tpo` have 7 fields
    (no ID). The 8-field format misreads them (UNVERIFIED, evident from `textscan`). Rust: accept
    7-field rows and number them like the `.db` file (sort by `diam`, `D%03d`).
  * Units: `diam` m, `cost` per metre, `pressure` = maximum allowed pressure (m of water).
* Orifices `TO` (l.123-136): `!Orifices` section (`in_diam,N`), else `orifices.db` (one diameter per
  line). Column `in_diam` (m).
* Clients `TC` (l.146-179): `!Clients` section `client,borne,proba,cible` (`cible` in L/s). Else built
  from terminals: for each terminal `b` (TN order) and `k = 1..nbrob`, client `"C"+k+b`,
  `proba=opentaps`, `cible=targetflow`. Then sorted by `(borne, client)` (string order: `C10P2` before `C1P2`).
* `apply_client_data(TN,TC,pardsg)` (l.180, `apply_client_data.m:1-33`): `TN.nbrob` := number of
  clients per node. `uniform_dem` derived from `TC`. If uniform, `opentaps := TC.proba(1)`,
  `targetflow := TC.cible(1)`. Else if several clients share a terminal, `bounded_flow := 1` (warning
  `NW5:BoundedFlowRequired` if it was 0).
* Terminals `TB` (l.183-209): `!Borne` section `borne,pressure,N` (must equal the set of terminal
  nodes, else error), else one row per terminal with `pressure = pardsg.pressure`. Sorted by `borne`.

### 1.4 Reading a `.dsg` (`matlab/InOut/readDesign.m`, `parseDsgToTables.m`)

* File `<lieu>/<nom>/<nom><version>[_Secu%1.2f][_fusion%d][_split%d].dsg` (`readDesign.m:19-45`).
* `!Clients` is mandatory (error otherwise, `parseDsgToTables.m:40-45`).
* `!Pipes`: 8 fields `pred,succ,lon,lon1,ID1,lon2,ID2,N` (l.51-55). No `lon` clamp here.
* `!Nodes`: 7 fields `noeud,haut,xdata,ydata,orif_ideal,orif_com,statut` (l.61-65). The orifice
  columns are **not** client counts. `nbrob` comes from `!Clients` (`apply_client_data`).
* `!Diameters`: `ID,Nominal,SDR,diam,cost,Pmax,type,Roughness[,N]` (l.71-74). `Pmax` is renamed to
  `pressure` later (`prepareReinforcementTables.m:19-24`, `makeReinfDsg.m:92-94`).
* `!Orifices`: first field only (l.80-81). `!Default properties`: adds `pressure=2` if missing (l.89-91).
* `!Borne`: as for the `.tpo`.
* IDs are cleaned by `normalize_id_column` (`readDesign.m:133-150`). `bounded_flow` defaults to 1
  (l.153-157). `apply_client_data`. `extract_parameters` (l.158-161).
* `TC` is **not** sorted on this path (order of the file). This changes the Monte Carlo draw order.
* Status `N` of a pipe: `"N"` new/normal, `"B"` blocked (never reinforced), `"R"` reinforcement
  produced by a previous design (`prepareReinforcementTables.m:125-131`).
* Old example `.dsg` files write `ID2 = 0` on empty segments. `prepareReinforcementTables` requires
  `"fictif"` there (l.108-116) and rejects them. Rust: map `0`/empty IDs on zero-length segments to
  `fictif` at import (recommended, not done by MATLAB).

---

## 2. Preparation of the tables

### 2.1 Table glossary

| Table | Rows | Columns used by the design |
| --- | --- | --- |
| `TN` | nodes | `noeud, haut` (physical elevation, m), `xdata, ydata, nbrob, statut`, later `degree`, `orif_ideal, orif_com` |
| `TA` | arcs (links) | `pred, succ, lon` (m), `lon1, ID1, lon2, ID2` (existing segments), `N`, `equip`, `pmax, haut, statut, nbrob, pressure`, `diam1, diam2` |
| `TC` | clients | `client, borne, proba, cible` (L/s) |
| `TB` | terminals | `borne, pressure` (m), `N` |
| `TD` | pipe catalogue | `ID, Nominal, SDR, diam, cost, pressure, type, Roughness, resist, cond` |
| `TO` | orifice catalogue | `in_diam` (m) |
| `TR` | arcs | `ID` (= `TA.succ`), `rexist`, one column per real pipe ID |
| `TL`, `TLv` | arcs | `ID` (= `TA.succ`), one column per terminal with clients (L/s) |

Node and arc names are strings. MATLAB keeps them as `categorical` built from strings, whose category
order is the sorted (character-code) order. All sorts by name below use plain lexicographic order of
the UTF-8/ASCII strings (UNVERIFIED for non-ASCII names).

### 2.2 `prepareReinforcementTables.m`

1. `Pmax` → `pressure` (l.19-24). Required columns `ID, diam, cost, pressure, Nominal, SDR, Roughness`
   (l.26-28). Non-empty. IDs as strings, non-missing, unique. Any existing `fictif` row is removed
   (l.36) and rebuilt. Real diameters finite and > 0 (l.42-43).
2. `TD.resist = bescal * diam^(-nu)` (linear resistance, per metre), `TD.cond = resist^(-1/lambda)` (l.46-47).
3. Fictitious pipe `fictif` (l.53-65): copy of row 1, then `ID=Nominal=SDR="fictif"`, `diam=0`,
   `cost=0`, `pressure=1000`, `resist=inf`, `cond=0`, `Roughness=0` (`type` keeps the value of the
   first catalogue row). Append, then `sortrows(TD,'diam')` (stable). **The fictitious pipe is row 1**
   because every real diameter is > 0. Later code relies on this (`equivResist.m:124`, `processSol.m:53`).
4. Existing equipment (l.72-143):
   * if none of `lon1, ID1, lon2, ID2` exists: bare network, `lon1=lon2=0`, `ID1=ID2="fictif"`.
     if some exist, all four must exist.
   * `lon, lon1, lon2` finite ≥ 0.
   * `lon_k > 0 ⇔ ID_k ≠ "fictif"` for k = 1, 2 (errors `NW5:EquipmentID`).
   * all IDs in `TD.ID`.
   * `N` defaults to `"N"`, must be in {`N`,`B`,`R`}.
   * `equipped = lon1>0 | lon2>0`. For equipped arcs `lon1+lon2 = lon` (see bug B1: the code tests
     `abs(ecart) > 1e-7`, l.134-137. The intended test is `abs(lon1+lon2-lon) <= 1e-7`).
   * a blocked arc (`N=="B"`) must be equipped.
   * `TA.equip = equipped`.

### 2.3 `validate_and_align_Tables.m` and `peel_orient_tree.m`

`validate_and_align_Tables` (l.1-72): no duplicate node IDs (`NW:DuplicateNodeIds`). Every `pred`,
`succ` in `TN`. Every node in some arc. Every `TB.borne` in `TN` and in some arc. Warning on self-loops.

`peel_orient_tree` (l.73-231 of the file, the function is the whole file): groups arcs by unordered
node pair, builds the simple graph on all `TN` nodes, `is_connected`, `has_cycle = (#pairs > n −
#components)`, `has_parallel`, `source_degree` (distinct neighbours of the source). If it is a tree,
it repeatedly removes leaves other than the source and swaps `pred/succ` of every arc whose `pred`
is the leaf, so all arcs point away from the source. Only `pred`/`succ` change. Row order and other
columns are kept. **`lon1/ID1` and `lon2/ID2` are not swapped** when an arc is reversed (harmless for
the hydraulics, see §7.2 B14). The driver stops unless tree, no cycle, no parallel arc, source degree 1
(`makeReinfDsg.m:103-105`).

Rust: orient by BFS/DFS from the source. Same acceptance rule.

### 2.4 `enrichTablesReinf.m`

Inputs: `TN.haut` physical, `source_height` = physical source elevation.

* `Hsource = source_height + pump_pressure` (l.24).
* `TA.pmax = Hsource − min(z_pred, z_succ)` (l.27-36): maximum static pressure on the link (m).
* `TN.degree` = number of incident arcs (l.40-44).
* Join from the successor node (l.47-51): `TA.haut = z_succ − Hsource` (**negative** when the node is
  below the source head), `TA.statut = statut(succ)`, `TA.nbrob = nbrob(succ)`.
* If `uniform_dem==1`: `TB.pressure = pardsg.pressure` for every terminal (l.54-56). `TB` is returned
  and exported.
* `TA.pressure = TB.pressure(succ)` for arcs ending at a terminal, else 0 (l.61-64).
* l.69 `TA.statut(a) = 0;`: **bug B2**: `a` is undefined (its definition, l.67-68, is commented out).
  Intended behaviour (diagnostic table, `Diagnostic_makeReinfDsg.md:115`): delete the line. The arc
  keeps the status of its successor node.
* `TA = sortrows(TA, {'statut','succ'})` (l.72). **This is the arc order of every later matrix**:
  intermediate arcs (statut 1) first, then terminal arcs (statut 2), each group by successor name.
  `TN.haut` is never modified.

CHECKED by `test_physical_elevations` and `test_reinforcement_integration` (§8): with source at 100 m,
first node at 110 m and pump 20 m, `pmax` of the first link is 20 m.

### 2.5 `validate_existing_pressure.m`

For every arc and segment k ∈ {1,2} with `lon_k > 0`: `ID_k` must be in the catalogue
(`NW5:UnknownPipe`), its class finite or `+Inf`, not NaN and ≥ 0 (`NW5:ExistingPressureData`), and
`TD.pressure(ID_k) ≥ TA.pmax` (equality allowed). All violations are reported in one error
`NW5:ExistingPressure` with lines
`Liaison <pred> -> <succ>, troncon <k>, tuyau <ID> : pression requise %.6g m, pression admissible %.6g m.`
(l.30-45). `N` plays no role. A negative `pmax` is never a violation.

### 2.6 `equivResist.m` → `TR`

Per arc a (length `L = lon`):

* real segments: `real_k = lon_k > 0 && ID_k ≠ fictif` (l.93-94).
* `rexist_a = (Σ_k real lon_k · resist(ID_k)) / L` for equipped arcs (l.100-108), that is the
  length-weighted mean linear resistance of the existing pipes in series. `cexist = rexist^(-1/λ)`, 0 if
  unequipped.
* for each catalogue row d (fictitious included): `TR(a,d) = (cexist_a + cond_d)^(−λ)` (l.115-117),
  the linear resistance of the existing pipe and pipe d in parallel. For an unequipped arc
  `TR(a,:) = TD.resist` exactly (l.120).
* table columns: `ID` (= `TA.succ`), then one column per `TD` row named by `TD.ID`, **except the
  first, renamed `rexist`** (l.123-126). Because the fictitious pipe is row 1, `TR.rexist` is the
  existing resistance (`inf` when unequipped) and the following columns are the real pipes in
  catalogue order (diameter ascending).
* l.36-40 has the same inverted length check as B1.

Physical meaning: the head loss of link a with flow Q (m³/s) is `L · r · |Q|^λ`, with r the linear
resistance. Parallel pipes add conductances `c = r^(−1/λ)`.

### 2.7 `build_graph_artifacts.m`

With arcs in `TA` order and nodes in `TN` order:

* `Afull(node, arc) = −1` at `pred`, `+1` at `succ` (l.56-57). Source = the node that is never a
  successor (exactly one, l.61-67). `M_incid` = `Afull` without the source row (l.70-71).
* `M_path = M_incid \ B`, where `B(:, j)` injects one unit at `succ(j)` (l.83-89). Result:
  **`M_path(i, j) = 1` iff arc i lies on the path from the source to `succ(j)`** (arc j included),
  else 0. Rust: compute it by walking up parents from each arc. No linear solve needed.
* `idArcT = (TA.statut == 2)`, `Path2Term = M_path(:, idArcT)` (l.92-95).

---

## 3. Design loads `TL` and `TLv` (`build_incidence_loads.m`)

Common part:

* `[G, borneU] = findgroups(TC.borne)`: `borneU` = sorted distinct terminals that have clients. It
  fixes the **column order of `TL`** (l.59-60). Terminals without clients have no column.
* `terminalArcIdx(k)` = arc whose `succ` is `borneU(k)`. Must exist and have `statut==2` (l.63-72).
* `Path2Term = M_path(:, terminalArcIdx)` (l.75).
* `useCvar = pardsg.use_cvar` if present (must be 0 or 1, `NW5:LoadRiskMeasure`), else false (l.92-98).
* `qos = pardsg.servicequal` ∈ (0, 1] (l.99-101). The second call passes `servicequal := vmax_secu`.
* Output: `TL` with column `ID = string(TA.succ)` first, then one column per `borneU` (l.205, 275, 277-278).
  Values in L/s. Entry `TL(i,k)` is the design flow of arc i **conditional on terminal k
  demanding water**. It is 0 when arc i is not on the path to k.
* Later uses take the row maximum: `Qmax_i = max_k TL(i,k)` (`build_reinf_model.m:229-231`,
  `processSol.m:249-250`) and `Qv_i = max_k TLv(i,k)`.

### 3.1 Uniform demand (`uniform_dem == 1`, l.208-276): exact binomial

* `nbdep = M_path * TA.nbrob` (l.212): number of clients downstream of each arc (`TA.nbdep` is stored).
* `p = opentaps` must be in (0, 1] (l.218-222), error `NW5:LoadProbability`.
* Arcs with `nbdep > 1` (l.225-226): `n = nbdep − 1` (one client is known to be open, the other n
  clients open independently with probability p, `Z ~ Bin(n, p)`). The load factor (number of open
  clients) is `ld`:
  * `p == 1`: `ld = 1 + n` (VaR and CVaR, l.229-230).
  * CVaR (`useCvar`), `qos == 1`: `ld = 1 + n` (l.232-233).
  * CVaR, `qos < 1` (l.234-240):
    `k = binoinv(qos, n, p)`,
    `excess = n·p·P(Bin(n−1, p) ≥ k) − k·P(Bin(n, p) > k)`,
    `ld = 1 + k + max(excess, 0)/(1 − qos)`.
    Implement with upper tails (`binocdf(x, N, p, 'upper') = P(X > x)`).
  * VaR (default, l.242-261):
    `k = binoinv(qos, n, p)` = smallest integer k with `F(k) ≥ qos`, F the CDF of Bin(n, p).
    `p2 = F(k)`, `p1 = F(k−1)` (F(−1) = 0).
    if `p2 > p1 && p1 > 0`: `ld = k + (qos − p1)/(p2 − p1)`.
    else if `p2 == p1 && p1 > 0`: `ld = k`.
    else (`p1 == 0`, that is k = 0): `ld = 1`.
    Reading: `ld − 1` interpolates linearly between k−1 and k extra open clients.
* Arcs with `nbdep ≤ 1`: `ld = 1` (l.263).
* `flowFactor = max(ld · targetflow, 1e-12)` (L/s, l.270-271).
* `TL(i, k) = Path2Term(i, k) · flowFactor(i)` (l.274): every non-zero entry of a row is equal.

CHECKED against example files (§5.3): with `n = 6, p = 0.6, qos = 0.65` the formula gives
4.62474279835391, the value written for arc `Source→A` of `Toy_bbu…dsg` and `Toy_blu…dsg`.

Binomial functions for Rust: compute the CDF by summing the PMF in log space (or `statrs`) and find
`k` by scanning upward. `n` is at most the number of clients. Match MATLAB within 1e-12 relative.

### 3.2 Heterogeneous demand (`uniform_dem == 0`, l.103-206): Monte Carlo

1. Checks: `TC.proba` finite real in [0, 1] (`NW5:Probabilities`), `TC.cible` finite real ≥ 0
   (`NW5:Demands`) (l.108-116).
2. `nS = nScenLoad`, `nC = height(TC)`.
3. Scenarios (l.124): `scenarios = (rand(nC, nS) < TC.proba) .* TC.cible`, an nC × nS matrix. Entry
   (i, j) is the demand of client i in scenario j. **RNG**: the global MATLAB generator, never seeded
   in the driver (`rng(2,'twister')` is commented out, l.122). Results are therefore not reproducible
   between runs, and `TL` and `TLv` use two independent draws (`Diagnostic_makeReinfDsg.md:125`).
   MATLAB fills `rand(nC,nS)` column by column: scenario 1 clients 1..nC, then scenario 2, ….
4. `demande_borne = B2C * scenarios` (nB × nS), `B2C(k, i) = 1` if client i belongs to `borneU(k)`
   (l.127-129).
5. `demande_noeud` (nbArcs × nS): row `terminalArcIdx(k)` = `demande_borne(k, :)`, other rows 0.
   `load_flow = M_path * demande_noeud` (l.134-137): flow of every arc in every scenario.
6. For each terminal k (l.149-204):
   * `I` = arcs on the path to k. `J` = scenarios with `demande_borne(k, j) > 0`. `m = |J|` must be
     > 0 (`NW5:NoActiveLoadScenarios`, l.156-158).
   * for each arc of `I`, `Y` = its loads over `J`, sorted ascending (1-based below).
   * `idx1 = ceil(qos · m)`, `qc1 = Y(idx1)` (l.163-165).
   * **CVaR** (l.166-176): if `qos == 1`, `q = Y(m)` (maximum). Else
     `q = qc1 + Σ_j max(Y_j − qc1, 0) / ((1 − qos)·m)`.
   * **VaR** (l.177-202): `lt = #{Y < qc1}`, `le = #{Y ≤ qc1}`, `pc2 = lt/m`, `pc1 = le/m`.
     `qc2 = Y(lt)` if `lt > 0` else `qc1`. If `pc1 > pc2 && pc2 > 0`:
     `θ = (qos − pc2)/(pc1 − pc2)`, `q = θ·qc1 + (1 − θ)·qc2`. Else `q = qc1`.
   * `TL(I, k) = q`. Other entries of column k stay 0.

The VaR rule is a linear interpolation of the empirical CDF between the largest value below the
quantile and the quantile itself. CHECKED statistically: with the clients of `Toy_bbd.tpo`,
200000 scenarios (NumPy, seed 1) give `Qmax/targetflow` = 5.5811, 2.8534, 2.6530, 1, 1.8533, 2, 1
against 5.5738, 2.8534, 2.6475, 1, 1.8526, 2, 1 written in `Toy_bbd_Secu1.00_fusion0_split0.dsg`.

RNG for Rust (to replay `test_build_incidence_loads_cvar` exactly): MATLAB `rng(42)` with the
default `twister` generator is MT19937 initialised by `init_genrand(42)`, and `rand` returns
`genrand_res53` doubles (`(a>>5 · 67108864 + b>>6) / 2^53`), the same stream as NumPy
`RandomState(42).random_sample()` (UNVERIFIED, widely reported, for example first value 0.3745401188473625).
Use `rand_mt::Mt19937GenRand32` (or equivalent) with `res53`, consuming values in column-major order.
For production runs expose a `seed` parameter (MATLAB has none) and draw **one** scenario set shared
by `TL` and `TLv`, or keep two draws for fidelity. Document the choice (see `docs/forum/a_faire.md:42`).

Note on conditioning (`Diagnostic_makeReinfDsg.md:128`): the binomial branch conditions on "one given
client is open", the Monte Carlo branch on "terminal k has positive demand". With several clients per
terminal these differ. Keep both as specified.

### 3.3 Velocity loads

`TLv` is computed with `servicequal := vmax_secu` (`makeReinfDsg.m:161-163`), then
`TLv = max(TLv, TL)` elementwise (l.165). `dat` returned by the second call is identical.

---

## 4. The LP/MILP model (`build_reinf_model.m`)

### 4.1 Index sets and data

* Arcs a = 1..A in `TA` order (§2.4). Arc a ends at node `succ(a)`. Constraint row a belongs to that node.
* Real pipes d = 1..D: rows of `TD` without `fictif`, in `TD` order (diameter ascending) (l.57-62).
* `L_a = TA.lon`, `c_d = TD.cost`, `D_d = TD.diam`, `P_d = TD.pressure`.
* `R_{a,d} = TR(a, ID_d)` (taken by column name, l.70-73, 97).
* `Q_a = max_k TL(a,k) / 1000` (m³/s) (l.229-231). `Qv_a = max_k TLv(a,k) / 1000` (l.92-93).
* `E_a = L_a · rexist_a · Q_a^λ` if equipped (`TR.rexist < inf`), else 0 (l.228, 232-235):
  head loss of the existing pipe alone.
* `P(j) = {a : M_path(a, j) = 1}`: arcs on the path from the source to `succ(j)`.
* `isTerm_j = (TA.statut(j) == 2)`.

### 4.2 Variables (column layout, l.115-163)

`z = [x; y]`, `nVar = A·(D+1)`.

* `x_{a,d}` at 1-based position `(a−1)·D + d`: **fraction** of the length of arc a covered by
  reinforcement pipe d (arc-major blocks of D).
* `y_a` at position `A·D + a`: 1 if arc a receives a reinforcement.

### 4.3 Bounds

* Pressure admissibility (l.86): `okP_{d,a} = (P_d ≥ pmax_a)`.
* Velocity admissibility (l.102-105): `dmin_a = 2·sqrt(Qv_a / (π·vmax))`, `okV_{d,a} = (D_d ≥ dmin_a)`.
  Every candidate must carry the whole control flow alone ("share = 1", decided 2026-11-18,
  `Diagnostic_makeReinfDsg.md:62`).
* `0 ≤ x_{a,d} ≤ ub_{a,d}`, `ub = 1` if `okP ∧ okV` else 0 (l.109-115, 124).
* `y_a ≤ uby_a = (N_a ≠ "B")` (l.118).
* `y_a ≥ lby_a` (l.129-141): with `existingDiam_{a,k} = TD.diam(ID_k)` for segments with
  `lon_k > 0` and `+inf` otherwise, `minExisting_a = min_k existingDiam_{a,k}`.
  `lby_a = 1` if the arc is unequipped **or** `minExisting_a < dmin_a` (existing pipe alone too fast).
* If `lby_a > uby_a` for some arc: error `NW5:ReinforcementBlocked` listing the successor nodes (l.146-152).
* `model.activ_variables = (ub == 1)` (l.155), informative only.

### 4.4 Objective and equalities

* `min f'z`, `f_{(a−1)D+d} = L_a · c_d` (l.180). `f_{AD+a} = TA.fixedCost(a)` if `TA` has a
  `fixedCost` column, else 0 (l.181-183). No reader creates `fixedCost`. Keep the hook.
* Equalities (l.184-187): for each a, `Σ_d x_{a,d} − y_a = 0`.
  For an unequipped arc (`y_a = 1`) the fractions sum to 1. For a non-reinforced arc all x are 0.

### 4.5 Inequalities: head loss at every node

One row per arc j (that is per non-source node):

```
Σ_{a∈P(j)} [ Σ_d (L_a·R_{a,d}·Q_a^λ)·x_{a,d} − E_a·y_a ]  ≤  b_j − Σ_{a∈P(j)} E_a
```

which is equivalent to `Σ_{a∈P(j)} [ y_a·(Σ_d L_a R_{a,d} Q_a^λ x_{a,d}/y_a) + (1−y_a)·E_a ] ≤ b_j`:
the path head loss with the chosen equipment (existing alone, or existing in parallel with the
reinforcement mixture) must not exceed the budget `b_j`.

Right-hand side `b_j` (l.196-238), with `H_j = −TA.haut(j) = Hsource − z_succ(j)`:

| Node | `b_j` |
| --- | --- |
| non-terminal | `H_j` ("intrusion prevention": residual pressure ≥ 0) |
| terminal, `bounded_flow=1`, `uniform_dem=1` | `H_j/secu − TA.pressure(j) − q²/faucetcoef` |
| terminal, `bounded_flow=1`, `uniform_dem=0` | `H_j/secu − TA.pressure(j)` |
| terminal, `bounded_flow=0`, `uniform_dem=1` | `H_j/secu − q²/faucetcoef` |
| terminal, `bounded_flow=0`, `uniform_dem=0` | error "The case with nonuniform demands and unbounded flows is not treated" (l.218) |

with `q = targetflow/1000` (m³/s), `secu` used as `max(secu, eps)` (eps = 2.220446049250313e-16),
`faucetcoef` as `max(faucetcoef, eps)`. The final `bineq = b − M_pathᵀ·E` (l.237-238).

Matrix assembly (l.244-265): `A_loc(a, (a−1)D+d) = L_a·R_{a,d}·Q_a^λ`. `Aineq_x = M_pathᵀ·A_loc`.
`Aineq_y = −(M_path .* E)ᵀ`, that is `Aineq_y(j, a) = −M_path(a, j)·E_a`. `Aineq = [Aineq_x, Aineq_y]`.
Both `Q` in loss terms and `E` use the design load `TL`, not `TLv`.

There is **no slack variable and no budget constraint** (unlike NW4, §9). An infeasible budget
(for example a node above the source head) makes the solver fail.

### 4.6 Solving and clean-up (`solveModel.m`)

* If all `y` bounds are fixed (`lb == ub` on every y): `linprog(f, Aineq, bineq, Aeq, beq, lb, ub)`
  (l.26-30). Otherwise `intlinprog` with `intcon` = all y indices (l.31-37). For a `.tpo` input all
  arcs are bare, `lby = uby = 1`, so it is always an LP.
* `exitflag ≠ 1` → error `NW5:DesignFailure` with "No solution satisfies all constraints…" when
  `exitflag == −2`, "The solver did not return a solution." when `z` is empty, else "The solver did not
  certify z as an optimal solution.", followed by `Solver diagnostic :\n<message>` (l.40-53).
* Checks on the raw `z` (l.62-72): `|Aeq·z| < 1e-6`, `bineq − Aineq·z > −1e-6`,
  `max(0, lb − z, z − ub) < 1e-7`, y within 1e-6 of {0, 1}.
* Clean-up (l.77-90): `y < 1e-6 → 0`, `|1 − y| < 1e-6 → 1`. `x_mat = reshape(x, D, A)` (D × A, column
  a = arc a). Entries with `|x| < 1e-7` → 0. Columns with `y = 0` → 0. Columns with `y = 1` divided by
  their sum (restores `Σ_d x = 1`).
* `slack = bineq − Aineq·[x_new; y]`. Warning if `min(slack) < −1e-6` (l.95-100).
* `new_fval = f'·[x_new; y]`. Prints the relative change when `> 1e-5` (l.103-108).
* Output `sol`: `z, x` (raw), `x_mat` (cleaned), `y`, `slack`, `fval` (solver objective), `new_fval`,
  `exitflag`, `message`, `ok = (exitflag == 1)`, `reinfArc = (y > 0.99)` (l.146-160).

MATLAB solver: since R2023b the default `linprog` algorithm is `'dual-simplex-highs'` and
`intlinprog` uses HiGHS (UNVERIFIED for the R2025a runs of the diagnostic). The Rust port uses the
`highs` crate:

* columns = `nVar` in the order above, column bounds `[lb, ub]`, cost `f`. Set integrality on the y
  columns only when some y is free.
* rows = A inequality rows `(−inf, bineq_j]` then A equality rows `[0, 0]` (or drop the equality rows
  and substitute `y_a = Σ_d x_{a,d}`: not done by MATLAB. Keep the rows for fidelity).
* columns with `ub = 0` can be removed. Keep the index mapping.
* apply the same post-solve checks and clean-up with the same tolerances.

**Degeneracy**: equal loads on consecutive arcs give tied optima (CHECKED on `Toy_Borne`, two vertices
with cost 450.4052827 and different splits). Different solvers can return different vertices.
Tests must compare costs and constraint satisfaction, and compare lengths only where the optimum is unique.

---

## 5. Post-processing (`processSol.m`)

Signature: `[TA_out, TN_out, TCP, TAH, reinfcost, ctx, TS] = processSol(TA, TN, TD, TC, TO, TL, TR,
dat, pardsg, partec, model, sol, ctx)` (l.1). All computations use the **original tree** (`TL`,
`dat.M_path` before any parallel arc is added).

### 5.1 Steps

A. **Integrity** (`integrity`, l.444-516, then l.31-32 and l.35-63): `secu ≥ 1`. `TA`, `TD`, `TL`
   non-empty. `sol.ok`. `M_path` is A × A. `TA` has `lon, lon1, lon2, ID1, ID2, statut, haut, pred,
   succ`; `TD` has `ID, diam, cost, resist`; at most one `fictif`. Real diameters valid and IDs unique.
   `x_mat` is D × A, `y` has A entries. `x ≥ −1e-6`, y within 1e-6 of an integer in [0,1].
   `|Σ_d x_{·,a} − round(y_a)| ≤ 1e-6`. `lon > 0` finite, `lon1, lon2 ≥ 0`. For equipped arcs
   `|lon1 + lon2 − lon| ≤ 1e-9·max(1, lon)`. Every bare arc reinforced (`NW5:DisconnectedArc`). No
   blocked arc reinforced (`NW5:BlockedArc`). Length/ID consistency. `TL` finite ≥ 0. When
   `uniform_dem==1 && bounded_flow==0`, `TO.in_diam` finite > 0. `Td` = real rows of `TD`, must have
   the same order as `TR` columns 3..end (l.50-55) and finite `resist > 0`. `diam1, diam2` filled from
   `TD` if missing (l.41-46).

   Two tables are built (l.71-80): `Teq` (existing equipment, `nature="equip"`) and `Tre`
   (reinforcement: `ID1=ID2="fictif"`, `lon1=lon2=diam1=diam2=0`, `N="N"` renamed `"R"`,
   `nature="reinf"`), both with columns `pred, succ, lon, lon1, ID1, lon2, ID2, N, equip, diam1, diam2, nature`.

B. **Mixture reduction** (l.84-101): `X = sol.x_mat`, `reinforced = y > 0.5`,
   `equipped = lon1>0 | lon2>0`. `costOptim = Σ_a L_a·Σ_d c_d X_{d,a}` (before any change).
   `candidateR = TR` real columns (transposed, D × A), must be finite > 0. For each arc with **more
   than two** positive fractions: `X(:,a) = reducePipeMixture(X(:,a), candidateR(:,a), Td.cost)`
   (§5.2). `costPipesAfterReduction` recomputed.

C. **IDs of the reinforcement** (l.105-119): `idXtran = Xᵀ > 0` (computed **once, after B**). For
   each reinforced arc, the 1 or 2 used pipes sorted by **decreasing diameter**: `Tre.ID1/diam1` = the
   larger, `Tre.ID2/diam2` = the smaller (if any). More than 2 or 0 pipes → assertion error.

D. `costFixed = f_yᵀ·y` (l.123).

E. **Junction removal on doubled arcs** (l.138-190). The model assumes that, when a reinforcement
   uses two pipes in series, their junction is also connected to the existing pipe. The output does
   not create that node, so the series proportions are re-solved to keep the link resistance:
   * `doubled = reinforced & equipped`. Assert `equipped | reinforced`. Assert `rexist` finite > 0 on
     equipped arcs and `inf` elsewhere.
   * For arcs that are doubled **and** use exactly two pipes (`splitDoubled`):
     `rTarget_a = Σ_d X_{d,a}·candidateR_{d,a}` (the model's equivalent resistance).
     `ids` = the two pipes in **catalogue order** (diameter ascending), `ρ = Td.resist(ids)`.
     `ΔC = rTarget^(−1/λ) − rexist^(−1/λ)` (must be > 0, `NW5:Junction`). `ρT = ΔC^(−λ)`.
     if `ρ1 == ρ2`: require `|ρT − ρ1| ≤ 1e-8·max(1, ρ1)` and keep X.
     else `η1 = (ρT − ρ2)/(ρ1 − ρ2)`, require `η1 ∈ [−1e-8, 1 + 1e-8]`, clamp to [0,1],
     `X(ids, a) = [η1; 1 − η1]`.
     Because `g(ρ) = (cexist + ρ^(−1/λ))^(−λ)` is increasing, `ρT` lies between ρ1 and ρ2, so η1 is
     always admissible up to round-off.
   * Check: `rBefore = Td.resistᵀ·X` (reinforced arcs, `inf` elsewhere),
     `rEqBefore = parallelResistance(rexist, rBefore, λ)`. Relative error to `rTarget` < 1e-7 on
     `splitDoubled` (l.173-189). `costInter` (= `costPipesBeforeRounding`) recomputed (l.190).
   * After this step the reinforcement can cost more than the LP optimum.

F. **Commercial lengths** (l.199-226). For each reinforced arc, `J` = pipes of `idXtran` (from C),
   sorted by decreasing diameter, `L = lon`:
   * one pipe: `Tre.lon1 = L`.
   * two pipes: `L1 = L·X(J1, a)`. If `pipelength > 0.1`: `L1 = min(L, ceil(L1/pipelength)·pipelength)`.
     `Tre.lon1 = L1`, `Tre.lon2 = L − L1`, `X(J, a) = [L1; L − L1]/L`.
     The larger diameter is rounded **up**, so the link resistance can only decrease (margins grow).
     No tolerance is applied before `ceil` (a value 12.000000001 becomes 18 with 6 m pipes). The
     unused helper `fitCommercialLengths` (l.416-433) has the tolerance `1e-10·max(1, L)`.
     Rust: follow l.210-211 exactly, and note the risk in the code.
   * Then `ID_k = "fictif"` and `diam_k = 0` wherever `lon_k == 0` (l.220-223). If rounding gives
     `L1 = L`, the arc ends with a single pipe. If E clamped η1 to 0 or 1, `lon1` can be 0 with a real
     `ID2` (allowed by the output check).
   * `costFinal = Σ_a L_a·Σ_d c_d X_{d,a}` (l.226).

G. **Margins** (l.233-311), on the tree:
   * `rNew = Td.resistᵀ·X` on reinforced arcs (`inf` elsewhere). `rEq = parallelResistance(rexist,
     rNew, λ)` (l.240-246) with `parallelResistance(r1, r2) = (c1 + c2)^(−λ)`, `c = r^(−1/λ)` and
     `c = 0` for `r = inf`. Must be finite > 0. `TAH.equivResist = rEq` (linear, per metre).
   * `Qmax = max_k TL / 1000`. `TAH.designFlow = Qmax·1000` (L/s) (l.249-251).
   * `connectionLoss = L·rEq·Qmax^λ`. `pathLoss = M_pathᵀ·connectionLoss` (l.260-262).
   * `available = −TA.haut`, divided by `max(secu, eps)` on terminals (l.265-267).
   * `required` = 0, on terminals `TA.pressure` if `bounded_flow==1`, plus `q²/max(faucetcoef, eps)` if
     `uniform_dem==1`. Error `NW5:Orifices` if `bounded_flow==0 && uniform_dem≠1` (l.268-281).
   * `residualPressure = available − pathLoss`, `margin = residualPressure − required`,
     `violation = max(0, −margin)` (l.284-286).
   * `TS` (l.287-294): columns `noeud` (= `TA.succ`), `constraint_type` (`"intrusion_prevention"`, or
     `"demand_satisfaction"` on terminals), `head_required`, `head_available` (= residual pressure),
     `head_margin`, `head_deficit`, units m. Warning `NW5:PressureAfterRounding` if some
     violation > 1e-6 (l.295-299).
   * `TAH = [pred, succ, equivResist, designFlow, pathLoss, margin]`.
   * With unchanged equipment, `margin` equals the LP `slack` (CHECKED by the integration test).

H. **Orifices** (NW4 case only, l.316-325): `ideal = commercial = 1` for every arc. If
   `uniform_dem==1 && bounded_flow==0`, for terminal arcs with `margin ≥ 0.1`:
   `ideal = min(1, orifcoef·sqrt(q)·margin^(−1/4))` (q in m³/s, margin in m, result in m).
   catalogue `cand = [TO.in_diam; 1]` (1 m means "no orifice").
   `commercial = cand(argmin_j |ideal^(−4) − cand_j^(−4)|)` (first minimum on ties).
   For `bounded_flow==1` no orifice is computed (old outputs had orifices there, §5.3).

J. **Outputs** (l.331-398):
   * `TN_out` = `TN` with `orif_ideal`, `orif_com` (1 by default, arc values written at `succ`),
     columns `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut` (physical `haut` unchanged).
   * `TA_out = [Teq(equipped, :); Tre(reinforced, :)]`: existing pipes first, then reinforcements,
     each in `TA` order. A doubled link appears twice (two parallel arcs with the same `pred, succ`).
     Checks: `height = A + #doubled`. `|lon1 + lon2 − lon| ≤ 1e-8·max(1, lon)`. Length/ID consistency.
   * `TCP = TA_out(:, {pred, succ, nature, diam1, diam2})` with `nature` `"old"`/`"new"`.
   * `reinfcost = struct(costOptim, costInter, costFinal, costFixed)` (this field order).
   * `ctx.nbsplit = 0`, `ctx.split = false`, `ctx.reinforcement = struct(reducedMixtureArcs,
     costPipesAfterReduction, costPipesBeforeRounding, costPipesAfterRounding, fixedCost,
     requiresSimulationGraphRebuild = true[, costLP = sol.fval][, slackLP, marginChange = margin − slack])`.
   * `TN.degree` is recomputed on `TA_out` (l.361-367) but not returned.

Unused local functions: `fitCommercialLengths` (l.416-433), `loadFactorForExport` (l.435-442).

### 5.2 `reducePipeMixture(x, resistance, cost)` (`reducePipeMixture.m`)

Input: fractions `x ≥ 0` on one arc, the equivalent resistances `TR` of the candidates (with the
existing pipe in parallel) and their costs. If at most two are positive, return `x`. Otherwise
(l.16-50):

1. `mass = Σx`, `target = (resistanceᵀx)/mass`.
2. Active pipes sorted by (resistance ascending, cost ascending). Keep the first of each equal resistance.
3. Lower convex hull of the points (resistance, cost) by a monotone chain: while the hull has ≥ 2
   points a, b and the new point c satisfies `(cost_b − cost_a)·(r_c − r_b) ≥ (cost_c − cost_b)·(r_b −
   r_a)`, pop b.
4. If the hull has one point or `target ≤ r(hull_1)`: all mass on `hull_1`. If `target ≥ r(hull_end)`:
   all on `hull_end`. Else find the first hull point b with `r_b ≥ target`, a its predecessor,
   `w = (r_b − target)/(r_b − r_a)`, `x_a = mass·w`, `x_b = mass·(1 − w)`.
5. Assert resistance preserved within `1e-10·max(|rᵀx|, realmin)` and cost not increased beyond
   `1e-10·max(1, |costᵀx|)`.

The result keeps `Σx` and `rᵀx` and minimises cost over pairs (CHECKED by `test_reducePipeMixture`
in MATLAB, exhaustive pair search).

### 5.3 Example `.dsg` files: which are design outputs

None is in the format written by the current code. They have no `!Hydraulics`, `!ComplementTA`
or `!Costs` section. They write `ID2 = 0` instead of `fictif` and the pipe status `N` instead of `R`.
Their `!loadfactor` holds load factors, not `designFlow, margin`. They come from an older
`makeDesign`/`CreateDsg`. The comparisons below use an
independent re-implementation of §2-§5 (uniform demand, bare network, HiGHS via SciPy), CHECKED
2026-11-20:

| File | Input and parameters (from the file) | Result |
| --- | --- | --- |
| `examples/dsg_examples/Toy/Toy_blu_Secu1.00_fusion0_split0.dsg` | `tpo_examples/Toy/Toy_blu.tpo` (7-field `!Diameters` = `DiamBaseToy.db`, `!Orifices` section 0.0017…0.0044), uniform, `bounded_flow=0`, `opentaps=0.6`, `targetflow=0.12`, `servicequal=0.65`, `vmax=9`, `vmax_secu=0.95`, `secu=1`, `pressure=2`, `pipelength=6`, `faucetcoef=1.87e-8`, `orifcoef=0.59`, λ, ν, β defaults | load factors equal (all 7, to 1e-14), LP pipe choice equal on all 7 arcs, `orif_ideal(P1)` = 0.004796589912316905 vs file 0.0047965899123169, `orif_com(P1)` = 0.0044 equal, other terminals margin < 0.1 → 1 (equal). LP cost 602.56548, file cost 602.9082 |
| `…/Toy/Toy_bbu_Secu1.00_fusion0_split0.dsg` | `Toy_bbu.tpo`, same but `bounded_flow=1`, `vmax=1.5` | load factors equal, LP pipe choice equal on all 7 arcs. LP cost 649.08633, file 648.4830. File has `orif_ideal(P1)=0.00565…` although bounded (current code writes 1) |
| `…/Toy/Toy_Secu1.00_fusion0_split0.dsg` | `Toy_blu.tpo` with `opentaps=0.28`, `vmax=1.5`, `bounded_flow=0`, orifices from `tpo_examples/Toy/orifices.db` (up to 0.0056) | LP pipe choice equal on all 7 arcs, `orif_ideal(P3)` 0.006438404306132354 vs 0.00643840430613235, `orif_ideal(P1)` differs (0.0051941 vs 0.0052153) because its path crosses a rounded arc. LP cost 514.79229, file 514.4766 |
| `…/Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg` | `Toy_blu.tpo` with one client per terminal (`nbrob=1`), `opentaps=0.28`, `vmax=1.5`, catalogue = the 7 rows of the file, `P4` pressure 3 is ignored (uniform) | LP pipe choice equal (degenerate LP, see §4.6). LP cost 450.40528, file 450.6330 |
| `…/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg` | `Toy_bbd.tpo`, heterogeneous clients, Monte Carlo, `bounded_flow=1`, `vmax=19` | not reproducible exactly (unseeded RNG), load factors (= `Qmax/targetflow`) agree within Monte Carlo noise (§3.2) |
| `…/Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg` | `tpo_examples/Ceiba/Ceiba.tpo` + `DiamBaseCeiba.db`, uniform, `bounded_flow=0`, `vmax=90`, `orifcoef=0.62`, `faucetcoef=1.64e-8`, `opentaps=0.19` | not reproducible: same pipe set on 387 of 396 arcs, but file cost 4628.10 < current LP optimum 4644.76, so an older model was used |
| `tpo_examples/Red 2/Red 2_Secu1.00_fusion0_split0.dsg` | `Red 2.tpo` | not reproducible (different pipes, no splits, older algorithm) |
| `…/Toy/ToyDevReinf.dsg` | hand-written **input** for a reinforcement run (`input_type='dsg'`, version `DevReinf`): bare arc `C→P3`, blocked arc `C→P4` | not an output, cannot run in MATLAB because of B1 |
| `Toy2b_NW4.dsg`, `CeibaNW4.dsg`, `ToyXlx.dsg`, `Esperenza/*.dsg`, `Red 1/Red 1.mat` | NW4 outputs, simulation inputs (two sources for Esperenza), binary | not NW5 design outputs |

**Old rounding rule** (explains every split-length difference of the four Toy files, CHECKED on 9
split arcs): the **longer** of the two LP lengths is rounded to the **nearest** multiple of
`pipelength`, the other takes the rest. Example `Toy_bbu`, arc `A→B`: LP 52.82 m D009 + 447.18 m D004
→ file 50 + 450. Current code (`ceil` on the larger diameter) gives 54 + 446.

LP lengths (m) to use as golden values (tolerance 1e-3 m, arcs not listed use one pipe on the whole
length, pipe given in the file):

| Case | Arc | LP lengths |
| --- | --- | --- |
| Toy_blu | A→C | D004 4.4555, D002 65.5445 |
| Toy_blu | B→P2 | D004 70.0956, D002 109.9044 |
| Toy_blu | C→P3 | D002 4.3538, D001 85.6462 |
| Toy_bbu | A→B | D009 52.8204, D004 447.1796 |
| Toy_bbu | C→P3 | D002 46.3937, D001 43.6063 |
| Toy_bbu | C→P4 | D002 78.1753, D001 31.8247 |
| Toy_Secu | A→B | D004 219.6846, D002 280.3154 |
| Toy_Secu | C→P4 | D002 27.2241, D001 82.7759 |
| Toy_Borne | A→B | D002 240.0399, D001 259.9601 |
| Toy_Borne | A→C | D002 4.0624, D001 65.9376 (degenerate) |

Recommended Rust validation: reproduce load factors exactly, LP cost to 1e-6 relative, LP lengths of
non-degenerate arcs to 1e-3 m, and orifices on paths without split arcs to 1e-12 relative. Do not
compare rounded lengths with the files (different rule).

---

## 6. Outputs

### 6.1 File name and folder (`data4DsgFile.m`)

* Suggested name `sprintf('%s%s_Secu%1.2f_fusion%1.0f_split%d', nom, version, secu, fusion, split)`
  (l.50-51), for example `Toy_blu_Secu1.00_fusion0_split0`. Because the review forces `fusion = split = 0`,
  the suffix is always `_fusion0_split0`.
* An `inputdlg` lets the user edit the name. Cancel → `filename = ''`, no export, no metadata
  (l.52-56, `makeReinfDsg.m:180-183`). Rust: take the name as an argument.
* Folder `<root>/out/<nom>/`, created if missing (l.59-64). File `<name>.<output_type>`.
  `output_type` ∈ {`dsg`, `json`} (l.33-35). `.mat` export was removed.

### 6.2 `.dsg` (`export_neatwork_csv.m`)

Text file, sections in this order, header line `!<Section>`, then one comma-joined line per row,
**no header row, no trailing `,N` added by the writer** (l.7-41, 49-70). Values: numbers with
`num2str(x, '%.15g')` (l.81), strings as is (no quoting, so names with commas break the file),
logical columns as `true`/`false`. Line ending `\n`. The file is opened in `'w'` for `!Nodes` and
`'a'` afterwards.

| # | Section | Columns | Source |
| --- | --- | --- | --- |
| 1 | `!Nodes` | `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut` | `TN_out`, sorted by `(statut, noeud)` (`data4DsgFile.m:70-81`), physical heights |
| 2 | `!Pipes` | `pred, succ, lon, lon1, ID1, lon2, ID2, N` | `TA_out` in its order (existing, then reinforcement), `N` ∈ {`N`,`B`,`R`}, empty segment `0,fictif` |
| 3 | `!Default properties` | `name, value` | `pardsg` fields (ASCII order) then `partec` fields (`bescal, faucetcoef, lambda, nu, orifcoef`) (l.95, 139-148) |
| 4 | `!loadfactor` | `succ, designFlow, margin` | `TAH` (L/s, m) (l.101) |
| 5 | `!Diameters` | `ID, Nominal, SDR, diam, cost, pressure, type, Roughness, N` | `TD(:,1:8)` + `'N'` (l.104-106), **includes the `fictif` row** (first: `fictif,fictif,fictif,0,0,1000,<type>,0,N`) |
| 6 | `!Orifices` | `in_diam, N, N, N, N, N` | `TO` + 5 `'N'` (l.109-111) |
| 7 | `!Clients` | `client, borne, proba, cible` (+ any other `TC` column) | `TC` after the uniform update |
| 8 | `!Borne` | `borne, pressure, N` | `TB` (pressure replaced by `pardsg.pressure` if uniform), `N` column added if absent (l.115-122) |
| 9 | `!Hydraulics` | `pred, succ, equivResist, designFlow, pathLoss, margin` | `TAH` |
| 10 | `!ComplementTA` | `pred, succ, nature, diam1, diam2` | `TCP` (`old`/`new`) |
| 11 | `!Costs` | `name, value` | `costOptim, costInter, costFinal, costFixed` |
| 12 | `!ConstraintSatisfaction` | `noeud, constraint_type, head_required, head_available, head_margin, head_deficit` | `TS`, only if non-empty |

`!Default properties` content after review, in order: `bounded_flow, fusion, limitbudget,
nScenLoad, opentaps, pipelength, pressure, pump_pressure, secu, servicequal, seuil, split,
targetflow, uniform_dem, use_cvar, use_efficient_pipes, vmax, vmax_secu, watertemp, bescal,
faucetcoef, lambda, nu, orifcoef` (`nScenLoad` sorts before `opentaps` because ASCII `n` < `o`.
UNVERIFIED that no dialog-only field is added).

`%.15g` examples: `1.87e-08`, `100000000`, `0.00089396`, `2`. Rust: implement `%.15g` exactly (the
NW4 port has Java formatters, not this one).

### 6.3 `.json` (`save_dsg_json.m`)

`jsonencode(P, 'PrettyPrint', true)` with (l.4-29):

* `schema: "NW-json-v1"`.
* tables as arrays of row objects (`table2struct`, categorical → string, sparse → full):
  `TN` (TN_out columns `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut`, **not** re-sorted),
  `TA` (all `TA_out` columns: `pred, succ, lon, lon1, ID1, lon2, ID2, N, equip, diam1, diam2, nature`),
  `TB` (without the added `N`), `TC`, `TD` (8 persistent columns, no `N`), `TO` (`in_diam` only),
  `TFact` (`succ, designFlow, margin`), `TAH`, `TCP`, `TS`.
* `TPar` (array of `{name, value}`), `pardsg`, `partec`, `reinfcost` as objects. `dat` is not saved.

MATLAB `jsonencode` quirks to reproduce if byte equality is wanted (UNVERIFIED details): a one-row
table becomes an object, not an array. `Inf`/`NaN` become `null`. Logical → `true/false`. Numbers are
written with up to 15 significant digits. Pretty-print indentation is 2 spaces. Prefer a documented
Rust JSON layout and test semantic equality.

Reader `read_Dsg_json.m` restores `TA, TN, TC, TB, TD, TO, TPar, pardsg, partec` (diagnostic: `TB`
fixed). No reader exists for `TFact, TAH, TCP, TS, reinfcost`.

### 6.4 Sidecars

* `.inp` (EPANET) and `<nomv>_TableFusion.csv`: mentioned in `examples/tpo_examples/README.md` and in
  `makeReinfDsg.m:209-211` (`meta.traceFile`), but **no writer exists** in the snapshot.
* `.mat`: removed (`matlab/InOut/README.md`).

### 6.5 Run metadata (`makeReinfDsg.m:186-229`, `write_meta.m`)

`out/<nom>/runs/meta_<nom>_<datetime>.json`, where `<datetime>` is MATLAB's default `datetime("now")`
text (for example `20-Nov-2026 14:03:12`, with spaces and colons, UNVERIFIED locale). Pretty-printed JSON:

```
schemaVersion "NW_design_meta_v1", network, version, date ("yyyy-MM-ddTHH:mm:ssZ", local time
labelled Z), useFusion, nFusions, useSplit, nbsplit, vmax, secu, uniform_dem, bounded_flow, pressure,
solver {message, fval}[, traceFile]
```

Rust: use a portable file name (`meta_<nom>_<yyyymmddTHHMMSS>.json`) and a real UTC time. Record the
deviation.

### 6.6 Compatibility with NW4 `.dsg`

| Item | NW4 (`crates/neatwork-core/src/engine/make_design.rs`, `project/design.rs`) | NW5 output | Consequence |
| --- | --- | --- | --- |
| Node rows | 7 fields, heights relative to the source plus `hsource`, non-terminal orifices `0`, "no orifice" = `MAXDIAM` (10000.0 in files) | 7 fields, physical heights, non-terminal and "no orifice" = `1` | NW4 reads it, `1` means a 1 m orifice (no loss), equivalent in practice |
| Pipe rows | one row per arc, `ID2 = 0` when unused, status `N` | parallel rows for doubled links, `ID2 = fictif`, status `R` | NW4 cannot handle parallel pipes, `fictif` is unknown to NW4 (UNVERIFIED behaviour) |
| `!loadfactor` | `node, loadFactor, 0` (dimensionless, rounded `0.##`) | `succ, designFlow (L/s), margin (m)` | **semantic clash**: an NW4 reader takes `designFlow` as a load factor |
| `!Diameters` | IDs renumbered in order of use, `D001` first used | catalogue order (diameter ascending), plus `fictif` row with `diam = 0` | NW4 can divide by `diam = 0` (UNVERIFIED) |
| Extra sections | none | `!Clients, !Borne, !Hydraulics, !ComplementTA, !Costs, !ConstraintSatisfaction` | ignored by NW4 according to `docs/design/FormatPourNW5.md:55` (UNVERIFIED in the Rust NW4 reader) |

The NW4 engine of this repository must keep reading NW4 files unchanged. The NW5 reader must accept
both layouts (old NW5 `!loadfactor` with `pred, succ, lf` too, see `Toy_bbu…dsg`).

---

## 7. Unfinished parts, bugs and ambiguities

### 7.1 Blocking regressions in the snapshot

* **B1** `prepareReinforcementTables.m:134-137` and `equivResist.m:36-40`:
  `assert(all(abs(ecart) > 1e-7))` is inverted. Any input with an equipped arc whose segments sum to
  `lon` fails with `NW5:EquipmentLength`. `test_equivResist`, `test_reinforcement_integration` and the
  `ToyDevReinf.dsg` run fail. Intended (`Diagnostic_makeReinfDsg.md:114`, and the tests): accept
  `|lon1 + lon2 − lon| ≤ 1e-7`. Rust: use that test.
* **B2** `enrichTablesReinf.m:69` `TA.statut(a) = 0;` with undefined `a`: every call fails
  (`makeReinfDsg`, `test_physical_elevations`, `test_existing_pressure`,
  `test_reinforcement_integration`). Intended: remove the line (`Diagnostic_makeReinfDsg.md:115`). The
  tests expect the arc to keep its successor status.

The diagnostic (dated 2026-11-17/18) reports 8/8 tests passing under MATLAB R2025a before these two
edits. The test expectations are therefore the reference.

### 7.2 Other defects and ambiguities

* **B3** Default driver input `Toy2b.tpo` does not exist.
* **B4** 7-field `!Diameters` sections in the Toy `.tpo` files are misread (§1.3).
* **B5** Old `.dsg` files use `ID2 = 0`, rejected by `prepareReinforcementTables` (§1.4).
* **B6** Interactive dialogs (`promptParamDialog`, `prompt_pipelength`, `inputdlg` in `data4DsgFile`).
* **B7** Monte Carlo is unseeded. `TL` and `TLv` use independent draws. `TC` order differs between
  `.tpo` (sorted) and `.dsg` (file order) inputs.
* **B8** `use_cvar` is forced to 0 by the review, so CVaR is reachable only from the tests.
* **B9** Missing validations (diagnostic §1.6): `vmax > 0`, `nScenLoad` integer > 0, binary flags.
  `targetflow` defaults to 0 (gives flows of 1e-12 L/s and a trivial cheapest design). `opentaps`
  defaults to 1. `orifcoef` and `faucetcoef` default to 1.0, far from the NW4 values.
* **B10** For heterogeneous demand without a `!Borne` section, `TB.pressure` is the pre-review
  `pressure`. A reviewed `pressure` is not propagated (uniform demand overwrites it, §2.4).
* **B11** `ceil` in commercial rounding without tolerance (§5.1 F).
* **B12** Degenerate LPs (§4.6).
* **B13** `bounded_flow=0` with heterogeneous demand is rejected late (`build_reinf_model.m:218`,
  `processSol.m:275`) although the review allows it when each terminal has one client
  (`docs/forum/a_faire.md` §2: extension planned, not implemented).
* **B14** Reversing an arc in `peel_orient_tree` does not swap the segment order `lon1/ID1` ↔ `lon2/ID2`.
* **B15** Velocities in the final pipes (after mixture, junction removal and rounding) are not
  recomputed. Only the conservative candidate filter guarantees them. The final hydraulic split
  between parallel pipes is not computed (needs simulation, `ctx.reinforcement.requiresSimulationGraphRebuild`).
* **B16** Terminals with no client get no `TL` column. Their arc gets `Q = 0` unless it is on another
  client's path, so the cheapest admissible pipe is chosen ("équipement minimal",
  `build_incidence_loads.m:263-269`).
* **B17** The `.tpo` parser clamps `lon` to ≥ 0.01 m. The `.dsg` parser does not, and `processSol`
  requires `lon > 0`.
* **B18** `existing` resistance of an arc ignores the length of a `fictif` segment (`equivResist`
  divides by the whole `lon`, `test_equivResist` expects 4 for 25 m of resist 16 + 75 m fictif). Such
  arcs are rejected earlier by `prepareReinforcementTables`, so the case is only reachable in tests.

### 7.3 Fusion of degree-2 nodes and split (not implemented)

* `buildContext.m:1-26` creates `ctx.useFusion = fusion`, `ctx.changed = false`, an empty
  `ctx.TableFusion` with columns `noeud_supprime, extremite1, long1, extremite2, long2, lon_total`,
  `ctx.log = {}`, `ctx.split = split`.
* `fusion_noeuds_degre_2.m` is **not called** (`makeReinfDsg.m:145`, and the comment l.147-152:
  fusion and splitting are a deferred design decision. The Python/Rust port must not infer they are
  implemented). For the record, its algorithm (l.1-125): recompute `pmax = −min(z_pred, z_succ)` (old
  relative convention), `allowedPipes = buildAllowedPipes(TA,TD)` (`TA.pmax ≤ TD.pressure`). Repeat
  until no change: for the first node of degree 2 that is not the source, with exactly one incoming
  and one outgoing arc having identical allowed-pipe rows, replace the two arcs by one arc
  `pred → succ` of length `l1 + l2`, `pmax = max`, and `haut, statut, nbrob, pressure` of the
  downstream arc. Delete the node. Append a row to `TableFusion`. Finally sort nodes by
  `(statut, noeud)` and arcs by `(statut, succ)`.
* **No code undoes a fusion**: `TableFusion` is only counted in the metadata (`nFusions`) and named
  in `traceFile`. No function splits a merged arc back. A future implementation must cut each
  merged arc at `long1` and reassign the two pipe segments, which the current rounding does not
  guarantee to fall on the removed node.
* **Split** (Denis Gilbert's idea, `docs/design/types de clients.md:118-119`): create a node at the
  junction of two diameters and two arcs instead of one split arc. Not implemented. `processSol` sets
  `ctx.nbsplit = 0`, `ctx.split = false` (l.380-381).
* The Python port in `python/design/parserTPO.py:486-626` has a `fusion_nodes_degre_2` copy of the
  same dead algorithm. It is not a reference.

Rust: expose `fusion` and `split` as parameters that are accepted, forced to 0 and written to the
output, exactly like MATLAB. Do not implement them in phase 5.

---

## 8. Unit tests in `tests/matlab` (expected values for Rust tests)

How to run in MATLAB: `setup`, then the test name (`tests/matlab/README.md`). Toolboxes:
Optimization (solver tests), Statistics (binomial). The Rust tests must rebuild each fixture
in memory. Units: lengths m, flows L/s in tables, heights m.

### 8.1 `test_reinforcement_integration.m` (full chain without I/O)

Fixture (l.97-117), run for `source_height ∈ {0, 100}` (added to all `TN.haut`) and for permuted
input rows (`TA` rows [3 1 2], `TN` [4 2 1 3], `TD` [2 1], `TB` [2 1], `TC` [2 1]):

* Nodes `S, J, B1, B2`: `haut = [0, −50, −200, −15]`, `xdata = [0,1,2,2]`, `ydata = [0,0,1,−1]`,
  `statut = [0,1,2,2]`, `nbrob = [0,0,1,1]`.
* Arcs `S→J` 10 m bare. `J→B1` 20 m existing `D1` on 20 m. `J→B2` 20 m existing `D1` on 20 m.
  `lon2 = 0`, `ID2 = fictif`.
* `TB`: `B1, B2` pressure 9 (overwritten to 1 because uniform). `TD`: `D1` diam 0.1 cost 2 pressure
  1000 (`petit`), `D2` diam 0.2 cost 5 pressure 1000 (`grand`). `TC`: one client per terminal,
  proba 1, cible 1.
* `p`: `uniform_dem=1, bounded_flow=1, pressure=1, targetflow=1, opentaps=1, servicequal=0.95,
  vmax_secu=0.99, use_cvar=1, vmax=0.2, secu=1, pipelength=6, pump_pressure=0`.
* `t`: `lambda=2, nu=4, bescal=100, faucetcoef=1e-4, orifcoef=0.001`.
  So `resist(D1) = 1e6`, `resist(D2) = 62500`.

Expected (tolerance `1e-8·max(1,|expected|)`, l.119-123):

| Quantity | Expected |
| --- | --- |
| `TA.succ` order after enrichment | `J, B1, B2` |
| `TA.haut` | `[−50, −200, −15]` (for both source heights) |
| `TA.pmax` | `[50, 200, 50]` |
| `TA.pressure` | `[0, 1, 1]`, `TB.pressure = [1, 1]` |
| `TN.degree` (S, J, B1, B2) | `[1, 3, 1, 1]` |
| `M_path` | `[1 1 1; 0 1 0; 0 0 1]` |
| `TL{:, {B1, B2}}` | `[2 2; 1 0; 0 1]` (L/s) |
| `TL.ID` | `TA.succ` |
| `sol.y` | `[1; 0; 1]` |
| `sol.x_mat` (rows D1, D2, columns J, B1, B2) | `[0 0 1; 1 0 0]` |
| `sol.fval` | 90 |
| `sol.slack` | all ≥ −1e-6, and equal to `TAH.margin` |
| `processSol` output | 4 physical arcs, 4 nodes, physical `haut, xdata, ydata` preserved, 2 `old` + 2 `new` in `TCP` |
| `TAH.succ` | `J, B1, B2` |
| `TAH.equivResist` | `[62500, 1e6, 250000]` |
| `TAH.pathLoss` | `[2.5, 22.5, 7.5]` |
| `TAH.margin` = `TS.head_margin` | `[47.5, 176.49, 6.49]` |
| `TS.head_deficit` | `[0, 0, 0]` |
| `costOptim = costFinal = costPipesAfterRounding` | 90, `costFixed` 0 |
| arcs per successor in `TA_out` | `J`: 1, `B1`: 1, `B2`: 2, catalogue cost of `new` pipes 90 |

Hand check: trunk `10·62500·0.002² = 2.5`. Terminal requirement `1 + 0.001²/1e-4 = 1.01`.
`dmin(trunk) = 2·sqrt(0.002/(π·0.2)) = 0.1128 > 0.1`, so only D2 is allowed on `S→J`.

### 8.2 `test_build_reinf_model.m`

Fixture (l.127-150): `t = {lambda 2, faucetcoef 1e-6}`, `p = {vmax 1, targetflow 0.4, secu 1.25,
bounded_flow 1, uniform_dem 1}`; `TD = [fictif 0 m, pressure 0, cost 0, resist Inf. D1 0.05, 10, 2,
1e6. D2 0.2, 30, 5, 1e4]` (columns `ID, diam, pressure, cost, resist`), `cond = resist^(−1/2)`.
`TA` (succ `J, B1, B2`): `lon [10, 20, 30]`, `haut [−1000, −1500, −2000]`, `pmax [1, 20, 1]`,
`statut [0, 2, 2]`, `pressure [0, 2, 3]`, `N [N, N, B]`, `ID1 [fictif, D1, fictif]`,
`ID2 [fictif, fictif, D1]`, `lon1 [0, 20, 0]`, `lon2 [0, 0, 30]`.
`TL = {B1: [2, 0.5, 1], B2: [1, 1, 0]}`, `TLv = {B1: [10, 1, 0.5], B2: [3, 0.5, 1]}`.
`TR.rexist = [Inf, 1e6, 1e6]`, `TR.D_d = (rexist^(−1/2) + cond_d)^(−2)`.
`M_path = [1 1 1; 0 1 0; 0 0 1]`. Note `statut(J) = 0` in this fixture (non-terminal).

| Check | Expected |
| --- | --- |
| sizes | `nbA = 3, nbD = 2, nVar = 9`, `Aeq`, `Aineq` sparse 3 × 9 |
| `f` | `[20, 50, 40, 100, 60, 150, 0, 0, 0]` |
| `lb` | `[0×6, 1, 0, 0]` |
| `ub` | `[0, 1, 0, 1, 1, 1, 1, 1, 0]` (D1 too slow on J, D1 class 10 < pmax 20 on B1, B2 blocked) |
| `activ_variables` | `ub == 1` |
| `Aeq` | `[1 1 0 0 0 0 −1 0 0; 0 0 1 1 0 0 0 −1 0; 0 0 0 0 1 1 0 0 −1]`, `beq = 0` |
| `Aineq` | `[Aineq_x Aineq_y]` |
| Oracle (l.25-75) | for `(bounded, uniform) ∈ {(0,1), (1,0), (1,1)}`, λ ∈ {1.2, 1.781, 2} (with `TR` and `cond` recomputed), and `X ∈ {0, 0.5·1, [.2 .7 0; .8 0 .3]}` with `y = Σx`: `Aineq·z − bineq` = path loss − available, with loss `L_a·((1−y_a)·rexist⁺ + Σ_d x_{d,a}·TR_{a,d})·Qmax_a^λ` (`rexist⁺ = 0` if Inf) and available as in §4.5, `Aeq·z = 0`. Tolerance `1e-9·max(1,|·|)` |
| `vmax = 0.18`, `TLv = 0.5` everywhere, no block | `ub(1:6) = [0,1,0,1,0,1]`, `lb(7:9) = [1,1,1]` |
| `fixedCost = [7, 11, 13]` | `f = [f(1:6); 7; 11; 13]` |
| `TR` columns reordered `{ID, D2, rexist, D1}` | same `Aineq`, `ub` |
| errors | no `resist` → `NW5:MissingResistance`, no `cond` → `NW5:MissingConductance`, no or two `fictif` → `NW5:FictitiousPipe`, `TR` 2 rows → `NW5:TRSize`, rows permuted → `NW5:TRAlignment`, no `D1` column → `NW5:TRVariables`, `N(1) = B` → `NW5:ReinforcementBlocked` mentioning `J`, heterogeneous + unbounded → any error |
| MILP (intcon 7:9) | cost 50, `z = [0,1,0,0,0,0,1,0,0]` |

`checkMandatorySpeedReinforcement` (l.152-202), all `N = "N"`, `TLv(B1 row) = 2` (D1 carries 1.9635
L/s at 1 m/s): `lb(7:9) = [1,1,0]`. The old choice `z = [0,1,0,0,0,0,1,0,0]` still satisfies
`Aineq·z ≤ bineq + 1e-8` and `Aeq`. `N(2) = B` → `NW5:ReinforcementBlocked` mentioning `B1`. Existing
`D2` 19.9 m + `D1` 0.1 m on B1 (`rexist = (19.9·1e4 + 0.1·1e6)/20`) → `lb(8) = 1`. `D2` on the whole
20 m with `lon2 = 0 fictif` → `lb(8) = 0`. And with `N = B` → `lb(8) = ub(8) = 0`. `TLv(B2 row) = 2`
with the original `TA` → `lb(9) = 1`.

### 8.3 `test_solveModel.m`

Model (l.63-74): `nbA = 3, nbD = 2`, `f = [2,6,3,9,4,8,5,7,11]`,
`Aeq` as in §8.2 pattern, `Aineq = [1 0 0 0 0 0 0 0 0; 0 0 0 0 0 0 0 −4 0; 0 0 0 0 1 1 0 0 0]`,
`bineq = [0.25, −2, 0]`, `lb = [0×6, 1, 0, 0]`, `ub = [1×8, 0]`. Tolerance 1e-7.

| Case | Expected `[x_mat(:); y]` | Cost |
| --- | --- | --- |
| MILP (y2 free) | `[.25, .75, 1, 0, 0, 0, 1, 1, 0]` | 20 |
| LP (`lb(7:9) = [1,1,0]`) | same | 20 |
| `bineq(2) = 0` | `[.25, .75, 0, 0, 0, 0, 1, 0, 0]` | 10 |
| all y fixed at 0 | zeros | 0 |
| `Aineq(1,1:2) = [0 1]`, `bineq(1) = 1e-9`, `f(1:2) = [6, 2]` (LP) | `[1, 0, 1, 0, 0, 0, 1, 1, 0]` (x2 ≈ 1e-9 cleaned) | `new_fval = fval = 21` within 1e-6 |
| `Aineq(1,1:2) = [1 1]`, `bineq(1) = 0.5`, LP and MILP | error `NW5:DesignFailure`, message contains `Solver diagnostic` | none |

Also: `sol.ok`, `exitflag == 1`, non-empty message, `size(x_mat) = [2 3]`, `y` exactly 0/1,
`reinfArc = logical(y)`, `Σ x_mat = y`, `slack = bineq − Aineq·revised ≥ −1e-6`,
`fval = f'·z`.

### 8.4 `test_reducePipeMixture.m`

Cases (`test_reducePipeMixture.m:6-21`): `(x, ρ, cost)` =
`([.2,.5,.3], [10,5,1], [1,9,10])`, `([.2,.5,.3], [10,5,1], [1,3,10])`, `([.2,.5,.3], [5,5,5],
[7,2,8])`, `([.2,.5,.3], [1,2,3], [3,2,1])`, plus 100 random trials (`rng(42)`, n ∈ [3,15], `x`
normalised, `ρ` integers 1..20, `cost = 20·rand`) and the same with
`ρ_eq = (ρ^(−1/1.781) + 7^(−1/1.781))^(−1.781)`. Assertions: ≤ 2 non-zeros, ≥ 0, `Σ` kept to 1e-12,
`ρᵀx` kept to `1e-10·max(1,|target|)`, cost equal to the best feasible pair (exhaustive) to
`1e-10·max(1,|best|)`. Rust: same properties with any RNG (the expected values are computed inside).

### 8.5 `test_build_incidence_loads_cvar.m`

* One arc `B1` (statut 2), `M_path = 1`. Clients on B1: proba `[1, 0.3, 0.7]`, cible `[1, 2, 4]`.
  `uniform_dem = 0`, `nScenLoad = 101`.
* Without `use_cvar` and with `use_cvar = 0`, after `rng(42)`: identical `TL`.
* `use_cvar = 1`, α ∈ {0.5, 0.65, 0.95, 1}, after `rng(42)`: `TL.B1` = mean of the upper tail of
  mass `(1−α)·101` of the sorted scenario totals (whole part + fractional part of the next value).
  α = 1 → maximum. Within 1e-12. At α = 0.65 CVaR ≥ VaR. The expected value is computed in the test
  from the same draw `(rand(3,101) < proba).*cible` (column sums).
* All proba = 1, α = 0.95 → `TL.B1 = 7`.
* Uniform branch, `targetflow = 0.4`, `nbrob = n+1` for n ∈ {0, 1, 4, 20}, p ∈ {0.01, 0.3, 0.95, 1},
  α ∈ {0.5, 0.65, 0.95, 0.999, 1}: `TL.B1/0.4` = discrete upper-tail mean of `1 + Bin(n,p)` (α = 1 →
  `1 + n`), within 1e-9.
* Default (no `use_cvar`) equals `use_cvar = 0` for the uniform branch (`servicequal 0.65`, `opentaps 0.3`).
* `checkUniformProbabilityBounds`: arcs `J, B1, B2` (statut `[0,2,2]`, nbrob `[0,2,1]`),
  `M_path = [1 1 1; 0 1 0; 0 0 1]`, clients `B1, B1, B2` proba 1 cible 0.4, `targetflow 0.4`,
  `opentaps 1`: for VaR and CVaR, α ∈ {0.5, 0.95, 1}: `TL{:, {B1,B2}} = [1.2 1.2; 0.8 0; 0 0.4]`
  (1e-12). `opentaps = 0` → `NW5:LoadProbability`, also on a single-client network.

### 8.6 `test_equivResist.m`

λ ∈ {1.2, 1.781, 2}. `TD = [D0 fictif diam 0 resist Inf. D1 petit 0.1 resist 16. D2 grand 0.2
resist 4]`; arcs (succ, lon 100): `Z` bare (`absent` IDs), `A` `D0` on 100 m, `Y` `D1` on 100 m,
`B` `D1` 25 + `D2` 75, `X` `D1` 25 + `D0` 75, `C` `D2` on segment 2 (100 m).

* `TR` size 6 × 4, `TR.ID = TA.succ`, columns `{ID, rexist, D1, D2}`.
* Rows Z and A: exactly `TD.resist` (`[Inf, 16, 4]`).
* `rexist` of Y, B, X, C: `[16, 7, 4, 4]` (1e-12).
* `TR(a, d) = rexist·qe^λ` where `ratio = (resist_d/rexist)^(1/λ)`, `qe = ratio/(1+ratio)`. Equal to the
  candidate loss `resist_d·(1/(1+ratio))^λ`. And `< min(rexist, resist_d)`.
* Same result with `diam1/diam2` given, and with `TD.ID` as cell or categorical.
* Errors: `lon1(Y) = 99` → `NW5:EquipmentLength`. Unknown `ID1`/`ID2`. Missing `ID1`. Missing `diam`.
  `diam1 = NaN` → error.

### 8.7 `test_existing_pressure.m`

Fixture (l.72-79 of the function file): arcs `S→J` (bare, pmax 100, `N`), `J→B1` (`D30` on segment
1, 10 m, pmax 30, `N`), `J→B2` (`D20` on segment 2, 10 m, pmax 20, `B`). Catalogue `D20` 20 m,
`D30` 30 m (no `fictif`). Accepted as is (also without `N`, with categorical IDs, permuted rows).
Rejections (`NW5:ExistingPressure`, message must contain the listed texts): `pmax(B1) = 31` →
`J, B1, D30, 31 m, 30 m, troncon 1`. `pmax(B2) = 21` → `J, B2, D20, 21 m, 20 m, troncon 2`. Both
segments of B1 = 5 m and `pmax = 31` → both sections listed. Accepted: `ID2 = D20` on a 0 m segment.
all `pmax = −1`. A bare arc with an empty catalogue. Class `Inf`. Class `NaN` → `NW5:ExistingPressureData`.
Pump check (l.81-102): nodes `S 100, J 110, B 90`. Arcs `S→J` `P15`, `J→B` `P100`. Pump 0 → accepted.
pump 20 → `pmax(S→J) = 20`, rejected with `S, J, P15, 20 m, 15 m, troncon 1`. With `P15` class 20 → accepted.

### 8.8 `test_physical_elevations.m`

Nodes `S, J, B` with `haut = h + [0, 10, −10]`, h ∈ {100, 350}, pump ∈ {0, 20}, rows permuted or not:
after `enrichTablesReinf`, `TN` unchanged (`haut, xdata, ydata, statut`), `TA.haut(J, B) = [10, −10] −
pump`, `TA.pmax(J, B) = [0, 10] + pump` (1e-10). Export check: `data4DsgFile` with a stubbed dialog,
JSON and DSG, with and without `dat.source_height`: exported nodes equal the input nodes, including
`xdata = 500000.125…` and `ydata = 200000.25…` (1e-10), so coordinates must be written with ≥ 15
significant digits.

### 8.9 `test_processSol_smoke.m` (script)

Fixture: `TD = [fictif 0 m cost 0; D1 0.05 cost 5; D2 0.10 cost 8]`,
`bescal = 9.42e-4·1.27^(−0.219)`, ν 4.781, λ 1.781, `faucetcoef 1.83e-7`, `orifcoef 0.001`. Arcs `S→J`
60 m bare, `J→B1` 30 m existing D2, `J→B2` 42 m existing D2. `haut [−2, −5, −6]`, statut `[0, 2, 2]`.
`TL = {B1: [0.7, 0.7, 0], B2: [0.5, 0, 0.5]}`. `pardsg = {targetflow 0.4, pipelength 6, secu 1,
uniform_dem 1, bounded_flow 1}`; `sol.x_mat = [1 0.4 0. 0 0.6 0]`, `y = [1, 1, 0]`, `fval = 200`.

MATLAB assertions: 4 physical arcs, 2 old + 2 new, two arcs `J→B1`, no `fictif` in `ID1`, lengths
consistent, `equivResist(B1) < rexist(B1)`, `equivResist(B2) = rexist(B2)`, finite margins,
`TS.head_margin = TAH.margin`, `costFinal = costPipesAfterRounding`. A bare arc not reinforced →
`NW5:DisconnectedArc`. Existing pipe only on `lon2` (B2: `lon2 = 42 D2`) → kept as `lon1 = 0, lon2 = 42,
ID2 = D2`. All arcs bare, `x_mat = [1 1 1. 0 0 0]` → 3 arcs, all `ID1 = D1`.

Values derived for this specification (independent computation, not asserted by MATLAB, use as
additional Rust checks with tolerance 1e-9 relative): `resist(D1) = 1484.370440077769`,
`resist(D2) = 53.99056694027162`. B1: `rTarget = 26.119516262580962`, `ρT = 183.34169986313574`,
`η1(D1) = 0.09043131503181466`, `L1(D2) = 27.28706054904556` → rounded to 30, so the reinforcement of
B1 is a single D2 pipe on 30 m. `equivResist = [1484.370440077769, 15.710263367299225,
53.99056694027162]`; `pathLoss = [0.21419688800488, 0.21533039532629, 0.21719213579228]`.
`margin = [1.78580311199512, 3.91035266478300, 4.90849092431701]`. `costOptim = 504`,
`costInter = 531.8611816471366`, `costFinal = 540`.

### 8.10 `test_constraint_satisfaction.m`

Arcs `S→J, J→B1, J→B2`, 1 m each, existing `D1` (`rexist 1e5`), not reinforced (`y = 0`),
and, for the arcs in the order J, B1, B2: `haut = [−2, −10, 1]`, `statut = [0, 2, 2]`,
`pressure = [0, 1, 1]`. `TL.B = [1, 1, 1]`. `pardsg = {targetflow 0.4, pipelength
1, secu 1.2, uniform_dem 1, bounded_flow 1}`; `faucetcoef 1.83e-7`. Expected (1e-12):
`TS.noeud = [J, B1, B2]`. Types `[intrusion_prevention, demand_satisfaction, demand_satisfaction]`.
`head_available = [2, 10/1.2, −1/1.2] − [0.1, 0.2, 0.2]`. `head_required = [0, 1 + 0.0004²/1.83e-7,
same]`; `head_deficit = max(0, −head_margin)`. Margin(B1) > 0, margin(B2) < 0 (warning
`NW5:PressureAfterRounding`). JSON export contains `TS` (3 elements, `TS(3).noeud = 'B2'`). DSG
export contains `!ConstraintSatisfaction`. Both writers work without `TS`.

### 8.11 `test_client_data_precedence.m`

Parser fixtures (l.67-101): nodes `S 100 (statut 0), J 90 (1), B1 80 (2), B2 70 (2)`, pipes
`J→B2, S→J, J→B1` 100 m, catalogue `D1,P50,11,0.05,2,1000,1,0`, `opentaps 0.1`, `targetflow 9`,
`bounded_flow 1`, `pressure 2`. Expected: `nbrob(S, J, B1, B2) = [0, 0, 2, 1]` whatever the file
counts. Uniform clients (0.7, 0.6) → `uniform_dem = 1`, `opentaps = 0.7`, `targetflow = 0.6`.
heterogeneous clients `C1 (B1, 0.2, 1)`, `C2 (B1, 0.8, 0.4)`, `C3 (B2, 0.2, 0.5)` → `uniform_dem = 0`,
globals unchanged (0.1, 9). `.tpo` without clients → 3 clients, proba 0.1, cible 9. `.dsg` keeps
orifices `orif_ideal(B1, B2) = [0.004, 0.006]`, `orif_com = [0.005, 0.007]`. `.dsg` without
`!Clients` → error containing `ne contient pas de section Clients`. `checkSimulationAfterReview`
belongs to the simulation (`matlab/Simul/genscen.m`) and is out of scope here.

---

## 9. Differences from the NW4 Java design (already ported in `make_design.rs`)

| Topic | NW4 (`crates/neatwork-core/src/engine/make_design.rs`, `project/topographie.rs`) | NW5 |
| --- | --- | --- |
| Scope | new network only | reinforcement of an existing tree (bare network = special case), statuses `N/B/R`, MILP when some `y` is free |
| Load factor | normal approximation with a printed table (`calcul_proba`, `loi_normale.rs`), rounded to `0.##`, per node, computed by the GUI | exact binomial VaR with linear interpolation, or Monte Carlo per terminal for heterogeneous clients, optional CVaR, no rounding |
| Multiple clients per terminal | expanded into fictitious terminals linked by 1 m pipes | handled directly with `nbrob`/`TC` |
| Variables | pipe lengths in metres per (pipe, diameter), plus one slack per node | length fractions per (arc, real pipe) plus `y_a` |
| Objective | cost of lengths + `coeffM·Σ slack`, `coeffM = total length × last diameter cost` | cost of fractions × length (+ optional fixed cost on y) |
| Constraints | head loss − slack ≤ budget, length equality, budget `limitbudget` | head loss ≤ budget (no slack, infeasible → error), `Σx = y`, no budget |
| Terminal budget | `−z − q²·(1/faucetcoef + (orifcoef/orifice)^4)` (orifice term when an orifice is imposed) | `(−z)/secu − pressure·[bounded] − q²/faucetcoef·[uniform]` |
| Pressure filter | `−height(succ) < pressure_d` (strict, successor only, source at 0) | `pressure_d ≥ Hsource − min(z_pred, z_succ)`, pump included, physical heights |
| Velocity | none (only imposed min/max diameters) | `diam_d ≥ 2·sqrt(Qv/(π·vmax))` with `Qv` at `vmax_secu`, mandatory reinforcement when the existing pipe is too small |
| Imposed diameters/lengths | `imposdiam1/2`, `imposlength1`, `imposdiammin/max` | none |
| Head loss | `β·Q^p/D^q` per catalogue row | `bescal·D^(−ν)·Q^λ` (same form, global ν, λ, β) |
| Two-diameter rule | first two diameters with `x ≥ 0.1 m` in catalogue order | `reducePipeMixture` (lower convex hull, keeps resistance) |
| Commercial rounding | `get_length` + energy comparison (`decoupe`) | larger diameter length rounded **up** to a multiple of `pipelength` (old NW5: longer length to the **nearest** multiple) |
| Orifices (ideal) | `((orifcoef⁴q²)/(margin + (orifcoef/orifice)⁴q²))^(1/4)` when margin > 0 | `min(1, orifcoef·q^(1/2)·margin^(−1/4))` when margin ≥ 0.1, only if `uniform_dem=1, bounded_flow=0` (identical to NW4 with no prior orifice) |
| Orifices (commercial) | bracket in the sorted catalogue + `1e-6`, `1`, choose upper if `ideal ≥ √2·lower` or `lower·upper ≤ ideal²`, none → `MAXDIAM` | nearest in `d^(−4)` among catalogue ∪ {1}, none → 1 |
| Safety | none | `secu` divides the static head of terminals |
| Outputs | `.dsg` NW4 layout, diameters renumbered by use | §6, catalogue IDs kept, `fictif` row, diagnostics sections, JSON |
| Solver | MOSEK in Java, HiGHS in the Rust port (`lp.rs`) | MATLAB `linprog`/`intlinprog` (HiGHS-based by default since R2023b, UNVERIFIED for the runs) |

The NW4 port's `Lp` structure (`engine/lp.rs`) can host the NW5 LP, but the NW5 model needs integer
columns for the MILP case. Check that `lp.rs` exposes integrality before reusing it.

---

## 10. Proposed Rust module layout and order of work

1. `nw5::design::params`: `pardsg`/`partec` with defaults and the review rules of §1.2 (non-interactive).
2. `nw5::design::prepare`: catalogue + `fictif`, equipment checks (B1 fixed), orientation, enrichment
   (B2 fixed), existing-pressure check, `TR`, `M_path`.
3. `nw5::design::loads`: binomial VaR/CVaR, Monte Carlo VaR/CVaR with an MT19937 `res53` source.
4. `nw5::design::model`: LP/MILP build and HiGHS solve, clean-up of §4.6.
5. `nw5::design::post`: §5 (mixture, junction, rounding, margins, orifices, outputs).
6. `nw5::io`: `.dsg` writer (§6.2, `%.15g`), JSON, metadata.
7. Tests: §8 fixtures (all expected values above), then §5.3 golden comparisons.
