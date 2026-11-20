//! `MakeDesign`, `RunMakeDesign` and `Decoupe`: choose the pipe diameters
//! and orifices of a topography with a linear program.

use super::defs::{CDiameter, CoreNetwork, MAXDIAM, compute_path};
use super::lp::{BoundKey, Lp};
use crate::java::{
    JHashtable, JProperties, decimal_format, double_to_string, math_round, parse_double, parse_int,
};
use crate::{Error, Result};

/// Result of a make-design run.
#[derive(Clone, Debug)]
pub struct MakeDesign {
    pub net: CoreNetwork,
    pub dvector: Vec<CDiameter>,
    /// Diameter rows `Dnnn, nominal, SDR, diameter, cost, max, type,
    /// roughness`, used ones first.
    pub diameters_content: Vec<Vec<String>>,
    /// Orifice catalog after the two artificial bounds were added.
    pub orifices_content: Vec<String>,
    /// The LP that was solved.
    pub lp: Lp,
}

fn prop(p: &JProperties, key: &str, default: &str) -> Result<f64> {
    let v = p.get_property_or(key, default);
    parse_double(v).ok_or_else(|| Error::Parameter(format!("{key}={v}")))
}

impl MakeDesign {
    /// `new MakeDesign(topo, dvector, ovector, properties, loadFactor)`.
    /// `solve` returns the LP column values (the Java program called MOSEK).
    pub fn run(
        mut net: CoreNetwork,
        dvector: Vec<CDiameter>,
        mut ovector: Vec<f64>,
        p: &JProperties,
        load_factor: &JHashtable<String, f64>,
        solve: &mut dyn FnMut(&Lp) -> Result<Vec<f64>>,
    ) -> Result<MakeDesign> {
        let outflow = prop(p, "topo.targetflow.value", "0.2")? / 1000.0;
        let _qualite = prop(p, "topo.servicequal.value", "0.8")?;
        let _rated = prop(p, "topo.opentaps.value", "0.4")?;
        let length_com = prop(p, "topo.pipelength.value", "6.0")?;
        let alpha = prop(p, "topo.faucetcoef.value", "0.00000002")?;
        let prix_max = prop(p, "topo.limitbudget.value", "1000000000")?;
        let coeff_orifice = prop(p, "topo.orifcoef.value", "0.59")?;

        // getAllPath
        for i in 0..net.nodes.0.len() {
            compute_path(&net.pipes, &mut net.nodes, i)?;
        }
        let mut load = Vec::with_capacity(net.pipes.0.len());
        for pipe in &net.pipes.0 {
            let lf = load_factor.get(&pipe.nodes_end).ok_or_else(|| {
                Error::Network(format!("no load factor for node {}", pipe.nodes_end))
            })?;
            let lf = parse_double(&double_to_string(*lf)).unwrap_or(*lf);
            load.push(lf * outflow);
        }
        let n = dvector.len() * net.pipes.0.len() + net.nodes.0.len() - 1;
        let m = net.nodes.0.len() + net.pipes.0.len();
        for t in &mut net.taps {
            t.faucet_coef = alpha;
        }
        let mut x = vec![0.0; n];
        let lp = run_make_design(
            &mut x,
            &mut net,
            &dvector,
            &mut ovector,
            &load,
            outflow,
            length_com,
            n,
            m,
            prix_max,
            coeff_orifice,
            solve,
        )?;

        // Diameter references: number the used diameters D001, D002, ...
        let mut cpt = 0;
        let mut deja_fait: JHashtable<i32, Vec<String>> = JHashtable::new();
        let mut deja_fait2: Vec<usize> = Vec::new();
        for pi in 0..net.pipes.0.len() {
            let idx1 = parse_int(&net.pipes.0[pi].ref_diam1)
                .ok_or_else(|| Error::Network("bad diameter reference".into()))?;
            net.pipes.0[pi].ref_diam1 =
                reference(idx1, &mut deja_fait, &mut deja_fait2, &mut cpt, &dvector)?;
            let idx2 = parse_int(&net.pipes.0[pi].ref_diam2)
                .ok_or_else(|| Error::Network("bad diameter reference".into()))?;
            if net.pipes.0[pi].l2 != 0.0 {
                net.pipes.0[pi].ref_diam2 =
                    reference(idx2, &mut deja_fait, &mut deja_fait2, &mut cpt, &dvector)?;
            }
        }
        let mut diameters_content: Vec<Vec<String>> = deja_fait.values().cloned().collect();
        for (i, d) in dvector.iter().enumerate() {
            if !deja_fait2.contains(&i) {
                cpt += 1;
                diameters_content.push(diametre_vector(cpt, d));
            }
        }
        let orifices_content = ovector.iter().map(|o| double_to_string(*o)).collect();
        Ok(MakeDesign {
            net,
            dvector,
            diameters_content,
            orifices_content,
            lp,
        })
    }

    /// `getDesignData(hsource)`: pipe rows (11 fields), then diameter rows
    /// (8 fields) and orifice rows (6 fields).
    pub fn design_data(&self, hsource: f64) -> Vec<Vec<String>> {
        let net = &self.net;
        let mut v = Vec::new();
        let src = net.nodes.0[0].name.clone();
        let mut source = vec![src.clone(), src, double_to_string(hsource)];
        source.extend(std::iter::repeat_n("0".to_string(), 8));
        v.push(source);
        let np = net.pipes.0.len();
        let nt = net.taps.len();
        for i in 0..np {
            let pipes = &net.pipes.0[i];
            let nodes = &net.nodes.0[i + 1];
            let mut line = vec![
                pipes.nodes_end.clone(),
                pipes.nodes_beg.clone(),
                double_to_string(nodes.height + hsource),
                double_to_string(pipes.length),
                nodes.taps.to_string(),
                decimal_format("0.###", pipes.l1),
                decimal_format("0.###", pipes.l2),
                pipes.ref_diam1.clone(),
                pipes.ref_diam2.clone(),
            ];
            let tap_index = (i + nt).checked_sub(np);
            match (
                net.is_tap(&nodes.name),
                tap_index.and_then(|k| net.taps.get(k)),
            ) {
                (true, Some(t)) => {
                    line.push(decimal_format("0.########", t.orif_ideal));
                    line.push(decimal_format("0.########", t.orif_com));
                }
                _ => {
                    line.push("0".to_string());
                    line.push("0".to_string());
                }
            }
            v.push(line);
        }
        v.extend(self.diameters_content.iter().cloned());
        for o in &self.orifices_content {
            let mut z = vec![o.clone()];
            z.extend(std::iter::repeat_n("N".to_string(), 5));
            v.push(z);
        }
        v
    }

    /// `getResultPressureData(hsource)`: node, height, pressure, suction.
    pub fn pressure_data(&self, hsource: f64) -> Vec<Vec<String>> {
        let net = &self.net;
        let mut v = vec![vec![
            net.nodes.0[0].name.clone(),
            decimal_format("0.##", hsource),
            "0".to_string(),
            "0".to_string(),
        ]];
        for i in 1..=net.pipes.0.len() {
            let n = &net.nodes.0[i];
            v.push(vec![
                n.name.clone(),
                decimal_format("0.##", n.height + hsource),
                decimal_format("0.##", n.pressure),
                decimal_format("0.##", n.suction),
            ]);
        }
        v
    }
}

fn reference(
    index: i32,
    deja_fait: &mut JHashtable<i32, Vec<String>>,
    deja_fait2: &mut Vec<usize>,
    cpt: &mut i32,
    dvector: &[CDiameter],
) -> Result<String> {
    if let Some(line) = deja_fait.get(&index) {
        return Ok(line[0].clone());
    }
    let d = usize::try_from(index)
        .ok()
        .and_then(|i| dvector.get(i))
        .ok_or_else(|| Error::Network(format!("diameter index {index} out of range")))?;
    *cpt += 1;
    let line = diametre_vector(*cpt, d);
    let r = line[0].clone();
    deja_fait.put(index, line);
    deja_fait2.push(index as usize);
    Ok(r)
}

fn diametre_vector(cpt: i32, d: &CDiameter) -> Vec<String> {
    vec![
        format!("D{}", decimal_format("000", f64::from(cpt))),
        d.nominal.clone(),
        double_to_string(d.sdr),
        double_to_string(d.diam),
        double_to_string(d.cost),
        double_to_string(d.pression),
        d.pipe_type.to_string(),
        double_to_string(d.rugosite),
    ]
}

/// `RunMakeDesign`: build and solve the LP, split each pipe in at most two
/// diameters, then compute the orifices.
#[expect(clippy::too_many_arguments, reason = "mirrors the Java constructor")]
#[expect(
    clippy::needless_range_loop,
    reason = "index loops keep the Java order"
)]
fn run_make_design(
    x: &mut [f64],
    net: &mut CoreNetwork,
    dvector: &[CDiameter],
    ovector: &mut Vec<f64>,
    load: &[f64],
    outflow: f64,
    lcom: f64,
    n: usize,
    m: usize,
    prix_max: f64,
    coef_orif: f64,
    solve: &mut dyn FnMut(&Lp) -> Result<Vec<f64>>,
) -> Result<Lp> {
    let nb_diam = dvector.len();
    let nb_pipes = net.pipes.0.len();
    let nb_nodes = net.nodes.0.len();
    let nb_taps = net.taps.len();
    if nb_diam == 0 || nb_nodes < 2 {
        return Err(Error::Network(
            "the design needs diameters and at least one pipe".into(),
        ));
    }

    // LengthTab
    let mut l = 0usize;
    for node in &net.nodes.0[1..] {
        l = l + node.path.len() - 1;
    }
    let l = (l + nb_pipes) * nb_diam + nb_nodes - 1 + nb_diam * nb_pipes;

    // MatriceA
    let mut acoeff = vec![0.0; l];
    let mut airow = vec![0usize; l];
    let ncol = nb_diam * nb_pipes + nb_nodes - 1;
    let mut aicolumn = vec![0usize; ncol];
    let mut aicolumn2 = vec![0usize; ncol];
    let (mut i1, mut i2, mut i3) = (0usize, 0usize, 0usize);
    let mut diam: Option<&CDiameter> = None;
    let cell =
        |v: &mut Vec<f64>, rows: &mut Vec<usize>, i: usize, val: f64, row: usize| -> Result<()> {
            if i >= v.len() {
                return Err(Error::Network("constraint matrix overflow".into()));
            }
            v[i] = val;
            rows[i] = row;
            Ok(())
        };
    for ipipes in 0..nb_pipes {
        for idiam in 0..nb_diam {
            aicolumn[i3] = i1;
            if i3 != 0 {
                aicolumn2[i3 - 1] = aicolumn[i3];
            }
            let pipes = &net.pipes.0[ipipes];
            for inodes in 1..nb_nodes {
                let nodes = &net.nodes.0[inodes];
                let mut ipath = nodes.path.len() as isize - 1;
                while ipath > 0 {
                    let n_beg = &nodes.path[ipath as usize];
                    let n_end = &nodes.path[ipath as usize - 1];
                    if crate::java::eq_ignore_case(&pipes.nodes_beg, n_beg)
                        && crate::java::eq_ignore_case(&pipes.nodes_end, n_end)
                    {
                        ipath = 0;
                        let d = &dvector[idiam];
                        diam = Some(d);
                        let v = load[ipipes].powf(d.p) / d.diam.powf(d.q) * d.beta;
                        cell(&mut acoeff, &mut airow, i1, v, inodes - 1)?;
                        i1 += 1;
                        i2 += 1;
                    } else {
                        ipath -= 1;
                    }
                }
            }
            debug_assert_eq!(i1, i2);
            // length constraint
            cell(&mut acoeff, &mut airow, i1, 1.0, nb_nodes + ipipes - 1)?;
            i1 += 1;
            i2 += 1;
            // budget constraint
            let cost = diam
                .map(|d| d.cost)
                .ok_or_else(|| Error::Network("a pipe is on no path to the source".into()))?;
            cell(&mut acoeff, &mut airow, i1, cost, nb_nodes + nb_pipes - 1)?;
            i1 += 1;
            i2 += 1;
            i3 += 1;
        }
    }
    // slack variables
    for i in (l - nb_nodes + 1)..l {
        aicolumn[i3] = i1;
        aicolumn2[i3 - 1] = aicolumn[i3];
        cell(&mut acoeff, &mut airow, i1, -1.0, i - (l - nb_nodes + 1))?;
        i1 += 1;
        i2 += 1;
        i3 += 1;
    }
    let _ = i2;
    aicolumn2[n - 1] = l;

    // Cvector
    let mut c = vec![0.0; n];
    let mut j = 0;
    for ci in c.iter_mut().take(n - nb_nodes + 1) {
        *ci = dvector[j].cost;
        if j < nb_diam - 1 {
            j += 1;
        } else {
            j = 0;
        }
    }
    let total_length: f64 = net.pipes.0.iter().fold(0.0, |acc, p| acc + p.length);
    let coeff_m = total_length * dvector[nb_diam - 1].cost;
    for ci in c.iter_mut().skip(n - nb_nodes + 1) {
        *ci = coeff_m;
    }

    // InitializeVariable
    let mut bkx = vec![BoundKey::Fr; n];
    let mut blx = vec![0.0; n];
    let mut bux = vec![0.0; n];
    for i in 0..nb_pipes {
        let pipes = &net.pipes.0[i];
        let node = &net.nodes.0[net.nodes.position(&pipes.nodes_end)?];
        for (jd, d) in dvector.iter().enumerate() {
            let k = i * nb_diam + jd;
            if -node.height < d.pression
                && d.diam > pipes.imposdiammin
                && d.diam < pipes.imposdiammax
            {
                blx[k] = 0.0;
                bux[k] = pipes.length;
                bkx[k] = BoundKey::Ra;
            } else {
                blx[k] = 0.0;
                bux[k] = 0.0;
                bkx[k] = BoundKey::Fx;
            }
            if pipes.imposdiam1 == d.diam {
                bux[k] = pipes.imposlength1;
                blx[k] = bux[k];
                bkx[k] = BoundKey::Fx;
            }
            if pipes.imposdiam2 == d.diam {
                bux[k] = pipes.length - pipes.imposlength1;
                blx[k] = bux[k];
                bkx[k] = BoundKey::Fx;
            }
        }
    }
    for k in (nb_pipes * nb_diam)..(nb_pipes * nb_diam + nb_nodes - 1) {
        bux[k] = 1.0e30;
        blx[k] = 0.0;
        bkx[k] = BoundKey::Lo;
    }

    // InitializeConstraints
    let mut cste = vec![0.0; m];
    let mut bkc = vec![BoundKey::Fr; m];
    let mut blc = vec![0.0; m];
    for i in 0..(nb_nodes - nb_taps - 1) {
        cste[i] = -net.nodes.0[i + 1].height;
        bkc[i] = BoundKey::Up;
        blc[i] = -1.0e30;
    }
    for i in (nb_nodes - nb_taps - 1)..(nb_nodes - 1) {
        let node = &net.nodes.0[i + 1];
        let tap = &net.taps[i - (nb_nodes - nb_taps - 1)];
        cste[i] = if tap.orifice == MAXDIAM {
            -node.height - (outflow * outflow) / tap.faucet_coef
        } else {
            -node.height
                - outflow
                    * outflow
                    * ((1.0 / tap.faucet_coef) + (coef_orif / tap.orifice).powf(4.0))
        };
        bkc[i] = BoundKey::Up;
        blc[i] = -1.0e30;
    }
    for i in (nb_nodes - 1)..(m - 1) {
        cste[i] = net.pipes.0[i + 1 - nb_nodes].length;
        bkc[i] = BoundKey::Fx;
        blc[i] = cste[i];
    }
    cste[m - 1] = prix_max;
    bkc[m - 1] = BoundKey::Up;
    blc[m - 1] = -1.0e30;

    let lp = Lp {
        numcon: m,
        numvar: n,
        numanz: l,
        bkc,
        blc,
        buc: cste.clone(),
        bkx,
        blx,
        bux,
        ptrb: aicolumn,
        ptre: aicolumn2,
        asub: airow,
        aval: acoeff,
        c,
    };
    let sol = solve(&lp)?;
    x.copy_from_slice(&sol[..n]);

    decoupe(x, lcom, net, dvector);
    let cst_value = cst_value2(net, load)?;
    gestion_orifice_parfait(
        net, &cste, &cst_value, outflow, coef_orif, nb_nodes, nb_taps,
    );
    if !ovector.is_empty() {
        gestion_orifice_commercial(net, ovector, nb_nodes, nb_taps);
    }
    cst_value2(net, load)?;
    for i in 1..nb_nodes {
        net.nodes.0[i].suction = x[i + nb_diam * (nb_nodes - 1) - 1];
    }
    Ok(lp)
}

/// `Decoupe`: keep at most two diameters per pipe and round the split to
/// whole commercial pipe lengths.
fn decoupe(x: &mut [f64], lcom: f64, net: &mut CoreNetwork, dvector: &[CDiameter]) {
    let nb_diam = dvector.len();
    for i in 0..net.pipes.0.len() {
        let pipes = &mut net.pipes.0[i];
        pipes.l2 = 0.0;
        let mut indice1 = 0usize;
        let mut indice2: Option<usize> = None;
        let mut j = 0;
        while j < nb_diam && x[i * nb_diam + j] < 0.1 {
            j += 1;
        }
        if j != nb_diam {
            indice1 = i * nb_diam + j;
            let d = &dvector[j];
            pipes.d1 = d.diam;
            pipes.l1 = x[indice1];
            pipes.p1 = d.p;
            pipes.q1 = d.q;
            pipes.beta1 = d.beta;
            pipes.ref_diam1 = j.to_string();
            j += 1;
        }
        while j < nb_diam && x[i * nb_diam + j] < 0.1 {
            j += 1;
        }
        if j != nb_diam {
            pipes.d2 = pipes.d1;
            pipes.l2 = pipes.l1;
            pipes.p2 = pipes.p1;
            pipes.q2 = pipes.q1;
            pipes.beta2 = pipes.beta1;
            pipes.ref_diam2 = pipes.ref_diam1.clone();
            let k = i * nb_diam + j;
            indice2 = Some(k);
            let d = &dvector[j];
            pipes.d1 = d.diam;
            pipes.l1 = x[k];
            pipes.p1 = d.p;
            pipes.q1 = d.q;
            pipes.beta1 = d.beta;
            pipes.ref_diam1 = j.to_string();
        }
        if let Some(indice2) = indice2 {
            let mut l1 = get_length(pipes.l1, lcom);
            let mut l2 = get_length(pipes.l2, lcom);
            if l1 + l2 > lcom {
                l2 = lcom - l1;
                l1 = lcom - l2;
            }
            if pipes.imposdiam1 == 0.0 {
                let e1 = calcul_energy(l1, l2, pipes.d1, pipes.d2, pipes.q1, pipes.q2);
                let e2 = calcul_energy(l2, l1, pipes.d2, pipes.d1, pipes.q2, pipes.q1);
                if e1 < e2 {
                    pipes.l1 += l2;
                    pipes.l2 -= l2;
                } else {
                    pipes.l1 -= l1;
                    pipes.l2 += l1;
                }
                x[indice1] = pipes.l2;
                x[indice2] = pipes.l1;
            }
        }
    }
}

fn get_length(x: f64, lcom: f64) -> f64 {
    let r = math_round(x / lcom) as f64;
    if r < x / lcom {
        (x / lcom - r) * lcom
    } else {
        (x / lcom - r + 1.0) * lcom
    }
}

fn calcul_energy(l1: f64, l2: f64, d1: f64, d2: f64, q1: f64, q2: f64) -> f64 {
    ((l1 / d1.powf(q1) + l2 / d2.powf(q2)) - (l1 + l2) / d1.powf(q1)).abs()
}

/// `CstValue2`: head loss from the source to every node with the chosen
/// diameters; also stores it as the node pressure.
fn cst_value2(net: &mut CoreNetwork, load: &[f64]) -> Result<Vec<f64>> {
    let nn = net.nodes.0.len();
    let mut cst = vec![0.0; nn + net.pipes.0.len()];
    for i in 1..nn {
        let mut acc = 0.0;
        for j in 0..net.nodes.0[i].path.len().saturating_sub(1) {
            let n_end = &net.nodes.0[i].path[j];
            let pos = net.pipes.position(n_end)?;
            let p = &net.pipes.0[pos];
            let lf = load[pos];
            acc = acc
                + p.l1 * lf.powf(p.p1) / p.d1.powf(p.q1) * p.beta1
                + p.l2 * lf.powf(p.p2) / p.d2.powf(p.q2) * p.beta2;
        }
        cst[i - 1] = acc;
        net.nodes.0[i].pressure = acc;
    }
    Ok(cst)
}

fn gestion_orifice_parfait(
    net: &mut CoreNetwork,
    cste: &[f64],
    cst_value: &[f64],
    outflow: f64,
    coef_orif: f64,
    nb_nodes: usize,
    nb_taps: usize,
) {
    for i in (nb_nodes - nb_taps - 1)..(nb_nodes - 1) {
        if cste[i] > cst_value[i] {
            let tap = &mut net.taps[i + nb_taps + 1 - nb_nodes];
            let d = ((coef_orif.powf(4.0) * outflow.powf(2.0))
                / (cste[i] - cst_value[i]
                    + (coef_orif / tap.orifice).powf(4.0) * outflow.powf(2.0)))
            .powf(0.25);
            tap.orif_ideal = d;
            tap.orifice = d;
        }
    }
}

fn gestion_orifice_commercial(
    net: &mut CoreNetwork,
    ovector: &mut Vec<f64>,
    nb_nodes: usize,
    nb_taps: usize,
) {
    let dmin = 0.000001;
    let dmax = 1.0;
    ovector.push(dmin);
    ovector.push(dmax);
    ovector.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for i in (nb_nodes - nb_taps - 1)..(nb_nodes - 1) {
        let tap = &mut net.taps[i + nb_taps + 1 - nb_nodes];
        let mut orif_sup = dmin;
        let mut orif_inf = dmin;
        let mut j = 0;
        while j < ovector.len() && tap.orif_ideal > orif_sup {
            orif_sup = ovector[j];
            if j > 0 {
                orif_inf = ovector[j - 1];
            }
            j += 1;
        }
        tap.orif_com = if tap.orif_ideal >= 2f64.sqrt() * orif_inf
            || orif_inf * orif_sup <= tap.orif_ideal * tap.orif_ideal
        {
            orif_sup
        } else {
            orif_inf
        };
        if tap.orif_com == dmin || tap.orif_com == dmax {
            tap.orif_com = MAXDIAM;
        }
    }
}
