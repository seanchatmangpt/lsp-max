//! Western Electric SPC Rules for scan-latency monitoring.
//!
//! Ported from wasm4pm/src/spc.rs. Unlike wasm4pm (where UCL/CL/LCL are
//! pre-computed externally), `SpcMonitor` owns the sliding window and derives
//! mean/std-dev via Welford online update, recomputing on window eviction.
//!
//! | Rule | Signal | Min buffer |
//! |------|--------|-----------|
//! | 1 | Any point beyond ±3σ | 2 |
//! | 2 | 9 consecutive same side of mean | 9 |
//! | 3 | 6 consecutive strictly monotone | 6 |
//! | 4 | 2 of 3 consecutive beyond ±2σ | 3 (reliable from 9) |

use std::collections::VecDeque;

const WINDOW_CAPACITY: usize = 50;

/// A Western Electric special-cause signal.
#[derive(Debug, Clone, PartialEq)]
pub enum SpcAlert {
    /// Rule 1: latest sample beyond ±3σ. Carries the sample value in ms.
    Rule1(f64),
    /// Rule 2: 9 consecutive samples on the same side of the mean.
    Rule2,
    /// Rule 3: 6 consecutive samples strictly increasing or decreasing.
    Rule3,
    /// Rule 4: 2 of 3 consecutive samples beyond ±2σ on the same side.
    Rule4,
}

/// Sliding-window SPC monitor for scan-latency samples (milliseconds).
///
/// Call `push(sample_ms)` after every `scan_uri_classified()` invocation.
/// Returns the highest-priority alert when a Western Electric rule fires.
/// Priority: Rule1 > Rule4 > Rule2 > Rule3.
#[derive(Debug)]
pub struct SpcMonitor {
    window: VecDeque<f64>,
    mean: f64,
    m2: f64,
    count: u64,
}

impl Default for SpcMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SpcMonitor {
    /// Create a new monitor with an empty window.
    pub fn new() -> Self {
        Self {
            window: VecDeque::with_capacity(WINDOW_CAPACITY),
            mean: 0.0,
            m2: 0.0,
            count: 0,
        }
    }

    /// Current window mean in milliseconds (0.0 with no samples).
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Sample standard deviation (Bessel-corrected). Returns 0.0 with <2 samples.
    pub fn std_dev(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        (self.m2 / (self.count - 1) as f64).sqrt()
    }

    /// Push a new duration sample (ms) and evaluate Western Electric rules.
    ///
    /// Rules are evaluated against the pre-sample baseline (mean/σ snapshot before
    /// `update_stats`). This matches Western Electric semantics: the control-chart
    /// limits are derived from prior data, not from data that includes the very point
    /// being evaluated (which would let outliers dilute their own deviation).
    pub fn push(&mut self, sample_ms: f64) -> Option<SpcAlert> {
        // Snapshot BEFORE incorporating the new sample.
        let base_mean = self.mean;
        let base_sd = self.std_dev();
        let base_n = self.window.len();

        self.update_stats(sample_ms);
        self.window.push_back(sample_ms);
        if self.window.len() > WINDOW_CAPACITY {
            self.window.pop_front();
            self.recompute_stats();
        }

        // Need at least 2 prior samples to have a meaningful baseline.
        if base_n < 2 || base_sd == 0.0 {
            return None;
        }

        // Rule 1 (highest priority) — evaluated against pre-sample baseline.
        if (sample_ms - base_mean).abs() > 3.0 * base_sd {
            tracing::warn!(
                spc_rule = 1,
                sample_ms,
                mean_ms = base_mean,
                sigma_ms = base_sd,
                "SPC Rule 1: scan latency outlier beyond ±3σ"
            );
            return Some(SpcAlert::Rule1(sample_ms));
        }

        if self.window.len() < 9 {
            return None;
        }

        let recent: Vec<f64> = self.window.iter().copied().rev().take(9).rev().collect();

        // Rule 4 — 2 of 3 consecutive beyond ±2σ.
        {
            let last_3 = &recent[6..];
            let above = last_3
                .iter()
                .filter(|&&v| v > base_mean + 2.0 * base_sd)
                .count();
            let below = last_3
                .iter()
                .filter(|&&v| v < base_mean - 2.0 * base_sd)
                .count();
            if above >= 2 || below >= 2 {
                tracing::warn!(
                    spc_rule = 4,
                    mean_ms = base_mean,
                    "SPC Rule 4: 2 of 3 consecutive scan latencies beyond ±2σ"
                );
                return Some(SpcAlert::Rule4);
            }
        }

        // Rule 2 — 9 consecutive on same side of mean.
        {
            let above = recent.iter().filter(|&&v| v > base_mean).count();
            let below = recent.iter().filter(|&&v| v < base_mean).count();
            if above == 9 || below == 9 {
                tracing::warn!(
                    spc_rule = 2,
                    mean_ms = base_mean,
                    "SPC Rule 2: 9 consecutive scan latencies on same side of mean"
                );
                return Some(SpcAlert::Rule2);
            }
        }

        // Rule 3: last 6 of the 9 strictly monotone.
        {
            let last_6 = &recent[3..];
            let incr = last_6.windows(2).all(|w| w[1] > w[0]);
            let decr = last_6.windows(2).all(|w| w[1] < w[0]);
            if incr || decr {
                tracing::warn!(
                    spc_rule = 3,
                    increasing = incr,
                    "SPC Rule 3: 6 consecutive scan latencies monotonically trending"
                );
                return Some(SpcAlert::Rule3);
            }
        }

        None
    }

    fn update_stats(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    fn recompute_stats(&mut self) {
        let n = self.window.len();
        if n == 0 {
            self.mean = 0.0;
            self.m2 = 0.0;
            self.count = 0;
            return;
        }
        let mean: f64 = self.window.iter().sum::<f64>() / n as f64;
        let m2: f64 = self.window.iter().map(|&x| (x - mean).powi(2)).sum();
        self.mean = mean;
        self.m2 = m2;
        self.count = n as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule2_fires_on_nine_consecutive_above_mean() {
        let mut mon = SpcMonitor::new();
        // Warm up: alternating to establish a stable baseline.
        for i in 0..20u32 {
            let v = if i % 2 == 0 { 9.5 } else { 10.5 };
            let alert = mon.push(v);
            assert!(!matches!(alert, Some(SpcAlert::Rule2)), "premature at {i}");
        }
        let baseline_mean = mon.mean();
        assert!(
            (baseline_mean - 10.0).abs() < 0.5,
            "bad baseline: {baseline_mean}"
        );

        // Anchor: one below-mean sample ensures the elevation run starts clean.
        // Without this, the last warmup sample (10.5) can sit above the drifting
        // mean and make the window [10.5, 11.0×8] appear as 9-in-a-row.
        let _ = mon.push(baseline_mean - 1.0);

        // Push 8 elevated samples — Rule 2 must NOT fire yet.
        for i in 0..8u32 {
            let alert = mon.push(baseline_mean + 1.0);
            assert!(!matches!(alert, Some(SpcAlert::Rule2)), "too early at {i}");
        }
        // 9th consecutive above-mean → Rule 2 fires.
        assert_eq!(mon.push(baseline_mean + 1.0), Some(SpcAlert::Rule2));
    }

    #[test]
    fn rule2_does_not_fire_on_alternating_samples() {
        let mut mon = SpcMonitor::new();
        for i in 0..29u32 {
            let v = if i % 2 == 0 { 9.0 } else { 11.0 };
            assert!(
                !matches!(mon.push(v), Some(SpcAlert::Rule2)),
                "fired at {i}"
            );
        }
    }

    #[test]
    fn rule1_fires_on_outlier() {
        let mut mon = SpcMonitor::new();
        for i in 0..20u32 {
            mon.push(10.0 + (i % 3) as f64 * 0.5);
        }
        let mean = mon.mean();
        let sd = mon.std_dev();
        let outlier = mean + 4.0 * sd;
        assert!(matches!(mon.push(outlier), Some(SpcAlert::Rule1(_))));
    }
}
