//! Post-processing of the solution (`processSol.m`, `reducePipeMixture.m`),
//! `rust/nw5/specs/design.md` section 5.

use neatwork_nw5::{ArcState, Error, Result};

use crate::loads::LoadTable;
use crate::model::Solution;
use crate::params::DesignParams;
use crate::prepare::{Catalog, DesignArc, PathMatrix, Segment, Tr};

/// `reducePipeMixture`: keep `Σx` and `resistanceᵀx` with at most two of
/// the active pipes, at minimum cost (lower convex hull).
pub fn reduce_pipe_mixture(x: &[f64], resistance: &[f64], cost: &[f64]) -> Result<Vec<f64>> {
    let bad = || Error::Validation("NW5:Mixture: Invalid pipe mixture data.".into());
    if x.len() != resistance.len() || x.len() != cost.len() {
        return Err(bad());
    }
    for i in 0..x.len() {
        if !(x[i].is_finite() && x[i] >= 0.0)
            || !(resistance[i].is_finite() && resistance[i] > 0.0)
            || !(cost[i].is_finite() && cost[i] >= 0.0)
        {
            return Err(bad());
        }
    }
    let active: Vec<usize> = (0..x.len()).filter(|&i| x[i] > 0.0).collect();
    if active.len() <= 2 {
        return Ok(x.to_vec());
    }
    let mass: f64 = x.iter().sum();
    let rx: f64 = resistance.iter().zip(x).map(|(r, v)| r * v).sum();
    let target = rx / mass;
    let mut ids = active;
    ids.sort_by(|&a, &b| {
        resistance[a]
            .total_cmp(&resistance[b])
            .then(cost[a].total_cmp(&cost[b]))
    });
    let mut kept: Vec<usize> = Vec::with_capacity(ids.len());
    for &i in &ids {
        if kept
            .last()
            .is_none_or(|&l| resistance[i] - resistance[l] > 0.0)
        {
            kept.push(i);
        }
    }
    let mut hull: Vec<usize> = Vec::with_capacity(kept.len());
    for &c in &kept {
        while hull.len() >= 2 {
            let (a, b) = (hull[hull.len() - 2], hull[hull.len() - 1]);
            let left = (cost[b] - cost[a]) * (resistance[c] - resistance[b]);
            let right = (cost[c] - cost[b]) * (resistance[b] - resistance[a]);
            if left < right {
                break;
            }
            hull.pop();
        }
        hull.push(c);
    }
    let mut out = vec![0.0; x.len()];
    let (first, last) = (hull[0], hull[hull.len() - 1]);
    if hull.len() == 1 || target <= resistance[first] {
        out[first] = mass;
    } else if target >= resistance[last] {
        out[last] = mass;
    } else {
        let k = hull
            .iter()
            .position(|&h| resistance[h] >= target)
            .unwrap_or(hull.len() - 1);
        let (a, b) = (hull[k - 1], hull[k]);
        let w = (resistance[b] - target) / (resistance[b] - resistance[a]);
        out[a] = mass * w;
        out[b] = mass * (1.0 - w);
    }
    let r_out: f64 = resistance.iter().zip(&out).map(|(r, v)| r * v).sum();
    if !((r_out - rx).abs() <= 1e-10 * rx.abs().max(f64::MIN_POSITIVE)) {
        return Err(Error::Validation(
            "NW5:Mixture: Pipe reduction changed the target resistance.".into(),
        ));
    }
    let c_in: f64 = cost.iter().zip(x).map(|(c, v)| c * v).sum();
    let c_out: f64 = cost.iter().zip(&out).map(|(c, v)| c * v).sum();
    if !(c_out <= c_in + 1e-10 * c_in.abs().max(1.0)) {
        return Err(Error::Validation(
            "NW5:Mixture: Pipe reduction increased the cost.".into(),
        ));
    }
    Ok(out)
}

/// Origin of a physical pipe row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nature {
    /// Existing equipment (`equip`, `old` in `TCP`).
    Old,
    /// Reinforcement (`reinf`, `new` in `TCP`).
    New,
}

impl Nature {
    /// `old` or `new`.
    pub fn as_str(self) -> &'static str {
        match self {
            Nature::Old => "old",
            Nature::New => "new",
        }
    }
}

/// A physical pipe row of the designed network (`TA_out`).
#[derive(Clone, Debug, PartialEq)]
pub struct OutArc {
    /// Position of the link in the design arcs.
    pub arc: usize,
    /// Existing pipe or reinforcement.
    pub nature: Nature,
    /// `lon1, ID1` (larger diameter of a reinforcement) and `lon2, ID2`.
    pub segments: [Segment; 2],
    /// `N`: `R` for a reinforcement of an `N` link.
    pub state: ArcState,
}

/// `TAH` row: hydraulics of a link.
#[derive(Clone, Debug, PartialEq)]
pub struct TahRow {
    /// `pred`
    pub pred: String,
    /// `succ`
    pub succ: String,
    /// `equivResist`: linear resistance of the link, per m.
    pub equiv_resist: f64,
    /// `designFlow`, L/s.
    pub design_flow: f64,
    /// `pathLoss`: head loss from the source to `succ`, m.
    pub path_loss: f64,
    /// `margin`, m.
    pub margin: f64,
}

/// `TCP` row.
#[derive(Clone, Debug, PartialEq)]
pub struct TcpRow {
    /// `pred`
    pub pred: String,
    /// `succ`
    pub succ: String,
    /// `nature`: `old` or `new`.
    pub nature: Nature,
    /// `diam1`, m (0 when no pipe).
    pub diam1: f64,
    /// `diam2`, m (0 when no pipe).
    pub diam2: f64,
}

/// Kind of head constraint at a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintType {
    /// Non-terminal node: residual pressure ≥ 0.
    IntrusionPrevention,
    /// Terminal: demand satisfied.
    DemandSatisfaction,
}

impl ConstraintType {
    /// `intrusion_prevention` or `demand_satisfaction`.
    pub fn as_str(self) -> &'static str {
        match self {
            ConstraintType::IntrusionPrevention => "intrusion_prevention",
            ConstraintType::DemandSatisfaction => "demand_satisfaction",
        }
    }
}

/// `TS` row (`!ConstraintSatisfaction`), m.
#[derive(Clone, Debug, PartialEq)]
pub struct TsRow {
    /// `noeud` (= `succ`).
    pub node: String,
    /// `constraint_type`
    pub constraint_type: ConstraintType,
    /// `head_required`
    pub head_required: f64,
    /// `head_available`: residual pressure.
    pub head_available: f64,
    /// `head_margin`
    pub head_margin: f64,
    /// `head_deficit = max(0, −margin)`.
    pub head_deficit: f64,
}

/// `reinfcost` and the costs of `ctx.reinforcement`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Costs {
    /// `costOptim`: pipe cost of the solver solution.
    pub optim: f64,
    /// `costPipesAfterReduction`
    pub after_reduction: f64,
    /// `costInter` (= `costPipesBeforeRounding`).
    pub inter: f64,
    /// `costFinal` (= `costPipesAfterRounding`).
    pub final_cost: f64,
    /// `costFixed`
    pub fixed: f64,
    /// `costLP`: solver objective.
    pub lp: f64,
}

/// Output of [`process_solution`].
#[derive(Clone, Debug, Default)]
pub struct PostResult {
    /// `TA_out`: existing pipes, then reinforcements, each in arc order.
    pub out_arcs: Vec<OutArc>,
    /// `TAH`, in arc order.
    pub tah: Vec<TahRow>,
    /// `TCP`, in `out_arcs` order.
    pub tcp: Vec<TcpRow>,
    /// `TS`, in arc order.
    pub ts: Vec<TsRow>,
    /// `(orif_ideal, orif_com)` at `succ` of each arc, m (1 = none).
    pub orifices: Vec<(f64, f64)>,
    /// Costs.
    pub costs: Costs,
    /// Arcs whose mixture had more than two pipes (`reducedMixtureArcs`).
    pub reduced_mixture_arcs: Vec<usize>,
    /// Final fractions `x[a][d]` after rounding.
    pub x: Vec<Vec<f64>>,
    /// Warnings (`NW5:PressureAfterRounding`).
    pub warnings: Vec<String>,
}

/// `parallelResistance`: `(c1 + c2)^(−λ)`, `c = r^(−1/λ)`, 0 for `inf`.
pub fn parallel_resistance(r1: f64, r2: f64, lambda: f64) -> f64 {
    let c = |r: f64| {
        if r.is_finite() {
            r.powf(-1.0 / lambda)
        } else {
            0.0
        }
    };
    (c(r1) + c(r2)).powf(-lambda)
}

fn pipe_cost(arcs: &[DesignArc], catalog: &Catalog, x: &[Vec<f64>]) -> f64 {
    arcs.iter()
        .zip(x)
        .map(|(a, xa)| {
            a.length
                * catalog
                    .pipes
                    .iter()
                    .zip(xa)
                    .map(|(p, v)| p.cost * v)
                    .sum::<f64>()
        })
        .sum()
}

/// Inputs of [`process_solution`] other than the solution.
#[derive(Clone, Copy, Debug)]
pub struct PostInput<'a> {
    /// Arcs in model order.
    pub arcs: &'a [DesignArc],
    /// Path matrix.
    pub paths: &'a PathMatrix,
    /// Real catalogue.
    pub catalog: &'a Catalog,
    /// `TR`
    pub tr: &'a Tr,
    /// `TL`
    pub loads: &'a LoadTable,
    /// Parameters.
    pub params: &'a DesignParams,
    /// Orifice catalogue `TO.in_diam`, m.
    pub orifices: &'a [f64],
}

fn fail(code: &str, msg: String) -> Error {
    Error::Validation(format!("{code}: {msg}"))
}

/// `processSol` steps A-J (`rust/nw5/specs/design.md` 5.1).
pub fn process_solution(input: PostInput<'_>, sol: &Solution) -> Result<PostResult> {
    let PostInput {
        arcs,
        paths,
        catalog,
        tr,
        loads,
        params: p,
        orifices,
    } = input;
    let nb_a = arcs.len();
    let nb_d = catalog.pipes.len();
    let lambda = p.lambda;
    // A. integrity
    if !(p.secu >= 1.0) {
        return Err(fail("NW5:Input", "secu must be >= 1".into()));
    }
    if sol.x.len() != nb_a || sol.x.iter().any(|r| r.len() != nb_d) || sol.y.len() != nb_a {
        return Err(fail(
            "NW5:Input",
            "solution sizes do not match the arcs and the catalogue".into(),
        ));
    }
    if tr.rexist.len() != nb_a
        || tr.r.len() != nb_a
        || paths.paths.len() != nb_a
        || loads.values.len() != nb_a
    {
        return Err(fail(
            "NW5:PipeOrder",
            "rows in TR and TA are not aligned.".into(),
        ));
    }
    if sol.y.iter().any(|&v| !(v < 1e-6 || v > 1.0 - 1e-6)) {
        return Err(fail("NW5:Input", "Variable y is not binary".into()));
    }
    for (a, xa) in sol.x.iter().enumerate() {
        if xa.iter().any(|&v| !(v >= -1e-6)) {
            return Err(fail(
                "NW5:Input",
                format!("negative fraction on arc {}", a + 1),
            ));
        }
        let s: f64 = xa.iter().sum();
        if !((s - sol.y[a].round()).abs() <= 1e-6) {
            return Err(fail(
                "NW5:Input",
                format!("fractions of arc {} do not sum to y", a + 1),
            ));
        }
    }
    let reinforced: Vec<bool> = sol.y.iter().map(|&v| v > 0.5).collect();
    let equipped: Vec<bool> = arcs.iter().map(DesignArc::equipped).collect();
    for (a, arc) in arcs.iter().enumerate() {
        if !(arc.length.is_finite() && arc.length > 0.0) {
            return Err(fail(
                "NW5:Input",
                format!("link {} -> {} has no positive length", arc.pred, arc.succ),
            ));
        }
        if !equipped[a] && !reinforced[a] {
            return Err(fail(
                "NW5:DisconnectedArc",
                format!(
                    "Non equipped base arc {} -> {} should be reinforced.",
                    arc.pred, arc.succ
                ),
            ));
        }
        if arc.state == ArcState::Blocked && reinforced[a] {
            return Err(fail(
                "NW5:BlockedArc",
                format!("blocked link {} -> {} is reinforced", arc.pred, arc.succ),
            ));
        }
        if equipped[a] && !(tr.rexist[a].is_finite() && tr.rexist[a] > 0.0)
            || !equipped[a] && tr.rexist[a].is_finite()
        {
            return Err(fail(
                "NW5:Resistance",
                "TR.rexist does not match the base equipement.".into(),
            ));
        }
        if tr.r[a].iter().any(|&r| !(r.is_finite() && r > 0.0)) {
            return Err(fail(
                "NW5:Resistance",
                "Resistances candidates invalides dans TR.".into(),
            ));
        }
        if loads.values[a]
            .iter()
            .any(|&q| !(q.is_finite() && q >= 0.0))
        {
            return Err(fail("NW5:Input", "TL must be finite and >= 0".into()));
        }
    }
    if catalog
        .pipes
        .iter()
        .any(|q| !(q.resist.is_finite() && q.resist > 0.0))
    {
        return Err(fail(
            "NW5:Resistance",
            "TD.resist for real pipes must be finite and positive.".into(),
        ));
    }
    let nw4_orifices = p.uniform_dem && !p.bounded_flow;
    if nw4_orifices && orifices.iter().any(|&d| !(d.is_finite() && d > 0.0)) {
        return Err(fail(
            "NW5:Input",
            "orifice diameters must be finite and > 0".into(),
        ));
    }
    let diam = |s: &Segment| {
        s.pipe
            .and_then(|d| catalog.pipes.get(d))
            .map_or(0.0, |q| q.diameter)
    };
    let cost: Vec<f64> = catalog.pipes.iter().map(|q| q.cost).collect();
    let resist: Vec<f64> = catalog.pipes.iter().map(|q| q.resist).collect();

    // B. mixture reduction
    let mut x = sol.x.clone();
    let cost_optim = pipe_cost(arcs, catalog, &x);
    let mut reduced = Vec::new();
    for (a, xa) in x.iter_mut().enumerate() {
        if xa.iter().filter(|&&v| v > 0.0).count() > 2 {
            *xa = reduce_pipe_mixture(xa, &tr.r[a], &cost)?;
            reduced.push(a);
        }
    }
    let cost_after_reduction = pipe_cost(arcs, catalog, &x);

    // C. IDs of the reinforcement, larger diameter first
    let used: Vec<Vec<usize>> = x
        .iter()
        .map(|xa| {
            let mut j: Vec<usize> = (0..nb_d).filter(|&d| xa[d] > 0.0).collect();
            j.sort_by(|&a, &b| {
                catalog.pipes[b]
                    .diameter
                    .total_cmp(&catalog.pipes[a].diameter)
            });
            j
        })
        .collect();
    for a in (0..nb_a).filter(|&a| reinforced[a]) {
        if used[a].is_empty() || used[a].len() > 2 {
            return Err(fail(
                "NW5:Solution",
                format!("Nombre de tuyaux incorrect sur l'arc {}", a + 1),
            ));
        }
    }

    // D. fixed cost
    let cost_fixed: f64 = arcs.iter().zip(&sol.y).map(|(a, y)| a.fixed_cost * y).sum();

    // E. junction removal on doubled arcs
    let mut split_doubled = Vec::new();
    for a in 0..nb_a {
        let ids: Vec<usize> = (0..nb_d).filter(|&d| x[a][d] > 0.0).collect();
        if !(reinforced[a] && equipped[a] && ids.len() == 2) {
            continue;
        }
        let r_target: f64 = ids.iter().map(|&d| x[a][d] * tr.r[a][d]).sum();
        let rho = [resist[ids[0]], resist[ids[1]]];
        let delta_c = r_target.powf(-1.0 / lambda) - tr.rexist[a].powf(-1.0 / lambda);
        if !(delta_c > 0.0) {
            return Err(fail(
                "NW5:Junction",
                format!(
                    "Arc {} : resistance equivalent incompatible avec l'existant.",
                    a + 1
                ),
            ));
        }
        let rho_t = delta_c.powf(-lambda);
        split_doubled.push((a, r_target));
        if rho[0] == rho[1] {
            if !((rho_t - rho[0]).abs() <= 1e-8 * rho[0].max(1.0)) {
                return Err(fail(
                    "NW5:Junction",
                    format!("Arc {} : resistance cible incoherente.", a + 1),
                ));
            }
            continue;
        }
        let eta = (rho_t - rho[1]) / (rho[0] - rho[1]);
        if !((-1e-8..=1.0 + 1e-8).contains(&eta)) {
            return Err(fail(
                "NW5:Junction",
                format!("Arc {} : proportions inadmissibles.", a + 1),
            ));
        }
        let eta = eta.clamp(0.0, 1.0);
        x[a][ids[0]] = eta;
        x[a][ids[1]] = 1.0 - eta;
    }
    for &(a, r_target) in &split_doubled {
        let r_before: f64 = (0..nb_d).map(|d| resist[d] * x[a][d]).sum();
        let r_eq = parallel_resistance(tr.rexist[a], r_before, lambda);
        if !((r_eq - r_target).abs() / r_target.abs().max(f64::MIN_POSITIVE) < 1e-7) {
            return Err(fail(
                "NW5:Junction",
                "La suppression du raccordement ne conserve pas la resistance.".into(),
            ));
        }
    }
    let cost_inter = pipe_cost(arcs, catalog, &x);

    // F. commercial lengths: the larger diameter is rounded up, no tolerance
    // before ceil (processSol.m:210-211, B11)
    let mut new_segments = vec![[Segment::default(); 2]; nb_a];
    for a in (0..nb_a).filter(|&a| reinforced[a]) {
        let j = &used[a];
        let l = arcs[a].length;
        if j.len() == 1 {
            new_segments[a][0] = Segment {
                length: l,
                pipe: Some(j[0]),
            };
        } else {
            let mut l1 = l * x[a][j[0]];
            if p.pipelength > 0.1 {
                l1 = l.min((l1 / p.pipelength).ceil() * p.pipelength);
            }
            new_segments[a] = [
                Segment {
                    length: l1,
                    pipe: Some(j[0]),
                },
                Segment {
                    length: l - l1,
                    pipe: Some(j[1]),
                },
            ];
            x[a][j[0]] = l1 / l;
            x[a][j[1]] = (l - l1) / l;
        }
        for s in &mut new_segments[a] {
            if s.length == 0.0 {
                s.pipe = None;
            }
        }
    }
    let cost_final = pipe_cost(arcs, catalog, &x);

    // G. margins on the tree
    let mut r_eq = Vec::with_capacity(nb_a);
    for a in 0..nb_a {
        let r_new = if reinforced[a] {
            (0..nb_d).map(|d| resist[d] * x[a][d]).sum()
        } else {
            f64::INFINITY
        };
        let r = parallel_resistance(tr.rexist[a], r_new, lambda);
        if !(r.is_finite() && r > 0.0) {
            return Err(fail(
                "NW5:Resistance",
                "A link has an infinite resistance.".into(),
            ));
        }
        r_eq.push(r);
    }
    let q_max: Vec<f64> = (0..nb_a).map(|a| loads.row_max(a) / 1000.0).collect();
    let connection: Vec<f64> = (0..nb_a)
        .map(|a| arcs[a].length * r_eq[a] * q_max[a].powf(lambda))
        .collect();
    let path_loss = paths.path_sum(&connection);
    let cible = p.targetflow / 1000.0;
    let mut tah = Vec::with_capacity(nb_a);
    let mut ts = Vec::with_capacity(nb_a);
    let mut warnings = Vec::new();
    let mut margins = Vec::with_capacity(nb_a);
    for (a, arc) in arcs.iter().enumerate() {
        let mut available = -arc.haut;
        let mut required = 0.0;
        if arc.terminal() {
            available /= p.secu.max(f64::EPSILON);
            if p.bounded_flow {
                required = arc.pressure;
            } else if !p.uniform_dem {
                return Err(fail(
                    "NW5:Orifices",
                    "Demandes heterogenes non bornees non traitees.".into(),
                ));
            }
            if p.uniform_dem {
                required += cible * cible / p.faucetcoef.max(f64::EPSILON);
            }
        }
        let residual = available - path_loss[a];
        let margin = residual - required;
        margins.push(margin);
        tah.push(TahRow {
            pred: arc.pred.clone(),
            succ: arc.succ.clone(),
            equiv_resist: r_eq[a],
            design_flow: q_max[a] * 1000.0,
            path_loss: path_loss[a],
            margin,
        });
        ts.push(TsRow {
            node: arc.succ.clone(),
            constraint_type: if arc.terminal() {
                ConstraintType::DemandSatisfaction
            } else {
                ConstraintType::IntrusionPrevention
            },
            head_required: required,
            head_available: residual,
            head_margin: margin,
            head_deficit: (-margin).max(0.0),
        });
    }
    let violated: Vec<f64> = ts
        .iter()
        .map(|t| t.head_deficit)
        .filter(|&d| d > 1e-6)
        .collect();
    if !violated.is_empty() {
        warnings.push(format!(
            "NW5:PressureAfterRounding: {} contraintes hydrauliques depassees apres ajustement. Violation maximale : {} m.",
            violated.len(),
            crate::prepare::fmt_g(violated.iter().copied().fold(0.0, f64::max), 6)
        ));
    }

    // H. orifices (NW4 case only)
    let mut orif = vec![(1.0, 1.0); nb_a];
    if nw4_orifices {
        let cand: Vec<f64> = orifices
            .iter()
            .copied()
            .chain(std::iter::once(1.0))
            .collect();
        for (a, arc) in arcs.iter().enumerate() {
            if !(arc.terminal() && margins[a] >= 0.1) {
                continue;
            }
            let ideal = (p.orifcoef * cible.sqrt() * margins[a].powf(-0.25)).min(1.0);
            let mut best = cand[0];
            let mut best_gap = f64::INFINITY;
            for &c in &cand {
                let gap = (ideal.powi(-4) - c.powi(-4)).abs();
                if gap < best_gap {
                    best_gap = gap;
                    best = c;
                }
            }
            orif[a] = (ideal, best);
        }
    }

    // J. outputs
    let mut out_arcs = Vec::new();
    for (a, arc) in arcs.iter().enumerate().filter(|(a, _)| equipped[*a]) {
        out_arcs.push(OutArc {
            arc: a,
            nature: Nature::Old,
            segments: arc.segments,
            state: arc.state,
        });
    }
    for (a, arc) in arcs.iter().enumerate().filter(|(a, _)| reinforced[*a]) {
        out_arcs.push(OutArc {
            arc: a,
            nature: Nature::New,
            segments: new_segments[a],
            state: if arc.state == ArcState::New {
                ArcState::Reinforcement
            } else {
                arc.state
            },
        });
    }
    for o in &out_arcs {
        let l = arcs[o.arc].length;
        let [s1, s2] = o.segments;
        let consistent = |s: &Segment| (s.length == 0.0) == s.pipe.is_none();
        if !((s1.length + s2.length - l).abs() <= 1e-8 * l.max(1.0))
            || !consistent(&s1)
            || !consistent(&s2)
        {
            return Err(fail(
                "NW5:Output",
                format!(
                    "Longueurs et ID des troncons incoherents ({} -> {}).",
                    arcs[o.arc].pred, arcs[o.arc].succ
                ),
            ));
        }
    }
    let tcp = out_arcs
        .iter()
        .map(|o| TcpRow {
            pred: arcs[o.arc].pred.clone(),
            succ: arcs[o.arc].succ.clone(),
            nature: o.nature,
            diam1: diam(&o.segments[0]),
            diam2: diam(&o.segments[1]),
        })
        .collect();
    Ok(PostResult {
        out_arcs,
        tah,
        tcp,
        ts,
        orifices: orif,
        costs: Costs {
            optim: cost_optim,
            after_reduction: cost_after_reduction,
            inter: cost_inter,
            final_cost: cost_final,
            fixed: cost_fixed,
            lp: sol.fval,
        },
        reduced_mixture_arcs: reduced,
        x,
        warnings,
    })
}
