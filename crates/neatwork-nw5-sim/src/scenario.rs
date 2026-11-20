//! Monte-Carlo scenario generation (`ScenarioGenerator`, spec section 3.2).

use crate::error::SimError;
use crate::json::Value;
use crate::options::SimulationOptions;
use crate::prep::Model;
use crate::result::GenerationInfo;
use crate::rng::Pcg64;
use crate::validate::Resolved;

/// Generated scenarios.
#[derive(Clone, Debug)]
pub(crate) struct Scenarios {
    /// Open clients of each scenario (indices in the sorted clients).
    pub(crate) open: Vec<Vec<usize>>,
    /// Demand of each terminal, L/s, per scenario.
    pub(crate) demand: Vec<Vec<f64>>,
    /// Open clients of each terminal, per scenario.
    pub(crate) count: Vec<Vec<u32>>,
    /// Probability and target flow used for each client.
    pub(crate) probability: Vec<f64>,
    pub(crate) target: Vec<f64>,
    pub(crate) info: GenerationInfo,
}

fn generation_error(message: &str, details: Vec<(String, Value)>) -> SimError {
    SimError::ScenarioGeneration {
        message: message.to_string(),
        details,
    }
}

/// Draw `resolved.scenario_count` non-empty scenarios with numpy's PCG64.
pub(crate) fn generate(
    model: &Model,
    resolved: &Resolved,
    options: &SimulationOptions,
    seed: u64,
) -> Result<Scenarios, SimError> {
    let c = model.clients.len();
    let n = resolved.scenario_count;
    let (probability, target): (Vec<f64>, Vec<f64>) = match (
        resolved.uniform,
        resolved.open_taps_probability,
        resolved.target_flow_lps,
    ) {
        (true, Some(p), Some(q)) => (vec![p; c], vec![q; c]),
        _ => (
            model.clients.iter().map(|x| x.proba).collect(),
            model.clients.iter().map(|x| x.cible).collect(),
        ),
    };
    if c == 0 {
        return Err(generation_error(
            "The network contains no clients.",
            Vec::new(),
        ));
    }
    let mut product = 1.0;
    for p in &probability {
        product *= 1.0 - p;
    }
    let vp = 1.0 - product;
    if vp.is_nan() || vp <= 0.0 {
        return Err(generation_error(
            "No non-empty demand scenario can be generated.",
            vec![("validScenarioProbability".into(), Value::Float(vp))],
        ));
    }
    let mut rng = Pcg64::new(seed);
    let mut open: Vec<Vec<usize>> = Vec::with_capacity(n);
    let (mut batches, mut draws) = (0usize, 0usize);
    let mut bits: Vec<u64> = Vec::new();
    while open.len() < n {
        batches += 1;
        if batches > options.max_generation_batches {
            return Err(generation_error(
                "Scenario generation exceeded the maximum number of batches.",
                vec![
                    ("requestedScenarios".into(), Value::from(n)),
                    ("generatedScenarios".into(), Value::from(open.len())),
                    ("generationBatches".into(), Value::from(batches - 1)),
                    ("validScenarioProbability".into(), Value::Float(vp)),
                ],
            ));
        }
        let remaining = n - open.len();
        let estimate = (options.oversampling_factor * remaining as f64 / vp).ceil();
        let estimate = if estimate.is_finite() && estimate < usize::MAX as f64 {
            estimate as usize
        } else {
            usize::MAX
        };
        let d = remaining.max(estimate).min(options.max_draws_per_batch);
        draws += d;
        // R[c, j] is draw c*d + j (C order)
        bits.clear();
        bits.resize((c * d).div_ceil(64), 0);
        for (ci, &p) in probability.iter().enumerate() {
            for j in 0..d {
                if rng.next_f64() < p {
                    let b = ci * d + j;
                    bits[b / 64] |= 1 << (b % 64);
                }
            }
        }
        let is_open = |ci: usize, j: usize| {
            let b = ci * d + j;
            bits[b / 64] >> (b % 64) & 1 == 1
        };
        for j in 0..d {
            if open.len() == n {
                break;
            }
            let column: Vec<usize> = (0..c).filter(|&ci| is_open(ci, j)).collect();
            if !column.is_empty() {
                open.push(column);
            }
        }
    }
    let terminals = model.terminals.len();
    let mut demand = Vec::with_capacity(n);
    let mut count = Vec::with_capacity(n);
    for column in &open {
        let mut dk = vec![0.0; terminals];
        let mut ck = vec![0u32; terminals];
        for &ci in column {
            let t = model.client_terminal[ci];
            dk[t] += target[ci];
            ck[t] += 1;
        }
        demand.push(dk);
        count.push(ck);
    }
    Ok(Scenarios {
        open,
        demand,
        count,
        probability,
        target,
        info: GenerationInfo {
            valid_scenario_probability: vp,
            generated_draw_count: draws,
            generation_batch_count: batches,
        },
    })
}
