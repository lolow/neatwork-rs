# NW5 simulation engine: specification for the Rust port

Status: phase 1 of [../PLAN.md](../PLAN.md). Written 2026-11-20 from `reference/neatwork5` (commit `a129c37a`, see `reference/neatwork5/SOURCE_COMMIT`).

This document specifies the NeatWork 5 hydraulic simulation as implemented by the Python "python-core" package. A Rust implementation that follows it must reproduce python-core outputs within `|a - b| <= 1e-10 + 1e-8 * |b|` and must reproduce every integer output exactly (scenario sets, iteration counts, counts, statuses).

## 0. Conventions and verification status

### 0.1 Path abbreviations

| Abbreviation | Path (under `reference/neatwork5/`) |
| --- | --- |
| `sim/` | `application/python-core/app/neatwork/simulation/` |
| `app/` | `application/python-core/app/` |
| `tests/` | `application/python-core/tests/` |
| `matlab/` | `matlab/Simul/` |
| `examples/` | `examples/dsg_examples/` |

Citations use `file:line` and point at the source revision above.

### 0.2 Vocabulary

| Term | Meaning |
| --- | --- |
| node | Graph vertex. `statut` 0 = source (reservoir), 1 = transit (junction), 2 = terminal (borne, tap stand). |
| pipe / arc | Directed edge `pred -> succ`. It has up to two segments (`lon1`/`ID1`, `lon2`/`ID2`). |
| source pipe | A pipe whose `pred` is a source node. Status 0. |
| terminal pipe | A pipe whose `succ` is a terminal node. Status 2. |
| transit pipe | Any other pipe. Status 1. |
| client | A consumer attached to one terminal (`borne`). It has an opening probability `proba` and a target flow `cible` in L/s. |
| borne | A terminal node. The `bornes` table gives its required pressure in metres. |
| scenario | One random draw of open/closed clients. Only non-empty draws are kept. |
| NW4 mode | `uniform_dem = 1`, `bounded_flow = 0`: terminal flow depends on pressure and on the orifice and faucet losses. |
| bounded-flow mode | `bounded_flow = 1` ("irrigation" mode): terminal flow is capped by the demand. A fictitious terminal loss gives the required pressure at full demand. |

### 0.3 Verification done for this spec

- Every python-core file under `sim/` was read in full. The callers (`app/services/simulation_service.py`, `app/schemas/*.py`, `app/neatwork/domain/*.py`, `app/neatwork/parsers/dsg_parser.py`, `parsing_utils.py`) and the MATLAB originals in `matlab/` were read too.
- python-core was run headlessly on 8 example files (section 7). The SuiteSparse CHOLMOD library is not installed on this machine. A dense-Cholesky stand-in for `sksparse.cholmod` was used (section 7.2). These outputs agree with the CHOLMOD-generated regression fixtures. The worst relative difference is 2.2e-8 on a pressure of 6.9e-10 m, an absolute difference of 2.2e-16. Iteration statistics are identical.
- An independent clean-room reimplementation was written only from the facts in this spec. It does not import python-core, and it uses a pure-Python PCG64. It reproduced python-core on all 7 networks:
  - identical scenario matrices, draw counts and batch counts (also on a forced 4-batch run)
  - identical iteration counts for all 700 scenarios
  - flows within 3.5e-18 m3/s absolute
  - zero violations of `1e-10 + 1e-8*|b|` on every exported statistic.
- A second run with a reordered (reverse-permuted) Cholesky changed no integer output. Floating outputs moved by less than 2e-12 relative, except for quantities that are numerically zero (for example a minimum velocity of 1e-20 m/s).
- Statements marked **UNVERIFIED** were not checked by execution.

---

## 1. Inputs

### 1.1 Pipeline and entry points

```
DSG text --parse_dsg--> NeatworkDomainPayload --pydantic--> SimulationDomainInput
   --build_simulation_data--> SimulationData (5 pandas tables + param dict, validated)
   --Simulation.__init__--> prepare() (validation + HydraulicPreprocessor)
                         -> resolve_simulation_options / resolve_solver_options
                         -> ScenarioGenerator.generate()  (ScenarioSet)
                         -> ScenarioProblemBuilder
   --Simulation.run()--> SimulationBatchRunner.run() -> per scenario: build problem, InteriorPointSolver.solve
                         -> SimulationBatchResult
   --compute_stats()--> SimulationStatisticsProcessor.compute() -> SimulationStatistics
   --statistics_payload()--> SimulationJsonExporter.build_payload() -> JSON dict (schema 1.1)
```

There are two production entry points:

1. **Headless / tests** (`tests/regression/regression_helpers.py:16-41`): `parse_dsg`, then `SimulationDomainInput.model_validate(...)`, `build_simulation_data`, `Simulation(inst, options=SimulationRunOptions(scenario_count, seed))`, `run()`, `compute_stats()`, `statistics_payload()`.
2. **HTTP API** `POST /api/simulation/run` (`app/main.py:96`). It calls `run_hydraulic_simulation` (`app/services/simulation_service.py:382`), which adds runtime parameter overrides, edge open/closed overrides, a generated seed and UI aggregates. See section 5.5.

A legacy constructor `SimulationData(name, version, output, dossier, file_name)` (`sim/data.py:41-280`) reads a DSG file with its own parser. It is not used by the API or by the tests. Its differences are listed in 1.7. The Rust port must implement the domain path.

### 1.2 DSG reader (`app/neatwork/parsers/dsg_parser.py`, `parsing_utils.py`)

This section gives the parts that change simulation inputs. The full DSG format belongs in `specs/formats.md`.

1. **Lines** (`parsing_utils.py:3-8`): split on line breaks. Strip each line and remove a leading BOM. Drop empty lines and lines that start with `#`.
2. **Sections** (`parsing_utils.py:10-39`): a header is a line starting with `!`. The name is compared after `strip`, BOM removal, lowercasing, and replacing `_` and `-` with a space. A section runs from the line after its header to the next `!` line, or to the end of the file. The first matching header wins: the reader returns at the next header after a match. A missing section yields `[]`.
3. **Rows** (`parsing_utils.py:41-71`): split on `,` (no quoting rules) and strip each cell. **A row with fewer cells than the expected column count is silently dropped.** Each cell goes through `to_number` (`parsing_utils.py:44-56`):
   - empty gives `""`
   - otherwise `float(text)`. An integral value is returned as `int`, any other value as float
   - if `float()` fails, the text is kept.

   Consequence: numeric-looking identifiers are normalized, for example `"007"` becomes `7` and then `"7"`, `"1.50"` becomes `"1.5"`, and `"1e3"` becomes `"1000"`. The Rust port must apply the same normalization to **every** identifier field (node, pipe endpoint, client, borne, diameter ID). Otherwise sorting and mapping change.
4. Sections and columns used (`dsg_parser.py:54-87`):

| Section | Columns (in order) | Required cells |
| --- | --- | --- |
| `!Nodes` | `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut` | 7 |
| `!Pipes` | `pred, succ, lon, lon1, ID1, lon2, ID2, N` | 8 |
| `!Clients` | `client, borne, proba, cible` | 4 |
| `!Diameters` | `ID, Nominal, SDR, diam, cost, Pmax, Type, Roughness, N` | 9 |
| `!Borne` | `borne, pressure, N` | 3 |
| `!Default properties` | `name, value` (rows with at least 2 cells) | 2 |

   - The section name is `!Borne`, not `!Bornes`. If `!Borne` is missing, the parser builds rows with keys `outlet, elevation_m, ...` (`dsg_parser.py:194-208`). These rows **fail** the `SimulationBorneInput` model: `required_pressure_m` is missing. A DSG without `!Borne` therefore cannot be simulated through the domain path. This was verified on `ToyXlx.dsg`, `Toy2b_NW4.dsg` and `CeibaNW4.dsg`, which all fail with a pydantic error.
5. **Parameters** (`dsg_parser.py:23-47, 84-87`): `params = {**DSG_DEFAULT_PARAMS, **file_params}`. The DSG defaults differ from the core defaults:
   - **`"lambda": 0`**: a DSG without `lambda` fails validation with `INVALID_POSITIVE_PARAMETER`
   - the key `"bounded-flow"` (hyphen)
   - the key `"serviceequal"`.

   File values come later in dict order, so they win after name normalization (1.4.1).
6. Pipe edge IDs: `A1, A2, ...` in file row order (1-based). This is done by `_build_pipes_table` (`sim/domain_adapter.py:168`) because DSG pipes carry no ID.
7. Clients are converted into reusable "behaviours" (`app/neatwork/domain/client_behavior.py:31-100`). Each `(proba, cible)` pair gets `behaviorId = "CB_" + SHA1("<proba>|<cible>|")[:10].upper()`, where the numbers use canonical decimal formatting. Clients then reference `behaviorId`. `materialize_effective_clients` (`client_behavior.py:102-151`) resolves them back to `(client, borne, proba, cible)` rows in the original order. The values are unchanged. The Rust port can skip the behaviour indirection for simulation. It is needed only for the `demand` block of the API payload (5.5).
8. Pydantic conversion (`app/schemas/simulation_domain.py`):
   - Identifiers are `str(value).strip()` and must not be empty (`:18-27`).
   - `ID1`/`ID2` values in `{None, "", 0, "0"}` become `None` (`:189-195`).
   - `Type` becomes a string (`:336-347`).
   - Pipes are open unless `enabled == False` or `operational_status`, after `strip().upper()` and replacing `-` and space with `_`, is one of `CLOSED, DISABLED, INACTIVE, OFF, OUT_OF_SERVICE` (`:166-187`). DSG pipes have neither field, so they are open.

### 1.3 Domain model (`SimulationDomainInput`, `app/schemas/simulation_domain.py:412-457`)

All models accept extra fields and strip whitespace from strings (`:36-40`). The accepted aliases are listed because JSON projects use them.

| Table | Field (aliases) | Type, default | Unit / meaning |
| --- | --- | --- | --- |
| nodes | `node_id` (`nodeId`, `noeud`, `id`) | str, required | |
| | `elevation_m` (`elevationM`, `haut`) | float, required | m, the datum of the hydraulic head |
| | `x` (`xdata`), `y` (`ydata`) | float, 0 | layout only |
| | `ideal_orifice_m` (`idealOrificeM`, `orif_ideal`) | float, 0 | ideal orifice diameter, m |
| | `commercial_orifice_m` (`commercialOrificeM`, `orif_com`) | float, 0 | commercial orifice diameter, m |
| | `status` (`statut`) | int, required | 0 / 1 / 2 |
| pipes | `edge_id` (`edgeId`, `external_edge_id`, `id`) | str or None | if None: `A{index+1}` |
| | `predecessor_id` (`pred`, `sourceId`), `successor_id` (`succ`, `targetId`) | str, required | |
| | `length_m` (`lon`, `length`) | float or None | m |
| | `first_length_m` (`lon1`) | float or None | m. None gives `length_m`, then 0 |
| | `first_diameter_id` (`ID1`), `second_diameter_id` (`ID2`) | str or None | catalogue references |
| | `second_length_m` (`lon2`) | float, 0 | m |
| | `enabled` (`is_open`, `isOpen`), `operational_status` (`status`, `edgeStatus`) | bool True / str None | open state |
| clients | `client_id` (`client`), `terminal_id` (`borne`) | str | |
| | `behavior_id` | str or None | references `client_behaviors` |
| | `open_probability` (`proba`), `target_flow_lps` (`cible`) | float or None | legacy inline demand. Probability is in [0,1], flow in L/s |
| client_behaviors | `behavior_id`, `open_probability` (in [0,1]), `target_flow_lps` (>= 0), `profile_id`, `name` | | |
| bornes | `terminal_id` (`borne`), `required_pressure_m` (`pressure`) | str, float required | m |
| diameters | `diameter_id` (`ID`), `nominal_size` (`Nominal`) | str, required | catalogue key and label |
| | `inner_diameter` (`diam`) | float, required | inner diameter, m |
| | `cost` (0), `sdr` (0), `maximum_pressure` (`Pmax`, 0) | float | not used by the simulation |
| | `type_code` (`Type`, "0"), `roughness` (`Roughness`, 0) | str / float | not used by the simulation |
| domain | `name`, `version` (""), `source_format` ("DSG", uppercased), `params`, `technical_params`, `design_params` (dicts), `tables` | | |

### 1.4 Domain to `SimulationData` (`sim/domain_adapter.py:262-302`)

Steps, in order:

1. `resolve_client_demand_profile` (`app/neatwork/domain/client_demand.py:19-53`):
   - Materialize the clients. These conditions raise a plain `ValueError`: an empty list, a probability outside [0,1], a target `<= 0`.
   - `uniform = (len(set(probas)) == 1 and len(set(targets)) == 1)`, using exact float equality.
   - If uniform, `open_probability = probas[0]` and `target_flow_lps = targets[0]`.
2. `synchronize_client_demand_params` (`client_demand.py:55-66`): `params["uniform_dem"] = 1 if uniform else 0`. If uniform, also `params["opentaps"] = p` and `params["targetflow"] = q`. **The client table overrides the file's `opentaps`/`targetflow`.**
3. Build the 5 tables (`domain_adapter.py:111-259`):
   - nodes: `noeud, haut, xdata, ydata, orif_ideal, orif_com, statut`
   - pipes: `external_edge_id, pred, succ, lon, lon1, ID1, lon2, ID2, is_open, N`, where `lon1 = first_length_m ?? length_m ?? 0` and `lon = length_m ?? lon1 + lon2`
   - clients: `client, borne, proba, cible`
   - bornes: `borne, pressure, N`
   - diameters: `ID, Nominal, SDR, diam, cost, Pmax, Type, Roughness, N`.
4. `normalize_simulation_params` (`domain_adapter.py:77-108`): see 1.4.1.
5. Uniform rule, again on the built client table (`domain_adapter.py:285-291`): if `proba.nunique() != 1 or cible.nunique() != 1`, set `uniform_dem = 0` **and force `bounded_flow = 1`**. Otherwise set `uniform_dem = 1`. NaN does not occur here because step 1 checked the values.
6. `SimulationData.from_tables(...)` (`sim/data.py:79-110`): `param = {**DEFAULT_SIMULATION_PARAMS, **params}`, then `validate()` (section 6.1).

#### 1.4.1 Parameter normalization

`raw = technical_params ∪ design_params ∪ params`, with later sources winning. The result starts from `DEFAULT_SIMULATION_PARAMS`. For each `(name, value)` in raw, in insertion order:

- skip `None`
- name: apply `strip`, then replace `-` and space with `_`. Insert `_` before every uppercase letter that is not the first character. Collapse `__`, lowercase, then apply `PARAMETER_ALIASES` (`sim/constants.py:62-79`)
- value: `bool` becomes 0/1. Int and float are kept. Anything else becomes `float(str(v).strip())`, and an error raises `ValueError`
- names in `INTEGER_SIMULATION_PARAMS` (`bounded_flow, fusion, nbsim, orif_com, split, uniform_dem, use_efficient_pipes`) become `int(v)`, which truncates. All others become `float(v)`.

Unknown names pass through. For example `nScenLoad` becomes `n_scen_load`.

#### 1.4.2 Parameter table

Defaults are from `sim/constants.py:22-46`. "Used" says where the value reaches the simulation numerics. Parameters marked "no" are only checked or reported. They have no effect on simulation results.

| Name | Default | Unit | Type | Used |
| --- | --- | --- | --- | --- |
| `bescal` | 8.9396e-4 | SI | float | beta (1.6, step 5) |
| `nu` | 4.781 | none | float | beta exponent of the diameter |
| `lambda` | 1.781 | none | float | friction exponent of the flow (4.1). **DSG default is 0** |
| `faucetcoef` | 1.0 | m5 s^-2 (the loss coefficient `1/faucetcoef` is in s2 m^-5) | float | NW4 faucet loss `1/faucetcoef` |
| `orifcoef` | 1.0 | none | float | NW4 orifice loss `(orifcoef/d)^4` |
| `orif_com` | 1 | flag | int | NW4 mode: 1 uses `orif_com`, 0 uses `orif_ideal` |
| `bounded_flow` | 0 | flag | int | mode selection (4.2) |
| `uniform_dem` | 1 | flag | int | mode selection. Forced from the client table |
| `opentaps` | 1 | probability | float | uniform mode: opening probability of every client |
| `targetflow` | 0.0 | L/s | float | uniform mode: target per client. Also the unbounded satisfaction denominator |
| `nbsim` | 100 | count | int | scenario count. The run options can override it |
| `pressure` | 2 | m | float | **not** used by the solver. The borne table is used (4.2). Checked >= 0. The API override copies it into every borne |
| `servicequal`, `vmax`, `vmax_secu` | 0.65, 1.5, 0.95 | | float | no (only checked) |
| `secu`, `seuil`, `pipelength`, `limitbudget`, `watertemp`, `fusion`, `split`, `use_efficient_pipes` | 1.0, 0.0, 6.0, inf, 20.0, 0, 0, 0 | | | no (design or legacy parameter) |

Solver options come from `SimulationData.options` (`sim/data.py:327-343`). They are fixed defaults with no input path:

| Key | Value | Field (`SolverOptions`, `sim/models.py:193-212`) |
| --- | --- | --- |
| `tolx` | 1e-8 (m3/s) | `flow_tolerance` |
| `tolpress` | 1e-6 (m) | `pressure_tolerance` |
| `tolcent` | 1e-10 | `centering_tolerance` (checked but **unused**) |
| `maxiter` | 30 | `max_iterations` |
| `sigma` | 0.1 | `barrier_reduction` |
| `tau` | 2.5 | `centering_exponent` |
| `a_secur` | 0.999 | `step_safety` |
| `tolpseudoA` | 1e-10 | not read |

`resolve_solver_options` (`sim/options.py:207-306`) checks these values:

- `flow_tolerance`, `pressure_tolerance`, `centering_tolerance` and `centering_exponent` must be finite and > 0
- `max_iterations` must be > 0
- `0 < barrier_reduction < 1` and `0 < step_safety < 1`.

A violation raises `INVALID_SOLVER_OPTION`.

### 1.5 Units

- All user-facing flows are in **L/s**: `cible`, `targetflow`, demands, and the `sourceFlow` statistics.
- The solver works in **m3/s**: `lps_to_m3s(v) = v / 1000.0` and `m3s_to_lps(v) = v * 1000.0` (`app/neatwork/hydraulic_units.py`).
- Conversion points:
  - demand to m3/s at `sim/problem_builder.py:221`
  - target flow to m3/s at `sim/batch_runner.py:119`
  - source flows back to L/s at `sim/statistics.py:267`
  - mean edge flow to L/s in the API at `simulation_service.py:235`.
- Lengths, diameters, elevations, pressures and heads are in **m**. Velocities are in m/s. Satisfaction is a ratio.
- Head loss on a pipe: `beta * |q|^lambda` in m. Terminal loss: `alpha * q^2` in m.

### 1.6 Hydraulic preprocessing (`sim/preprocessor.py`)

`SimulationData.prepare()` (`sim/data.py:300-325`) runs `validate()` again, then `HydraulicPreprocessor(self).prepare()` on deep copies (`preprocessor.py:34-56`):

1. **Type normalization** (`:58-162`):
   - IDs: `str(...).strip()`
   - numbers: `pd.to_numeric`
   - `statut`: int
   - `ID1`, `ID2`: string, so `None` becomes `"None"`. This has no effect: an ID is mapped only for a segment length > 0
   - `is_open`: bool, or a string in `{TRUE,1,YES,OPEN,ENABLED,ON}` / `{FALSE,0,NO,CLOSED,DISABLED,OFF,INACTIVE}`, else `ValueError`. If the column is missing, all pipes are open.
2. **Pipe status** (`:164-186`): default 1. For a `pred` that is a source node, set 0. Then, for a `succ` that is a terminal node, set 2. Status 2 wins. Validation forbids source-to-terminal pipes (6.1).
3. **Gravity term** (`:189-220`): `gravit = -haut(pred)` on source pipes, `+haut(succ)` on terminal pipes, `0` otherwise.
4. **Diameters and costs** (`:222-292`):
   - for `lon1 > 0`: `diam1 = diam(ID1)` and `cost1 = cost(ID1)`. For `lon1 == 0`: `diam1 = 1.0` and `cost1 = 0`
   - the same rule applies to `diam2` and `cost2` with `lon2`
   - errors (plain `ValueError`): non-finite values, or an active diameter `<= 0`.
5. **Beta** (`:294-339`), with pandas evaluation order `(bescal * lon) / (diam ** nu)`:
   ```
   beta = 0
   if lon1 > 0: beta  = bescal * lon1 / diam1^nu
   if lon2 > 0: beta += bescal * lon2 / diam2^nu
   ```
   A beta that is not finite or is `<= 0` raises an error. Use `powf` (C `pow`), not `powi`.
6. `total_cost = lon1*cost1 + lon2*cost2` (`:342-352`). The sum is returned as `totcost` but is unused by the simulation.
7. **Static terminal alphas** (`:354-443`). For terminal pipes, using the orifice diameters of `succ`:
   ```
   faucet_alpha = 1/faucetcoef if faucetcoef > 0 else 0
   alpha_ideal = (orifcoef / orif_ideal)^4 + faucet_alpha   if orif_ideal > 0 else 0
   alpha_com   = (orifcoef / orif_com)^4   + faucet_alpha   if orif_com   > 0 else 0
   ```
   - The `^4` is `numpy.power(x, 4)`, which calls C `pow(x, 4.0)`. Use `powf(4.0)`. `powi(4)` can differ by 1-2 ulp.
   - Non-terminal pipes get 0.
   - An optional pipe column `ecart` gives `alpha = ecart / (targetflow/1000)^2`. Otherwise `alpha = alpha_ideal`. The domain path never builds `ecart`, and the static `alpha` column is not used by the problem builder.
8. **Sorting** (`:445-476`), all stable:
   - nodes by `(statut, noeud)`
   - pipes by `(statut, succ, pred, external_edge_id)`
   - clients by `(borne, client)`
   - bornes by `borne`
   - diameters by `(diam, ID)`.

   Strings compare by Unicode code point, so `"A10" < "A2"` and `"C10P2" < "C1P2"`. **This order defines every vector in the engine, and the order of the random draws (3.2).**
9. **Incidence matrix** (`:478-543`): `M` is `nbNtra × nbA`. Rows are the transit nodes in sorted order, columns the pipes in sorted order. `M[pred, j] = -1` and `M[succ, j] = +1`, only where the endpoint is a transit node. Source and terminal nodes have no row.
10. **Solver data** (`:545-592`):
    - masks `idNter/idNtra/idNsrc` (nodes) and `idAter/idAtra/idAsrc` (pipes) with their counts
    - `gravite`, `beta`, `alpha`
    - ID arrays: `node_ids`, `edge_ids`, `source/transit/terminal_node_ids`, `source/transit/terminal_edge_ids`.
11. **Consistency checks** (`:594-651`, plain `RuntimeError`):
    - the incidence shape
    - array lengths
    - `nbAter == nbNter == len(bornes)`
    - `nbAsrc > 0`
    - finite `gravite/beta/alpha`.

Because terminals have exactly one incoming pipe (6.1), sorted terminal pipes are aligned one-to-one with sorted terminal nodes. The problem builder checks this (`sim/problem_builder.py:102-142`).

### 1.7 Legacy loader differences (`sim/data.py:128-280`)

Not used by the API or the tests. Listed for completeness:

- it reads `!Bornes` (plural). For a missing section, the pressure of every borne is `param['pressure']`
- it uses `float()` for all parameters (no aliasing)
- it uses the core defaults (`lambda` 1.781)
- the uniform rule is the same, but it does **not** override `opentaps`/`targetflow` from the clients.

---

## 2. Options

### 2.1 Run options (`SimulationRunOptions`, `sim/models.py:30-46`)

| Field | Default | Meaning |
| --- | --- | --- |
| `scenario_count` | None, which gives `int(param["nbsim"])` | number of non-empty scenarios to generate and solve |
| `seed` | None | PCG64 seed (3.1). None means OS entropy, which is not reproducible |
| `open_taps_probability` | None | **ignored** in uniform mode (the param is used). In non-uniform mode a value is an error |
| `target_flow_lps` | None | same rule as `open_taps_probability` |
| `max_generation_batches` | 20 | scenario-generation batch limit |
| `oversampling_factor` | 1.10 | draw oversampling. Must be > 1 |
| `max_draws_per_batch` | 100000 | cap on the draws per batch |

`Simulation(inst, seed=..., options=...)` (`sim/engine.py:56-136`): passing both `seed` and `options` raises `ValueError`. `seed` alone means `SimulationRunOptions(seed=seed)`.

### 2.2 Resolution (`resolve_simulation_options`, `sim/options.py:39-204`)

```
uniform      = bool(int(param.get("uniform_dem", 1)))
scenarios    = requested.scenario_count ?? int(param.get("nbsim", 100))
if uniform:
    p = float(param.get("opentaps", 1.0))           # requested.open_taps_probability NOT consulted
    q = _resolve_uniform_target_flow(inst)          # options.py:16-36
        # param targetflow if > 0, else the single positive finite distinct client 'cible', else None
else:
    p = q = None   # and error if requested.open_taps_probability / target_flow_lps is given
```

All issues are collected, then raised together as `SimulationValidationError`. The issue codes are in 6.2.

`Simulation` then builds the runtime parameter map `params = {**inst.param, nbsim: scenarios, opentaps: p (if not None), targetflow: q (if not None)}` (`sim/engine.py:87-100`). The problem builder reads `uniform_dem`, `bounded_flow`, `orif_com` and `lambda` from it.

### 2.3 Simulation modes

| `uniform_dem` | `bounded_flow` | Case (design doc) | Supported | Terminal model |
| --- | --- | --- | --- | --- |
| 1 | 0 | Case 1, NW4 | yes | alpha = orifice and faucet loss / n_open^2. Flow bound 20 × demand |
| 0 | 1 | Case 2, irrigation | yes | alpha = P_req / d^2. Terminal flow <= demand |
| 1 | 1 | Case 3 | yes (same as case 2) | same as case 2 |
| 0 | 0 | Case 4 | **no**: validation `UNSUPPORTED_SIMULATION_CASE`, and the builder raises an error | none |

Because of 1.4 step 5, non-uniform clients always force `bounded_flow = 1`.

### 2.4 Scenario types

python-core implements **only Monte-Carlo random scenarios** (section 3). There is no tap-by-tap or hand-made scenario mode in NW5 (python-core, MATLAB `matlab/` or the older Python port). The NW4 Java scenario types remain the NW4 engine's concern.

### 2.5 API options (`SimulationOptionsInput`, `app/schemas/simulation_options.py:21-125`)

Extra fields are forbidden. The aliases are in the source. The fields are:

- `scenario_count` (`scenarioCount`, `numberOfScenarios`, `nbsim`): > 0. **`to_core_options` uses `scenario_count or 100`**, so the API ignores the file's `nbsim` (`:118`).
- `seed` (>= 0): if missing, the service draws `secrets.randbelow(2**32)` (`simulation_service.py:133-151`) and reports it.
- `open_taps_probability`, `target_flow_lps`: legacy. They are not applied: the core receives the client-table values.
- `orifice_mode` (`COMMERCIAL`/`IDEAL`): sets `orif_com` to 1/0.
- `bounded_flow` (bool).
- `service_quality`: metadata only.
- `required_pressure_m`: sets `params.pressure` **and every borne's `required_pressure_m`**.
- `orifice_coefficient` (> 0): sets `orifcoef`.
- `faucet_coefficient` (>= 0): sets `faucetcoef`.
- `water_temperature_c`, `commercial_pipe_length_m`: metadata only.
- `max_generation_batches`, `oversampling_factor`, `max_draws_per_batch`.

The overrides are applied to a copy of the domain before the demand profile is resolved (`simulation_service.py:302-343, 356-366`). Edge overrides `{edgeId, enabled}` set `enabled` and `operational_status` OPEN/CLOSED (`:43-131`). The errors `DUPLICATE_EDGE_ID`, `DUPLICATE_EDGE_OVERRIDE` and `UNKNOWN_EDGE_OVERRIDE` are raised as `SimulationValidationError`.

---

## 3. Scenario generation (`sim/scenario_generator.py`)

### 3.1 Random number generator: must be reproduced bit for bit

`rng = numpy.random.default_rng(seed)`. This is `Generator(PCG64(SeedSequence(seed)))`. `rng.random(shape)` produces float64 values in [0,1). The Rust port must implement numpy's SeedSequence, PCG64 (XSL-RR 128/64) and `next_double` exactly. The pure-Python code below matches numpy 2.5.4 bit for bit. The test used 1000 draws for each of the seeds 0, 1, 42, 12345, 2^32-1, 2^32, 2^40+7 and 2^64+3.

```python
M32 = 0xFFFFFFFF
INIT_A, MULT_A, INIT_B, MULT_B = 0x43b0d7e5, 0x931e8875, 0x8b51f9dd, 0x58f38ded
MIX_L, MIX_R, XSHIFT, POOL = 0xca01f9dd, 0x4973f715, 16, 4
def seed_words(seed):                 # non-negative int -> little-endian uint32 words; 0 -> [0]
    if seed == 0: return [0]
    w = []
    while seed: w.append(seed & M32); seed >>= 32
    return w
def seedseq_pool(seed):               # SeedSequence.mix_entropy, empty spawn_key
    ent = seed_words(seed); h = [INIT_A]
    def hashmix(v):
        v = (v ^ h[0]) & M32; h[0] = (h[0] * MULT_A) & M32; v = (v * h[0]) & M32; return v ^ (v >> XSHIFT)
    def mix(x, y):
        r = (MIX_L * x - MIX_R * y) & M32; return r ^ (r >> XSHIFT)
    m = [hashmix(ent[i] if i < len(ent) else 0) for i in range(POOL)]
    for s in range(POOL):
        for d in range(POOL):
            if s != d: m[d] = mix(m[d], hashmix(m[s]))
    for s in range(POOL, len(ent)):
        for d in range(POOL): m[d] = mix(m[d], hashmix(ent[s]))
    return m
def generate_state_u64(pool, n):      # SeedSequence.generate_state(n, uint64)
    h = INIT_B; out32 = []
    for i in range(2 * n):
        v = pool[i % POOL]; v ^= h; h = (h * MULT_B) & M32; v = (v * h) & M32; v ^= v >> XSHIFT; out32.append(v)
    return [out32[2*i] | (out32[2*i+1] << 32) for i in range(n)]
MULT = 0x2360ED051FC65DA44385DF649FCCF645; M128 = (1 << 128) - 1
class PCG64:
    def __init__(self, seed):
        v = generate_state_u64(seedseq_pool(seed), 4)
        initstate = (v[0] << 64) | v[1]; initseq = (v[2] << 64) | v[3]
        self.inc = ((initseq << 1) | 1) & M128; self.state = 0
        self.step(); self.state = (self.state + initstate) & M128; self.step()
    def step(self): self.state = (self.state * MULT + self.inc) & M128
    def next_u64(self):                # step first, then output from the new state
        self.step(); hi, lo = self.state >> 64, self.state & ((1 << 64) - 1)
        x = hi ^ lo; r = hi >> 58
        return ((x >> r) | (x << ((64 - r) & 63))) & ((1 << 64) - 1)
    def random(self): return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)
```

Test vectors for seed 42:

- the 4 state words are `0x9f1e2e6dcd540ab7, 0xd57873dc79fb94b6, 0x7d282a1b64d420b7, 0x336579714692d5ff`
- the first `next_u64` is `0xc621fbcd16d92688` (= 14276969152011380360)
- the first three doubles are `0.7739560485559633, 0.4388784397520523, 0.8585979199113825`.

numpy raises `ValueError: expected non-negative integer` for a negative seed. The API restricts `seed >= 0`.

### 3.2 Algorithm (`ScenarioGenerator.generate`, `:93-227`)

Inputs:

- the client rows **in preprocessed order**, sorted by `(borne, client)`
- `terminal_ids = dat["terminal_node_ids"]`, terminals sorted by ID
- per-client vectors:
  - uniform mode: `prob[c] = p` and `target[c] = q` for every client (`:35-39`)
  - otherwise the client's `proba` and `cible`.

```
C = number of clients; N = scenario_count
if C == 0: ScenarioGenerationError("The network contains no clients.")
vp = 1.0 - prod(1.0 - prob)                    # numpy product over the 1-D array
if vp <= 0: ScenarioGenerationError(details.validScenarioProbability)
rng = PCG64(seed); open = bool[C, N] all false
got = 0; batches = 0; draws = 0
while got < N:
    batches += 1
    if batches > max_generation_batches: ScenarioGenerationError(... generationBatches = batches-1)
    rem  = N - got
    d    = int(ceil(oversampling_factor * rem / vp))      # (1.1*rem)/vp, evaluated left to right
    d    = min(max(rem, d), max_draws_per_batch)
    draws += d
    R    = rng.random((C, d))      # C-ORDER: R[c, j] is draw number c*d + j of this batch
    S    = R < prob[c]              # strict <, per client row
    S    = S[:, any(S, axis=0)]     # keep non-empty columns, in order
    take = min(columns(S), rem)
    if take == 0: continue          # the RNG stream has still advanced by C*d draws
    open[:, got:got+take] = S[:, :take]; got += take
```

Then:

- `demand_lps[t, k] = Σ_{clients c of terminal t} open[c,k] * target[c]`. The sum runs in client order starting from 0.0, as in a CSR matvec.
- `count[t, k] = rint(Σ open[c,k])`.
- `total_demand_lps[k] = Σ_t demand_lps[t,k]`. It is used only in batch metadata.
- Clients whose `borne` is not a terminal raise `ScenarioGenerationError(unknownTerminalIds)`.
- Metadata: `scenarioCount`, `seed`, `validScenarioProbability = vp`, `generatedDrawCount = draws`, `generationBatchCount = batches`, `terminalIds`, `demandUnit = "L/s"` (`sim/engine.py:120-136`).

What must be bit-exact: the boolean matrix `open`, hence `count`, and the per-terminal demand sums. Rounding in `vp` changes the draw count only for a value of `1.1*rem/vp` exactly on an integer boundary. This is extremely unlikely. Compute `vp` as a left-to-right product all the same. numpy's product of a short contiguous vector is a sequential loop (**UNVERIFIED** for vectors longer than 8, where SIMD unrolling can reorder).

Reference counts with seed 42 and N=100:

| Network | Draws | Batches |
| --- | --- | --- |
| Toy p=0.6 × 7 clients | 111 | 1 |
| Toy_Secu | 123 | 1 |
| Toy_Borne | 151 | 1 |
| Toy_bbd with seed 7 and `max_draws_per_batch=30` | 102 | 4 |

### 3.3 Statistical equivalence with MATLAB

MATLAB uses `rand` (Mersenne Twister, global state) and the file order of the clients. Its scenario sets are different random draws, so MATLAB and python-core can only be compared statistically. The Rust port targets python-core and must be bit-exact with it.

---

## 4. Per-scenario optimization problem and IPM solver

### 4.1 Model

Use the open pipes of the scenario, in sorted pipe order: n variables `x` (flow, m3/s, positive in the `pred -> succ` direction). `A` is the incidence matrix restricted to the open columns (all transit rows are kept). The problem is:

```
min  f(x) = Σ_j [ beta_j |x_j|^(λ+1)/(λ+1) + alpha_j |x_j|^3 / 3 ] + Σ_j g_j x_j
s.t. A x = 0            (b = 0: no nodal demand; all demand leaves through terminal pipes)
     lo <= x <= up
```

- `g_j = gravit_j`: `-z_source` on source pipes, `+z_terminal` on terminal pipes.
- At the optimum the KKT multipliers `u` of `A x = 0` are the **hydraulic heads** of the transit nodes. With `charge = ∇f + Aᵀu`:
  - an interior pipe has `charge = 0`, so the head loss equals the head difference
  - for source pipes the head at the source equals its elevation
  - for terminal pipes the residual pressure after the terminal loss is `-charge`.
- Meshed networks need no special handling: the loops are implicit in `A`. Multiple sources need none either: each source pipe is a variable with `lo = 0`, and its gravity term fixes the source head.

### 4.2 Problem construction (`ScenarioProblemBuilder`, `sim/problem_builder.py`)

The constructor (`:49-100`):

- requires a prepared instance
- reads `λ = float(params["lambda"])`, which must be finite and > 0
- checks that `scenario.terminal_ids` equals the `succ` of the terminal pipes, in order
- aligns `P_req[t] = bornes.pressure[terminal_ids[t]]` (finite).

For scenario k (`build`, `:440-513`):

1. Vectors (`:185-261`): `d = demand_lps[:,k] / 1000` (m3/s) and `c = count[:,k]`.
   - Errors: non-finite or negative values, `d > 0` with `c <= 0`, or `active = d > 0` empty.
2. Terminal alpha (`:263-352`), a full-size vector that is 0 except on active terminal pipes:
   - **uniform and not bounded**: `alpha = alpha_X / c^2`. `X` is `com` for `orif_com == 1` and `ideal` otherwise. `c^2` is computed as `c*c` in float.
   - **non-uniform and not bounded**: error "Heterogeneous unbounded-flow simulation is not supported."
   - **bounded** (either uniform value): `alpha = P_req / d^2`, with `d^2 = d*d`.
   - Errors: non-finite or negative alpha.
3. Bounds (`:354-438`), with `D = Σ d` (numpy sum of the terminal vector):
   ```
   bounded:   G = D;        u_term = d
   unbounded: G = 20.0 * D; u_term = 20.0 * d          (UNBOUNDED_FLOW_MULTIPLIER, constants.py:18)
   up = G for every pipe; lo = -up
   up[terminal] = u_term; lo[terminal] = 0; lo[source] = 0
   lo = up = 0 on pipes closed by the user (is_open == False)
   ```
   - Errors: `D <= 0`, non-finite bounds, or `lo > up`.
4. `closed = (lo == up)` (exact) and `open = !closed`. Inactive terminals (`d = 0`) are therefore closed, as are user-closed pipes.
5. `served = active & is_open[terminal pipe]`.
6. The reduced problem has:
   - `A_open = M_incid[:, open]`
   - the open-order masks `terminal/transit/source_pipe_mask_open`
   - `active_terminal_client_counts = c[served]`
   - `balance_rhs = 0` (length nbNtra)
   - the full-size `alpha`, `beta` and `gravity` vectors
   - `bounded_flow`.
7. Checks (`:515-635`):
   - at least one open pipe and one open source pipe
   - `A_open` shape is `(nbNtra, n_open)`
   - `#open terminal pipes == #served`, and the counts are aligned
   - `lo < up` on every open pipe
   - all arrays finite
   - **no empty row in `A_open`**: a transit node with all its pipes closed raises "Closing pipes isolated one or more transit nodes".

All failures raise `ScenarioPreparationError`. The batch records them per scenario (4.6).

### 4.3 Friction function and derivatives (`_friction_derivatives`, `sim/solver.py:183-285`)

`tolx = flow_tolerance = 1e-8`. Let `R = {|x_j| > tolx}` and `S = {|x_j| <= tolx}`. Use `p = |x|^(λ-1)` on R (`powf`), and `0` on S.

| Quantity | On R (regular) | On S (small flow, regularized quadratic) |
| --- | --- | --- |
| `fvec` | `(beta*p/(λ+1) + alpha*|x|/3) * x^2` | `tolx^(λ+1)/(λ+1)*beta + 0.5*beta*(x^2*tolx^(-2) - tolx^(λ+1))` (**no alpha term**) |
| `gradf` | `beta*p*x + alpha*|x|*x + g` | `tolx^(λ-1)*beta*x + tolx*alpha*x + g` |
| `F` (2nd derivative) | `λ*beta*p + 2*alpha*|x|` | `tolx^(λ-1)*beta + tolx*alpha` |
| `G` (3rd derivative) | `λ*(λ-1)*beta*p/x + 2*alpha*sign(x)` | `0` |

The objective is `Σ fvec + g·x`. It is reported in `SolverResult.objective_value` but is never exported. The small-flow `fvec` omits the alpha term while `gradf` and `F` include it. This is an inconsistency, kept as is because it does not affect the iterates. The constants `tolx^(λ+1)`, `tolx^(λ-1)` and `tolx^(-2)` are computed with `pow`.

### 4.4 Initial point (`_build_initial_state`, `sim/solver.py:61-181`)

```
x = 0 (n), u = 0 (m = nbNtra); lo, up = open-pipe bounds (copies)
if not bounded: x[terminal] = up[terminal]; up *= 5; lo *= 5      # bounds scaled by 5 inside the solver
else:           x[terminal] = (up[terminal] + lo[terminal]) / 2   # = d/2
r0 = A x - b
x_sum = 0
for each open source pipe i (in open order):
    sel = transit_mask_open | {i}
    B = A[:, sel]; M = B Bᵀ
    L = chol_with_regularization(M)                                  # 4.5.3
    du = M⁻¹ r0 ; dx = -Bᵀ du
    cand = x; cand[sel] += dx; x_sum += cand
x = x_sum / n_sources
require: x, x-lo, up-x finite and x-lo > 0, up-x > 0 strictly, else SolverNumericalError
          "Initial solver point is not strictly inside the flow bounds."
s = 1/(x-lo); t = 1/(up-x)
mu = (s·(x-lo) + t·(up-x)) / (2n)        # ≈ 1.0; must be finite and > 0
```

Each candidate satisfies `A cand = 0`: it is the minimum-norm correction on the transit pipes plus one source pipe. Averaging spreads the supply equally over the sources. If there are no open source pipes, `SolverNumericalError`.

### 4.5 Predictor-corrector iterations (`solve`, `sim/solver.py:330-523`)

#### 4.5.1 Algorithm (replicate exactly, including the quirks)

```
state = initial point (4.4)
(gradf, F, G) = derivs(x)
invD = ones(n)                                  # QUIRK Q1: first iteration uses invD = 1, not 1/D
M = A diag(invD) Aᵀ;  L = chol_with_regularization(M)
k = 0; converged = false
while k < max_iterations (30):
    k += 1
    charge = gradf + Aᵀu
    rp     = A x - b
    rd     = gradf + Aᵀu - s + t
    if has_converged(x, lo, up, charge, rp):                 # 4.5.2
        converged = true; (gradf, F, G) = derivs(x); break
    # predictor
    rho   = -charge
    du_p  = M⁻¹ (rp + A (invD ∘ rho))
    dx_p  = -invD ∘ (Aᵀ du_p - rho)
    ds_p  = -(s ∘ dx_p)/(x - lo) - s
    dt_p  =  (t ∘ dx_p)/(up - x) - t
    θa    = step_length(x, lo, up, s, t, dx_p, ds_p, dt_p)
    mu_a  = ((s + θa ds_p)·(x + θa dx_p - lo) + (t + θa dt_p)·(up - x - θa dx_p)) / (2n)
    σμ    = min(1.0, (mu_a / mu)^tau) * mu                   # tau = 2.5
    # corrector
    rcs   = s + (dx_p ∘ ds_p - σμ)/(x - lo)
    rct   = t - (dx_p ∘ dt_p + σμ)/(up - x)
    rc    = -(rd + 0.5 * dx_p ∘ G ∘ dx_p + rcs - rct)
    du    = M⁻¹ (rp + A (invD ∘ rc))
    dx    = -invD ∘ (Aᵀ du - rc)
    ds    = -(s ∘ dx)/(x - lo) - rcs
    dt    =  (t ∘ dx)/(up - x) - rct
    θ     = step_safety (0.999) * step_length(x, lo, up, s, t, dx, ds, dt)
    if !finite(θ) or θ <= 0: SolverNumericalError("Interior-point step length is invalid.", iteration=k, theta)
    x += θ dx; u += θ du; s += θ ds; t += θ dt
    mu = sigma (0.1) * (s·(x-lo) + t·(up-x)) / (2n)
    if any non-finite in x,u,s,t,mu: SolverNumericalError(..., iteration=k)
    (gradf, F, G) = derivs(x)
    D    = F + s/(x-lo) + t/(up-x)
    invD = max(1/D, 1e-16) elementwise                      # _MIN_INVD, solver.py:21; NaN propagates
    if any non-finite invD: SolverNumericalError(..., iteration=k)
    M = A diag(invD) Aᵀ;  L = chol_with_regularization(M)
# after loop (both exits)
charge = gradf + Aᵀu; rp = A x - b; rd = gradf + Aᵀu - s + t
status = CONVERGED if converged else MAX_ITERATIONS
diagnostics = (‖rp‖∞, ‖rd‖∞, mu)
iterations = k
```

`step_length` (`sim/numerical_utils.py:46-68`):

```
θ1 = min over dx<0 of -(x-lo)/dx   (1 if no such index)
θ2 = min over dx>0 of (up-x)/dx    (1 if none)
θ3 = min over ds<0 of s/(-ds)      (1 if none)
θ4 = min over dt<0 of t/(-dt)      (1 if none)
return min(1.0, θ1, θ2, θ3, θ4)    # numpy min: NaN propagates
```

Semantics to keep:

- **Iteration count.** The convergence test is evaluated at the start of an iteration, on the current iterate. If the initial point already converges, `iterations = 1`.
- **Quirk Q2: no test after the last update.** At `k == 30` the loop ends and nothing tests the 30th update. The status is `MAX_ITERATIONS`, even for an iterate that satisfies the criteria.
- **NaN handling.** numpy `min`, `maximum` and `np.min` propagate NaN, which the finiteness checks then catch. Rust `f64::min/max` ignore NaN, so code the NaN propagation explicitly.
- **Negative ratio in the power.** For `mu_a/mu < 0`, Python's `float ** 2.5` returns a complex number and `min()` raises `TypeError`. The batch does not catch this crash. `powf` gives `NaN` silently, and then `min(1.0, NaN)`. This case was never observed. The port must treat it as `SolverNumericalError` and document the deviation.
- **Unused fields.** `centering_tolerance` and `tolpseudoA` are not used.

#### 4.5.2 Convergence test (`_has_converged`, `sim/solver.py:287-328`)

```
at_lower = (x - lo) < tolx ; at_upper = (up - x) < tolx ; interior = !(at_lower | at_upper)
fail if ‖rp‖∞ > tolx (1e-8)
fail if interior non-empty and max |charge[interior]| > tolpress (1e-6)
fail if at_upper non-empty and max charge[at_upper] > tolpress
fail if at_lower non-empty and min charge[at_lower] < -tolpress
converged = no failure
```

`lo` and `up` here are the solver-internal bounds, scaled by 5 in unbounded mode.

#### 4.5.3 Cholesky with regularization (`chol_with_regularization`, `sim/numerical_utils.py:70-125`)

Python uses CHOLMOD (`sksparse.cholmod.cho_factor`, scikit-sparse >= 0.5). Any SPD solver gives results within tolerance (verified with dense LAPACK Cholesky). Failure semantics:

```
try factor(M) -> return
delta = 1e-14
repeat 8 times: try factor(M + delta I) -> return ; delta *= 10     # 1e-14 ... 1e-7
SolverNumericalError("Cholesky factorization failed after adaptive regularization.",
                     details {regularizationAttempts: 8, lastDelta: 1e-7})   # no "iteration" key
```

- A failed factorization means "not positive definite".
- From the second factorization of a scenario on, CHOLMOD reuses the symbolic analysis (`factor.factorize`). This makes no numerical difference.
- **UNVERIFIED:** scikit-sparse 0.5 `solve()` can raise `CholmodNotPositiveDefiniteError` for an exactly singular factor. The batch does not catch that exception, so it aborts the run. The Rust port must map any linear-algebra failure to `SolverNumericalError`.
- `M = A diag(invD) Aᵀ` is SPD for a network where each connected part of the open transit graph touches an open source or terminal pipe. In other networks the regularization starts.

### 4.6 Batch execution (`SimulationBatchRunner.run`, `sim/batch_runner.py:133-357`)

Arrays are entity × scenario and are initialized to NaN, or to false or -1 as noted:

- `flow (nbA)`, `charge (nbA)`, `node_dual (nbNtra)`, `satisfaction (nbAter)`, `alpha (nbA)`
- `open_pipe_mask` (false), `served_mask` (false)
- `iterations` (-1)
- `active_terminal_mask = demand_lps > 0`.

For k in 0..N, in sequence:

1. Cancellation check: `SimulationCancelledError`, propagated.
2. `build(k)`. On `ScenarioPreparationError`: status `preparation_failed`, failure `{stage: preparation, code: "SCENARIO_PREPARATION_FAILED", message, details}`, then continue.
3. Record `open_pipe_mask`, `served_mask` and `alpha` for the scenario.
4. `solve`. On `SolverNumericalError`: status `solver_failed`, failure `{stage: solver, code: "SOLVER_NUMERICAL_FAILED", ...}`. `iterations[k] = details.iteration` for an int value, else -1. Continue.
5. `iterations[k] = result.iterations`. If not converged: status `not_converged`, failure `{stage: solver, code: "SOLVER_MAX_ITERATIONS", message: "Solver did not converge within the iteration limit.", details {iterations, primalResidualInf, dualResidualInf, barrierParameter}}`, then continue. **Non-converged scenarios are excluded from all statistics.**
6. Success:
   - `flow[:,k]` is 0 on closed pipes and the solver x on open pipes
   - `charge[open,k]` is the solver charge, and **stays NaN on closed pipes**
   - `node_dual[:,k] = u`
   - `satisfaction[:,k]` comes from 4.7
   - status `success`.

Other exceptions (shape mismatches, unexpected errors) propagate. Per-scenario durations come from `perf_counter` and are not reproducible.

### 4.7 Satisfaction (`_compute_satisfaction`, `sim/batch_runner.py:70-131`)

The result is a terminal vector, with 0 for terminals that are not served:

- bounded: `sat[served] = x_terminal / d[served]`
- unbounded: `sat[served] = x_terminal / (c[served] * targetflow_lps/1000)`. This requires `target_flow_lps`, which is `options.target_flow_lps`.

The denominators must be finite and > 0, and the result must be finite (else `RuntimeError`, propagated). Satisfaction can be above 1 in unbounded mode. It is not clipped.

---

## 5. Post-processing, statistics and output schemas

### 5.1 Statistics (`SimulationStatisticsProcessor.compute`, `sim/statistics.py:151-185`)

- For `successful_scenario_count <= 0`, raise `StatisticsProcessingError("... no scenario succeeded")`.
- `ok` = the success mask. Every table below uses only the columns in `ok`.

Helper definitions:

- **Linear percentile** (numpy `nanpercentile`, method `linear`). For the sorted finite values `v[0..n-1]` and percentile `q`:
  - `vi = (n-1) * (q/100)`, with `q/100` a float division
  - `lo = floor(vi)` and `hi = lo + 1`. For `vi >= n-1`, both are `n-1`
  - `γ = vi - lo`, `δ = v[hi] - v[lo]`
  - for `γ >= 0.5` the result is `v[hi] - δ*(1-γ)`. Otherwise it is `v[lo] + δ*γ` (numpy `_lerp`).
- **Lower tail mean** `cvar_q`: the mean of the values `<= pct_q`. Ties are included.
- **Upper tail mean**: the mean of the values `>= pct_q`.
- **Masked row statistics** (`_masked_row_statistics`, `:59-111`): values outside the mask are NaN. Per row, over the remaining values:
  - `min`, `mean`, `max`
  - `std` with ddof=0, `sqrt(Σ(v-mean)^2 / n)`
  - `cv = std/mean` for a finite `mean` with `|mean| > 1e-12` (`COEFFICIENT_OF_VARIATION_MEAN_EPSILON`). Otherwise `cv` is NaN
  - the requested tail means.

  A row with no values gives NaN, which is turned into 0.0 for `min`, `mean`, `max`, `std` and the tails. `cv` stays NaN.

| Table (JSON key) | Rows, order | Columns and formula |
| --- | --- | --- |
| `terminalSatisfaction` (`:187-246`) | terminals (sorted ID) | Mask `E = (count > 0) & ok`. `minSat, moySat, maxSat, stdSat, cvSat, cVarSat5, cVarSat20` are masked row statistics of `satisfaction` over `E`, with tails at 5 and 20. `activeScenarioCount = Σ_k (count[t,k] > 0)` over **all** scenarios. `evaluatedActiveScenarioCount = Σ E`. `waterShortfallCount = Σ (E & sat < 1 - 1e-9)` (`SATISFACTION_SHORTFALL_TOLERANCE`, `:18`). If `evaluatedActiveScenarioCount == 0`, all seven statistics become NaN, which exports as null. |
| `sourceFlow` (`:248-278`) | source **nodes** in sorted order, paired by position with source **pipes** in sorted pipe order | `F = 1000 * flow[source pipes, ok]` in L/s. `minSt, moySt, maxSt`. `cVarSt5` = lower tail at 5. `cVarSt95` = upper tail at 95. Different numbers of source nodes and source pipes raise an error. |
| `pipeVelocity` (`:280-319`) | source and transit pipes in sorted order | `v = |flow[j, ok]| / (π * diam1_j^2 / 4)`. Columns `edgeId, startNodeId (pred), endNodeId (succ), minVelocityMps, meanVelocityMps, maxVelocityMps`. Uses **`diam1` only**. For `lon1 == 0` that value is 1.0 (quirk Q4). |
| `terminalPressure` (`:321-336`) | terminals | Masked row statistics of `-charge[terminal pipe]` over `served & ok`. Columns `minPressureM, meanPressureM, maxPressureM`. A row with no observation gives 0.0. |
| `transitPressure` (`:338-364`) | transit nodes (sorted) | `P = u[i, ok] - haut_i`. Columns `minPressureM, meanPressureM, maxPressureM`. |
| `summary` (`:366-394`) | one row | `scenarioCount, successfulScenarios, failedScenarios, successRate = successful/N, seed, durationSeconds, minIterations, meanIterations (float), maxIterations` (over successful scenarios), `failureCountByCode` (`{code: count}` in first-occurrence order). |
| `failures` (`:396-421`) | one per failure, in scenario order | `scenarioIndex, stage, code, message, details`. |

Physical meaning of the pressure outputs:

- Transit pressure is the hydraulic head minus the elevation: the gauge pressure in m.
- Terminal pressure is `-charge` on the terminal pipe, which is the pressure left after the modelled terminal loss.
  - In NW4 mode it is about 0 whenever the pipe is not at a bound. Values like -3.5e-7 m appear, within `tolpress`.
  - In bounded mode it is the **excess** over `alpha*q^2`. At full demand this is the pressure above `P_req`. For a reduced flow it is about 0.

Sums and means use numpy pairwise summation. The port does not need bit-equality here: differences are about 1e-16 relative.

### 5.2 Fragile integer output: `waterShortfallCount`

In bounded mode the solver stops with terminal flows within `tolx` = 1e-8 m3/s of the upper bound. Satisfactions therefore sit just below 1, for example `1 - 2e-7`. These all count as shortfall because `< 1 - 1e-9`. In the reference runs many values lie close to the threshold:

| Network | Values with `1-1e-8 <= sat <= 1-1e-10` | Smallest distance to `1 - 1e-9` |
| --- | --- | --- |
| Toy_bbu | 76 | 6.3e-11 |
| Toy_bbd | 59 | 3.0e-11 |
| Esperenza2S | 97 | 7.2e-11 |

To reproduce these counts, the Rust iterates must follow python-core's iterates closely, to well below 1e-11 absolute. They do with the algorithm of 4.5 (verified: identical counts in the clean-room and permuted-Cholesky runs). Any algorithmic deviation changes these counts:

- a different initial `invD`
- a different stopping test
- an extra iteration.

For satisfactions this close to 1, the count is a solver artifact, not a hydraulic fact. Report this to the NW5 authors (open risk R2).

### 5.3 JSON statistics payload (`SimulationJsonExporter.build_payload`, `sim/json_exporter.py:91-154`)

`schemaVersion` is `"1.1"`. Top-level keys, in this order:

```json
{
  "schemaVersion": "1.1",
  "network": {"name": "<str>", "version": "<str>"},
  "demand": {"terminals": []},
  "units": {"sourceFlow": "L/s", "pipeVelocity": "m/s", "terminalPressure": "m",
            "transitPressure": "m", "duration": "s", "satisfaction": "ratio"},
  "summary": {"scenarioCount": 100, "successfulScenarios": 100, "failedScenarios": 0,
              "successRate": 1.0, "seed": 42, "durationSeconds": 0.07,
              "minIterations": 8, "meanIterations": 9.92, "maxIterations": 11,
              "failureCountByCode": {}},
  "scenarioSelection": {"successfulMask": [true, ...], "includedScenarioIndices": [0, ...],
                        "excludedScenarioIndices": []},
  "statistics": {
    "terminalSatisfaction": [{"terminalId": "P1", "minSat": 0.8598821527310329, "moySat": 0.9062580318953543,
        "maxSat": 1.1784864426533057, "cVarSat5": 0.8598821527310329, "cVarSat20": 0.8615065092191111,
        "stdSat": 0.06742169574781912, "cvSat": 0.07439569457587362, "activeScenarioCount": 60,
        "evaluatedActiveScenarioCount": 60, "waterShortfallCount": 57}, ...],
    "sourceFlow": [{"sourceId": "Source", "minSt": 0.20509759870411964, "moySt": 0.5026551840882982,
        "maxSt": 0.6154213807636134, "cVarSt5": 0.26118087160542697, "cVarSt95": 0.6115340987127238}],
    "pipeVelocity": [{"edgeId": "A1", "startNodeId": "Source", "endNodeId": "A",
        "minVelocityMps": 0.1272548344372364, "meanVelocityMps": 0.3118773824478235,
        "maxVelocityMps": 0.3818442849308502}, ...],
    "terminalPressure": [{"terminalId": "P1", "minPressureM": -3.517220266147092e-07,
        "meanPressureM": -1.96961645030728e-08, "maxPressureM": -2.78595813085758e-10}, ...],
    "transitPressure": [{"transitNodeId": "A", "minPressureM": 9.089963222578085,
        "meanPressureM": 9.347273623527961, "maxPressureM": 9.871428615053558}, ...]
  },
  "failures": [{"scenarioIndex": 3, "stage": "solver", "code": "SOLVER_MAX_ITERATIONS",
                "message": "...", "details": {...}}]
}
```

The sample values are from Toy_blu, seed 42, N=100.

- Serialization (`:16-66, 156-171`): NaN and ±inf become `null`, numpy scalars become native values, and the output uses `json.dumps(ensure_ascii=False, allow_nan=False, indent=2)` with insertion order.
- Floats print as Python `repr`: the shortest round-trip form, for example `1.3129139494292002e-19`. Integer-valued fields (counts, `seed`, `minIterations`) are JSON integers, and `successRate` prints as `1.0`.
- Units: L/s, m/s, m and ratio, as in `units`.
- The `demand.terminals` list is empty in the core. The API service fills it (5.5).

Legacy table names (`to_legacy_dict`, `:47-56`): `TableSat`, `TableSt`, `TableResV`, `TablePress`, `TablePrB`.

### 5.4 Spreadsheet exporter (`sim/spreadsheet_exporter.py`)

It converts the JSON payload, without recomputation, into the sheets `Summary`, `Terminal Satisfaction`, `Source Flow`, `Pipe Velocity`, `Terminal Pressure`, `Transit Pressure` and `Failures`:

- `Summary` has rows `Property, Value` taken from `schemaVersion`, `network`, and `summary` (scenario count, successes, failures, rate, seed, duration, mean iterations)
- the record sheets have a header row from the first record's keys
- styling: header fill `1E3A5F`, white bold font, frozen `A2`, column width `max(10, min(len+2, 38))`.

**Bug:** `_cell_value` calls `isinstance(value(dict,list,tuple))` (`:87`). Verified: it raises `TypeError: 'str' object is not callable` on the first non-empty record sheet, so the XLSX export is broken in this revision. Port the intended behaviour: nested values become JSON text.

### 5.5 API response (`app/schemas/simulation_execution.py`, `app/services/simulation_service.py`)

`POST /api/simulation/run` takes `SimulationExecutionRequest {projectId, domain, options, edgeOverrides[], executionId?}` and returns `SimulationExecutionResponse`:

- `projectId`.
- `nodes[]` (`:153-228`): `{id, pressureM, lowPressure}`, in this order:
  - sources: `pressureM = 0`, `lowPressure = false`
  - transit nodes: `meanPressureM`, `lowPressure = false`
  - terminals: `meanPressureM` for a terminal served in at least one successful scenario, else 0. `lowPressure = observed and minPressureM < borne.required_pressure`. In bounded mode this compares the **excess** pressure with the requirement (5.1), which is a semantic quirk.
- `edges[]` (`:230-251`): every pipe in sorted order, `{id, flowRateLps = 1000*mean(flow over successful scenarios) (signed), velocityMps = meanVelocityMps or 0 for terminal pipes}`.
- `summary` (`:253-300`):
  - `status` is `"COMPLETED"`, or `"COMPLETED_WITH_ERRORS"` for a run with a failed scenario
  - the counts, `successRate`, `seed` (None raises an error), `durationSeconds`, and min/mean/max iterations
  - `openEdges`/`closedEdges` from `is_open`
  - `lowPressureNodes`.
- `statistics`: the 5.3 payload with `demand = {"terminals": terminal_demand_profiles_payload(domain)}` (`client_demand.py:142-243`). Per terminal it gives `{terminalId, clientCount, groups[{groupId, behaviorIds, behaviorNames, clientCount, openProbability, targetFlowLps, profileId}]}`, with the terminals ordered by status-2 nodes, then bornes, then clients (first seen).
- `executionParameters {values, sources}` (`:435-517`): values that report the resolved run. `sources` is one of `USER_OVERRIDE`, `PROJECT_DEFAULT`, `GENERATED`, `CLIENT_DEMAND_TABLE` or `NOT_APPLICABLE`.

Other routes:

- `POST /api/simulation/export`: the same pipeline, returning the JSON payload as a file named `simulation_statistics_<safe name>.json`.
- `POST /api/simulation/exports/xlsx {statistics}`: 5.4 (broken).
- `POST /api/simulation/executions/{id}/cancel`.

Errors: `SimulationValidationError`, `ScenarioGenerationError`, `ScenarioPreparationError`, `SolverNumericalError` and `StatisticsProcessingError` return HTTP 422 `{"detail": exc.to_dict()}`. Cancellation returns 409 (`app/main.py:54-78, 131-134`).

---

## 6. Validation rules and error cases

### 6.1 Input validation (`validate_simulation_data`, `sim/validation.py:1219-1293`)

It runs at `from_tables` and again in `prepare()`. All issues are collected, then raised once as `SimulationValidationError` (code `SIMULATION_INPUT_INVALID`). Each issue is `{code, message, entityType, entityId, field, details}` (`sim/exceptions.py:13-114`). The checks run in this order. IDs are compared after `str().strip()`.

| Code | Condition (source line) |
| --- | --- |
| `MISSING_TABLE`, `INVALID_TABLE_TYPE`, `MISSING_TABLE_COLUMN` | A required table or column is missing (`:136-172`). Required columns are at `:23-33`. A table with missing columns skips its own validator. |
| `EMPTY_NODE_TABLE`, `EMPTY_NODE_ID`, `DUPLICATE_NODE_ID` | (`:174-227`) |
| `INVALID_NODE_NUMBER` | `haut, xdata, ydata, orif_ideal, orif_com` not finite (`:229-246`) |
| `INVALID_NODE_STATUS`, `UNSUPPORTED_NODE_STATUS` | status not numeric, or not in {0,1,2} (`:248-280`) |
| `MISSING_SOURCE_NODE`, `MISSING_TERMINAL_NODE` | (`:282-303`) |
| `EMPTY_PIPE_TABLE`, `DUPLICATE_PIPE_ID`, `EMPTY_PIPE_ID` | (`:305-363`) |
| `UNKNOWN_PIPE_NODE`, `PIPE_SELF_LOOP` | (`:365-406`) |
| `INVALID_PIPE_LENGTH`, `NEGATIVE_PIPE_LENGTH` | `lon, lon1, lon2` not finite, or < 0 (`:408-450`) |
| `ZERO_PIPE_LENGTH` | `lon1 + lon2 <= 0` (`:452-466`) |
| `PIPE_LENGTH_MISMATCH` | `!isclose(lon, lon1+lon2, rtol=1e-6, atol=1e-9)` (`:468-482`) |
| `MISSING_PIPE_DIAMETER`, `UNKNOWN_PIPE_DIAMETER` | a positive segment with a blank or unknown ID (`:484-541`) |
| `EMPTY_CLIENT_TABLE`, `DUPLICATE_CLIENT_ID`, `EMPTY_CLIENT_ID`, `UNKNOWN_CLIENT_TERMINAL` | (`:543-610`) |
| `INVALID_CLIENT_PROBABILITY`, `CLIENT_PROBABILITY_OUT_OF_RANGE` | not finite, or outside [0,1] (`:612-638`) |
| `INVALID_CLIENT_TARGET_FLOW` | not finite, or <= 0 (`:640-653`) |
| `TERMINAL_WITHOUT_CLIENT` | (`:655-669`) |
| `NO_ACTIVE_CLIENT_PROBABILITY` | applies to `uniform_dem == 1` with a finite `opentaps`. Raised for `opentaps <= 0`, else for no client with `proba > 0` (`:671-702`) |
| `EMPTY_BORNE_TABLE`, `DUPLICATE_BORNE_ID`, `MISSING_TERMINAL_BORNE`, `UNKNOWN_BORNE_TERMINAL` | the borne IDs must equal the terminal IDs exactly (`:704-764`) |
| `INVALID_BORNE_PRESSURE` | not finite, or < 0 (`:766-781`) |
| `EMPTY_DIAMETER_TABLE`, `DUPLICATE_DIAMETER_ID` | (`:783-817`) |
| `INVALID_DIAMETER_VALUE` | `diam > 0`. `cost, Pmax, Roughness >= 0`. `Type` finite (`:819-858`) |
| `MISSING_SIMULATION_PARAMETER` | one of `bescal, bounded_flow, faucetcoef, lambda, nbsim, nu, opentaps, orif_com, orifcoef, pressure, servicequal, targetflow, uniform_dem, vmax, vmax_secu` is missing (`:874-899`) |
| `INVALID_BINARY_PARAMETER` | `bounded_flow, fusion, orif_com, split, uniform_dem, use_efficient_pipes` not in {0,1} (`:901-918`) |
| `INVALID_SCENARIO_COUNT` | `nbsim` not finite, not a positive integer (`:920-944`) |
| `INVALID_POSITIVE_PARAMETER` | `bescal, lambda, nu, orifcoef, vmax, vmax_secu` not finite, or <= 0 (`:946-968`) |
| `INVALID_NON_NEGATIVE_PARAMETER` | `faucetcoef, pressure` < 0 (`:970-990`) |
| `PARAMETER_OUT_OF_RANGE` | `opentaps, servicequal` outside [0,1] (`:992-1010`) |
| `UNSUPPORTED_SIMULATION_CASE` | `bounded_flow == 0 and uniform_dem == 0` (`:1012-1030`) |
| `INVALID_TARGET_FLOW_PARAMETER` | `bounded_flow == 0` and `targetflow <= 0` (`:1032-1047`) |
| `INVALID_TERMINAL_ORIFICE` | `bounded_flow == 0` and a terminal's `orif_com` (if `orif_com == 1`) or `orif_ideal` is not > 0 (`:1049-1078`) |
| `INVALID_PARAMETER_CONTAINER` | param is not a dict (`:1280-1286`) |
| `DIRECT_SOURCE_TO_TERMINAL_PIPE` | (`:1115-1124`) |
| `SOURCE_HAS_INCOMING_PIPE`, `SOURCE_WITHOUT_OUTGOING_PIPE` | (`:1134-1156`) |
| `TERMINAL_HAS_OUTGOING_PIPE`, `INVALID_TERMINAL_IN_DEGREE` | a terminal must have exactly 1 incoming pipe (`:1158-1183`) |
| `NODE_UNREACHABLE_FROM_SOURCE` | BFS along the pipe direction from all sources, **closed pipes included** (`:1185-1217`) |

Reachability ignores `is_open`. A network split by closed pipes therefore passes validation and fails later, per scenario (4.2 step 7), or in the solver.

### 6.2 Option errors (`sim/options.py`)

| Code | Condition |
| --- | --- |
| `INVALID_SCENARIO_COUNT` | resolved count <= 0 |
| `INVALID_GENERATION_BATCH_COUNT` | `max_generation_batches <= 0` |
| `INVALID_OVERSAMPLING_FACTOR` | `oversampling_factor <= 1` |
| `INVALID_MAX_DRAWS_PER_BATCH` | `max_draws_per_batch <= 0` |
| `OPEN_TAPS_PROBABILITY_OUT_OF_RANGE` | uniform mode and `opentaps` outside [0,1] |
| `ZERO_OPEN_TAPS_PROBABILITY` | uniform mode and `opentaps == 0` |
| `INVALID_UNIFORM_TARGET_FLOW` | uniform mode and no positive target (2.2) |
| `UNSUPPORTED_UNIFORM_PROBABILITY_OVERRIDE` | non-uniform mode and an `open_taps_probability` is given |
| `UNSUPPORTED_UNIFORM_FLOW_OVERRIDE` | non-uniform mode and a `target_flow_lps` is given |
| `INVALID_SOLVER_OPTION` | see 1.4.2 |

### 6.3 Runtime errors

| Stage | Error (code) | Effect |
| --- | --- | --- |
| Preprocessing | plain `ValueError` / `RuntimeError` (1.6) | aborts. Unreachable after validation, except for diameter mapping |
| Scenario generation | `ScenarioGenerationError` (`SCENARIO_GENERATION_FAILED`): no clients, `vp <= 0`, batch limit exceeded (details `requestedScenarios, generatedScenarios, generationBatches, validScenarioProbability`), unknown terminal | aborts |
| Builder constructor | `ScenarioPreparationError` (not prepared, bad lambda, terminal misalignment, pressure alignment) | aborts |
| Per scenario | `ScenarioPreparationError` (`SCENARIO_PREPARATION_FAILED`) | scenario `preparation_failed` |
| Per scenario | `SolverNumericalError` (`SOLVER_NUMERICAL_FAILED`): no source, initial point not interior, invalid initial mu, Cholesky failure, invalid step, non-finite iterate or invD | scenario `solver_failed` |
| Per scenario | not converged in 30 iterations (`SOLVER_MAX_ITERATIONS`) | scenario `not_converged` |
| Statistics | `StatisticsProcessingError` (`STATISTICS_PROCESSING_FAILED`): no success, misaligned shapes, number of source nodes differs from source pipes, invalid `diam1` | aborts |
| Any | `SimulationCancelledError` (`SIMULATION_CANCELLED`) | aborts (HTTP 409) |

Serialization (`sim/exceptions.py:116-265`): `{code, message, scenarioIndex?, details?}`. The `ScenarioFailure.to_dict` form is `{scenarioIndex, stage, code, message, details?}` (`sim/models.py:84-114`).

---

## 7. Running python-core headlessly and reference outputs

### 7.1 Environment

- `pyproject.toml` requires `numpy`, `pandas>=2.2,<3`, `scipy`, `pydantic>=2.8`, `fastapi`, `scikit-sparse>=0.5`, and `openpyxl` for the XLSX export.
- scikit-sparse needs the SuiteSparse headers (`cholmod.h`), which this machine lacks.
- The runs below used the system Python 3.15.2 with numpy 2.5.4, scipy 1.18.0, pandas 3.0.6 and pydantic 2.14.3. pandas 3 is outside the declared `<3` range but worked.
- They also used a stand-in `sksparse.cholmod` module, a dense `scipy.linalg.cho_factor` that raises on non-SPD input. The real CHOLMOD is expected to give the same results within tolerance. Use it on a machine with SuiteSparse.

### 7.2 Commands

```bash
S=/tmp/nw5ref            # any scratch directory outside reference/
CORE=reference/neatwork5/application/python-core
python3 -m venv --system-site-packages $S/venv && $S/venv/bin/pip install pytest
mkdir -p $S/shim/sksparse && touch $S/shim/sksparse/__init__.py
cat > $S/shim/sksparse/cholmod.py <<'EOF'
import numpy as np, scipy.linalg as sla
class CholmodNotPositiveDefiniteError(Exception): pass
class CholeskyFactor:
    def factorize(self, A):
        d = A.toarray() if hasattr(A, "toarray") else np.asarray(A)
        try: self._c = sla.cho_factor(d, lower=True)
        except sla.LinAlgError as e: raise CholmodNotPositiveDefiniteError(str(e)) from e
        return self
    def solve(self, b): return sla.cho_solve(self._c, np.asarray(b, dtype=float))
def cho_factor(A, **kw): return CholeskyFactor().factorize(A)
EOF
```

Reference runner. It writes `<prefix>.stats.json` (the 5.3 payload without `durationSeconds`) and `<prefix>.raw.json` (per-scenario arrays: demand, counts, flows, charges, duals, satisfaction, iterations, statuses, ID orders, scenario metadata):

```python
# ref_run.py  <dsg> <seed> <nscen> <out_prefix>
import json, sys
from pathlib import Path
import numpy as np
sys.path.insert(0, "<abs path to>/reference/neatwork5/application/python-core")
from app.neatwork.parsers.dsg_parser import parse_dsg
from app.neatwork.simulation import Simulation, SimulationRunOptions, build_simulation_data
from app.schemas.simulation_domain import SimulationDomainInput
dsg, seed, n, out = Path(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
dom, _ = parse_dsg(name=dsg.stem, content=dsg.read_text(encoding="utf-8"))
inst = build_simulation_data(SimulationDomainInput.model_validate(dom.model_dump(mode="json", by_alias=True)))
sim = Simulation(inst=inst, options=SimulationRunOptions(scenario_count=n, seed=seed))
batch = sim.run(); sim.compute_stats(); payload = sim.statistics_payload()
payload["summary"].pop("durationSeconds", None)
Path(out + ".stats.json").write_text(json.dumps(payload, indent=2, allow_nan=False))
nan2none = lambda a: [[None if isinstance(v, float) and not np.isfinite(v) else v for v in r] for r in np.asarray(a).tolist()]
Path(out + ".raw.json").write_text(json.dumps({
  "edge_ids": batch.edge_ids.tolist(), "terminal_ids": batch.terminal_ids.tolist(),
  "transit_node_ids": batch.transit_node_ids.tolist(),
  "demand_by_terminal_lps": batch.demand_by_terminal_lps.tolist(),
  "open_client_count_by_terminal": batch.open_client_count_by_terminal.tolist(),
  "flow_by_pipe_m3s": nan2none(batch.flow_by_pipe_m3s), "charge_by_pipe_m": nan2none(batch.charge_by_pipe_m),
  "node_dual": nan2none(batch.node_dual_by_transit_node), "satisfaction": nan2none(batch.satisfaction_by_terminal),
  "iterations": batch.iterations.tolist(), "status": [s.value for s in batch.status_by_scenario],
  "scenario_meta": sim.sim_res["ScenarioMetadata"]}))
```

```bash
PYTHONPATH=$S/shim PYTHONDONTWRITEBYTECODE=1 $S/venv/bin/python ref_run.py \
  reference/neatwork5/examples/dsg_examples/Toy/Toy_blu_Secu1.00_fusion0_split0.dsg 42 100 $S/toy_blu_seed42
# python-core's own tests (do not write into reference/: disable the pytest cache and bytecode)
cd $CORE && PYTHONPATH=$S/shim:. PYTHONDONTWRITEBYTECODE=1 $S/venv/bin/python -m pytest -p no:cacheprovider \
  tests/regression tests/simulation tests/reference_validation -q
```

- `SimulationRunOptions(seed=..., scenario_count=...)` must be passed explicitly. With `seed=None` the run is not reproducible.
- Use `Path.with_suffix` carefully in a runner: names like `Toy_Secu1.00_...` contain dots.
- The working scripts used for this spec are `ref_run.py`, `pcg_check.py` and the clean-room `spec_check.py`. They and their outputs are in the session scratch directory, which is temporary. Phase 2 of the plan must copy them into the repository.

### 7.3 Test status in this revision

- `tests/regression/test_simulation_regression.py`: the 3 parametrized cases **fail**. The cause is a key mismatch: the fixtures `tests/fixtures/reference/toy_{blu,bbu,bbd}_seed42.json` are schema `"1.0"`. They lack `demand` and the terminal `stdSat`, `cvSat`, `activeScenarioCount`, `evaluatedActiveScenarioCount` and `waterShortfallCount`. Every numeric field they do contain matches the current output. The worst differences are `6.8e-9` relative (1e-16 absolute) on Toy_blu and `2.2e-8` relative (2e-16 absolute) on Toy_bbu and Toy_bbd. `test_same_seed_produces_same_results` passes. Tolerance in these tests: `rel 1e-6, abs 1e-8` (`tests/regression/regression_helpers.py:56-58`).
- `tests/simulation/test_terminal_reliability_statistics.py`. This is a synthetic batch of 5 scenarios, 2 terminals, with scenario 2 failed (`:9-55`).
  - Expected for P3: `activeScenarioCount` 4, `evaluatedActiveScenarioCount` 3, `waterShortfallCount` 1, `maxSat` 1.2, `stdSat = std([1.0, 0.5, 1.2])` (ddof 0), `cvSat = std/mean`.
  - Expected for P4: active 3, evaluated 3, shortfall 2.
  - Two tests pass. `test_statistics_can_be_computed_for_unserved_terminals` fails with `KeyError: 'PX'`: the test looks up a terminal that its own batch does not contain (a test bug).
- `tests/reference_validation`:
  - `test_equivalence_formulas.py` passes. It asserts that `beta*q^λ` with `bescal = 10.667*C^-1.852`, `λ = 1.852` and `nu = 4.871` equals Hazen-Williams, and that the bounded-flow delivery `d*sqrt(P/P_req)` equals EPANET pressure-driven demand.
  - `test_epanet_reference.py` is skipped because it needs `wntr==1.5.0`.

### 7.4 Example networks and reference values (seed 42, N = 100, all scenarios converge)

| File (`examples/`) | Mode after loading | Pipes / terminals / transit | Draws | Iterations min / mean / max | Mean source flow (L/s) | First terminal: `moySat`, shortfall count |
| --- | --- | --- | --- | --- | --- | --- |
| `Toy/Toy_blu_Secu1.00_fusion0_split0.dsg` | uniform, NW4 (bounded 0, `orif_com` 1) | 7 / 4 / 3 | 111 | 8 / 9.92 / 11 | Source 0.502655184088 | P1 0.9062580318953543, 57 |
| `Toy/Toy_bbu_Secu1.00_fusion0_split0.dsg` | uniform, bounded | 7 / 4 / 3 | 111 | 8 / 12.2 / 19 | 0.479576949045 | P1 0.9999999603347202, 40 |
| `Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg` | non-uniform, bounded | 7 / 4 / 3 | 111 | 8 / 12.46 / 22 | 0.474313696506 | P1 0.9999996736783022, 43 |
| `Toy/Toy_Secu1.00_fusion0_split0.dsg` | uniform, NW4 | 7 / 4 / 3 | 123 | 7 / 9.53 / 10 | 0.250259911539 | P1 1.0021850522473097, 17 |
| `Toy/Toy_Borne_Secu1.00_fusion0_split0.dsg` | uniform, NW4 | 7 / 4 / 3 | 151 | 7 / 8.35 / 11 | 0.158244140313 | P1 0.9705374177838952, 17 |
| `Esperenza/Esperenza2S.dsg` (meshed, 2 sources. The file says `uniform_dem 0`, but the clients are uniform, so the mode is uniform) | uniform, bounded | 14 / 5 / 6 | 111 | 8 / 10.05 / 13 | S1 17.196006634655, S2 1.559927709757 | P3 0.9688152413142659, 89 |
| `Esperenza/Esperenza2S copy.dsg` | same. It differs only in `orif_ideal`, which is unused in bounded mode | 14 / 5 / 6 | 111 | 8 / 10.05 / 13 | identical | identical |
| `Ceiba/CeibaNW4_Secu1.00_fusion0_split0.dsg` | uniform, NW4 | 396 / 222 / 174 | 111 | 9 / 9.37 / 13 | TP 5.25317612621 | P1 1.0313921067400336, 0 |

These files cannot run through the domain path:

| File | Reason |
| --- | --- |
| `Toy/ToyXlx.dsg`, `Toy/Toy2b_NW4.dsg`, `Ceiba/CeibaNW4.dsg` | no `!Borne` section (1.2) |
| `Toy/ToyDevReinf.dsg` | `UNKNOWN_PIPE_DIAMETER` (12 issues) |
| `Red 1/Red 1.mat` | MATLAB only |

Recommended Rust acceptance suite:

- the 8 runnable files above, with seeds 42 and one more seed, and N = 100
- compare every `.stats.json` value, and the raw per-scenario flows, charges, duals and satisfaction
- tolerance `|a-b| <= 1e-10 + 1e-8|b|`
- integers (scenario matrices, counts, iterations, statuses) must be exactly equal.

---

## 8. Differences between python-core and the MATLAB original (`matlab/`)

| # | Topic | MATLAB | python-core | Effect |
| --- | --- | --- | --- | --- |
| D1 | RNG | `rand` (Mersenne Twister, global state, unseeded in `runSimul.m`) | PCG64 `default_rng(seed)` | different scenario sets. Only statistical comparison possible |
| D2 | Draw order | `TC` in file order (`genscen.m:25-63`) | clients sorted by `(borne, client)` | different mapping of draws to clients |
| D3 | Draw cap | none | `max_draws_per_batch` 100000. Batch limit 20 in both | rarely matters |
| D4 | Bounded alpha | `param.pressure ./ dem.^2` (global pressure, `prepare_sim.m:186`) | per-borne `P_req` from the bornes table (`problem_builder.py:331-339`) | differs for borne pressures that differ from `pressure` |
| D5 | First IPM iteration | `invD = 1/D` from the initial point (`debits.m:33-38`) | `invD = 1` (`solver.py:377-384`) | different iterates and iteration counts |
| D6 | Cholesky regularization | first retry `delta = max(1e-14, 10*max(1, 0)) = 10`, ×10 up to 8 tries. Returns silently on failure (`chol_with_regularization.m:337-376`) | `1e-14` up to `1e-7`. Raises `SolverNumericalError` | python-core regularizes far less |
| D7 | Safeguards | none | `invD >= 1e-16`, finiteness checks, step check | none |
| D8 | Initial-point solve | `M \ rinit` (`init_point_debits.m:39`) | regularized Cholesky | same result for an SPD matrix M |
| D9 | Non-convergence | prints a message. The scenario is kept in the statistics | `not_converged`, excluded from the statistics | statistics differ in that case |
| D10 | Percentiles (cVar) | nearest rank `y(max(1, round(a*n/100)))`. For `a < 100/n` the result is `y(1)` (`utils/percentile.m`) | numpy linear interpolation (5.1) | different cVar values |
| D11 | Terminal satisfaction rows | over `OuvT` (open terminals). Empty gives 0 | over `count > 0 & success`. Empty gives null. Adds std, cv and counts | none |
| D12 | Terminal pressure loop | `x = X(i, OuvT(i,:))` uses pipe row `i`, not terminal `i`, for the emptiness test (`stat_processing.m:147`) | served mask | MATLAB bug |
| D13 | Closed pipes | not supported | `is_open`, edge overrides | none |
| D14 | Uniform parameters | `param.opentaps` / `param.targetflow` (`genscen.m:12-15`). `uniform & bounded` sets `TB.pressure = param.pressure` (`runSimul.m`) | values synchronized from the client table. Per-borne pressure unless the API override is used | none |
| D15 | Satisfaction formulas | `x/xsup` (bounded). `x/clients/targetflow/1e-3` | same | none |
| D16 | Friction, step length, convergence test, predictor-corrector | `friction_derivatives.m`, `debits.m` | identical formulas | none |

The older Python port (`python/simulation/`) uses scikit-sparse's 0.4 API (`cholesky`). It draws on clients in table order and already has the `invD = 1` start. python-core replaces it.

---

## 9. Quirks the port must reproduce (or consciously change)

| Id | Quirk | Source | Recommendation |
| --- | --- | --- | --- |
| Q1 | First IPM iteration with `invD = 1` | `solver.py:377` | reproduce (affects iterates and counts) |
| Q2 | No convergence test after the 30th update | `solver.py:389-492` | reproduce |
| Q3 | Small-flow objective without the alpha term | `solver.py:244-248` | not exported. Reproduce it only for an exposed objective |
| Q4 | Velocity uses `diam1`, which is 1.0 for `lon1 == 0` | `statistics.py:296` | reproduce. Report upstream |
| Q5 | Source-flow labels pair sorted source nodes with source pipes sorted by `succ`. Labels can be wrong with several sources. One source with 2 pipes is an error | `statistics.py:251-265` | reproduce. Report upstream |
| Q6 | `waterShortfallCount` depends on the solver tolerance (5.2) | `statistics.py:217` | reproduce. Report upstream |
| Q7 | Terminal pressure is the post-loss residual, or the excess in bounded mode. The API `lowPressure` compares it with `P_req` | `statistics.py:327`, `simulation_service.py:218` | reproduce. Document in the UI |
| Q8 | XLSX exporter crash | `spreadsheet_exporter.py:87` | implement the intended behaviour |
| Q9 | DSG without `!Borne` cannot be simulated. DSG `lambda` default 0. Rows with missing cells silently dropped. Numeric IDs normalized | 1.2 | reproduce for parity. Consider warnings |
| Q10 | In uniform mode `SimulationRunOptions.open_taps_probability/target_flow_lps` are ignored. The API scenario count defaults to 100, not `nbsim` | `options.py:293-326`, `simulation_options.py:118` | reproduce |
| Q11 | `(mu_a/mu)^2.5` with a negative ratio crashes. NaN semantics of `min`/`max` | 4.5.1 | map to `SolverNumericalError` |
| Q12 | CHOLMOD solve errors not caught | 4.5.3 | map to `SolverNumericalError` |
| Q13 | Validation runs twice. Preprocessing raises plain exceptions | `data.py:109, 313` | single validation is fine |
| Q14 | Stale regression fixtures and the `PX` test bug | 7.3 | regenerate references in phase 2 |

## 10. Open risks

- **R1: CHOLMOD.** The references were produced with a dense Cholesky stand-in. They agree with the CHOLMOD-made fixtures on every field those fixtures contain (worst 2.2e-8 relative, at 2e-16 absolute), including the iteration statistics. The terminal count fields (`waterShortfallCount`, `evaluatedActiveScenarioCount`) are not in the old fixtures and are **UNVERIFIED** against real CHOLMOD. Their sensitivity analysis (5.2) suggests they are stable, with a margin of at least 3e-11.
- **R2: Threshold-sensitive integers.** `waterShortfallCount` and, to a lesser degree, the iteration counts require the Rust IPM to follow python-core's operation order closely: the same formulas, the same order of updates, and `powf`. A different but "better" solver breaks parity on these counts.
- **R3: Reproducibility of numpy reductions.** Sums in python-core use numpy pairwise or BLAS order. The port will differ at about 1e-16 relative, which is well within tolerance. The tail means use `<=`/`>=` against an interpolated threshold. A value equal to the threshold in one implementation and 1 ulp away in the other changes a tail mean. This was not observed. **UNVERIFIED** in general.
- **R4: Upstream churn.** The fixtures are already stale relative to the code (schema 1.0 vs 1.1). Pin the python-core commit for the references (phase 2).
