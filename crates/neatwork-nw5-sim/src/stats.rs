//! Statistics over the successful scenarios (`SimulationStatisticsProcessor`,
//! spec section 5.1) and the JSON payload of schema 1.1 (spec 5.3).

use crate::error::SimError;
use crate::json::Value;
use crate::result::{PipeKind, ScenarioStatus, SimulationResult};

/// `SATISFACTION_SHORTFALL_TOLERANCE`.
const SHORTFALL_TOLERANCE: f64 = 1.0e-9;
/// `COEFFICIENT_OF_VARIATION_MEAN_EPSILON`.
const CV_MEAN_EPSILON: f64 = 1.0e-12;

/// Satisfaction statistics of one terminal (`TableSat` row).
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalSatisfaction {
    /// Terminal ID.
    pub terminal_id: String,
    /// `minSat`, `moySat`, `maxSat` over the evaluated active scenarios
    /// (NaN when there is none).
    pub min_sat: f64,
    /// Mean.
    pub moy_sat: f64,
    /// Maximum.
    pub max_sat: f64,
    /// Mean of the values at or below the 5th percentile.
    pub cvar_sat5: f64,
    /// Mean of the values at or below the 20th percentile.
    pub cvar_sat20: f64,
    /// Population standard deviation.
    pub std_sat: f64,
    /// `std / mean` (NaN for a mean near 0).
    pub cv_sat: f64,
    /// Scenarios with an open client (all scenarios).
    pub active_scenario_count: usize,
    /// Active and successful scenarios.
    pub evaluated_active_scenario_count: usize,
    /// Evaluated scenarios with satisfaction below `1 - 1e-9`.
    pub water_shortfall_count: usize,
}

/// Flow statistics of one source node, L/s (`TableSt` row).
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFlow {
    /// Source node ID.
    pub source_id: String,
    /// Minimum.
    pub min_st: f64,
    /// Mean.
    pub moy_st: f64,
    /// Maximum.
    pub max_st: f64,
    /// Mean of the values at or below the 5th percentile.
    pub cvar_st5: f64,
    /// Mean of the values at or above the 95th percentile.
    pub cvar_st95: f64,
}

/// Velocity statistics of a source or transit pipe, m/s (`TableResV` row).
#[derive(Clone, Debug, PartialEq)]
pub struct PipeVelocity {
    /// Arc ID.
    pub edge_id: String,
    /// Upstream node.
    pub start_node_id: String,
    /// Downstream node.
    pub end_node_id: String,
    /// Minimum.
    pub min_velocity: f64,
    /// Mean.
    pub mean_velocity: f64,
    /// Maximum.
    pub max_velocity: f64,
}

/// Pressure statistics of a node, m (`TablePress` / `TablePrB` row).
#[derive(Clone, Debug, PartialEq)]
pub struct PressureStats {
    /// Node ID.
    pub node_id: String,
    /// Minimum.
    pub min_pressure: f64,
    /// Mean.
    pub mean_pressure: f64,
    /// Maximum.
    pub max_pressure: f64,
}

/// Run summary.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// Scenarios.
    pub scenario_count: usize,
    /// Successful scenarios.
    pub successful_scenarios: usize,
    /// Failed scenarios.
    pub failed_scenarios: usize,
    /// `successful / scenarios`.
    pub success_rate: f64,
    /// Seed.
    pub seed: u64,
    /// Wall time, s.
    pub duration_seconds: f64,
    /// Iterations over the successful scenarios.
    pub min_iterations: i64,
    /// Mean iterations.
    pub mean_iterations: f64,
    /// Maximum iterations.
    pub max_iterations: i64,
    /// Failures by code, in order of first occurrence.
    pub failure_count_by_code: Vec<(String, usize)>,
}

/// All statistics tables.
#[derive(Clone, Debug, PartialEq)]
pub struct Statistics {
    /// Per terminal, sorted.
    pub terminal_satisfaction: Vec<TerminalSatisfaction>,
    /// Per source node, sorted.
    pub source_flow: Vec<SourceFlow>,
    /// Per source and transit pipe, in pipe order.
    pub pipe_velocity: Vec<PipeVelocity>,
    /// Per terminal: `-charge` of the terminal pipe over the served
    /// scenarios (0 when never served).
    pub terminal_pressure: Vec<PressureStats>,
    /// Per transit node: head minus elevation.
    pub transit_pressure: Vec<PressureStats>,
    /// Summary.
    pub summary: Summary,
    /// Scenarios included in the statistics.
    pub successful_mask: Vec<bool>,
}

/// numpy pairwise summation (`pairwise_sum`, block 128, 8 accumulators).
pub(crate) fn np_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for v in a {
            res += v;
        }
        res
    } else if n <= 128 {
        let mut r = [0.0f64; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i < n - (n % 8) {
            for k in 0..8 {
                r[k] += a[i + k];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        np_sum(&a[..n2]) + np_sum(&a[n2..])
    }
}

fn mean(a: &[f64]) -> f64 {
    np_sum(a) / a.len() as f64
}

fn fmin(a: &[f64]) -> f64 {
    a.iter().copied().fold(f64::INFINITY, f64::min)
}

fn fmax(a: &[f64]) -> f64 {
    a.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// numpy `percentile(v, q)` with the `linear` method (`v` non-empty).
fn percentile(v: &[f64], q: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    let vi = (n - 1) as f64 * (q / 100.0);
    let below = vi.floor();
    let (lo, hi) = if vi >= (n - 1) as f64 {
        (n - 1, n - 1)
    } else {
        (below as usize, below as usize + 1)
    };
    let gamma = vi - below;
    let (a, b) = (s[lo], s[hi]);
    let diff = b - a;
    if gamma >= 0.5 {
        b - diff * (1.0 - gamma)
    } else {
        a + diff * gamma
    }
}

/// Row statistics over the values where `mask` holds (`_masked_row_statistics`).
/// NaN entries for an empty selection, except `cv` handled by the caller.
struct Masked {
    min: f64,
    mean: f64,
    max: f64,
    std: f64,
    cv: f64,
    tails: Vec<f64>,
    count: usize,
}

fn masked(data: &[f64], mask: &[bool], lower_tails: &[f64]) -> Masked {
    let sel: Vec<f64> = data
        .iter()
        .zip(mask)
        .filter(|(_, m)| **m)
        .map(|(v, _)| *v)
        .collect();
    let count = sel.len();
    if count == 0 {
        return Masked {
            min: f64::NAN,
            mean: f64::NAN,
            max: f64::NAN,
            std: f64::NAN,
            cv: f64::NAN,
            tails: vec![f64::NAN; lower_tails.len()],
            count,
        };
    }
    // nanmean/nanvar sum over the whole row with zeros at masked entries
    let zeroed: Vec<f64> = data
        .iter()
        .zip(mask)
        .map(|(v, m)| if *m { *v } else { 0.0 })
        .collect();
    let avg = np_sum(&zeroed) / count as f64;
    let sq: Vec<f64> = data
        .iter()
        .zip(mask)
        .map(|(v, m)| if *m { (v - avg) * (v - avg) } else { 0.0 })
        .collect();
    let std = (np_sum(&sq) / count as f64).sqrt();
    let cv = if avg.is_finite() && avg.abs() > CV_MEAN_EPSILON {
        std / avg
    } else {
        f64::NAN
    };
    let tails = lower_tails
        .iter()
        .map(|&q| tail_mean(data, mask, percentile(&sel, q), false))
        .collect();
    Masked {
        min: fmin(&sel),
        mean: avg,
        max: fmax(&sel),
        std,
        cv,
        tails,
        count,
    }
}

/// Mean of the selected values on one side of a threshold (`np.where` then
/// `nanmean`).
fn tail_mean(data: &[f64], mask: &[bool], threshold: f64, upper: bool) -> f64 {
    let mut count = 0usize;
    let row: Vec<f64> = data
        .iter()
        .zip(mask)
        .map(|(v, m)| {
            let keep = *m
                && if upper {
                    *v >= threshold
                } else {
                    *v <= threshold
                };
            if keep {
                count += 1;
                *v
            } else {
                0.0
            }
        })
        .collect();
    if count == 0 {
        f64::NAN
    } else {
        np_sum(&row) / count as f64
    }
}

fn nan_to_zero(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v }
}

/// Compute the statistics of a batch (`None` if no scenario succeeded).
pub(crate) fn compute(r: &SimulationResult) -> Option<Statistics> {
    let ok: Vec<usize> = (0..r.scenarios.len())
        .filter(|&k| r.scenarios[k].status == ScenarioStatus::Success)
        .collect();
    if ok.is_empty() {
        return None;
    }
    let all = vec![true; ok.len()];
    let terminal_pipes = r.terminal_pipes();
    let mut terminal_satisfaction = Vec::with_capacity(r.terminals.len());
    let mut terminal_pressure = Vec::with_capacity(r.terminals.len());
    for (t, node) in r.terminals.iter().enumerate() {
        let sat: Vec<f64> = ok.iter().map(|&k| r.scenarios[k].satisfaction[t]).collect();
        let eval: Vec<bool> = ok
            .iter()
            .map(|&k| r.scenarios[k].open_clients[t] > 0)
            .collect();
        let st = masked(&sat, &eval, &[5.0, 20.0]);
        let shortfall = sat
            .iter()
            .zip(&eval)
            .filter(|(v, e)| **e && **v < 1.0 - SHORTFALL_TOLERANCE)
            .count();
        let none = st.count == 0;
        let fix = |v: f64| if none { f64::NAN } else { nan_to_zero(v) };
        terminal_satisfaction.push(TerminalSatisfaction {
            terminal_id: node.id.clone(),
            min_sat: fix(st.min),
            moy_sat: fix(st.mean),
            max_sat: fix(st.max),
            cvar_sat5: fix(st.tails[0]),
            cvar_sat20: fix(st.tails[1]),
            std_sat: fix(st.std),
            cv_sat: st.cv,
            active_scenario_count: r.scenarios.iter().filter(|s| s.open_clients[t] > 0).count(),
            evaluated_active_scenario_count: st.count,
            water_shortfall_count: shortfall,
        });
        let j = terminal_pipes[t];
        let press: Vec<f64> = ok.iter().map(|&k| -r.scenarios[k].charge[j]).collect();
        let served: Vec<bool> = ok.iter().map(|&k| r.scenarios[k].served[t]).collect();
        let st = masked(&press, &served, &[]);
        terminal_pressure.push(PressureStats {
            node_id: node.id.clone(),
            min_pressure: nan_to_zero(st.min),
            mean_pressure: nan_to_zero(st.mean),
            max_pressure: nan_to_zero(st.max),
        });
    }
    // source flows per source node (sum of its source pipes), L/s
    let mut source_flow = Vec::with_capacity(r.sources.len());
    for src in &r.sources {
        let pipes: Vec<usize> = (0..r.pipes.len())
            .filter(|&j| r.pipes[j].kind == PipeKind::Source && r.pipes[j].from == src.id)
            .collect();
        let f: Vec<f64> = ok
            .iter()
            .map(|&k| {
                pipes
                    .iter()
                    .fold(0.0, |acc, &j| acc + r.scenarios[k].flow[j])
                    * 1000.0
            })
            .collect();
        let lower = tail_mean(&f, &all, percentile(&f, 5.0), false);
        let upper = tail_mean(&f, &all, percentile(&f, 95.0), true);
        source_flow.push(SourceFlow {
            source_id: src.id.clone(),
            min_st: fmin(&f),
            moy_st: mean(&f),
            max_st: fmax(&f),
            cvar_st5: nan_to_zero(lower),
            cvar_st95: nan_to_zero(upper),
        });
    }
    let mut pipe_velocity = Vec::new();
    for p in r
        .pipes
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind != PipeKind::Terminal)
    {
        let (j, p) = p;
        let d = p.velocity_diameter;
        let area = std::f64::consts::PI * (d * d) / 4.0;
        let v: Vec<f64> = ok
            .iter()
            .map(|&k| r.scenarios[k].flow[j].abs() / area)
            .collect();
        pipe_velocity.push(PipeVelocity {
            edge_id: p.id.clone(),
            start_node_id: p.from.clone(),
            end_node_id: p.to.clone(),
            min_velocity: fmin(&v),
            mean_velocity: mean(&v),
            max_velocity: fmax(&v),
        });
    }
    let transit_pressure = r
        .transit_nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let v: Vec<f64> = ok
                .iter()
                .map(|&k| -node.elevation + r.scenarios[k].head[i])
                .collect();
            PressureStats {
                node_id: node.id.clone(),
                min_pressure: fmin(&v),
                mean_pressure: mean(&v),
                max_pressure: fmax(&v),
            }
        })
        .collect();
    let iterations: Vec<i64> = ok.iter().map(|&k| r.scenarios[k].iterations).collect();
    let iter_f: Vec<f64> = iterations.iter().map(|&i| i as f64).collect();
    let mut by_code: Vec<(String, usize)> = Vec::new();
    for f in &r.failures {
        match by_code.iter_mut().find(|(c, _)| c == f.code) {
            Some(e) => e.1 += 1,
            None => by_code.push((f.code.to_string(), 1)),
        }
    }
    let n = r.scenarios.len();
    Some(Statistics {
        terminal_satisfaction,
        source_flow,
        pipe_velocity,
        terminal_pressure,
        transit_pressure,
        summary: Summary {
            scenario_count: n,
            successful_scenarios: ok.len(),
            failed_scenarios: n - ok.len(),
            success_rate: ok.len() as f64 / n as f64,
            seed: r.seed,
            duration_seconds: r.duration_seconds,
            min_iterations: iterations.iter().copied().min().unwrap_or(0),
            mean_iterations: mean(&iter_f),
            max_iterations: iterations.iter().copied().max().unwrap_or(0),
            failure_count_by_code: by_code,
        },
        successful_mask: r
            .scenarios
            .iter()
            .map(|s| s.status == ScenarioStatus::Success)
            .collect(),
    })
}

fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn pressure_rows(rows: &[PressureStats], key: &str) -> Value {
    Value::List(
        rows.iter()
            .map(|p| {
                map(vec![
                    (key, Value::from(p.node_id.as_str())),
                    ("minPressureM", Value::Float(p.min_pressure)),
                    ("meanPressureM", Value::Float(p.mean_pressure)),
                    ("maxPressureM", Value::Float(p.max_pressure)),
                ])
            })
            .collect(),
    )
}

/// The statistics payload of schema 1.1 (`SimulationJsonExporter.build_payload`).
pub fn statistics_payload(r: &SimulationResult) -> Result<Value, SimError> {
    let Some(s) = &r.statistics else {
        return Err(SimError::StatisticsProcessing {
            message: "Statistics cannot be computed because no scenario succeeded.".into(),
            details: vec![
                ("scenarioCount".into(), Value::from(r.scenarios.len())),
                (
                    "failedScenarioCount".into(),
                    Value::from(r.scenarios.len() - r.successful_count()),
                ),
            ],
        });
    };
    let sm = &s.summary;
    let summary = map(vec![
        ("scenarioCount", Value::from(sm.scenario_count)),
        ("successfulScenarios", Value::from(sm.successful_scenarios)),
        ("failedScenarios", Value::from(sm.failed_scenarios)),
        ("successRate", Value::Float(sm.success_rate)),
        (
            "seed",
            Value::Int(i64::try_from(sm.seed).unwrap_or(i64::MAX)),
        ),
        ("durationSeconds", Value::Float(sm.duration_seconds)),
        ("minIterations", Value::Int(sm.min_iterations)),
        ("meanIterations", Value::Float(sm.mean_iterations)),
        ("maxIterations", Value::Int(sm.max_iterations)),
        (
            "failureCountByCode",
            Value::Map(
                sm.failure_count_by_code
                    .iter()
                    .map(|(c, n)| (c.clone(), Value::from(*n)))
                    .collect(),
            ),
        ),
    ]);
    let included: Vec<Value> = (0..s.successful_mask.len())
        .filter(|&k| s.successful_mask[k])
        .map(Value::from)
        .collect();
    let excluded: Vec<Value> = (0..s.successful_mask.len())
        .filter(|&k| !s.successful_mask[k])
        .map(Value::from)
        .collect();
    let selection = map(vec![
        (
            "successfulMask",
            Value::List(s.successful_mask.iter().map(|b| Value::Bool(*b)).collect()),
        ),
        ("includedScenarioIndices", Value::List(included)),
        ("excludedScenarioIndices", Value::List(excluded)),
    ]);
    let terminal_satisfaction = Value::List(
        s.terminal_satisfaction
            .iter()
            .map(|t| {
                map(vec![
                    ("terminalId", Value::from(t.terminal_id.as_str())),
                    ("minSat", Value::Float(t.min_sat)),
                    ("moySat", Value::Float(t.moy_sat)),
                    ("maxSat", Value::Float(t.max_sat)),
                    ("cVarSat5", Value::Float(t.cvar_sat5)),
                    ("cVarSat20", Value::Float(t.cvar_sat20)),
                    ("stdSat", Value::Float(t.std_sat)),
                    ("cvSat", Value::Float(t.cv_sat)),
                    ("activeScenarioCount", Value::from(t.active_scenario_count)),
                    (
                        "evaluatedActiveScenarioCount",
                        Value::from(t.evaluated_active_scenario_count),
                    ),
                    ("waterShortfallCount", Value::from(t.water_shortfall_count)),
                ])
            })
            .collect(),
    );
    let source_flow = Value::List(
        s.source_flow
            .iter()
            .map(|f| {
                map(vec![
                    ("sourceId", Value::from(f.source_id.as_str())),
                    ("minSt", Value::Float(f.min_st)),
                    ("moySt", Value::Float(f.moy_st)),
                    ("maxSt", Value::Float(f.max_st)),
                    ("cVarSt5", Value::Float(f.cvar_st5)),
                    ("cVarSt95", Value::Float(f.cvar_st95)),
                ])
            })
            .collect(),
    );
    let pipe_velocity = Value::List(
        s.pipe_velocity
            .iter()
            .map(|p| {
                map(vec![
                    ("edgeId", Value::from(p.edge_id.as_str())),
                    ("startNodeId", Value::from(p.start_node_id.as_str())),
                    ("endNodeId", Value::from(p.end_node_id.as_str())),
                    ("minVelocityMps", Value::Float(p.min_velocity)),
                    ("meanVelocityMps", Value::Float(p.mean_velocity)),
                    ("maxVelocityMps", Value::Float(p.max_velocity)),
                ])
            })
            .collect(),
    );
    Ok(map(vec![
        ("schemaVersion", Value::from("1.1")),
        (
            "network",
            map(vec![
                ("name", Value::from(r.network_name.as_str())),
                ("version", Value::from(r.network_version.as_str())),
            ]),
        ),
        ("demand", map(vec![("terminals", Value::List(Vec::new()))])),
        (
            "units",
            map(vec![
                ("sourceFlow", Value::from("L/s")),
                ("pipeVelocity", Value::from("m/s")),
                ("terminalPressure", Value::from("m")),
                ("transitPressure", Value::from("m")),
                ("duration", Value::from("s")),
                ("satisfaction", Value::from("ratio")),
            ]),
        ),
        ("summary", summary),
        ("scenarioSelection", selection),
        (
            "statistics",
            map(vec![
                ("terminalSatisfaction", terminal_satisfaction),
                ("sourceFlow", source_flow),
                ("pipeVelocity", pipe_velocity),
                (
                    "terminalPressure",
                    pressure_rows(&s.terminal_pressure, "terminalId"),
                ),
                (
                    "transitPressure",
                    pressure_rows(&s.transit_pressure, "transitNodeId"),
                ),
            ]),
        ),
        (
            "failures",
            Value::List(r.failures.iter().map(|f| f.to_value()).collect()),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numpy_linear_percentile() {
        let v = [3.0, 1.0, 2.0, 4.0];
        assert_eq!(percentile(&v, 5.0), 1.15);
        assert_eq!(percentile(&v, 100.0), 4.0);
        assert_eq!(percentile(&[7.0], 20.0), 7.0);
        assert!((percentile(&v, 95.0) - 3.85).abs() < 1e-15);
    }

    #[test]
    fn pairwise_sum_matches_plain_sum_on_integers() {
        let v: Vec<f64> = (0..1000).map(f64::from).collect();
        assert_eq!(np_sum(&v), 499_500.0);
        assert_eq!(np_sum(&[]), 0.0);
    }

    #[test]
    fn masked_population_statistics() {
        // python-core tests/simulation/test_terminal_reliability_statistics.py: P3
        let m = masked(&[1.0, 0.5, 9.0, 1.2], &[true, true, false, true], &[5.0]);
        let mean = (1.0 + 0.5 + 1.2) / 3.0;
        let std =
            (((1.0f64 - mean).powi(2) + (0.5 - mean).powi(2) + (1.2 - mean).powi(2)) / 3.0).sqrt();
        assert_eq!(m.count, 3);
        assert_eq!(m.max, 1.2);
        assert!((m.std - std).abs() < 1e-15);
        assert!((m.cv - std / mean).abs() < 1e-15);
    }
}
