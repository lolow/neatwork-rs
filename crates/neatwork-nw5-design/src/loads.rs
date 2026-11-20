//! Design loads `TL` and `TLv` (`build_incidence_loads.m`,
//! `rust/nw5/specs/design.md` section 3).

use neatwork_nw5::{Client, Error, Result};

use crate::LoadMethod;
use crate::params::DesignParams;
use crate::prepare::{DesignArc, PathMatrix};
use crate::rng::Twister;

/// `TL`: design flow of each arc conditional on each terminal demanding
/// water, L/s. Zero when the arc is not on the path of the terminal.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadTable {
    /// `ID`: successor node of each arc (row order).
    pub arcs: Vec<String>,
    /// Terminals with clients, sorted (column order).
    pub terminals: Vec<String>,
    /// `values[arc][terminal]`, L/s.
    pub values: Vec<Vec<f64>>,
}

impl LoadTable {
    /// `max_k TL(a, k)`, L/s.
    pub fn row_max(&self, a: usize) -> f64 {
        self.values
            .get(a)
            .map_or(0.0, |r| r.iter().fold(0.0, |m: f64, &v| m.max(v)))
    }

    /// `max(self, other)` elementwise (`makeReinfDsg.m:165`).
    pub fn max_with(&mut self, other: &LoadTable) {
        for (r, o) in self.values.iter_mut().zip(&other.values) {
            for (v, w) in r.iter_mut().zip(o) {
                *v = v.max(*w);
            }
        }
    }
}

/// Probabilities of `Bin(n, p)`: `pmf[k]`, `cdf[k] = P(X ≤ k)` with
/// `cdf[n] = 1`, `upper[k] = P(X > k)`.
struct Binomial {
    n: usize,
    cdf: Vec<f64>,
    upper: Vec<f64>,
}

impl Binomial {
    fn new(n: usize, p: f64) -> Binomial {
        let pmf: Vec<f64> = if p >= 1.0 {
            (0..=n).map(|k| if k == n { 1.0 } else { 0.0 }).collect()
        } else if p <= 0.0 {
            (0..=n).map(|k| if k == 0 { 1.0 } else { 0.0 }).collect()
        } else {
            // log-space recursion: no underflow of (1-p)^n near the mode
            let (lp, lq) = (p.ln(), (-p).ln_1p());
            let mut l = vec![0.0; n + 1];
            l[0] = n as f64 * lq;
            for k in 0..n {
                l[k + 1] = l[k] + ((n - k) as f64).ln() - ((k + 1) as f64).ln() + lp - lq;
            }
            l.iter().map(|v| v.exp()).collect()
        };
        let mut cdf = Vec::with_capacity(n + 1);
        let mut acc = 0.0;
        for v in &pmf {
            acc += v;
            cdf.push(acc);
        }
        cdf[n] = 1.0;
        let mut upper = vec![0.0; n + 1];
        let mut acc = 0.0;
        for k in (0..n).rev() {
            acc += pmf[k + 1];
            upper[k] = acc;
        }
        Binomial { n, cdf, upper }
    }

    /// `binocdf(k, n, p)`, any integer k.
    fn cdf(&self, k: i64) -> f64 {
        if k < 0 {
            0.0
        } else if k as usize >= self.n {
            1.0
        } else {
            self.cdf[k as usize]
        }
    }

    /// `binocdf(k, n, p, 'upper') = P(X > k)`.
    fn upper(&self, k: i64) -> f64 {
        if k < 0 {
            1.0
        } else if k as usize >= self.n {
            0.0
        } else {
            self.upper[k as usize]
        }
    }

    /// `binoinv(q, n, p)`: smallest k with `cdf(k) ≥ q`.
    fn inv(&self, q: f64) -> i64 {
        (0..=self.n)
            .find(|&k| self.cdf(k as i64) >= q)
            .unwrap_or(self.n) as i64
    }
}

/// Load factor `ld` of an arc with `n` other dependent clients
/// (`build_incidence_loads.m:225-263`).
pub fn binomial_load_factor(n: usize, p: f64, qos: f64, cvar: bool) -> f64 {
    if n == 0 {
        return 1.0;
    }
    if p == 1.0 || (cvar && qos == 1.0) {
        return 1.0 + n as f64;
    }
    let b = Binomial::new(n, p);
    let k = b.inv(qos);
    if cvar {
        // E[(Z-k)+] = n p P(Bin(n-1,p) ≥ k) − k P(Z > k)
        let b1 = Binomial::new(n - 1, p);
        let excess = n as f64 * p * b1.upper(k - 1) - k as f64 * b.upper(k);
        return 1.0 + k as f64 + excess.max(0.0) / (1.0 - qos);
    }
    let p2 = b.cdf(k);
    let p1 = b.cdf(k - 1);
    if p2 > p1 && p1 > 0.0 {
        k as f64 + (qos - p1) / (p2 - p1)
    } else if p2 == p1 && p1 > 0.0 {
        k as f64
    } else {
        1.0
    }
}

/// Terminals with clients (sorted) and their incoming arc.
fn terminal_columns(arcs: &[DesignArc], clients: &[Client]) -> Result<(Vec<String>, Vec<usize>)> {
    let mut terminals: Vec<String> = clients.iter().map(|c| c.terminal.clone()).collect();
    terminals.sort();
    terminals.dedup();
    let mut arc_of = Vec::with_capacity(terminals.len());
    for t in &terminals {
        let Some(a) = arcs.iter().position(|a| &a.succ == t) else {
            return Err(Error::Validation(format!(
                "NW5:Clients: terminal {t} of a client has no incoming link"
            )));
        };
        if !arcs[a].terminal() {
            return Err(Error::Validation(format!(
                "NW5:Clients: client node {t} is not a terminal"
            )));
        }
        arc_of.push(a);
    }
    Ok((terminals, arc_of))
}

/// `build_incidence_loads` at quantile level `qos`.
///
/// Uniform demand uses the exact binomial law unless `method` forces Monte
/// Carlo. Monte Carlo draws `nScenLoad` scenarios from `rng`, client by
/// client within each scenario (MATLAB column-major `rand(nC, nS)`).
pub fn incidence_loads(
    arcs: &[DesignArc],
    paths: &PathMatrix,
    clients: &[Client],
    p: &DesignParams,
    qos: f64,
    method: LoadMethod,
    rng: &mut Twister,
) -> Result<LoadTable> {
    if !(qos > 0.0 && qos <= 1.0) {
        return Err(Error::Validation(format!(
            "NW5:LoadQuantile: servicequal must be in (0,1] (got {qos})"
        )));
    }
    let (terminals, arc_of) = terminal_columns(arcs, clients)?;
    let na = arcs.len();
    let mut values = vec![vec![0.0; terminals.len()]; na];
    if p.uniform_dem && method == LoadMethod::Auto {
        let mut nbdep = vec![0.0; na];
        for (j, path) in paths.paths.iter().enumerate() {
            for &a in path {
                nbdep[a] += arcs[j].nbrob;
            }
        }
        let prob = p.opentaps;
        if !(prob.is_finite() && prob > 0.0 && prob <= 1.0) {
            return Err(Error::Validation(format!(
                "NW5:LoadProbability: opentaps must be in (0,1] (got {prob}); with opentaps=0 no client ever opens"
            )));
        }
        for (k, &t) in arc_of.iter().enumerate() {
            for &a in &paths.paths[t] {
                let ld = if nbdep[a] > 1.0 {
                    binomial_load_factor((nbdep[a] - 1.0).round() as usize, prob, qos, p.use_cvar)
                } else {
                    1.0
                };
                values[a][k] = (ld * p.targetflow).max(1e-12);
            }
        }
    } else {
        for c in clients {
            if !(c.probability.is_finite() && (0.0..=1.0).contains(&c.probability)) {
                return Err(Error::Validation(format!(
                    "NW5:Probabilities: client {} has probability {} outside [0,1]",
                    c.id, c.probability
                )));
            }
            if !(c.target_flow.is_finite() && c.target_flow >= 0.0) {
                return Err(Error::Validation(format!(
                    "NW5:Demands: client {} has demand {} (must be finite and >= 0)",
                    c.id, c.target_flow
                )));
            }
        }
        let col: Vec<usize> = clients
            .iter()
            .map(|c| terminals.iter().position(|t| *t == c.terminal).unwrap_or(0))
            .collect();
        let ns = p.n_scen_load;
        let nb = terminals.len();
        // demande_borne, stored scenario-major: demand[s*nb + k]
        let mut demand = vec![0.0; ns * nb];
        for s in 0..ns {
            for (i, c) in clients.iter().enumerate() {
                let u = rng.next_f64();
                if u < c.probability {
                    demand[s * nb + col[i]] += c.target_flow;
                }
            }
        }
        // terminals downstream of each arc
        let mut below: Vec<Vec<usize>> = vec![Vec::new(); na];
        for (k, &t) in arc_of.iter().enumerate() {
            for &a in &paths.paths[t] {
                below[a].push(k);
            }
        }
        // MATLAB sums M_path * demande_noeud in arc order
        for b in &mut below {
            b.sort_by_key(|&k| arc_of[k]);
        }
        let mut flow = vec![0.0; ns];
        let mut y = Vec::with_capacity(ns);
        for a in 0..na {
            if below[a].is_empty() {
                continue;
            }
            for (s, f) in flow.iter_mut().enumerate() {
                *f = below[a].iter().map(|&k| demand[s * nb + k]).sum();
            }
            for &k in &below[a] {
                y.clear();
                y.extend(
                    (0..ns)
                        .filter(|&s| demand[s * nb + k] > 0.0)
                        .map(|s| flow[s]),
                );
                if y.is_empty() {
                    return Err(Error::Validation(format!(
                        "NW5:NoActiveLoadScenarios: terminal {} has no active scenarios; check demand or increase nScenLoad",
                        terminals[k]
                    )));
                }
                y.sort_by(f64::total_cmp);
                values[a][k] = empirical_quantile(&y, qos, p.use_cvar);
            }
        }
    }
    Ok(LoadTable {
        arcs: arcs.iter().map(|a| a.succ.clone()).collect(),
        terminals,
        values,
    })
}

/// VaR with interpolation, or CVaR, of sorted values
/// (`build_incidence_loads.m:160-202`).
pub fn empirical_quantile(y: &[f64], qos: f64, cvar: bool) -> f64 {
    let m = y.len();
    let idx1 = ((qos * m as f64).ceil() as usize).clamp(1, m);
    let qc1 = y[idx1 - 1];
    if cvar {
        if qos == 1.0 {
            return y[m - 1];
        }
        let tail: f64 = y.iter().map(|v| (v - qc1).max(0.0)).sum();
        return qc1 + tail / ((1.0 - qos) * m as f64);
    }
    let lt = y.iter().filter(|&&v| v < qc1).count();
    let le = y.iter().filter(|&&v| v <= qc1).count();
    let pc2 = lt as f64 / m as f64;
    let pc1 = le as f64 / m as f64;
    let qc2 = if lt > 0 { y[lt - 1] } else { qc1 };
    if pc1 > pc2 && pc2 > 0.0 {
        let theta = (qos - pc2) / (pc1 - pc2);
        theta * qc1 + (1.0 - theta) * qc2
    } else {
        qc1
    }
}
