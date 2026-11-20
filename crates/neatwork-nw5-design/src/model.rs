//! The reinforcement LP/MILP (`build_reinf_model.m`) and its solution with
//! HiGHS (`solveModel.m`), `rust/nw5/specs/design.md` section 4.

use highs::{HighsModelStatus, RowProblem, Sense};
use neatwork_nw5::{ArcState, Error, Result};

use crate::loads::LoadTable;
use crate::params::DesignParams;
use crate::prepare::{Catalog, DesignArc, PathMatrix, Tr};

/// The LP/MILP in the MATLAB layout: `z = [x; y]`, `x(a, d)` at
/// `a·nb_d + d` (fraction of arc a covered by pipe d), `y(a)` at
/// `nb_a·nb_d + a`. Minimise `fᵀz` with `A_ineq·z ≤ b_ineq`,
/// `A_eq·z = b_eq`, `lb ≤ z ≤ ub`; the y columns are integer when some y is
/// free.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LpModel {
    /// Number of arcs.
    pub nb_a: usize,
    /// Number of real catalogue pipes.
    pub nb_d: usize,
    /// Objective.
    pub f: Vec<f64>,
    /// Lower bounds.
    pub lb: Vec<f64>,
    /// Upper bounds.
    pub ub: Vec<f64>,
    /// Inequality rows (one per arc, head loss at `succ`): `(column, value)`.
    pub a_ineq: Vec<Vec<(usize, f64)>>,
    /// Right-hand sides of the inequalities.
    pub b_ineq: Vec<f64>,
    /// Equality rows `Σ_d x(a, d) − y(a) = 0`.
    pub a_eq: Vec<Vec<(usize, f64)>>,
    /// Right-hand sides of the equalities (0).
    pub b_eq: Vec<f64>,
}

fn row_dot(row: &[(usize, f64)], z: &[f64]) -> f64 {
    row.iter()
        .map(|&(j, v)| v * z.get(j).copied().unwrap_or(0.0))
        .sum()
}

impl LpModel {
    /// `nVar = nb_a·(nb_d + 1)`.
    pub fn n_var(&self) -> usize {
        self.nb_a * (self.nb_d + 1)
    }

    /// The model is a MILP: some y has `lb ≠ ub` (`solveModel.m:26`).
    pub fn is_milp(&self) -> bool {
        let nx = self.nb_a * self.nb_d;
        (nx..self.n_var()).any(|j| self.lb[j] != self.ub[j])
    }

    /// Likely causes of an infeasible model, by arc index (not in MATLAB):
    /// an arc that needs a pipe but has no allowed catalogue pipe (pressure
    /// class or minimum diameter), or a head-loss row whose right-hand side
    /// cannot be met by any positive loss.
    pub fn infeasibility_hints(&self) -> Vec<(usize, InfeasibilityHint)> {
        let nx = self.nb_a * self.nb_d;
        let mut hints = Vec::new();
        for a in 0..self.nb_a {
            let needs_pipe = self.lb.get(nx + a).is_some_and(|&l| l == 1.0);
            let any_pipe = (0..self.nb_d).any(|d| self.ub[a * self.nb_d + d] > 0.0);
            if needs_pipe && !any_pipe {
                hints.push((a, InfeasibilityHint::NoAllowedPipe));
            }
        }
        for (a, (row, &b)) in self.a_ineq.iter().zip(&self.b_ineq).enumerate() {
            let credit: f64 = row.iter().filter(|(_, v)| *v < 0.0).map(|(_, v)| -v).sum();
            let loss = row.iter().any(|&(j, v)| v > 0.0 && self.ub[j] > 0.0);
            if loss && b + credit <= 0.0 {
                hints.push((a, InfeasibilityHint::NoHead(b + credit)));
            }
        }
        hints
    }

    /// `activ_variables = (ub == 1)`.
    pub fn activ_variables(&self) -> Vec<bool> {
        self.ub.iter().map(|&u| u == 1.0).collect()
    }

    /// `A_ineq·z`.
    pub fn ineq_activity(&self, z: &[f64]) -> Vec<f64> {
        self.a_ineq.iter().map(|r| row_dot(r, z)).collect()
    }

    /// `A_eq·z`.
    pub fn eq_activity(&self, z: &[f64]) -> Vec<f64> {
        self.a_eq.iter().map(|r| row_dot(r, z)).collect()
    }

    /// Dense `A_ineq`.
    pub fn a_ineq_dense(&self) -> Vec<Vec<f64>> {
        dense(&self.a_ineq, self.n_var())
    }

    /// Dense `A_eq`.
    pub fn a_eq_dense(&self) -> Vec<Vec<f64>> {
        dense(&self.a_eq, self.n_var())
    }

    /// `fᵀz`.
    pub fn objective(&self, z: &[f64]) -> f64 {
        self.f.iter().zip(z).map(|(a, b)| a * b).sum()
    }
}

fn dense(rows: &[Vec<(usize, f64)>], n: usize) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|r| {
            let mut v = vec![0.0; n];
            for &(j, x) in r {
                if let Some(e) = v.get_mut(j) {
                    *e += x;
                }
            }
            v
        })
        .collect()
}

/// `build_reinf_model` (`rust/nw5/specs/design.md` 4.1-4.5). `tl` gives the
/// head-loss flows, `tlv` the velocity flows.
pub fn build_model(
    arcs: &[DesignArc],
    paths: &PathMatrix,
    catalog: &Catalog,
    tr: &Tr,
    tl: &LoadTable,
    tlv: &LoadTable,
    p: &DesignParams,
) -> Result<LpModel> {
    let nb_a = arcs.len();
    let nb_d = catalog.pipes.len();
    if tr.rexist.len() != nb_a || tr.r.len() != nb_a || tr.r.iter().any(|r| r.len() != nb_d) {
        return Err(Error::Validation(
            "NW5:TRSize: TR and TA must have the same size".into(),
        ));
    }
    if paths.paths.len() != nb_a || tl.values.len() != nb_a || tlv.values.len() != nb_a {
        return Err(Error::Validation(
            "NW5:Size: path matrix and loads must have one row per arc".into(),
        ));
    }
    if !(p.secu.is_finite() && p.secu >= 1.0) {
        return Err(Error::Validation(
            "NW5:Parameters: pardsg.secu must be a finite real scalar greater than or equal to 1."
                .into(),
        ));
    }
    let nx = nb_a * nb_d;
    let n = nx + nb_a;
    let mut lb = vec![0.0; n];
    let mut ub = vec![0.0; n];
    let mut f = vec![0.0; n];
    let mut blocked = Vec::new();
    for (a, arc) in arcs.iter().enumerate() {
        let qv = tlv.row_max(a) / 1000.0;
        let dmin = 2.0 * (qv / (std::f64::consts::PI * p.vmax)).sqrt();
        for (d, pipe) in catalog.pipes.iter().enumerate() {
            let ok = pipe.max_pressure >= arc.pmax && pipe.diameter >= dmin;
            ub[a * nb_d + d] = if ok { 1.0 } else { 0.0 };
            f[a * nb_d + d] = arc.length * pipe.cost;
        }
        let min_existing = arc
            .segments
            .iter()
            .filter(|s| s.length > 0.0)
            .map(|s| {
                s.pipe
                    .and_then(|d| catalog.pipes.get(d))
                    .map_or(0.0, |q| q.diameter)
            })
            .fold(f64::INFINITY, f64::min);
        let lby = !arc.equipped() || min_existing < dmin;
        let uby = arc.state != ArcState::Blocked;
        lb[nx + a] = if lby { 1.0 } else { 0.0 };
        ub[nx + a] = if uby { 1.0 } else { 0.0 };
        f[nx + a] = arc.fixed_cost;
        if lby && !uby {
            blocked.push(arc.succ.clone());
        }
    }
    if !blocked.is_empty() {
        return Err(Error::Validation(format!(
            "NW5:ReinforcementBlocked: Renforcement obligatoire mais interdit (N=\"B\") sur les liaisons aboutissant aux noeuds : {}.",
            blocked.join(", ")
        )));
    }
    // right-hand side
    let cible = p.targetflow / 1000.0;
    let mut b = Vec::with_capacity(nb_a);
    for arc in arcs {
        let mut v = -arc.haut;
        if arc.terminal() {
            v /= p.secu.max(f64::EPSILON);
            if p.bounded_flow {
                v -= arc.pressure;
                if p.uniform_dem {
                    v -= cible * cible / p.faucetcoef.max(f64::EPSILON);
                }
            } else if p.uniform_dem {
                v -= cible * cible / p.faucetcoef.max(f64::EPSILON);
            } else {
                return Err(Error::Validation(
                    "NW5:Parameters: The case with nonuniform demands and unbounded flows is not treated".into(),
                ));
            }
        }
        b.push(v);
    }
    let q_lambda: Vec<f64> = (0..nb_a)
        .map(|a| (tl.row_max(a) / 1000.0).powf(p.lambda))
        .collect();
    let load_exist: Vec<f64> = (0..nb_a)
        .map(|a| {
            if tr.rexist[a] < f64::INFINITY {
                tr.rexist[a] * q_lambda[a] * arcs[a].length
            } else {
                0.0
            }
        })
        .collect();
    let on_path = paths.path_sum(&load_exist);
    let b_ineq: Vec<f64> = b.iter().zip(&on_path).map(|(b, e)| b - e).collect();
    let mut a_ineq = Vec::with_capacity(nb_a);
    for path in &paths.paths {
        let mut row = Vec::with_capacity(path.len() * (nb_d + 1));
        let mut sorted = path.clone();
        sorted.sort_unstable();
        for &a in &sorted {
            for d in 0..nb_d {
                row.push((a * nb_d + d, tr.r[a][d] * q_lambda[a] * arcs[a].length));
            }
        }
        for &a in &sorted {
            if load_exist[a] != 0.0 {
                row.push((nx + a, -load_exist[a]));
            }
        }
        a_ineq.push(row);
    }
    let a_eq = (0..nb_a)
        .map(|a| {
            let mut row: Vec<(usize, f64)> = (0..nb_d).map(|d| (a * nb_d + d, 1.0)).collect();
            row.push((nx + a, -1.0));
            row
        })
        .collect();
    Ok(LpModel {
        nb_a,
        nb_d,
        f,
        lb,
        ub,
        a_ineq,
        b_ineq,
        a_eq,
        b_eq: vec![0.0; nb_a],
    })
}

/// A likely cause of an infeasible model, see
/// [`LpModel::infeasibility_hints`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InfeasibilityHint {
    /// No catalogue pipe holds the static pressure `pmax` with at least the
    /// minimum diameter.
    NoAllowedPipe,
    /// Head available for the losses on the path, m (zero or negative).
    NoHead(f64),
}

/// Solution of the model after the clean-up of `solveModel.m`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solution {
    /// Raw solver values.
    pub z: Vec<f64>,
    /// Cleaned fractions `x[a][d]` (MATLAB `x_mat` transposed).
    pub x: Vec<Vec<f64>>,
    /// Cleaned `y` (exactly 0 or 1).
    pub y: Vec<f64>,
    /// `b_ineq − A_ineq·[x; y]` after clean-up.
    pub slack: Vec<f64>,
    /// Solver objective.
    pub fval: f64,
    /// `fᵀ[x; y]` after clean-up.
    pub new_fval: f64,
    /// HiGHS model status.
    pub status: String,
    /// Solved as a MILP.
    pub milp: bool,
    /// `y > 0.99`.
    pub reinforced: Vec<bool>,
    /// Clean-up warnings.
    pub warnings: Vec<String>,
}

fn failure(diagnostic: &str, message: &str) -> Error {
    Error::Solver(format!(
        "NW5:DesignFailure: {diagnostic}\nSolver diagnostic :\n{message}"
    ))
}

/// `solveModel`: HiGHS on one thread (simplex for an LP), the checks and the
/// clean-up of `solveModel.m:55-108`.
pub fn solve_model(model: &LpModel) -> Result<Solution> {
    let n = model.n_var();
    let nx = model.nb_a * model.nb_d;
    if model.f.len() != n || model.lb.len() != n || model.ub.len() != n {
        return Err(Error::Validation(
            "NW5:Model: inconsistent model sizes".into(),
        ));
    }
    let milp = model.is_milp();
    let mut pb = RowProblem::default();
    let cols: Vec<_> = (0..n)
        .map(|j| {
            if milp && j >= nx {
                pb.add_integer_column(model.f[j], model.lb[j]..=model.ub[j])
            } else {
                pb.add_column(model.f[j], model.lb[j]..=model.ub[j])
            }
        })
        .collect();
    for (row, &b) in model.a_ineq.iter().zip(&model.b_ineq) {
        pb.add_row(..=b, row.iter().map(|&(j, v)| (cols[j], v)));
    }
    for (row, &b) in model.a_eq.iter().zip(&model.b_eq) {
        pb.add_row(b..=b, row.iter().map(|&(j, v)| (cols[j], v)));
    }
    let mut solver = pb
        .try_optimise(Sense::Minimise)
        .map_err(|e| failure("The solver did not return a solution.", &format!("{e:?}")))?;
    solver.make_quiet();
    let opt =
        |k: &str, e: highs::TrySetOptionError| Error::Solver(format!("HiGHS option {k}: {e:?}"));
    solver
        .try_set_option("threads", 1)
        .map_err(|e| opt("threads", e))?;
    solver
        .try_set_option("parallel", "off")
        .map_err(|e| opt("parallel", e))?;
    if !milp {
        solver
            .try_set_option("solver", "simplex")
            .map_err(|e| opt("solver", e))?;
    }
    let solved = solver
        .try_solve()
        .map_err(|e| failure("The solver did not return a solution.", &format!("{e:?}")))?;
    let status = solved.status();
    let message = format!("{status:?}");
    match status {
        HighsModelStatus::Optimal => {}
        HighsModelStatus::Infeasible | HighsModelStatus::UnboundedOrInfeasible => {
            return Err(failure(
                "No solution satisfies all constraints. Check the model formulation and the input data.",
                &message,
            ));
        }
        _ => {
            return Err(failure(
                "The solver did not certify z as an optimal solution.",
                &message,
            ));
        }
    }
    let z = solved.get_solution().columns().to_vec();
    let fval = solved.objective_value();
    if z.len() != n {
        return Err(failure("The solver did not return a solution.", &message));
    }
    // checks on the raw solution (solveModel.m:62-72)
    if model
        .eq_activity(&z)
        .iter()
        .zip(&model.b_eq)
        .any(|(a, b)| !((a - b).abs() < 1e-6))
    {
        return Err(Error::Solver(
            "NW5:DesignFailure: Equality constraints not satisfied".into(),
        ));
    }
    let raw_slack: Vec<f64> = model
        .b_ineq
        .iter()
        .zip(model.ineq_activity(&z))
        .map(|(b, a)| b - a)
        .collect();
    if raw_slack.iter().any(|s| !(*s > -1e-6)) {
        return Err(Error::Solver(
            "NW5:DesignFailure: Inequality constraints not satisfied".into(),
        ));
    }
    let bound_gap = (0..n)
        .map(|j| (model.lb[j] - z[j]).max(z[j] - model.ub[j]))
        .fold(0.0, f64::max);
    if !(bound_gap < 1e-7) {
        return Err(Error::Solver(
            "NW5:DesignFailure: Bound constraints not satisfied".into(),
        ));
    }
    clean_solution(model, z, fval, message, milp)
}

/// Clean-up of a raw solution (`solveModel.m:68-108`).
pub fn clean_solution(
    model: &LpModel,
    z: Vec<f64>,
    fval: f64,
    status: String,
    milp: bool,
) -> Result<Solution> {
    let (nb_a, nb_d) = (model.nb_a, model.nb_d);
    let nx = nb_a * nb_d;
    if z.len() != model.n_var() {
        return Err(Error::Solver(
            "NW5:DesignFailure: The solver did not return a solution.".into(),
        ));
    }
    let mut y: Vec<f64> = z[nx..].to_vec();
    if y.iter().any(|&v| !(v < 1e-6 || v > 1.0 - 1e-6)) {
        return Err(Error::Solver(
            "NW5:DesignFailure: Variable y not close enough to binary".into(),
        ));
    }
    for v in &mut y {
        if v.abs() < 1e-6 {
            *v = 0.0;
        }
        if (1.0 - *v).abs() < 1e-6 {
            *v = 1.0;
        }
    }
    let mut x: Vec<Vec<f64>> = (0..nb_a)
        .map(|a| {
            z[a * nb_d..(a + 1) * nb_d]
                .iter()
                .map(|&v| if v.abs() < 1e-7 { 0.0 } else { v })
                .collect()
        })
        .collect();
    for (a, xa) in x.iter_mut().enumerate() {
        if y[a] == 1.0 {
            let s: f64 = xa.iter().sum();
            if !(s > 0.0) {
                return Err(Error::Solver(format!(
                    "NW5:DesignFailure: reinforced arc {} has no pipe",
                    a + 1
                )));
            }
            for v in xa.iter_mut() {
                *v /= s;
            }
        } else {
            xa.iter_mut().for_each(|v| *v = 0.0);
        }
    }
    let revised: Vec<f64> = x
        .iter()
        .flatten()
        .copied()
        .chain(y.iter().copied())
        .collect();
    let slack: Vec<f64> = model
        .b_ineq
        .iter()
        .zip(model.ineq_activity(&revised))
        .map(|(b, a)| b - a)
        .collect();
    let mut warnings = Vec::new();
    let min_slack = slack.iter().copied().fold(f64::INFINITY, f64::min);
    if !(min_slack > -1e-6) && !slack.is_empty() {
        warnings.push(format!(
            "After trimming the solution, the min slack is : {min_slack:.6}"
        ));
    }
    let new_fval = model.objective(&revised);
    let impact = (new_fval - fval) / (fval + f64::EPSILON);
    if impact.abs() > 1e-5 {
        warnings.push(format!(
            "Trimming changed total cost by {:.4} %",
            100.0 * impact
        ));
    }
    let reinforced = y.iter().map(|&v| v > 0.99).collect();
    Ok(Solution {
        z,
        x,
        y,
        slack,
        fval,
        new_fval,
        status,
        milp,
        reinforced,
        warnings,
    })
}
