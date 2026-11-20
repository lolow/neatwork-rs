//! Preparation of the tables (`rust/nw5/specs/design.md` section 2):
//! catalogue and fictitious pipe, existing equipment, tree orientation,
//! enrichment, existing-pressure check, equivalent resistances `TR` and the
//! path matrix.

use std::collections::{HashMap, VecDeque};

use neatwork_nw5::{ArcState, Client, Error, Network, NodeStatus, Pipe, Result};

use crate::params::DesignParams;

/// A real pipe of the catalogue (`TD` row other than `fictif`).
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogPipe {
    /// `ID`
    pub id: String,
    /// `diam`, m.
    pub diameter: f64,
    /// `cost` per m.
    pub cost: f64,
    /// `pressure`: maximum allowed pressure, m.
    pub max_pressure: f64,
    /// `resist = bescal·diam^(−ν)`: linear resistance per m.
    pub resist: f64,
    /// `cond = resist^(−1/λ)`.
    pub cond: f64,
}

/// The real pipes of `TD` in diameter order (stable). The fictitious pipe
/// of MATLAB (row 1) is implicit: a segment without pipe.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    /// Real pipes, diameter ascending: the column order of the LP.
    pub pipes: Vec<CatalogPipe>,
}

impl Catalog {
    /// Catalogue of a network (`prepareReinforcementTables.m:19-65`).
    pub fn new(pipes: &[Pipe], p: &DesignParams) -> Result<Catalog> {
        let mut out: Vec<CatalogPipe> = Vec::new();
        for pipe in pipes {
            let id = pipe.id.trim();
            if id == "fictif" {
                continue;
            }
            if id.is_empty() {
                return Err(Error::Validation("NW5:Catalog: a pipe has no ID".into()));
            }
            if out.iter().any(|q| q.id == id) {
                return Err(Error::Validation(format!(
                    "NW5:Catalog: duplicate pipe ID {id}"
                )));
            }
            if !(pipe.diameter.is_finite() && pipe.diameter > 0.0) {
                return Err(Error::Validation(format!(
                    "NW5:Catalog: pipe {id} must have a finite positive diameter"
                )));
            }
            let resist = p.bescal * pipe.diameter.powf(-p.nu);
            out.push(CatalogPipe {
                id: id.to_string(),
                diameter: pipe.diameter,
                cost: pipe.cost,
                max_pressure: pipe.max_pressure,
                resist,
                cond: resist.powf(-1.0 / p.lambda),
            });
        }
        if out.is_empty() {
            return Err(Error::Validation(
                "NW5:Catalog: the pipe catalogue is empty".into(),
            ));
        }
        out.sort_by(|a, b| a.diameter.total_cmp(&b.diameter));
        Ok(Catalog { pipes: out })
    }

    /// Catalogue with given resistances: rows `(ID, diam, cost, pressure,
    /// resist)`, kept in the given order (test fixtures).
    pub fn with_resistances(rows: &[(&str, f64, f64, f64, f64)], lambda: f64) -> Catalog {
        Catalog {
            pipes: rows
                .iter()
                .map(|&(id, diameter, cost, max_pressure, resist)| CatalogPipe {
                    id: id.to_string(),
                    diameter,
                    cost,
                    max_pressure,
                    resist,
                    cond: resist.powf(-1.0 / lambda),
                })
                .collect(),
        }
    }

    /// Position of a pipe ID.
    pub fn index(&self, id: &str) -> Option<usize> {
        self.pipes.iter().position(|p| p.id == id)
    }
}

/// One pipe segment of a link: `lon_k`, `ID_k`. `pipe` is `None` for the
/// fictitious pipe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    /// Length, m.
    pub length: f64,
    /// Catalogue position of the pipe.
    pub pipe: Option<usize>,
}

/// A link of the tree after enrichment (`TA` row).
#[derive(Clone, Debug, PartialEq)]
pub struct DesignArc {
    /// Position of the arc in the input network (`None` in test fixtures).
    pub source: Option<usize>,
    /// Arc identifier of the input network.
    pub id: String,
    /// `pred`, oriented away from the source.
    pub pred: String,
    /// `succ`
    pub succ: String,
    /// `lon`, m.
    pub length: f64,
    /// `lon1, ID1` and `lon2, ID2`: existing equipment (not swapped when
    /// the arc is reversed, as MATLAB, B14).
    pub segments: [Segment; 2],
    /// `N`
    pub state: ArcState,
    /// Hydraulic open state of the input arc.
    pub open: bool,
    /// `pmax = Hsource − min(z_pred, z_succ)`, m.
    pub pmax: f64,
    /// `haut = z_succ − Hsource`, m.
    pub haut: f64,
    /// `statut` of `succ`.
    pub status: NodeStatus,
    /// `nbrob`: clients at `succ`.
    pub nbrob: f64,
    /// `pressure`: required pressure at `succ` if terminal, else 0.
    pub pressure: f64,
    /// `fixedCost` of a reinforcement (no reader creates it).
    pub fixed_cost: f64,
}

impl DesignArc {
    /// `statut == 2`.
    pub fn terminal(&self) -> bool {
        self.status == NodeStatus::Terminal
    }

    /// `lon1 > 0 | lon2 > 0`.
    pub fn equipped(&self) -> bool {
        self.segments[0].length > 0.0 || self.segments[1].length > 0.0
    }
}

/// `M_path`: `paths[j]` lists the arcs on the path from the source to
/// `succ(j)`, arc j first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathMatrix {
    /// Arcs on the path of each arc.
    pub paths: Vec<Vec<usize>>,
}

impl PathMatrix {
    /// Paths of oriented arcs, by walking up the parents.
    pub fn from_arcs(arcs: &[DesignArc]) -> Result<PathMatrix> {
        let by_succ: HashMap<&str, usize> = arcs
            .iter()
            .enumerate()
            .map(|(i, a)| (a.succ.as_str(), i))
            .collect();
        let mut paths = Vec::with_capacity(arcs.len());
        for j in 0..arcs.len() {
            let mut path = vec![j];
            let mut k = j;
            while let Some(&parent) = by_succ.get(arcs[k].pred.as_str()) {
                if path.len() > arcs.len() {
                    return Err(Error::Validation("NW5:InvalidDesignTopology: cycle".into()));
                }
                path.push(parent);
                k = parent;
            }
            paths.push(path);
        }
        Ok(PathMatrix { paths })
    }

    /// From a dense matrix `m[a][j]` (1 if arc a is on the path of j).
    pub fn from_dense(m: &[Vec<f64>]) -> PathMatrix {
        let n = m.len();
        PathMatrix {
            paths: (0..n)
                .map(|j| (0..n).filter(|&a| m[a][j] != 0.0).collect())
                .collect(),
        }
    }

    /// Dense `M_path(a, j)`.
    pub fn dense(&self) -> Vec<Vec<f64>> {
        let n = self.paths.len();
        let mut m = vec![vec![0.0; n]; n];
        for (j, path) in self.paths.iter().enumerate() {
            for &a in path {
                m[a][j] = 1.0;
            }
        }
        m
    }

    /// `M_pathᵀ·v`: sum of `v` along the path of each arc.
    pub fn path_sum(&self, v: &[f64]) -> Vec<f64> {
        self.paths
            .iter()
            .map(|path| path.iter().map(|&a| v[a]).sum())
            .collect()
    }
}

/// Equivalent resistances `TR` (`equivResist.m`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tr {
    /// `rexist`: linear resistance of the existing pipes (`inf` if none).
    pub rexist: Vec<f64>,
    /// `r[a][d]`: existing pipe in parallel with catalogue pipe d.
    pub r: Vec<Vec<f64>>,
}

/// Arcs, catalogue, paths and resistances ready for the loads and the model.
#[derive(Clone, Debug)]
pub struct Prepared {
    /// Real catalogue.
    pub catalog: Catalog,
    /// Arcs sorted by `(statut, succ)`.
    pub arcs: Vec<DesignArc>,
    /// Path matrix in arc order.
    pub paths: PathMatrix,
    /// `TR` in arc order.
    pub tr: Tr,
    /// Source node.
    pub source: String,
    /// `Hsource = z_source + pump_pressure`.
    pub hsource: f64,
    /// `TN.degree`, in network node order.
    pub degree: Vec<usize>,
}

/// `|lon1 + lon2 − lon| ≤ 1e-7` on equipped arcs (B1: MATLAB tests the
/// inverse).
pub fn check_equipment_lengths(arcs: &[DesignArc]) -> Result<()> {
    for a in arcs.iter().filter(|a| a.equipped()) {
        let gap = a.segments[0].length + a.segments[1].length - a.length;
        if !(gap.abs() <= 1e-7) {
            return Err(Error::Validation(format!(
                "NW5:EquipmentLength: link {} -> {}: lon1 + lon2 must equal lon ({} + {} != {})",
                a.pred, a.succ, a.segments[0].length, a.segments[1].length, a.length
            )));
        }
    }
    Ok(())
}

/// `TR` (`equivResist.m:84-126`). Segments without pipe are ignored but the
/// existing resistance is still divided by the whole length (B18).
pub fn equiv_resist(arcs: &[DesignArc], catalog: &Catalog, lambda: f64) -> Result<Tr> {
    check_equipment_lengths(arcs)?;
    let mut tr = Tr::default();
    for a in arcs {
        let mut sum = 0.0;
        let mut real = false;
        for s in &a.segments {
            if s.length > 0.0
                && let Some(d) = s.pipe
            {
                let pipe = catalog.pipes.get(d).ok_or_else(|| {
                    Error::Validation(format!("NW5:UnknownPipe: link {} -> {}", a.pred, a.succ))
                })?;
                sum += s.length * pipe.resist;
                real = true;
            }
        }
        if real {
            let rexist = sum / a.length;
            let cexist = rexist.powf(-1.0 / lambda);
            tr.rexist.push(rexist);
            tr.r.push(
                catalog
                    .pipes
                    .iter()
                    .map(|p| (cexist + p.cond).powf(-lambda))
                    .collect(),
            );
        } else {
            tr.rexist.push(f64::INFINITY);
            tr.r.push(catalog.pipes.iter().map(|p| p.resist).collect());
        }
    }
    Ok(tr)
}

/// `%.<prec>g` of C.
pub fn fmt_g(x: f64, prec: usize) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "NaN".into()
        } else if x > 0.0 {
            "Inf".into()
        } else {
            "-Inf".into()
        };
    }
    if x == 0.0 {
        return "0".into();
    }
    let p = prec.max(1);
    let e = format!("{:.*e}", p - 1, x);
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let trim = |s: &str| {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if exp < -4 || exp >= p as i32 {
        format!(
            "{}e{}{:02}",
            trim(mant),
            if exp < 0 { '-' } else { '+' },
            exp.abs()
        )
    } else {
        let decimals = (p as i32 - 1 - exp).max(0) as usize;
        trim(&format!("{x:.decimals$}"))
    }
}

/// `validate_existing_pressure.m`: every existing segment must hold `pmax`.
pub fn validate_existing_pressure(arcs: &[DesignArc], catalog: &Catalog) -> Result<()> {
    let mut lines = Vec::new();
    for a in arcs {
        for (k, s) in a.segments.iter().enumerate() {
            if !(s.length > 0.0) {
                continue;
            }
            let Some(d) = s.pipe else { continue };
            let pipe = catalog.pipes.get(d).ok_or_else(|| {
                Error::Validation(format!(
                    "NW5:UnknownPipe: link {} -> {}, troncon {}",
                    a.pred,
                    a.succ,
                    k + 1
                ))
            })?;
            let class = pipe.max_pressure;
            if class.is_nan() || class < 0.0 {
                return Err(Error::Validation(format!(
                    "NW5:ExistingPressureData: pipe {} has an invalid pressure class {class}",
                    pipe.id
                )));
            }
            if class < a.pmax {
                lines.push(format!(
                    "Liaison {} -> {}, troncon {}, tuyau {} : pression requise {} m, pression admissible {} m.",
                    a.pred,
                    a.succ,
                    k + 1,
                    pipe.id,
                    fmt_g(a.pmax, 6),
                    fmt_g(class, 6)
                ));
            }
        }
    }
    if lines.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "NW5:ExistingPressure:\n{}",
            lines.join("\n")
        )))
    }
}

fn segment(
    length: f64,
    pipe: Option<&String>,
    catalog: &Catalog,
    pred: &str,
    succ: &str,
    k: usize,
) -> Result<Segment> {
    if !(length.is_finite() && length >= 0.0) {
        return Err(Error::Validation(format!(
            "NW5:EquipmentLength: link {pred} -> {succ}: lon{k} must be finite and >= 0"
        )));
    }
    let id = pipe
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "fictif");
    match (length > 0.0, id) {
        (false, None) => Ok(Segment { length, pipe: None }),
        (true, Some(id)) => match catalog.index(id) {
            Some(d) => Ok(Segment {
                length,
                pipe: Some(d),
            }),
            None => Err(Error::Validation(format!(
                "NW5:UnknownPipe: link {pred} -> {succ}: pipe {id} (ID{k}) is not in the catalogue"
            ))),
        },
        (true, None) => Err(Error::Validation(format!(
            "NW5:EquipmentID: link {pred} -> {succ}: segment {k} has a length but no pipe"
        ))),
        (false, Some(id)) => Err(Error::Validation(format!(
            "NW5:EquipmentID: link {pred} -> {succ}: segment {k} has pipe {id} but no length"
        ))),
    }
}

/// Steps 2-11 of `makeReinfDsg` (`rust/nw5/specs/design.md` 2.2-2.7).
pub fn prepare(
    net: &Network,
    clients: &[Client],
    p: &DesignParams,
    warnings: &mut Vec<String>,
) -> Result<Prepared> {
    let catalog = Catalog::new(&net.pipes, p)?;
    // nodes
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (i, n) in net.nodes.iter().enumerate() {
        if index.insert(n.id.as_str(), i).is_some() {
            return Err(Error::Validation(format!(
                "NW:DuplicateNodeIds: node {} appears twice",
                n.id
            )));
        }
    }
    let sources: Vec<usize> = (0..net.nodes.len())
        .filter(|&i| net.nodes[i].status == NodeStatus::Source)
        .collect();
    let [source] = sources[..] else {
        return Err(Error::Validation(format!(
            "NW5:Source: the network must have exactly one source node (found {})",
            sources.len()
        )));
    };
    // arcs: endpoints, self-loops, parallel arcs
    let n = net.nodes.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (k, a) in net.arcs.iter().enumerate() {
        let (Some(&u), Some(&v)) = (index.get(a.from.as_str()), index.get(a.to.as_str())) else {
            return Err(Error::Validation(format!(
                "NW5:InvalidDesignTopology: link {} -> {} has an unknown node",
                a.from, a.to
            )));
        };
        if u == v {
            return Err(Error::Validation(format!(
                "NW5:InvalidDesignTopology: self-loop at {}",
                a.from
            )));
        }
        let pair = (u.min(v), u.max(v));
        if pairs.contains(&pair) {
            return Err(Error::Validation(format!(
                "NW5:InvalidDesignTopology: parallel links between {} and {}",
                a.from, a.to
            )));
        }
        pairs.push(pair);
        adj[u].push((v, k));
        adj[v].push((u, k));
    }
    if adj[source].len() != 1 {
        return Err(Error::Validation(format!(
            "NW5:InvalidDesignTopology: the source must have exactly one link (found {})",
            adj[source].len()
        )));
    }
    // orientation by BFS from the source
    let mut parent: Vec<Option<usize>> = vec![None; n];
    let mut seen = vec![false; n];
    seen[source] = true;
    let mut queue = VecDeque::from([source]);
    while let Some(u) = queue.pop_front() {
        for &(v, _) in &adj[u] {
            if !seen[v] {
                seen[v] = true;
                parent[v] = Some(u);
                queue.push_back(v);
            }
        }
    }
    if let Some(i) = seen.iter().position(|s| !s) {
        return Err(Error::Validation(format!(
            "NW5:InvalidDesignTopology: node {} is not connected to the source",
            net.nodes[i].id
        )));
    }
    if net.arcs.len() != n - 1 {
        return Err(Error::Validation(
            "NW5:InvalidDesignTopology: the network has a cycle".into(),
        ));
    }
    // enrichment
    let z = |i: usize| net.nodes[i].elevation;
    let hsource = z(source) + p.pump_pressure;
    let mut arcs = Vec::with_capacity(net.arcs.len());
    let mut clamped = Vec::new();
    for (k, a) in net.arcs.iter().enumerate() {
        let (mut u, mut v) = (index[a.from.as_str()], index[a.to.as_str()]);
        if parent[v] != Some(u) {
            std::mem::swap(&mut u, &mut v);
        }
        let (pred, succ) = (net.nodes[u].id.clone(), net.nodes[v].id.clone());
        let segments = [
            segment(a.length1, a.pipe1.as_ref(), &catalog, &pred, &succ, 1)?,
            segment(a.length2, a.pipe2.as_ref(), &catalog, &pred, &succ, 2)?,
        ];
        let equipped = segments.iter().any(|s| s.length > 0.0);
        let mut length = a.length;
        if !length.is_finite() || (equipped && !(length > 0.0)) {
            return Err(Error::Validation(format!(
                "NW5:EquipmentLength: link {pred} -> {succ}: lon must be finite and > 0"
            )));
        }
        if !equipped && !(length >= 0.01) {
            // parseTpoToTables.m:73-74 (B17)
            clamped.push(format!("{pred} -> {succ}"));
            length = 0.01;
        }
        if a.state == ArcState::Blocked && !equipped {
            return Err(Error::Validation(format!(
                "NW5:EquipmentID: blocked link {pred} -> {succ} has no existing pipe"
            )));
        }
        let node = &net.nodes[v];
        let terminal = node.status == NodeStatus::Terminal;
        let pressure = if !terminal {
            0.0
        } else if p.uniform_dem {
            p.pressure
        } else {
            match net.terminals.iter().find(|t| t.id == succ) {
                Some(t) => t.pressure,
                None => {
                    warnings.push(format!(
                        "terminal {succ} has no !Borne row; pressure {} is used",
                        p.pressure
                    ));
                    p.pressure
                }
            }
        };
        arcs.push(DesignArc {
            source: Some(k),
            id: a.id.clone(),
            pmax: hsource - z(u).min(z(v)),
            haut: z(v) - hsource,
            status: node.status,
            nbrob: clients.iter().filter(|c| c.terminal == succ).count() as f64,
            pressure,
            pred,
            succ,
            length,
            segments,
            state: a.state,
            open: a.open,
            fixed_cost: 0.0,
        });
    }
    if !clamped.is_empty() {
        warnings.push(format!(
            "link length raised to 0.01 m: {}",
            clamped.join(", ")
        ));
    }
    check_equipment_lengths(&arcs)?;
    arcs.sort_by(|a, b| (a.status.code(), &a.succ).cmp(&(b.status.code(), &b.succ)));
    let paths = PathMatrix::from_arcs(&arcs)?;
    validate_existing_pressure(&arcs, &catalog)?;
    let tr = equiv_resist(&arcs, &catalog, p.lambda)?;
    let degree = net
        .nodes
        .iter()
        .map(|nd| adj[index[nd.id.as_str()]].len())
        .collect();
    Ok(Prepared {
        catalog,
        arcs,
        paths,
        tr,
        source: net.nodes[source].id.clone(),
        hsource,
        degree,
    })
}
