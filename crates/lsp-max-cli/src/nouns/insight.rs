use crate::nouns::config::ConfigService;
use crate::nouns::gate::GateService;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Bounded summary of the compositor ANDON gate.
#[derive(Debug, Serialize)]
pub struct GateSignal {
    pub andon_blocked: bool,
    /// False when the compositor process has not written the gate file yet.
    pub compositor_active: bool,
    /// ANDON codes parsed from the gate file, or empty when none are surfaced.
    pub active_andon_codes: Vec<String>,
}

/// Bounded summary of the config doctor surface.
#[derive(Debug, Serialize)]
pub struct ConfigSignal {
    /// Mirrors `ConfigDoctorResult.overall`: ADMITTED / PARTIAL / UNKNOWN.
    pub overall: String,
    pub admitted_keys: usize,
    pub partial_keys: usize,
    pub unknown_keys: usize,
}

/// Aggregate law-axis counts folded across governing surfaces.
///
/// `refused` and `unknown` are disjoint per `ConformanceVector` law and are
/// never merged: a refused axis is a known refusal, an unknown axis is an
/// unmeasured gap. Collapsing one into the other erases that distinction.
#[derive(Debug, Serialize, Default, PartialEq, Eq)]
pub struct AxisCounts {
    pub admitted: usize,
    pub refused: usize,
    pub unknown: usize,
}

/// A single law-state digest aggregating gate + config into bounded signals.
///
/// One call gives agents/CI the current law-state without fanning out across
/// every noun individually.
#[derive(Debug, Serialize)]
pub struct LawStateDigest {
    /// Worst-of fold: BLOCKED > UNKNOWN > PARTIAL > ADMITTED.
    /// UNKNOWN is never collapsed into ADMITTED or REFUSED.
    pub overall: String,
    pub gate: GateSignal,
    pub config: ConfigSignal,
    pub axes: AxisCounts,
    pub notes: Vec<String>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct InsightService;

impl InsightService {
    pub fn new() -> Self {
        Self
    }

    /// Aggregate the gate and config surfaces into one bounded digest.
    pub fn digest(&self) -> LawStateDigest {
        let gate = self.gate_signal();
        let config = self.config_signal();
        let axes = self.axis_counts(&gate, &config);
        let mut notes = Vec::new();

        if !gate.compositor_active {
            notes.push("gate: compositor not active — ANDON state UNKNOWN".to_string());
        }
        if gate.andon_blocked && gate.active_andon_codes.is_empty() {
            notes.push("gate: ANDON set without surfaced codes".to_string());
        }

        let overall = compute_overall(&gate, &config, &axes);
        LawStateDigest {
            overall,
            gate,
            config,
            axes,
            notes,
        }
    }

    /// Aggregate only the axis counts, for callers that want the law-axis fold
    /// without the full digest envelope.
    pub fn axes(&self) -> AxisCounts {
        let gate = self.gate_signal();
        let config = self.config_signal();
        self.axis_counts(&gate, &config)
    }

    fn gate_signal(&self) -> GateSignal {
        let ctx = GateService::new().check_agent_context();
        GateSignal {
            andon_blocked: ctx.andon_blocked,
            compositor_active: ctx.compositor_active,
            active_andon_codes: ctx.active_andon_codes,
        }
    }

    fn config_signal(&self) -> ConfigSignal {
        let doctor = ConfigService::new().doctor();
        let mut admitted_keys = 0;
        let mut partial_keys = 0;
        let mut unknown_keys = 0;
        for key in &doctor.keys {
            match key.status.as_str() {
                "ADMITTED" => admitted_keys += 1,
                "PARTIAL" => partial_keys += 1,
                "UNKNOWN" => unknown_keys += 1,
                _ => {}
            }
        }
        ConfigSignal {
            overall: doctor.overall,
            admitted_keys,
            partial_keys,
            unknown_keys,
        }
    }

    /// Fold governing axes into disjoint admitted/refused/unknown counts.
    ///
    /// Gate ANDON codes count as refused law axes. A gate set without surfaced
    /// codes contributes an unknown axis (the refusal is known but its codes
    /// are unmeasured). Config UNKNOWN keys count as unknown axes; PARTIAL and
    /// ADMITTED keys count as admitted axes. Refused and unknown stay distinct.
    fn axis_counts(&self, gate: &GateSignal, config: &ConfigSignal) -> AxisCounts {
        let mut counts = AxisCounts::default();

        counts.refused += gate.active_andon_codes.len();
        if gate.andon_blocked && gate.active_andon_codes.is_empty() {
            counts.unknown += 1;
        }

        counts.admitted += config.admitted_keys + config.partial_keys;
        counts.unknown += config.unknown_keys;

        counts
    }
}

impl Default for InsightService {
    fn default() -> Self {
        Self::new()
    }
}

/// Worst-of fold across the gate and config surfaces.
///
/// BLOCKED dominates (gate ANDON set). Otherwise any unknown axis or config
/// UNKNOWN demotes to UNKNOWN; any config PARTIAL demotes to PARTIAL; else
/// ADMITTED. UNKNOWN is never collapsed into ADMITTED or REFUSED.
fn compute_overall(gate: &GateSignal, config: &ConfigSignal, axes: &AxisCounts) -> String {
    if gate.andon_blocked {
        return "BLOCKED".to_string();
    }
    if axes.unknown > 0 || config.overall == "UNKNOWN" {
        return "UNKNOWN".to_string();
    }
    if config.overall == "PARTIAL" {
        return "PARTIAL".to_string();
    }
    "ADMITTED".to_string()
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Aggregate gate + config surfaces into one bounded law-state digest.
/// Read-only: reads the gate file and config surface; mutates nothing.
#[verb("digest")]
pub fn digest() -> Result<LawStateDigest> {
    Ok(InsightService::new().digest())
}

/// Emit only the folded law-axis counts (admitted/refused/unknown).
/// Refused and unknown are reported as disjoint counts.
#[verb("axes")]
pub fn axes() -> Result<AxisCounts> {
    Ok(InsightService::new().axes())
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(blocked: bool, active: bool, codes: &[&str]) -> GateSignal {
        GateSignal {
            andon_blocked: blocked,
            compositor_active: active,
            active_andon_codes: codes.iter().map(|c| c.to_string()).collect(),
        }
    }

    fn config(overall: &str, admitted: usize, partial: usize, unknown: usize) -> ConfigSignal {
        ConfigSignal {
            overall: overall.to_string(),
            admitted_keys: admitted,
            partial_keys: partial,
            unknown_keys: unknown,
        }
    }

    #[test]
    fn overall_blocked_dominates_even_with_clean_config() {
        let g = gate(true, true, &["WASM4PM-001"]);
        let c = config("ADMITTED", 6, 0, 0);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(compute_overall(&g, &c, &axes), "BLOCKED");
    }

    #[test]
    fn overall_unknown_when_config_unknown_and_gate_clear() {
        let g = gate(false, true, &[]);
        let c = config("UNKNOWN", 4, 0, 2);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(compute_overall(&g, &c, &axes), "UNKNOWN");
    }

    #[test]
    fn overall_partial_when_config_partial_and_no_unknown() {
        let g = gate(false, true, &[]);
        let c = config("PARTIAL", 5, 1, 0);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(compute_overall(&g, &c, &axes), "PARTIAL");
    }

    #[test]
    fn overall_admitted_when_all_clear() {
        let g = gate(false, true, &[]);
        let c = config("ADMITTED", 6, 0, 0);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(compute_overall(&g, &c, &axes), "ADMITTED");
    }

    #[test]
    fn refused_and_unknown_stay_distinct() {
        // Gate set with one surfaced code (refused) plus config unknown keys.
        let g = gate(true, true, &["GGEN-042"]);
        let c = config("UNKNOWN", 3, 1, 2);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(axes.refused, 1);
        assert_eq!(axes.unknown, 2);
        // The two counts are never folded together.
        assert_ne!(axes.refused, axes.unknown);
    }

    #[test]
    fn gate_set_without_codes_yields_unknown_axis_not_refused() {
        let g = gate(true, true, &[]);
        let c = config("ADMITTED", 6, 0, 0);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(axes.refused, 0);
        assert_eq!(axes.unknown, 1);
    }

    #[test]
    fn config_partial_counts_as_admitted_axis_not_unknown() {
        let g = gate(false, true, &[]);
        let c = config("PARTIAL", 4, 2, 0);
        let axes = InsightService::new().axis_counts(&g, &c);
        assert_eq!(axes.admitted, 6);
        assert_eq!(axes.unknown, 0);
    }

    #[test]
    fn digest_returns_bounded_overall() {
        let digest = InsightService::new().digest();
        let bounded = ["ADMITTED", "PARTIAL", "UNKNOWN", "BLOCKED"];
        assert!(
            bounded.contains(&digest.overall.as_str()),
            "overall '{}' is not bounded",
            digest.overall
        );
    }
}
