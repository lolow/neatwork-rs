//! Run, mode and solver options (spec sections 1.4.2, 2.1 and 2.5).

/// Orifice model of NW4 mode (`orif_com`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrificeMode {
    /// `orif_com = 1`: commercial orifices.
    Commercial,
    /// `orif_com = 0`: ideal orifices.
    Ideal,
}

/// Runtime overrides of the network parameters (API `SimulationOptionsInput`).
/// `None` keeps the value of the network.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeOverrides {
    /// Sets `orif_com`.
    pub orifice_mode: Option<OrificeMode>,
    /// Sets `bounded_flow`. Diverse client demand forces bounded flow anyway.
    pub bounded_flow: Option<bool>,
    /// Sets `pressure` and the required pressure of every borne, m.
    pub required_pressure_m: Option<f64>,
    /// Sets `orifcoef` (> 0).
    pub orifice_coefficient: Option<f64>,
    /// Sets `faucetcoef` (>= 0).
    pub faucet_coefficient: Option<f64>,
}

/// Interior-point options (`SolverOptions`). The defaults are the fixed
/// python-core values.
#[derive(Clone, Debug, PartialEq)]
pub struct SolverOptions {
    /// `tolx`, m3/s.
    pub flow_tolerance: f64,
    /// `tolpress`, m.
    pub pressure_tolerance: f64,
    /// `tolcent` (checked, unused as in python-core).
    pub centering_tolerance: f64,
    /// `maxiter`.
    pub max_iterations: usize,
    /// `sigma`.
    pub barrier_reduction: f64,
    /// `tau`.
    pub centering_exponent: f64,
    /// `a_secur`.
    pub step_safety: f64,
}

impl Default for SolverOptions {
    fn default() -> SolverOptions {
        SolverOptions {
            flow_tolerance: 1e-8,
            pressure_tolerance: 1e-6,
            centering_tolerance: 1e-10,
            max_iterations: 30,
            barrier_reduction: 0.1,
            centering_exponent: 2.5,
            step_safety: 0.999,
        }
    }
}

/// Options of a simulation run.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationOptions {
    /// Number of non-empty scenarios. `None` uses the `nbsim` parameter.
    pub scenario_count: Option<usize>,
    /// PCG64 seed. `None` draws a seed below 2^32 from OS entropy (as the
    /// python-core API does) and reports it in the result.
    pub seed: Option<u64>,
    /// Legacy uniform probability: ignored in uniform mode, an error with
    /// diverse demand (as python-core).
    pub open_taps_probability: Option<f64>,
    /// Legacy uniform target flow, L/s: same rule.
    pub target_flow_lps: Option<f64>,
    /// Scenario-generation batch limit.
    pub max_generation_batches: usize,
    /// Draw oversampling, > 1.
    pub oversampling_factor: f64,
    /// Cap on the draws per batch.
    pub max_draws_per_batch: usize,
    /// Parameter overrides.
    pub overrides: ModeOverrides,
    /// Open state of arcs by arc ID (`edgeOverrides`); they replace
    /// `Arc::open`.
    pub arc_open: Vec<(String, bool)>,
    /// Interior-point options.
    pub solver: SolverOptions,
}

impl Default for SimulationOptions {
    fn default() -> SimulationOptions {
        SimulationOptions {
            scenario_count: None,
            seed: None,
            open_taps_probability: None,
            target_flow_lps: None,
            max_generation_batches: 20,
            oversampling_factor: 1.10,
            max_draws_per_batch: 100_000,
            overrides: ModeOverrides::default(),
            arc_open: Vec::new(),
            solver: SolverOptions::default(),
        }
    }
}

impl SimulationOptions {
    /// Options with a scenario count and a seed, other values default.
    pub fn new(scenario_count: usize, seed: u64) -> SimulationOptions {
        SimulationOptions {
            scenario_count: Some(scenario_count),
            seed: Some(seed),
            ..SimulationOptions::default()
        }
    }
}
