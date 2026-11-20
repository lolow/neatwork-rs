//! Sparse Cholesky factorization of `M = A diag(w) Aᵀ` on the transit nodes
//! (spec section 4.5.3).
//!
//! The pattern of `M` is fixed for a run: off-diagonal entries come only from
//! the open transit pipes (both ends are transit nodes); source and terminal
//! pipes add to the diagonal. The symbolic analysis (minimum-degree ordering
//! and fill-in) is therefore done once. A tree network has no fill-in.

use std::collections::BTreeSet;

/// Symbolic factorization: ordering and the structure of `L`.
#[derive(Clone, Debug)]
pub(crate) struct Symbolic {
    n: usize,
    /// `perm[k]` = original row eliminated at step `k`.
    perm: Vec<usize>,
    /// `iperm[row]` = elimination step of `row`.
    iperm: Vec<usize>,
    /// Column `k` of `L` (strict lower part): rows `rowidx[colptr[k]..colptr[k+1]]`,
    /// sorted, in elimination numbering.
    colptr: Vec<usize>,
    rowidx: Vec<usize>,
    /// For column `j`: the entries `(k, p)` with `rowidx[p] == j`, `k < j`.
    rowlist: Vec<Vec<(usize, usize)>>,
}

/// Values of `M` (or of `L`) on the structure of `L`: diagonal and strict
/// lower entries.
#[derive(Clone, Debug)]
pub(crate) struct Values {
    pub(crate) diag: Vec<f64>,
    pub(crate) lower: Vec<f64>,
}

impl Symbolic {
    /// Analyse the pattern of an `n × n` symmetric matrix given by its
    /// off-diagonal pairs (original numbering).
    pub(crate) fn new(n: usize, edges: &[(usize, usize)]) -> Symbolic {
        let mut adj: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
        for &(a, b) in edges {
            if a != b {
                adj[a].insert(b);
                adj[b].insert(a);
            }
        }
        // minimum degree on the elimination graph (ties: smallest row)
        let mut eliminated = vec![false; n];
        let mut perm = Vec::with_capacity(n);
        let mut structs: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut by_degree: BTreeSet<(usize, usize)> = (0..n).map(|v| (adj[v].len(), v)).collect();
        while let Some((_, v)) = by_degree.pop_first() {
            eliminated[v] = true;
            perm.push(v);
            let nbrs: Vec<usize> = adj[v].iter().copied().filter(|&w| !eliminated[w]).collect();
            for &w in &nbrs {
                by_degree.remove(&(adj[w].len(), w));
            }
            for &a in &nbrs {
                adj[a].remove(&v);
                for &b in &nbrs {
                    if a != b {
                        adj[a].insert(b);
                    }
                }
            }
            for &w in &nbrs {
                by_degree.insert((adj[w].len(), w));
            }
            structs[v] = nbrs;
        }
        let mut iperm = vec![0; n];
        for (k, &v) in perm.iter().enumerate() {
            iperm[v] = k;
        }
        let mut colptr = Vec::with_capacity(n + 1);
        let mut rowidx = Vec::new();
        colptr.push(0);
        for &v in &perm {
            let mut col: Vec<usize> = structs[v].iter().map(|&w| iperm[w]).collect();
            col.sort_unstable();
            rowidx.extend(col);
            colptr.push(rowidx.len());
        }
        let mut rowlist = vec![Vec::new(); n];
        for k in 0..n {
            for p in colptr[k]..colptr[k + 1] {
                rowlist[rowidx[p]].push((k, p));
            }
        }
        Symbolic {
            n,
            perm,
            iperm,
            colptr,
            rowidx,
            rowlist,
        }
    }

    /// Zero values on the structure.
    pub(crate) fn zeros(&self) -> Values {
        Values {
            diag: vec![0.0; self.n],
            lower: vec![0.0; self.rowidx.len()],
        }
    }

    /// Position of the off-diagonal entry `(a, b)` (original rows) in
    /// `Values::lower`.
    pub(crate) fn position(&self, a: usize, b: usize) -> Option<usize> {
        let (i, j) = (self.iperm[a], self.iperm[b]);
        let (row, col) = if i > j { (i, j) } else { (j, i) };
        let range = self.colptr[col]..self.colptr[col + 1];
        self.rowidx[range.clone()]
            .binary_search(&row)
            .ok()
            .map(|p| range.start + p)
    }

    /// Elimination step of an original row.
    pub(crate) fn step(&self, row: usize) -> usize {
        self.iperm[row]
    }

    /// Numeric factorization of `M + shift·I` into `l` (left-looking).
    /// Returns `false` if the matrix is not positive definite.
    pub(crate) fn factor(
        &self,
        m: &Values,
        shift: f64,
        l: &mut Values,
        work: &mut Vec<f64>,
    ) -> bool {
        work.clear();
        work.resize(self.n, 0.0);
        for j in 0..self.n {
            let col = self.colptr[j]..self.colptr[j + 1];
            work[j] = m.diag[j] + shift;
            for p in col.clone() {
                work[self.rowidx[p]] = m.lower[p];
            }
            for &(k, p) in &self.rowlist[j] {
                let ljk = l.lower[p];
                work[j] -= ljk * ljk;
                for q in p + 1..self.colptr[k + 1] {
                    work[self.rowidx[q]] -= l.lower[q] * ljk;
                }
            }
            let d = work[j];
            if d.is_nan() || d <= 0.0 || d.is_infinite() {
                return false;
            }
            let ljj = d.sqrt();
            l.diag[j] = ljj;
            work[j] = 0.0;
            for p in col {
                let i = self.rowidx[p];
                l.lower[p] = work[i] / ljj;
                work[i] = 0.0;
            }
        }
        true
    }

    /// Solve `L Lᵀ x = b` (original numbering) in place.
    pub(crate) fn solve(&self, l: &Values, b: &mut [f64], work: &mut Vec<f64>) {
        work.clear();
        work.extend(self.perm.iter().map(|&r| b[r]));
        for j in 0..self.n {
            work[j] /= l.diag[j];
            let yj = work[j];
            for p in self.colptr[j]..self.colptr[j + 1] {
                work[self.rowidx[p]] -= l.lower[p] * yj;
            }
        }
        for j in (0..self.n).rev() {
            let mut v = work[j];
            for p in self.colptr[j]..self.colptr[j + 1] {
                v -= l.lower[p] * work[self.rowidx[p]];
            }
            work[j] = v / l.diag[j];
        }
        for (k, &r) in self.perm.iter().enumerate() {
            b[r] = work[k];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dense(sym: &Symbolic, m: &Values) -> Vec<Vec<f64>> {
        // back to original numbering
        let n = sym.n;
        let mut a = vec![vec![0.0; n]; n];
        for k in 0..n {
            a[sym.perm[k]][sym.perm[k]] = m.diag[k];
            for p in sym.colptr[k]..sym.colptr[k + 1] {
                let (r, c) = (sym.perm[sym.rowidx[p]], sym.perm[k]);
                a[r][c] = m.lower[p];
                a[c][r] = m.lower[p];
            }
        }
        a
    }

    #[test]
    fn solves_a_meshed_laplacian() {
        // ring of 5 nodes plus a chord, grounded diagonal
        let edges = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (1, 3)];
        let sym = Symbolic::new(5, &edges);
        let mut m = sym.zeros();
        for &(a, b) in &edges {
            let p = sym.position(a, b).unwrap_or(usize::MAX);
            m.lower[p] -= 1.0;
            m.diag[sym.step(a)] += 1.0;
            m.diag[sym.step(b)] += 1.0;
        }
        for v in 0..5 {
            m.diag[sym.step(v)] += 0.5 + v as f64;
        }
        let a = dense(&sym, &m);
        let mut l = sym.zeros();
        let mut work = Vec::new();
        assert!(sym.factor(&m, 0.0, &mut l, &mut work));
        let x_true = [1.0, -2.0, 0.5, 3.0, -1.0];
        let mut b: Vec<f64> = (0..5)
            .map(|i| (0..5).map(|j| a[i][j] * x_true[j]).sum())
            .collect();
        sym.solve(&l, &mut b, &mut work);
        for i in 0..5 {
            assert!((b[i] - x_true[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn tree_has_no_fill() {
        let edges = [(0, 1), (1, 2), (1, 3), (3, 4), (3, 5)];
        let sym = Symbolic::new(6, &edges);
        assert_eq!(sym.rowidx.len(), edges.len());
    }

    #[test]
    fn singular_matrix_fails() {
        let sym = Symbolic::new(2, &[(0, 1)]);
        let mut m = sym.zeros();
        m.diag = vec![1.0, 1.0];
        m.lower[0] = -1.0;
        let mut l = sym.zeros();
        let mut work = Vec::new();
        assert!(!sym.factor(&m, 0.0, &mut l, &mut work));
        assert!(sym.factor(&m, 1e-14, &mut l, &mut work));
    }
}
