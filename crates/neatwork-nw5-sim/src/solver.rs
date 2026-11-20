//! Per-scenario problem (`ScenarioProblemBuilder`, spec 4.2) and the
//! predictor-corrector interior-point solver (`InteriorPointSolver`, spec
//! 4.3 to 4.5), reproduced operation by operation.

use crate::chol::{Symbolic, Values};
use crate::error::StageError;
use crate::options::SolverOptions;
use crate::prep::Model;
use crate::result::{Diagnostics, PipeKind};

/// `UNBOUNDED_FLOW_MULTIPLIER`.
const UNBOUNDED_FLOW_MULTIPLIER: f64 = 20.0;
/// `_MIN_INVD`.
const MIN_INVD: f64 = 1e-16;
const DELTA_MIN: f64 = 1e-14;
const DELTA_MULT: f64 = 10.0;
const MAX_REG_TRIES: usize = 8;

/// Run-wide solver context: the model, the mode and the symbolic Cholesky.
pub(crate) struct Context<'a> {
    pub(crate) model: &'a Model,
    pub(crate) uniform: bool,
    pub(crate) bounded: bool,
    pub(crate) commercial: bool,
    pub(crate) lambda: f64,
    pub(crate) symbolic: Symbolic,
    /// Position of each transit-transit pipe in the factor structure.
    pub(crate) position: Vec<Option<usize>>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        model: &'a Model,
        uniform: bool,
        bounded: bool,
        commercial: bool,
        lambda: f64,
    ) -> Context<'a> {
        let edges: Vec<(usize, usize)> = model
            .rows
            .iter()
            .zip(&model.pipes)
            .filter_map(|(r, p)| match r {
                (Some(a), Some(b)) if p.open => Some((*a, *b)),
                _ => None,
            })
            .collect();
        let symbolic = Symbolic::new(model.transit.len(), &edges);
        let position = model
            .rows
            .iter()
            .zip(&model.pipes)
            .map(|(r, p)| match r {
                (Some(a), Some(b)) if p.open => symbolic.position(*a, *b),
                _ => None,
            })
            .collect();
        Context {
            model,
            uniform,
            bounded,
            commercial,
            lambda,
            symbolic,
            position,
        }
    }
}

/// A prepared scenario problem.
#[derive(Clone, Debug)]
pub(crate) struct Problem {
    /// Open pipes (indices in the sorted pipes).
    pub(crate) open: Vec<usize>,
    pub(crate) open_mask: Vec<bool>,
    /// Bounds of the open pipes (before the solver scaling).
    pub(crate) lo: Vec<f64>,
    pub(crate) up: Vec<f64>,
    /// Terminal loss coefficient of every pipe.
    pub(crate) alpha: Vec<f64>,
    pub(crate) served: Vec<bool>,
    pub(crate) demand_m3s: Vec<f64>,
    pub(crate) counts: Vec<u32>,
}

/// `ScenarioProblemBuilder.build`.
pub(crate) fn build(
    ctx: &Context,
    demand_lps: &[f64],
    counts: &[u32],
) -> Result<Problem, StageError> {
    let m = ctx.model;
    let d: Vec<f64> = demand_lps.iter().map(|v| v / 1000.0).collect();
    if d.iter().any(|v| !v.is_finite()) {
        return Err(StageError::new("Scenario demand contains invalid values."));
    }
    if d.iter().any(|v| *v < 0.0) {
        return Err(StageError::new("Scenario demand cannot be negative."));
    }
    let active: Vec<bool> = d.iter().map(|v| *v > 0.0).collect();
    let inconsistent: Vec<String> = (0..d.len())
        .filter(|&t| active[t] && counts[t] == 0)
        .map(|t| m.nodes[m.terminals[t]].id.clone())
        .collect();
    if !inconsistent.is_empty() {
        return Err(
            StageError::new("A terminal has positive demand but no open client.")
                .detail("terminalIds", inconsistent),
        );
    }
    if !active.iter().any(|a| *a) {
        return Err(StageError::new("An empty scenario cannot be prepared."));
    }
    // alpha (problem_builder.py:263-352)
    let n = m.pipes.len();
    let mut alpha = vec![0.0; n];
    if ctx.uniform && !ctx.bounded {
        let base = if ctx.commercial {
            &m.alpha_com
        } else {
            &m.alpha_ideal
        };
        for (t, &j) in m.terminal_pipe.iter().enumerate() {
            if active[t] {
                let c = f64::from(counts[t]);
                alpha[j] = base[j] / (c * c);
            }
        }
    } else if !ctx.uniform && !ctx.bounded {
        return Err(StageError::new(
            "Heterogeneous unbounded-flow simulation is not supported.",
        ));
    } else {
        for (t, &j) in m.terminal_pipe.iter().enumerate() {
            if active[t] {
                alpha[j] = m.preq[t] / (d[t] * d[t]);
            }
        }
    }
    if alpha.iter().any(|a| !a.is_finite()) {
        return Err(StageError::new(
            "Scenario alpha calculation produced invalid values.",
        ));
    }
    if alpha.iter().any(|a| *a < 0.0) {
        return Err(StageError::new("Scenario alpha values cannot be negative."));
    }
    // bounds (problem_builder.py:354-438)
    let total = crate::stats::np_sum(&d);
    if total.is_nan() || total <= 0.0 {
        return Err(StageError::new("Total scenario demand must be positive."));
    }
    let global = if ctx.bounded {
        total
    } else {
        UNBOUNDED_FLOW_MULTIPLIER * total
    };
    let mut up = vec![global; n];
    let mut lo = vec![-global; n];
    for (t, &j) in m.terminal_pipe.iter().enumerate() {
        up[j] = if ctx.bounded {
            d[t]
        } else {
            UNBOUNDED_FLOW_MULTIPLIER * d[t]
        };
        lo[j] = 0.0;
    }
    for j in 0..n {
        if m.kind[j] == PipeKind::Source {
            lo[j] = 0.0;
        }
        if !m.pipes[j].open {
            lo[j] = 0.0;
            up[j] = 0.0;
        }
    }
    if lo.iter().chain(&up).any(|v| !v.is_finite()) {
        return Err(StageError::new("Flow bounds contain invalid values."));
    }
    let open_mask: Vec<bool> = (0..n).map(|j| lo[j] != up[j]).collect();
    let open: Vec<usize> = (0..n).filter(|&j| open_mask[j]).collect();
    let served: Vec<bool> = (0..d.len())
        .map(|t| active[t] && m.pipes[m.terminal_pipe[t]].open)
        .collect();
    // checks (problem_builder.py:515-635)
    if open.is_empty() {
        return Err(StageError::new("The scenario contains no open pipe."));
    }
    if !open.iter().any(|&j| m.kind[j] == PipeKind::Source) {
        return Err(StageError::new(
            "The scenario contains no open source pipe.",
        ));
    }
    if open
        .iter()
        .any(|&j| lo[j] >= up[j] || lo[j].is_nan() || up[j].is_nan())
    {
        return Err(StageError::new(
            "Every open pipe must have a non-empty flow interval.",
        ));
    }
    let mut row_used = vec![false; m.transit.len()];
    for &j in &open {
        let (a, b) = m.rows[j];
        for r in [a, b].into_iter().flatten() {
            row_used[r] = true;
        }
    }
    let isolated: Vec<String> = (0..m.transit.len())
        .filter(|&r| !row_used[r])
        .map(|r| m.nodes[m.transit[r]].id.clone())
        .collect();
    if !isolated.is_empty() {
        return Err(
            StageError::new("Closing pipes isolated one or more transit nodes.")
                .detail("transitNodeIds", isolated),
        );
    }
    Ok(Problem {
        lo: open.iter().map(|&j| lo[j]).collect(),
        up: open.iter().map(|&j| up[j]).collect(),
        open,
        open_mask,
        alpha,
        served,
        demand_m3s: d,
        counts: counts.to_vec(),
    })
}

/// Solver output on the open pipes.
#[derive(Clone, Debug)]
pub(crate) struct Solution {
    pub(crate) x: Vec<f64>,
    pub(crate) u: Vec<f64>,
    pub(crate) charge: Vec<f64>,
    pub(crate) iterations: usize,
    pub(crate) converged: bool,
    pub(crate) diagnostics: Diagnostics,
}

/// Open-pipe view of the problem data.
struct Local {
    rows: Vec<(Option<usize>, Option<usize>)>,
    position: Vec<Option<usize>>,
    beta: Vec<f64>,
    alpha: Vec<f64>,
    gravity: Vec<f64>,
}

impl Local {
    /// `y = A x` (rows: transit nodes).
    fn a_mul(&self, x: &[f64], y: &mut [f64]) {
        y.fill(0.0);
        for (j, &(a, b)) in self.rows.iter().enumerate() {
            if let Some(r) = a {
                y[r] += -x[j];
            }
            if let Some(r) = b {
                y[r] += x[j];
            }
        }
    }

    /// `y = Aᵀ u`.
    fn at_mul(&self, u: &[f64], y: &mut [f64]) {
        for (j, &(a, b)) in self.rows.iter().enumerate() {
            let mut v = 0.0;
            if let Some(r) = a {
                v += -u[r];
            }
            if let Some(r) = b {
                v += u[r];
            }
            y[j] = v;
        }
    }
}

/// Factorization of `A diag(w) Aᵀ` with the adaptive regularization of
/// `chol_with_regularization` (pipes with `w = None` are left out).
fn factorize(
    sym: &Symbolic,
    local: &Local,
    w: &[Option<f64>],
    m: &mut Values,
    l: &mut Values,
    work: &mut Vec<f64>,
) -> Result<(), StageError> {
    m.diag.fill(0.0);
    m.lower.fill(0.0);
    for (j, &(a, b)) in local.rows.iter().enumerate() {
        let Some(wj) = w[j] else { continue };
        for r in [a, b].into_iter().flatten() {
            m.diag[sym.step(r)] += wj;
        }
        if let Some(p) = local.position[j] {
            m.lower[p] -= wj;
        }
    }
    if sym.factor(m, 0.0, l, work) {
        return Ok(());
    }
    let mut delta = DELTA_MIN;
    for _ in 0..MAX_REG_TRIES {
        if sym.factor(m, delta, l, work) {
            return Ok(());
        }
        delta *= DELTA_MULT;
    }
    Err(
        StageError::new("Cholesky factorization failed after adaptive regularization.")
            .detail("regularizationAttempts", MAX_REG_TRIES)
            .detail("lastDelta", delta / DELTA_MULT),
    )
}

/// Max with numpy's NaN propagation.
fn nan_max(acc: f64, v: f64) -> f64 {
    if acc.is_nan() || v.is_nan() {
        f64::NAN
    } else {
        acc.max(v)
    }
}

/// Min with numpy's NaN propagation.
fn nan_min(acc: f64, v: f64) -> f64 {
    if acc.is_nan() || v.is_nan() {
        f64::NAN
    } else {
        acc.min(v)
    }
}

/// `step_length` (`sim/numerical_utils.py:46-68`).
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors step_length(x, lo, up, s, t, dx, ds, dt)"
)]
fn step_length(
    x: &[f64],
    lo: &[f64],
    up: &[f64],
    s: &[f64],
    t: &[f64],
    dx: &[f64],
    ds: &[f64],
    dt: &[f64],
) -> f64 {
    let mut th = [1.0f64; 4];
    let mut found = [false; 4];
    for j in 0..x.len() {
        let mut take = |k: usize, v: f64| {
            th[k] = if found[k] { nan_min(th[k], v) } else { v };
            found[k] = true;
        };
        if dx[j] < 0.0 {
            take(0, -(x[j] - lo[j]) / dx[j]);
        }
        if dx[j] > 0.0 {
            take(1, (up[j] - x[j]) / dx[j]);
        }
        if ds[j] < 0.0 {
            take(2, s[j] / (-ds[j]));
        }
        if dt[j] < 0.0 {
            take(3, t[j] / (-dt[j]));
        }
    }
    th.iter().fold(1.0, |acc, &v| nan_min(acc, v))
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |acc, (x, y)| acc + x * y)
}

struct Derivatives {
    gradf: Vec<f64>,
    f: Vec<f64>,
    g: Vec<f64>,
}

/// `_friction_derivatives` (gradient, second and third derivatives).
fn derivatives(local: &Local, x: &[f64], lbd: f64, tolx: f64, out: &mut Derivatives) {
    let t_lm1 = tolx.powf(lbd - 1.0);
    for (j, &xj) in x.iter().enumerate() {
        let (b, a, g) = (local.beta[j], local.alpha[j], local.gravity[j]);
        let ax = xj.abs();
        if ax > tolx {
            let p = ax.powf(lbd - 1.0);
            out.gradf[j] = b * p * xj + a * ax * xj + g;
            out.f[j] = lbd * b * p + 2.0 * a * ax;
            out.g[j] = lbd * (lbd - 1.0) * b * p / xj + 2.0 * a * xj.signum();
        } else {
            out.gradf[j] = t_lm1 * b * xj + tolx * a * xj + g;
            out.f[j] = t_lm1 * b + tolx * a;
            out.g[j] = 0.0;
        }
    }
}

/// `_has_converged`.
fn has_converged(
    x: &[f64],
    lo: &[f64],
    up: &[f64],
    charge: &[f64],
    rp: &[f64],
    o: &SolverOptions,
) -> bool {
    let tolx = o.flow_tolerance;
    let tolp = o.pressure_tolerance;
    let primal = rp.iter().fold(0.0, |acc, v| nan_max(acc, v.abs()));
    if primal > tolx {
        return false;
    }
    let (mut interior, mut upper, mut lower) = (None::<f64>, None::<f64>, None::<f64>);
    for j in 0..x.len() {
        let at_lower = x[j] - lo[j] < tolx;
        let at_upper = up[j] - x[j] < tolx;
        if at_upper {
            upper = Some(upper.map_or(charge[j], |m| nan_max(m, charge[j])));
        }
        if at_lower {
            lower = Some(lower.map_or(charge[j], |m| nan_min(m, charge[j])));
        }
        if !(at_lower || at_upper) {
            interior = Some(interior.map_or(charge[j].abs(), |m| nan_max(m, charge[j].abs())));
        }
    }
    !(interior.is_some_and(|v| v > tolp)
        || upper.is_some_and(|v| v > tolp)
        || lower.is_some_and(|v| v < -tolp))
}

fn solver_error(message: &str, iteration: Option<usize>) -> StageError {
    let e = StageError::new(message);
    match iteration {
        Some(k) => e.detail("iteration", k),
        None => e,
    }
}

/// Reusable buffers of a run.
#[derive(Default)]
pub(crate) struct Workspace {
    m: Option<Values>,
    l: Option<Values>,
    work: Vec<f64>,
}

/// `InteriorPointSolver.solve`.
pub(crate) fn solve(
    ctx: &Context,
    prob: &Problem,
    o: &SolverOptions,
    ws: &mut Workspace,
) -> Result<Solution, StageError> {
    let model = ctx.model;
    let sym = &ctx.symbolic;
    let local = Local {
        rows: prob.open.iter().map(|&j| model.rows[j]).collect(),
        position: prob.open.iter().map(|&j| ctx.position[j]).collect(),
        beta: prob.open.iter().map(|&j| model.beta[j]).collect(),
        alpha: prob.open.iter().map(|&j| prob.alpha[j]).collect(),
        gravity: prob.open.iter().map(|&j| model.gravity[j]).collect(),
    };
    let kind: Vec<PipeKind> = prob.open.iter().map(|&j| model.kind[j]).collect();
    let n = prob.open.len();
    let nrow = model.transit.len();
    let mut mvals = ws.m.take().unwrap_or_else(|| sym.zeros());
    let mut lvals = ws.l.take().unwrap_or_else(|| sym.zeros());
    let work = &mut ws.work;
    let result = (|| -> Result<Solution, StageError> {
        // initial point (solver.py:61-181)
        let mut x = vec![0.0; n];
        let mut u = vec![0.0; nrow];
        let mut lo = prob.lo.clone();
        let mut up = prob.up.clone();
        if !ctx.bounded {
            for j in 0..n {
                if kind[j] == PipeKind::Terminal {
                    x[j] = up[j];
                }
            }
            for v in up.iter_mut() {
                *v *= 5.0;
            }
            for v in lo.iter_mut() {
                *v *= 5.0;
            }
        } else {
            for j in 0..n {
                if kind[j] == PipeKind::Terminal {
                    x[j] = (up[j] + lo[j]) / 2.0;
                }
            }
        }
        let sources: Vec<usize> = (0..n).filter(|&j| kind[j] == PipeKind::Source).collect();
        if sources.is_empty() {
            return Err(solver_error(
                "The solver cannot initialize without an open source pipe.",
                None,
            ));
        }
        let mut r0 = vec![0.0; nrow];
        local.a_mul(&x, &mut r0);
        let mut x_sum = vec![0.0; n];
        let mut w: Vec<Option<f64>> = vec![None; n];
        let mut du = vec![0.0; nrow];
        for &si in &sources {
            for j in 0..n {
                w[j] = if kind[j] == PipeKind::Transit || j == si {
                    Some(1.0)
                } else {
                    None
                };
            }
            factorize(sym, &local, &w, &mut mvals, &mut lvals, work)?;
            du.copy_from_slice(&r0);
            sym.solve(&lvals, &mut du, work);
            for j in 0..n {
                let mut cand = x[j];
                if w[j].is_some() {
                    let (a, b) = local.rows[j];
                    let mut v = 0.0;
                    if let Some(r) = a {
                        v += du[r];
                    }
                    if let Some(r) = b {
                        v += -du[r];
                    }
                    cand += v;
                }
                x_sum[j] += cand;
            }
        }
        let ns = sources.len() as f64;
        for j in 0..n {
            x[j] = x_sum[j] / ns;
        }
        let interior = (0..n).all(|j| {
            let (gl, gu) = (x[j] - lo[j], up[j] - x[j]);
            x[j].is_finite() && gl.is_finite() && gu.is_finite() && gl > 0.0 && gu > 0.0
        });
        if !interior {
            return Err(solver_error(
                "Initial solver point is not strictly inside the flow bounds.",
                None,
            ));
        }
        let mut s: Vec<f64> = (0..n).map(|j| 1.0 / (x[j] - lo[j])).collect();
        let mut t: Vec<f64> = (0..n).map(|j| 1.0 / (up[j] - x[j])).collect();
        let gap_lo: Vec<f64> = (0..n).map(|j| x[j] - lo[j]).collect();
        let gap_up: Vec<f64> = (0..n).map(|j| up[j] - x[j]).collect();
        let mut mu = (dot(&s, &gap_lo) + dot(&t, &gap_up)) / (2 * n) as f64;
        if !mu.is_finite() || mu <= 0.0 {
            return Err(StageError::new("Initial barrier parameter is invalid.").detail("mu", mu));
        }
        let lbd = ctx.lambda;
        let tolx = o.flow_tolerance;
        let mut der = Derivatives {
            gradf: vec![0.0; n],
            f: vec![0.0; n],
            g: vec![0.0; n],
        };
        derivatives(&local, &x, lbd, tolx, &mut der);
        let mut inv_d = vec![1.0; n];
        for j in 0..n {
            w[j] = Some(inv_d[j]);
        }
        factorize(sym, &local, &w, &mut mvals, &mut lvals, work)?;
        let mut converged = false;
        let mut iteration = 0;
        let mut atu = vec![0.0; n];
        let mut charge = vec![0.0; n];
        let mut rp = vec![0.0; nrow];
        let mut rd = vec![0.0; n];
        let mut rhs = vec![0.0; nrow];
        let mut tmp = vec![0.0; n];
        let mut dx_p = vec![0.0; n];
        let mut ds_p = vec![0.0; n];
        let mut dt_p = vec![0.0; n];
        let mut dx = vec![0.0; n];
        let mut ds = vec![0.0; n];
        let mut dt = vec![0.0; n];
        let mut rcs = vec![0.0; n];
        let mut rct = vec![0.0; n];
        let mut rc = vec![0.0; n];
        while iteration < o.max_iterations {
            iteration += 1;
            local.at_mul(&u, &mut atu);
            for j in 0..n {
                charge[j] = der.gradf[j] + atu[j];
                rd[j] = der.gradf[j] + atu[j] - s[j] + t[j];
            }
            local.a_mul(&x, &mut rp);
            if has_converged(&x, &lo, &up, &charge, &rp, o) {
                converged = true;
                derivatives(&local, &x, lbd, tolx, &mut der);
                break;
            }
            // predictor
            for j in 0..n {
                tmp[j] = inv_d[j] * -charge[j];
            }
            local.a_mul(&tmp, &mut rhs);
            for r in 0..nrow {
                rhs[r] += rp[r];
            }
            sym.solve(&lvals, &mut rhs, work);
            local.at_mul(&rhs, &mut atu);
            for j in 0..n {
                let rho = -charge[j];
                dx_p[j] = -inv_d[j] * (atu[j] - rho);
                ds_p[j] = -(s[j] * dx_p[j]) / (x[j] - lo[j]) - s[j];
                dt_p[j] = (t[j] * dx_p[j]) / (up[j] - x[j]) - t[j];
            }
            let theta_aff = step_length(&x, &lo, &up, &s, &t, &dx_p, &ds_p, &dt_p);
            let mut a1 = 0.0;
            let mut a2 = 0.0;
            for j in 0..n {
                a1 += (s[j] + theta_aff * ds_p[j]) * (x[j] + theta_aff * dx_p[j] - lo[j]);
                a2 += (t[j] + theta_aff * dt_p[j]) * (up[j] - x[j] - theta_aff * dx_p[j]);
            }
            let mu_aff = (a1 + a2) / (2 * n) as f64;
            let ratio = mu_aff / mu;
            if ratio < 0.0 {
                // python raises TypeError on a complex power (spec Q11)
                return Err(solver_error(
                    "Interior-point centering ratio is negative.",
                    Some(iteration),
                )
                .detail("ratio", ratio));
            }
            let pw = ratio.powf(o.centering_exponent);
            let sigma_mu = (if pw < 1.0 { pw } else { 1.0 }) * mu;
            // corrector
            for j in 0..n {
                rcs[j] = s[j] + (dx_p[j] * ds_p[j] - sigma_mu) / (x[j] - lo[j]);
                rct[j] = t[j] - (dx_p[j] * dt_p[j] + sigma_mu) / (up[j] - x[j]);
                rc[j] = -(rd[j] + 0.5 * dx_p[j] * der.g[j] * dx_p[j] + rcs[j] - rct[j]);
                tmp[j] = inv_d[j] * rc[j];
            }
            local.a_mul(&tmp, &mut rhs);
            for r in 0..nrow {
                rhs[r] += rp[r];
            }
            sym.solve(&lvals, &mut rhs, work);
            local.at_mul(&rhs, &mut atu);
            for j in 0..n {
                dx[j] = -inv_d[j] * (atu[j] - rc[j]);
                ds[j] = -(s[j] * dx[j]) / (x[j] - lo[j]) - rcs[j];
                dt[j] = (t[j] * dx[j]) / (up[j] - x[j]) - rct[j];
            }
            let theta = o.step_safety * step_length(&x, &lo, &up, &s, &t, &dx, &ds, &dt);
            if !theta.is_finite() || theta <= 0.0 {
                return Err(solver_error(
                    "Interior-point step length is invalid.",
                    Some(iteration),
                )
                .detail("theta", theta));
            }
            for j in 0..n {
                x[j] += theta * dx[j];
                s[j] += theta * ds[j];
                t[j] += theta * dt[j];
            }
            for r in 0..nrow {
                u[r] += theta * rhs[r];
            }
            let mut b1 = 0.0;
            let mut b2 = 0.0;
            for j in 0..n {
                b1 += s[j] * (x[j] - lo[j]);
                b2 += t[j] * (up[j] - x[j]);
            }
            mu = o.barrier_reduction * (b1 + b2) / (2 * n) as f64;
            let finite = x
                .iter()
                .chain(&u)
                .chain(&s)
                .chain(&t)
                .all(|v| v.is_finite())
                && mu.is_finite();
            if !finite {
                return Err(solver_error(
                    "Interior-point iteration produced non-finite values.",
                    Some(iteration),
                ));
            }
            derivatives(&local, &x, lbd, tolx, &mut der);
            for j in 0..n {
                let dj = der.f[j] + s[j] / (x[j] - lo[j]) + t[j] / (up[j] - x[j]);
                let v = 1.0 / dj;
                inv_d[j] = if v.is_nan() {
                    f64::NAN
                } else {
                    v.max(MIN_INVD)
                };
            }
            if inv_d.iter().any(|v| !v.is_finite()) {
                return Err(solver_error(
                    "Inverse diagonal scaling contains invalid values.",
                    Some(iteration),
                ));
            }
            for j in 0..n {
                w[j] = Some(inv_d[j]);
            }
            // python reports no iteration for a Cholesky failure
            factorize(sym, &local, &w, &mut mvals, &mut lvals, work)?;
        }
        local.at_mul(&u, &mut atu);
        let mut dual = 0.0f64;
        for j in 0..n {
            charge[j] = der.gradf[j] + atu[j];
            dual = nan_max(dual, (der.gradf[j] + atu[j] - s[j] + t[j]).abs());
        }
        local.a_mul(&x, &mut rp);
        let primal = rp.iter().fold(0.0, |acc, v| nan_max(acc, v.abs()));
        Ok(Solution {
            x,
            u,
            charge,
            iterations: iteration,
            converged,
            diagnostics: Diagnostics {
                primal_residual_inf: primal,
                dual_residual_inf: dual,
                barrier_parameter: mu,
            },
        })
    })();
    ws.m = Some(mvals);
    ws.l = Some(lvals);
    result
}

/// Terminal satisfaction of a converged scenario (`_compute_satisfaction`).
pub(crate) fn satisfaction(
    ctx: &Context,
    prob: &Problem,
    sol: &Solution,
    target_flow_lps: Option<f64>,
) -> Result<Vec<f64>, StageError> {
    let m = ctx.model;
    let mut sat = vec![0.0; m.terminals.len()];
    let terminal_flows: Vec<f64> = prob
        .open
        .iter()
        .zip(&sol.x)
        .filter(|(j, _)| m.kind[**j] == PipeKind::Terminal)
        .map(|(_, x)| *x)
        .collect();
    let served: Vec<usize> = (0..sat.len()).filter(|&t| prob.served[t]).collect();
    if served.is_empty() {
        return Ok(sat);
    }
    if terminal_flows.len() != served.len() {
        return Err(StageError::new(
            "Solver terminal flows are not aligned with served terminals.",
        ));
    }
    let target_m3s = if ctx.bounded {
        None
    } else {
        Some(
            target_flow_lps.ok_or_else(|| {
                StageError::new("target_flow_lps is required for unbounded-flow satisfaction.")
            })? / 1000.0,
        )
    };
    for (k, &t) in served.iter().enumerate() {
        let den = match target_m3s {
            None => prob.demand_m3s[t],
            Some(q) => f64::from(prob.counts[t]) * q,
        };
        if !den.is_finite() || den <= 0.0 {
            return Err(StageError::new(
                "Terminal satisfaction denominator contains invalid values.",
            ));
        }
        sat[t] = terminal_flows[k] / den;
        if !sat[t].is_finite() {
            return Err(StageError::new(
                "Terminal satisfaction calculation produced invalid values.",
            ));
        }
    }
    Ok(sat)
}
