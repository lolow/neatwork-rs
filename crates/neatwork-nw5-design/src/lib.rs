//! NeatWork 5 design: the reinforcement design `makeReinfDsg` of MATLAB
//! (`rust/nw5/specs/design.md`).
//!
//! [`design`] takes a topography (no pipe) or a network with existing pipes
//! (arc states `N`, `B`, `R`) and chooses for each link an optional new pipe
//! laid in parallel. The pipeline:
//!
//! 1. parameter review ([`params::review`]),
//! 2. tables ([`prepare::prepare`]): catalogue, orientation, enrichment, `TR`,
//! 3. loads `TL` and `TLv` ([`loads::incidence_loads`]),
//! 4. LP/MILP ([`model::build_model`], [`model::solve_model`], HiGHS),
//! 5. post-processing ([`post::process_solution`]).
//!
//! Corrections of the MATLAB snapshot (ADR-021): the equipment length test
//! accepts `|lon1 + lon2 − lon| ≤ 1e-7` (B1); the arc status line with an
//! undefined index is removed (B2); 7-field catalogues, `ID2 = 0` and missing
//! checks are handled by the readers and the review (B4, B5, B9).
//!
//! Random draws (Monte Carlo loads) use the MATLAB `twister` generator
//! seeded by [`DesignOptions::seed`]. MATLAB never seeds it; the default
//! seed 0 is the state of a new MATLAB session (`rng('default')`). `TL` and
//! `TLv` are drawn one after the other from the same stream, as MATLAB draws
//! them from its global stream.

#![expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "`!(x > y)` rejects NaN, as the MATLAB assertions do"
)]

pub mod loads;
pub mod model;
pub mod params;
pub mod post;
pub mod prepare;
mod rng;

use neatwork_nw5::{Error, Network, Result};

pub use loads::LoadTable;
pub use model::{LpModel, Solution};
pub use params::DesignParams;
pub use post::{ConstraintType, Costs, Nature, TahRow, TcpRow, TsRow};
pub use prepare::DesignArc;
pub use rng::Twister;

/// Risk measure of the design loads (ADR-023).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiskMeasure {
    /// Value at risk with linear interpolation (MATLAB default).
    Var,
    /// Conditional value at risk (`use_cvar = 1`).
    Cvar,
}

/// How the loads are computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadMethod {
    /// Exact binomial law for uniform demand, Monte Carlo otherwise (MATLAB).
    Auto,
    /// Monte Carlo also for uniform demand.
    MonteCarlo,
}

/// Steps reported to the progress callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Parameter review and tables.
    Prepare,
    /// Loads `TL` and `TLv`.
    Loads,
    /// LP/MILP construction.
    Model,
    /// Solver.
    Solve,
    /// Post-processing.
    PostProcess,
}

/// Options of a design run.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignOptions {
    /// Parameter overrides (`name`, value), applied over `!Default
    /// properties` as the MATLAB dialog does.
    pub params: Vec<(String, f64)>,
    /// Load computation.
    pub load_method: LoadMethod,
    /// Risk measure; `None` takes `use_cvar` of the parameters.
    pub risk: Option<RiskMeasure>,
    /// Seed of the Monte Carlo draws (`rng(seed, 'twister')`).
    pub seed: u32,
    /// Number of Monte Carlo scenarios; `None` takes `nScenLoad`.
    pub n_scen_load: Option<usize>,
}

impl Default for DesignOptions {
    fn default() -> DesignOptions {
        DesignOptions {
            params: Vec::new(),
            load_method: LoadMethod::Auto,
            risk: None,
            seed: 0,
            n_scen_load: None,
        }
    }
}

impl DesignOptions {
    /// Options with parameter overrides.
    pub fn with_params(params: &[(&str, f64)]) -> DesignOptions {
        DesignOptions {
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            ..DesignOptions::default()
        }
    }
}

/// Result of a design run.
#[derive(Clone, Debug)]
pub struct DesignResult {
    /// The designed network: existing pipes then reinforcements (a doubled
    /// link has two parallel arcs), node orifices, `loadflow` (= `TFact`:
    /// `succ, designFlow, margin`), reviewed parameters and clients.
    pub network: Network,
    /// Reviewed parameters.
    pub params: DesignParams,
    /// Links of the tree in model order (`TA` after enrichment).
    pub arcs: Vec<DesignArc>,
    /// Pipe IDs in LP column order (real catalogue, diameter ascending).
    pub pipe_ids: Vec<String>,
    /// `TL`: design loads.
    pub loads: LoadTable,
    /// `TLv`: velocity loads (`max(TLv, TL)`).
    pub loads_velocity: LoadTable,
    /// The LP/MILP.
    pub lp: LpModel,
    /// The cleaned solver solution (lengths before post-processing).
    pub solution: Solution,
    /// `TAH`: hydraulics per link.
    pub tah: Vec<TahRow>,
    /// `TCP`: nature and diameters per physical pipe row.
    pub tcp: Vec<TcpRow>,
    /// `TS`: head constraints per node.
    pub ts: Vec<TsRow>,
    /// Costs.
    pub costs: Costs,
    /// Successors of the links whose pipe mixture was reduced.
    pub reduced_mixture_arcs: Vec<String>,
    /// Warnings.
    pub warnings: Vec<String>,
}

impl DesignResult {
    /// `TFact`: `(succ, designFlow L/s, margin m)` per link.
    pub fn tfact(&self) -> &[(String, f64, f64)] {
        &self.network.loadflow
    }
}

/// Design a network (`makeReinfDsg`).
pub fn design(network: &Network, options: &DesignOptions) -> Result<DesignResult> {
    design_with_progress(network, options, &mut |_| {})
}

/// [`design`] with a progress callback called at the start of each stage.
pub fn design_with_progress(
    network: &Network,
    options: &DesignOptions,
    progress: &mut dyn FnMut(Stage),
) -> Result<DesignResult> {
    progress(Stage::Prepare);
    let reviewed = params::review(network, &options.params, options.risk, options.n_scen_load)?;
    let p = reviewed.params;
    let mut warnings = reviewed.warnings;
    let prep = prepare::prepare(network, &reviewed.clients, &p, &mut warnings)?;

    progress(Stage::Loads);
    let mut rng = Twister::new(options.seed);
    let tl = loads::incidence_loads(
        &prep.arcs,
        &prep.paths,
        &reviewed.clients,
        &p,
        p.servicequal,
        options.load_method,
        &mut rng,
    )?;
    let mut tlv = loads::incidence_loads(
        &prep.arcs,
        &prep.paths,
        &reviewed.clients,
        &p,
        p.vmax_secu,
        options.load_method,
        &mut rng,
    )?;
    tlv.max_with(&tl);

    progress(Stage::Model);
    let lp = model::build_model(
        &prep.arcs,
        &prep.paths,
        &prep.catalog,
        &prep.tr,
        &tl,
        &tlv,
        &p,
    )?;

    progress(Stage::Solve);
    let solution = model::solve_model(&lp).map_err(|e| explain_failure(e, &lp, &prep.arcs))?;
    warnings.extend(solution.warnings.iter().cloned());

    progress(Stage::PostProcess);
    let post = post::process_solution(
        post::PostInput {
            arcs: &prep.arcs,
            paths: &prep.paths,
            catalog: &prep.catalog,
            tr: &prep.tr,
            loads: &tl,
            params: &p,
            orifices: &network.orifices,
        },
        &solution,
    )?;
    warnings.extend(post.warnings.iter().cloned());
    let out = output_network(network, &prep, &reviewed.clients, &p, &post);
    Ok(DesignResult {
        network: out,
        params: p,
        pipe_ids: prep.catalog.pipes.iter().map(|q| q.id.clone()).collect(),
        reduced_mixture_arcs: post
            .reduced_mixture_arcs
            .iter()
            .map(|&a| prep.arcs[a].succ.clone())
            .collect(),
        arcs: prep.arcs,
        loads: tl,
        loads_velocity: tlv,
        lp,
        solution,
        tah: post.tah,
        tcp: post.tcp,
        ts: post.ts,
        costs: post.costs,
        warnings,
    })
}

/// Sections of an input design that describe an older result.
const STALE_SECTIONS: &[&str] = &[
    "hydraulics",
    "complementta",
    "costs",
    "constraintsatisfaction",
    "loadflow",
];

/// The designed network (`TN_out`, `TA_out`, `TB`, `TC`, `TD`, `TFact`).
fn output_network(
    input: &Network,
    prep: &prepare::Prepared,
    clients: &[neatwork_nw5::Client],
    p: &DesignParams,
    post: &post::PostResult,
) -> Network {
    let mut net = input.clone();
    for n in &mut net.nodes {
        n.orifice_ideal = 1.0;
        n.orifice_commercial = 1.0;
        n.taps = Some(clients.iter().filter(|c| c.terminal == n.id).count() as f64);
    }
    for (arc, &(ideal, com)) in prep.arcs.iter().zip(&post.orifices) {
        if let Some(n) = net.nodes.iter_mut().find(|n| n.id == arc.succ) {
            n.orifice_ideal = ideal;
            n.orifice_commercial = com;
        }
    }
    let pipe_id = |s: &prepare::Segment| {
        s.pipe
            .and_then(|d| prep.catalog.pipes.get(d))
            .map(|q| q.id.clone())
    };
    let mut ids: Vec<String> = Vec::new();
    net.arcs = post
        .out_arcs
        .iter()
        .map(|o| {
            let arc = &prep.arcs[o.arc];
            let mut id = arc.id.clone();
            while ids.contains(&id) {
                id.push('R');
            }
            ids.push(id.clone());
            neatwork_nw5::Arc {
                id,
                from: arc.pred.clone(),
                to: arc.succ.clone(),
                length: arc.length,
                length1: o.segments[0].length,
                pipe1: pipe_id(&o.segments[0]),
                length2: o.segments[1].length,
                pipe2: pipe_id(&o.segments[1]),
                state: o.state,
                open: arc.open,
            }
        })
        .collect();
    net.clients = clients.to_vec();
    if p.uniform_dem {
        for t in &mut net.terminals {
            t.pressure = p.pressure;
        }
    }
    let input_pipes = &input.pipes;
    net.pipes = prep
        .catalog
        .pipes
        .iter()
        .filter_map(|q| input_pipes.iter().find(|x| x.id.trim() == q.id).cloned())
        .collect();
    net.params = p.to_params(&input.params);
    net.loadflow = post
        .tah
        .iter()
        .map(|t| (t.succ.clone(), t.design_flow, t.margin))
        .collect();
    net.loadfactor_raw.clear();
    net.extra_sections.retain(|(h, _)| {
        !STALE_SECTIONS.contains(
            &neatwork_nw5::read::canonical_section(h)
                .replace(' ', "")
                .as_str(),
        )
    });
    net
}

/// Add the likely causes of an infeasible model to a solver failure.
fn explain_failure(e: Error, lp: &model::LpModel, arcs: &[prepare::DesignArc]) -> Error {
    let Error::Solver(msg) = e else { return e };
    let hints: Vec<String> = lp
        .infeasibility_hints()
        .into_iter()
        .filter_map(|(a, h)| {
            let arc = arcs.get(a)?;
            Some(match h {
                model::InfeasibilityHint::NoAllowedPipe => format!(
                    "no catalogue pipe holds the static pressure ({:.1} m) on the link to {}",
                    arc.pmax, arc.succ
                ),
                model::InfeasibilityHint::NoHead(h) => format!(
                    "no head left for losses at {} ({h:.2} m): the node is at or above the source, or its required pressure is too high",
                    arc.succ
                ),
            })
        })
        .collect();
    if hints.is_empty() {
        Error::Solver(msg)
    } else {
        Error::Solver(format!("{msg}\nLikely cause:\n  {}", hints.join("\n  ")))
    }
}
