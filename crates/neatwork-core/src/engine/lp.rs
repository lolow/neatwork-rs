//! The design linear program, in the MOSEK layout that `RunMakeDesign`
//! builds, and its solution with HiGHS (the Java program called MOSEK).

use crate::java::{double_to_string, parse_double};
use crate::{Error, Result};
use highs::{ColProblem, HighsModelStatus, Sense};

/// MOSEK bound keys (`mosek.boundkey`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundKey {
    /// Lower bound only.
    Lo,
    /// Upper bound only.
    Up,
    /// Fixed: lower = upper.
    Fx,
    /// Free.
    Fr,
    /// Range: lower and upper.
    Ra,
}

impl BoundKey {
    fn parse(s: &str) -> Option<BoundKey> {
        match s {
            "lo" => Some(BoundKey::Lo),
            "up" => Some(BoundKey::Up),
            "fx" => Some(BoundKey::Fx),
            "fr" => Some(BoundKey::Fr),
            "ra" => Some(BoundKey::Ra),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            BoundKey::Lo => "lo",
            BoundKey::Up => "up",
            BoundKey::Fx => "fx",
            BoundKey::Fr => "fr",
            BoundKey::Ra => "ra",
        }
    }
}

/// Minimize c'x subject to blc <= Ax <= buc and blx <= x <= bux, with A
/// stored by column: column j has entries `ptrb[j]..ptre[j]` of
/// `asub` (row) and `aval` (value).
#[derive(Clone, Debug, Default)]
pub struct Lp {
    pub numcon: usize,
    pub numvar: usize,
    pub numanz: usize,
    pub bkc: Vec<BoundKey>,
    pub blc: Vec<f64>,
    pub buc: Vec<f64>,
    pub bkx: Vec<BoundKey>,
    pub blx: Vec<f64>,
    pub bux: Vec<f64>,
    pub ptrb: Vec<usize>,
    pub ptre: Vec<usize>,
    pub asub: Vec<usize>,
    pub aval: Vec<f64>,
    pub c: Vec<f64>,
}

/// Result of a solve.
#[derive(Clone, Debug)]
pub struct LpSolution {
    /// Column values; all zero when the status is not optimal, as the Java
    /// program leaves them.
    pub x: Vec<f64>,
    pub optimal: bool,
    pub status: String,
}

fn bounds(key: BoundKey, lo: f64, up: f64) -> (f64, f64) {
    match key {
        BoundKey::Lo => (lo, f64::INFINITY),
        BoundKey::Up => (f64::NEG_INFINITY, up),
        BoundKey::Fx => (lo, lo),
        BoundKey::Fr => (f64::NEG_INFINITY, f64::INFINITY),
        BoundKey::Ra => (lo, up),
    }
}

impl Lp {
    /// Solve with HiGHS on one thread (deterministic).
    pub fn solve(&self) -> Result<LpSolution> {
        let mut pb = ColProblem::default();
        let rows: Vec<_> = (0..self.numcon)
            .map(|i| {
                let (lo, up) = bounds(self.bkc[i], self.blc[i], self.buc[i]);
                pb.add_row(lo..=up)
            })
            .collect();
        for j in 0..self.numvar {
            let (lo, up) = bounds(self.bkx[j], self.blx[j], self.bux[j]);
            let factors: Vec<_> = (self.ptrb[j]..self.ptre[j])
                .map(|p| (rows[self.asub[p]], self.aval[p]))
                .collect();
            pb.add_column(self.c[j], lo..=up, &factors);
        }
        let mut model = pb.optimise(Sense::Minimise);
        model.make_quiet();
        model.set_option("threads", 1);
        model.set_option("parallel", "off");
        model.set_option("solver", "simplex");
        let solved = model
            .try_solve()
            .map_err(|e| Error::Solver(format!("HiGHS failed: {e:?}")))?;
        let status = solved.status();
        if status == HighsModelStatus::Optimal {
            Ok(LpSolution {
                x: solved.get_solution().columns().to_vec(),
                optimal: true,
                status: format!("{status:?}"),
            })
        } else {
            Ok(LpSolution {
                x: vec![0.0; self.numvar],
                optimal: false,
                status: format!("{status:?}"),
            })
        }
    }

    /// Read the JSON written by [`Lp::to_json`] or by the reference harness:
    /// one `"key":value` pair per line.
    pub fn from_json(text: &str) -> Result<Lp> {
        let mut lp = Lp::default();
        let bad = |k: &str| Error::Solver(format!("bad LP field {k}"));
        for line in text.lines() {
            let line = line.trim().trim_end_matches(',');
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim_matches('"');
            let items = || {
                value
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim_matches('"'))
            };
            let nums = || {
                items()
                    .map(|s| parse_double(s).ok_or_else(|| bad(key)))
                    .collect::<Result<Vec<f64>>>()
            };
            let ints = || {
                items()
                    .map(|s| s.parse::<usize>().map_err(|_| bad(key)))
                    .collect::<Result<Vec<usize>>>()
            };
            let keys = || {
                items()
                    .map(|s| BoundKey::parse(s).ok_or_else(|| bad(key)))
                    .collect::<Result<Vec<BoundKey>>>()
            };
            match key {
                "numcon" => lp.numcon = value.parse().map_err(|_| bad(key))?,
                "numvar" => lp.numvar = value.parse().map_err(|_| bad(key))?,
                "numanz" => lp.numanz = value.parse().map_err(|_| bad(key))?,
                "bkc" => lp.bkc = keys()?,
                "blc" => lp.blc = nums()?,
                "buc" => lp.buc = nums()?,
                "bkx" => lp.bkx = keys()?,
                "blx" => lp.blx = nums()?,
                "bux" => lp.bux = nums()?,
                "ptrb" => lp.ptrb = ints()?,
                "ptre" => lp.ptre = ints()?,
                "asub" => lp.asub = ints()?,
                "aval" => lp.aval = nums()?,
                "c" => lp.c = nums()?,
                _ => {}
            }
        }
        let n = lp.numvar;
        let ok = lp.bkc.len() == lp.numcon
            && lp.blc.len() == lp.numcon
            && lp.buc.len() == lp.numcon
            && [
                lp.bkx.len(),
                lp.blx.len(),
                lp.bux.len(),
                lp.ptrb.len(),
                lp.ptre.len(),
                lp.c.len(),
            ]
            .iter()
            .all(|l| *l == n)
            && lp.asub.len() == lp.aval.len()
            && lp.ptre.iter().all(|p| *p <= lp.asub.len())
            && lp.asub.iter().all(|r| *r < lp.numcon);
        if ok {
            Ok(lp)
        } else {
            Err(Error::Solver("inconsistent LP file".into()))
        }
    }

    /// JSON text identical to the reference harness dump (`nw.lp.dump`).
    pub fn to_json(&self) -> String {
        let keys = |k: &[BoundKey]| {
            let v: Vec<String> = k.iter().map(|b| format!("\"{}\"", b.name())).collect();
            format!("[{}]", v.join(","))
        };
        let nums = |d: &[f64]| {
            let v: Vec<String> = d.iter().map(|x| double_to_string(*x)).collect();
            format!("[{}]", v.join(","))
        };
        let ints = |d: &[usize]| {
            let v: Vec<String> = d.iter().map(|x| x.to_string()).collect();
            format!("[{}]", v.join(","))
        };
        format!(
            "{{\n\"numcon\":{},\n\"numvar\":{},\n\"numanz\":{},\n\"bkc\":{},\n\"blc\":{},\n\"buc\":{},\n\"bkx\":{},\n\"blx\":{},\n\"bux\":{},\n\"ptrb\":{},\n\"ptre\":{},\n\"asub\":{},\n\"aval\":{},\n\"c\":{}\n}}\n",
            self.numcon,
            self.numvar,
            self.numanz,
            keys(&self.bkc),
            nums(&self.blc),
            nums(&self.buc),
            keys(&self.bkx),
            nums(&self.blx),
            nums(&self.bux),
            ints(&self.ptrb),
            ints(&self.ptre),
            ints(&self.asub),
            nums(&self.aval),
            nums(&self.c)
        )
    }
}

/// Solution file: one value per line, in Java `Double.toString` form, so a
/// Java program reads back the same doubles.
pub fn solution_text(x: &[f64]) -> String {
    x.iter().map(|v| double_to_string(*v) + "\n").collect()
}

/// Read a solution file written by [`solution_text`].
pub fn parse_solution(text: &str, numvar: usize) -> Result<Vec<f64>> {
    let v: Vec<f64> = text
        .split_whitespace()
        .map(|s| parse_double(s).ok_or_else(|| Error::Solver(format!("bad number {s}"))))
        .collect::<Result<_>>()?;
    if v.len() < numvar {
        return Err(Error::Solver(format!(
            "solution has {} values, the LP has {numvar} columns",
            v.len()
        )));
    }
    Ok(v[..numvar].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Lp {
        // min x0 + 2 x1  s.t.  x0 + x1 = 3,  x1 >= 1 (row), 0 <= x0 <= 2
        Lp {
            numcon: 2,
            numvar: 2,
            numanz: 3,
            bkc: vec![BoundKey::Fx, BoundKey::Lo],
            blc: vec![3.0, 1.0],
            buc: vec![3.0, 1.0e30],
            bkx: vec![BoundKey::Ra, BoundKey::Lo],
            blx: vec![0.0, 0.0],
            bux: vec![2.0, 1.0e30],
            ptrb: vec![0, 1],
            ptre: vec![1, 3],
            asub: vec![0, 0, 1],
            aval: vec![1.0, 1.0, 1.0],
            c: vec![1.0, 2.0],
        }
    }

    #[test]
    fn json_round_trip() -> Result<()> {
        let lp = tiny();
        let back = Lp::from_json(&lp.to_json())?;
        assert_eq!(back.to_json(), lp.to_json());
        Ok(())
    }

    #[test]
    fn highs_solves_tiny_lp() -> Result<()> {
        let s = tiny().solve()?;
        assert!(s.optimal);
        assert!((s.x[0] - 2.0).abs() < 1e-9 && (s.x[1] - 1.0).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn solution_text_round_trip() -> Result<()> {
        let x = vec![0.1, 1.0e-7, 250.0];
        assert_eq!(parse_solution(&solution_text(&x), 3)?, x);
        Ok(())
    }
}
