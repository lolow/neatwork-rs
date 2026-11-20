//! Design parameters (`pardsg`, `partec`) and the non-interactive review of
//! `review_design_parameters_reinf.m` (`rust/nw5/specs/design.md` section 1.2).

use neatwork_nw5::read::g15;
use neatwork_nw5::{Client, Error, Network, Params, Result};

use crate::RiskMeasure;

/// Parameters of a design run after review: `pardsg` and `partec` of MATLAB.
///
/// Flows in L/s, pressures and heights in m, velocities in m/s.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignParams {
    /// `bescal`: friction coefficient β.
    pub bescal: f64,
    /// `faucetcoef`: faucet loss is `q²/faucetcoef` (q in m³/s).
    pub faucetcoef: f64,
    /// `lambda`: flow exponent λ.
    pub lambda: f64,
    /// `nu`: diameter exponent ν.
    pub nu: f64,
    /// `orifcoef`: orifice coefficient.
    pub orifcoef: f64,
    /// `bounded_flow`: flow capped at the demand, terminal pressure required.
    pub bounded_flow: bool,
    /// `fusion`: read and written, no effect (ADR-023).
    pub fusion: f64,
    /// `limitbudget`: NW4 legacy, forced to 1e8.
    pub limitbudget: f64,
    /// `nScenLoad`: Monte Carlo scenarios.
    pub n_scen_load: usize,
    /// `opentaps`: opening probability of a client (uniform demand).
    pub opentaps: f64,
    /// `pipelength`: commercial pipe length.
    pub pipelength: f64,
    /// `pressure`: required terminal pressure (uniform demand).
    pub pressure: f64,
    /// `pump_pressure`: head gain at the source.
    pub pump_pressure: f64,
    /// `secu`: safety factor on the static head of terminals.
    pub secu: f64,
    /// `servicequal`: quantile level of the design load.
    pub servicequal: f64,
    /// `seuil`: NW4 legacy, unused.
    pub seuil: f64,
    /// `split`: read and written, no effect (ADR-023).
    pub split: f64,
    /// `targetflow`: demand of one open client (uniform demand), L/s.
    pub targetflow: f64,
    /// `uniform_dem`: all clients have the same probability and demand.
    pub uniform_dem: bool,
    /// `use_cvar`: CVaR instead of VaR for the loads.
    pub use_cvar: bool,
    /// `use_efficient_pipes`: read and written, no effect (ADR-023).
    pub use_efficient_pipes: f64,
    /// `vmax`: maximum velocity.
    pub vmax: f64,
    /// `vmax_secu`: quantile level of the velocity load.
    pub vmax_secu: f64,
    /// `watertemp`: unused, forced to 20.
    pub watertemp: f64,
}

impl Default for DesignParams {
    /// Defaults of `extract_parameters.m` (with `lambda` 1.781, ADR-021).
    fn default() -> DesignParams {
        DesignParams {
            bescal: 8.9396e-4,
            faucetcoef: 1.0,
            lambda: 1.781,
            nu: 4.781,
            orifcoef: 1.0,
            bounded_flow: false,
            fusion: 0.0,
            limitbudget: 1e8,
            n_scen_load: 200_000,
            opentaps: 1.0,
            pipelength: 6.0,
            pressure: 2.0,
            pump_pressure: 0.0,
            secu: 1.0,
            servicequal: 0.65,
            seuil: 0.0,
            split: 0.0,
            targetflow: 0.0,
            uniform_dem: true,
            use_cvar: false,
            use_efficient_pipes: 0.0,
            vmax: 1.5,
            vmax_secu: 0.95,
            watertemp: 20.0,
        }
    }
}

/// Names accepted as parameter overrides.
const NAMES: &[&str] = &[
    "bescal",
    "faucetcoef",
    "lambda",
    "nu",
    "orifcoef",
    "bounded_flow",
    "fusion",
    "limitbudget",
    "nScenLoad",
    "opentaps",
    "pipelength",
    "pressure",
    "pump_pressure",
    "secu",
    "servicequal",
    "seuil",
    "split",
    "targetflow",
    "uniform_dem",
    "use_cvar",
    "use_efficient_pipes",
    "vmax",
    "vmax_secu",
    "watertemp",
];

fn flag(v: f64, name: &str) -> Result<bool> {
    if v == 0.0 {
        Ok(false)
    } else if v == 1.0 {
        Ok(true)
    } else {
        Err(Error::Validation(format!(
            "NW5:Parameters: {name} must be 0 or 1 (got {v})"
        )))
    }
}

fn b(v: bool) -> f64 {
    if v { 1.0 } else { 0.0 }
}

impl DesignParams {
    /// Parameters as written to `!Default properties`: `pardsg` fields in
    /// ASCII order, then `partec` fields (`rust/nw5/specs/design.md` 6.2).
    pub fn entries(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("bounded_flow", b(self.bounded_flow)),
            ("fusion", self.fusion),
            ("limitbudget", self.limitbudget),
            ("nScenLoad", self.n_scen_load as f64),
            ("opentaps", self.opentaps),
            ("pipelength", self.pipelength),
            ("pressure", self.pressure),
            ("pump_pressure", self.pump_pressure),
            ("secu", self.secu),
            ("servicequal", self.servicequal),
            ("seuil", self.seuil),
            ("split", self.split),
            ("targetflow", self.targetflow),
            ("uniform_dem", b(self.uniform_dem)),
            ("use_cvar", b(self.use_cvar)),
            ("use_efficient_pipes", self.use_efficient_pipes),
            ("vmax", self.vmax),
            ("vmax_secu", self.vmax_secu),
            ("watertemp", self.watertemp),
            ("bescal", self.bescal),
            ("faucetcoef", self.faucetcoef),
            ("lambda", self.lambda),
            ("nu", self.nu),
            ("orifcoef", self.orifcoef),
        ]
    }

    /// `!Default properties` of the designed network: [`DesignParams::entries`]
    /// written with `%.15g`, then the input entries the design does not use
    /// (for example `nbsim`), kept as read.
    pub fn to_params(&self, input: &Params) -> Params {
        let mut entries: Vec<(String, String)> = self
            .entries()
            .into_iter()
            .map(|(k, v)| (k.to_string(), g15(v)))
            .collect();
        for (k, v) in &input.entries {
            let c = Params::canonical(k);
            if !NAMES.contains(&c) && !entries.iter().any(|(e, _)| e == c) {
                entries.push((c.to_string(), v.clone()));
            }
        }
        Params { entries }
    }
}

/// Parameters and clients after review.
#[derive(Clone, Debug)]
pub struct Reviewed {
    /// Reviewed parameters.
    pub params: DesignParams,
    /// Clients; with uniform demand they take `opentaps` and `targetflow`.
    pub clients: Vec<Client>,
    /// Review warnings.
    pub warnings: Vec<String>,
}

/// Non-interactive review (`apply_client_data`, `review_design_parameters_reinf`,
/// `makeReinfDsg.m:118-121`).
///
/// * `bounded_flow` defaults to 1 when the network has existing pipes and
///   no `bounded_flow` entry (`readDesign.m:153-157`: a `.dsg` input).
/// * `uniform_dem` is derived from the clients; an override is ignored.
/// * overrides replace file values; then the forced values of the review
///   apply (`fusion = split = use_efficient_pipes = 0`, `limitbudget = 1e8`,
///   `watertemp = 20`). `use_cvar` comes from `risk` when given.
/// * checks of the review and of diagnostic B9.
pub fn review(
    network: &Network,
    overrides: &[(String, f64)],
    risk: Option<RiskMeasure>,
    n_scen_load: Option<usize>,
) -> Result<Reviewed> {
    let mut net = network.clone();
    let mut warnings = Vec::new();
    if net.params.raw("bounded_flow").is_none() && net.arcs.iter().any(|a| a.equipped()) {
        net.params.set("bounded_flow", "1");
    }
    warnings.extend(net.apply_client_data());
    if net.clients.is_empty() {
        return Err(Error::Validation(
            "NW5:Clients: the network has no client".into(),
        ));
    }
    let uniform = net.params.get("uniform_dem") == 1.0;
    let mut exact: Vec<(&str, f64)> = Vec::new();
    for (name, value) in overrides {
        let c = Params::canonical(name);
        if !NAMES.contains(&c) {
            return Err(Error::Validation(format!(
                "NW5:Parameters: unknown parameter {name}"
            )));
        }
        if c == "uniform_dem" {
            warnings
                .push("uniform_dem is derived from the clients; the override is ignored".into());
            continue;
        }
        net.params.set(c, &g15(*value));
        exact.push((c, *value));
    }
    // overrides keep their full precision (the text above is %.15g)
    let g = |n: &str| {
        exact
            .iter()
            .rev()
            .find(|(k, _)| *k == n)
            .map_or_else(|| net.params.get(n), |(_, v)| *v)
    };
    let finite = |n: &str| -> Result<f64> {
        let v = g(n);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(Error::Validation(format!(
                "NW5:Parameters: {n} must be a finite number"
            )))
        }
    };
    let mut p = DesignParams {
        bescal: finite("bescal")?,
        faucetcoef: finite("faucetcoef")?,
        lambda: finite("lambda")?,
        nu: finite("nu")?,
        orifcoef: finite("orifcoef")?,
        bounded_flow: flag(g("bounded_flow"), "bounded_flow")?,
        fusion: 0.0,
        limitbudget: 1e8,
        n_scen_load: 0,
        opentaps: finite("opentaps")?,
        pipelength: g("pipelength"),
        pressure: finite("pressure")?,
        pump_pressure: g("pump_pressure"),
        secu: g("secu"),
        servicequal: finite("servicequal")?,
        seuil: g("seuil"),
        split: 0.0,
        targetflow: finite("targetflow")?,
        uniform_dem: uniform,
        use_cvar: false,
        use_efficient_pipes: 0.0,
        vmax: g("vmax"),
        vmax_secu: finite("vmax_secu")?,
        watertemp: 20.0,
    };
    // forced values (review l.54-65); the file values are only checked
    flag(g("fusion"), "fusion")?;
    flag(g("split"), "split")?;
    p.use_cvar = match risk {
        Some(RiskMeasure::Cvar) => true,
        Some(RiskMeasure::Var) => false,
        None => flag(g("use_cvar"), "use_cvar")?,
    };
    let scen = match n_scen_load {
        Some(n) => n as f64,
        None => g("nScenLoad"),
    };
    if !(scen.is_finite() && scen >= 1.0 && scen.fract() == 0.0) {
        return Err(Error::Validation(format!(
            "NW5:Parameters: nScenLoad must be a positive integer (got {scen})"
        )));
    }
    p.n_scen_load = scen as usize;
    // requireBounded (review l.22-23, 49-51)
    if !uniform {
        let mut bornes: Vec<&str> = net.clients.iter().map(|c| c.terminal.as_str()).collect();
        bornes.sort_unstable();
        bornes.dedup();
        if bornes.len() < net.clients.len() && !p.bounded_flow {
            warnings.push(
                "NW5:BoundedFlowRequired: diverse demand at a shared terminal needs bounded flow; bounded_flow set to 1"
                    .into(),
            );
            p.bounded_flow = true;
        }
    }
    if !(p.secu.is_finite() && p.secu >= 1.0) {
        return Err(Error::Validation(format!(
            "NW5:Parameters: secu must be a finite number >= 1 (got {})",
            p.secu
        )));
    }
    if !(p.pump_pressure.is_finite() && p.pump_pressure >= 0.0) {
        return Err(Error::Validation(format!(
            "NW5:Parameters: pump_pressure must be finite and >= 0 (got {})",
            p.pump_pressure
        )));
    }
    if !(p.pipelength >= 0.01) {
        warnings.push(format!(
            "pipelength {} is below 0.01 m; 0.01 is used",
            p.pipelength
        ));
        p.pipelength = 0.01;
    }
    if !(p.vmax.is_finite() && p.vmax > 0.0) {
        return Err(Error::Validation(format!(
            "NW5:Parameters: vmax must be > 0 (got {})",
            p.vmax
        )));
    }
    for (name, v) in [("servicequal", p.servicequal), ("vmax_secu", p.vmax_secu)] {
        if !(v > 0.0 && v <= 1.0) {
            return Err(Error::Validation(format!(
                "NW5:LoadQuantile: {name} must be in (0,1] (got {v})"
            )));
        }
    }
    if !(p.lambda > 0.0 && p.nu > 0.0 && p.bescal > 0.0) {
        return Err(Error::Validation(
            "NW5:Parameters: lambda, nu and bescal must be > 0".into(),
        ));
    }
    if !(p.targetflow >= 0.0) {
        return Err(Error::Validation(
            "NW5:Demands: targetflow must be >= 0".into(),
        ));
    }
    if !uniform && !p.bounded_flow {
        // B13: rejected by build_reinf_model.m:218, reported here before any work
        return Err(Error::Validation(
            "NW5:Parameters: the case with nonuniform demands and unbounded flows is not treated"
                .into(),
        ));
    }
    let mut clients = net.clients.clone();
    if uniform {
        for c in &mut clients {
            c.probability = p.opentaps;
            c.target_flow = p.targetflow;
        }
    }
    Ok(Reviewed {
        params: p,
        clients,
        warnings,
    })
}
