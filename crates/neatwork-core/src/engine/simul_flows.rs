//! `SimulFlows`, `Gradient`, `Hessien`, `Ndir`, `Amax` and `Utils`: the
//! projected reduced Newton method that computes the tap flows of one
//! simulation. The loops keep the Java summation order.

use crate::{Error, Result};

/// Java `Math.max(a, b)`: NaN wins, +0.0 is above -0.0.
pub fn jmax(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_negative() { b } else { a };
    }
    if a >= b { a } else { b }
}

/// Java `Math.min(a, b)`: NaN wins, -0.0 is below +0.0.
pub fn jmin(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_negative() { a } else { b };
    }
    if a <= b { a } else { b }
}

/// `Dat`: data of one flow computation.
#[derive(Clone, Debug)]
pub struct Dat {
    pub nb: usize,
    pub nt: usize,
    pub h: Vec<f64>,
    pub cible: f64,
    pub lbd: f64,
    pub s: Vec<Vec<i32>>,
    pub nbouvert: Vec<f64>,
    pub alpha: Vec<f64>,
    pub beta: Vec<f64>,
    pub maxiter: usize,
    pub tolr: f64,
    pub tolx: f64,
}

/// Inputs of `SimulFlows.run` besides `Dat`.
pub struct Flows<'a> {
    /// Node numbers (`Integer.parseInt(nodes_end)`) of the sub-network
    /// pipes, in pipe order.
    pub pipe_end: &'a [i32],
    pub sb: &'a [Vec<i32>],
}

fn idx(n_end: i32, len: usize) -> Result<usize> {
    let k = n_end - 2;
    if k >= 0 && (k as usize) < len {
        Ok(k as usize)
    } else {
        Err(Error::Network(format!(
            "node number {n_end} is outside the intermediate node range"
        )))
    }
}

fn flows_y(dat: &Dat, f: &Flows, x: &[f64], rows: usize) -> Result<Vec<f64>> {
    let mut y = vec![0.0; dat.s.len()];
    for j in 0..rows {
        let n_end = *f
            .pipe_end
            .get(j)
            .ok_or_else(|| Error::Network("path matrix larger than the pipe list".into()))?;
        let k = idx(n_end, y.len())?;
        for (l, xl) in x.iter().enumerate().take(dat.s[j].len()) {
            y[k] += f64::from(dat.s[j][l]) * xl;
        }
    }
    Ok(y)
}

/// `SimulFlows.run`. Writes the tap flows (l/s) to `big_f[1 + nB + j]` and
/// the node heads to `dual`.
pub fn run(
    dual: &mut [f64],
    big_f: &mut [f64],
    n_taps: usize,
    dat0: &Dat,
    f: &Flows,
    opentaps: f64,
) -> Result<()> {
    if opentaps < 0.001 {
        return Err(Error::Parameter(
            "opentaps doit être supérieur à 0.0001".into(),
        ));
    }
    let mut dat = dat0.clone();
    dat.nbouvert = vec![1.0; dat.nt];
    let mut x = vec![dat.cible; dat.nt];
    let mut izero: Vec<usize> = Vec::new();
    let mut convergence = 10000.0;
    let mut i = 0;
    let rows0 = f.pipe_end.len().saturating_sub(n_taps);
    let mut y = flows_y(&dat, f, &x, rows0)?;
    let mut gradf = gradient(&x, &y, &dat, dual, f.sb)?;
    while i < dat.maxiter && convergence > dat.tolr {
        i += 1;
        let h = hessien(&x, &y, &dat);
        let (dx_red, hred) = ndir(&h, &gradf, &izero);
        x = amax(&x, &dx_red, &gradf, &hred, &dat, f)?;
        y = flows_y(&dat, f, &x, dat.s.len())?;
        gradf = gradient(&x, &y, &dat, dual, f.sb)?;
        izero = (0..x.len())
            .filter(|&j| x[j] < dat.tolx && gradf[j] > 1e-3)
            .collect();
        let mut residu = gradf.clone();
        for &k in &izero {
            residu[k] = jmin(residu[k], 0.0);
        }
        convergence = residu.iter().map(|v| v.abs()).reduce(jmax).unwrap_or(0.0);
    }

    // Heads at the intermediate nodes.
    let nb = dat.nb;
    let pb: Vec<f64> = (0..nb).map(|j| dat.beta[j] * y[j].powf(dat.lbd)).collect();
    let n = f.sb.len();
    let m = f.sb.first().map_or(0, Vec::len);
    let mut node_list: Vec<(i32, usize)> = Vec::new();
    for (k, &n_end) in f.pipe_end.iter().enumerate() {
        if node_list.len() >= n {
            break;
        }
        node_list.push((n_end, k));
    }
    node_list.sort_by_key(|a| a.0);
    let mut reordered = vec![vec![0i32; m]; n];
    for (row, &(_, k)) in node_list.iter().enumerate() {
        reordered[row].clone_from_slice(&f.sb[k][..m]);
    }
    if reordered.len() != pb.len() {
        return Err(Error::Network(
            "Le nombre de lignes de la matrice doit correspondre à la taille du vecteur.".into(),
        ));
    }
    let mut ch = vec![0.0; m];
    for (j, c) in ch.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (r, p) in reordered.iter().zip(&pb) {
            sum += f64::from(r[j]) * p;
        }
        *c = sum;
    }
    let p: Vec<f64> = (0..nb).map(|j| ch[j] + dat.h[j]).collect();
    let rows = dat.s.len();
    if rows > 1 {
        dual[1..rows].copy_from_slice(&p[1..rows]);
    }
    for j in 0..dat.nt {
        big_f[1 + j + nb] = if x[j] >= 0.0 { x[j] * 1000.0 } else { 0.0 };
    }
    Ok(())
}

/// `Gradient.gradient`.
#[expect(
    clippy::needless_range_loop,
    reason = "index loops keep the Java order"
)]
fn gradient(
    x: &[f64],
    y: &[f64],
    dat: &Dat,
    dual: &mut [f64],
    sb: &[Vec<i32>],
) -> Result<Vec<f64>> {
    let nb = dat.s.len();
    let nt = dat.s.first().map_or(0, Vec::len);
    let n = nb + nt;
    let df_y: Vec<f64> = (0..nb).map(|i| dat.beta[i] * y[i].powf(dat.lbd)).collect();
    let mut df_x = vec![0.0; nt];
    for (i, d) in df_x.iter_mut().enumerate() {
        for j in 0..nb {
            *d += f64::from(dat.s[j][i]) * df_y[j];
        }
    }
    let mut f_y = vec![0.0; nb];
    for (i, d) in f_y.iter_mut().enumerate() {
        for j in 0..nb {
            *d += f64::from(sb[j][i]) * df_y[j];
        }
    }
    for i in 0..nt {
        df_x[i] += dat.beta[i + nb] * (dat.nbouvert[i] * x[i]).powf(dat.lbd);
    }
    let pression: Vec<f64> = (0..nt)
        .map(|i| df_x[i] + dat.alpha[i] * x[i].powf(2.0))
        .collect();
    let gradf: Vec<f64> = (nb..n).map(|i| pression[i - nb] + dat.h[i]).collect();
    if dual.len() < n + 1 {
        return Err(Error::Network("dual vector too short".into()));
    }
    dual[0] = 0.0;
    dual[1..=nb].copy_from_slice(&f_y);
    for i in (nb + 1)..=n {
        dual[i] = pression[i - nb - 1];
    }
    Ok(gradf)
}

fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = a.len();
    let inner = a.first().map_or(0, Vec::len);
    let cols = b.first().map_or(0, Vec::len);
    let mut r = vec![vec![0.0; cols]; rows];
    for i in 0..rows {
        for j in 0..cols {
            for k in 0..inner {
                r[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    r
}

/// `Hessien.hessien`.
fn hessien(x: &[f64], y: &[f64], dat: &Dat) -> Vec<Vec<f64>> {
    let nb = dat.nb;
    let nt = dat.nt;
    let d2f_y: Vec<f64> = (0..nb)
        .map(|i| dat.lbd * dat.beta[i] * y[i].powf(dat.lbd - 1.0))
        .collect();
    let s: Vec<Vec<f64>> = dat
        .s
        .iter()
        .map(|r| r.iter().map(|v| f64::from(*v)).collect())
        .collect();
    let cols = s.first().map_or(0, Vec::len);
    let st: Vec<Vec<f64>> = (0..cols)
        .map(|j| s.iter().map(|r| r[j]).collect())
        .collect();
    let mut diag = vec![vec![0.0; nb]; nb];
    for i in 0..nb {
        diag[i][i] = d2f_y[i];
    }
    let temp2 = mat_mul(&mat_mul(&st, &diag), &s);
    let n = x.len();
    let mut d2f_x = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let ident = if i == j { 1.0 } else { 0.0 };
            d2f_x[i][j] = temp2[i][j] + ident * 1e-6;
        }
    }
    let mut ddiag: Vec<f64> = (0..nt)
        .map(|i| dat.lbd * dat.beta[nb + i] * (dat.nbouvert[i] * x[i]).powf(dat.lbd - 1.0))
        .collect();
    for i in 0..nt {
        ddiag[i] += 2.0 * dat.alpha[i] * x[i];
    }
    for d in &mut ddiag {
        *d += 1e-6;
    }
    let mut h = vec![vec![0.0; nt]; nt];
    for i in 0..nt {
        for j in 0..nt {
            h[i][j] = d2f_x[i][j] + if i == j { ddiag[i] } else { 0.0 };
        }
    }
    h
}

/// `Ndir.ndir`: Newton direction in the reduced space (inactive taps fixed).
#[expect(
    clippy::needless_range_loop,
    reason = "index loops keep the Java order"
)]
fn ndir(h: &[Vec<f64>], gradf: &[f64], izero: &[usize]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = h.len();
    let mut hred: Vec<Vec<f64>> = h.to_vec();
    let mut rp: Vec<f64> = gradf.iter().map(|g| -g).collect();
    if !izero.is_empty() {
        for &i in izero {
            rp[i] = 0.0;
        }
        let d: Vec<f64> = (0..n)
            .map(|i| if izero.contains(&i) { 1e8 } else { hred[i][i] })
            .collect();
        for i in 0..n {
            hred[i][i] -= d[i];
        }
        for &i in izero {
            for j in 0..n {
                hred[i][j] = 0.0;
                hred[j][i] = 0.0;
            }
        }
        for i in 0..n {
            hred[i][i] += d[i];
        }
    }
    let r = chol(&hred);
    let mut dx = solve_cholesky(&r, &rp);
    for &i in izero {
        dx[i] = 0.0;
    }
    (dx, hred)
}

/// `Utils.chol`: upper factor R with R'R = A (stored as `L[j][i]`).
#[expect(
    clippy::needless_range_loop,
    reason = "index loops keep the Java order"
)]
fn chol(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for k in 0..j {
                sum -= l[k][i] * l[k][j];
            }
            if i == j {
                l[i][j] = jmax(sum, 0.0).sqrt();
            } else {
                l[j][i] = sum / l[j][j];
            }
        }
    }
    l
}

/// `Utils.solveLinearSystem(R, R', b)`.
fn solve_cholesky(r: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = r.len();
    // solveLowerTriangularSystem(R, b)
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut sum = b[i];
        for j in 0..i {
            sum -= r[j][i] * y[j];
        }
        y[i] = sum / r[i][i];
    }
    // solveUpperTriangularSystem(R', y): U[j][i] = R[i][j]
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = y[i];
        for j in (i + 1)..n {
            sum -= r[i][j] * x[j];
        }
        x[i] = sum / r[i][i];
    }
    x
}

/// `Amax.amax`: step length by backtracking from 1 (factor 0.8, at most 30
/// tries) until the decrease reaches half the quadratic model's.
fn amax(
    x: &[f64],
    dx: &[f64],
    gradf: &[f64],
    hred: &[Vec<f64>],
    dat: &Dat,
    f: &Flows,
) -> Result<Vec<f64>> {
    let itermax = 30;
    let k = 0.8;
    let gain = 0.5;
    let mut i = 0;
    let mut converged = false;
    let mut a = 1.0;
    let y = flows_y(dat, f, x, dat.s.len())?;
    let yx: Vec<f64> = y.iter().chain(x.iter()).copied().collect();
    let mut finit = 0.0;
    for (j, v) in yx.iter().enumerate() {
        finit += dat.beta[j] * v.powf(dat.lbd + 1.0);
    }
    finit /= dat.lbd + 1.0;
    let mut sum = 0.0;
    for (j, v) in x.iter().enumerate() {
        sum += dat.alpha[j] * v.powf(3.0);
    }
    finit += sum / 3.0;
    let mut result = 0.0;
    for (j, v) in x.iter().enumerate() {
        result += dat.h[dat.nb + j] * v;
    }
    finit += result;
    while !converged && i < itermax {
        i += 1;
        let xa: Vec<f64> = (0..x.len()).map(|j| jmax(0.0, x[j] + a * dx[j])).collect();
        let ya = flows_y(dat, f, &xa, dat.s.len())?;
        let mut dq = 0.0;
        for j in 0..gradf.len() {
            dq += gradf[j] * (xa[j] - x[j]);
        }
        for l in 0..hred.len() {
            for j in 0..hred[l].len() {
                dq += 0.5 * hred[l][j] * (xa[l] - x[l]) * (xa[j] - x[j]);
            }
        }
        let mut fxa = 0.0;
        for (j, v) in ya.iter().enumerate() {
            fxa += dat.beta[j] * v.powf(dat.lbd + 1.0);
        }
        for (j, v) in xa.iter().enumerate() {
            fxa += dat.beta[j + ya.len()] * v.powf(dat.lbd + 1.0);
        }
        fxa /= dat.lbd + 1.0;
        for (j, v) in xa.iter().enumerate() {
            fxa += dat.alpha[j] * v.powf(3.0) / 3.0;
        }
        for (j, v) in xa.iter().enumerate() {
            fxa += dat.h[dat.nb + j] * v;
        }
        if finit - fxa > -gain * dq {
            converged = true;
        } else {
            a *= k;
        }
    }
    Ok((0..x.len()).map(|j| jmax(0.0, x[j] + a * dx[j])).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_min_max() {
        assert!(jmax(0.0, f64::NAN).is_nan());
        assert!(jmax(-0.0, 0.0).is_sign_positive());
        assert!(jmin(0.0, -0.0).is_sign_negative());
        assert_eq!(jmax(1.0, 2.0), 2.0);
    }

    #[test]
    fn cholesky_solves() {
        let a = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
        let r = chol(&a);
        let x = solve_cholesky(&r, &[2.0, 1.0]);
        assert!((4.0 * x[0] + 2.0 * x[1] - 2.0).abs() < 1e-12);
        assert!((2.0 * x[0] + 3.0 * x[1] - 1.0).abs() < 1e-12);
    }
}
