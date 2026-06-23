//! Object-centric process-mining grounding for the breed-pipeline optimizer.
//!
//! The scalar and Pareto searches score breed pipelines by *composition* alone.
//! This module lets fitness be grounded in the **event log itself**: it reads an
//! OCEL 2.0 object-centric log, derives an object-centric directly-follows graph
//! (OC-DFG), and projects the log onto a small vector of bounded structural
//! signals — including the object-centric **convergence** and **divergence**
//! notions from van der Aalst's OCPM framework. A [`LogProfile`] then scores how
//! well a breed pipeline's cognitive-category coverage meets the *demands* of
//! that specific log.
//!
//! Honesty boundary (three-state law): every signal here is a **structural
//! proxy** computed from the log's shape. It is NOT engine-backed alignment
//! conformance (replay fitness / precision), which requires the wasm4pm engine.
//! Where the log is absent or unparseable, [`read_ocel_log`] returns `None` and
//! the caller stays on a lower-grounding evaluator; the proxy never presents
//! itself as an admitted conformance verdict.

use crate::pipeline::catalog::{breed_category, BreedCategory};
use crate::pipeline::fitness::BreedFitnessEvaluator;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// Distinct-activity normalizer for [`LogProfile::activity_variety`]. A bounded
/// cap, not a hard limit: logs with more activities simply saturate the signal.
const ACTIVITY_CAP: f64 = 12.0;

/// Distinct-object-type normalizer for [`LogProfile::object_type_spread`].
const OBJECT_TYPE_CAP: f64 = 6.0;

/// The number of [`BreedCategory`] variants, used to normalize category coverage.
const CATEGORY_COUNT: f64 = 7.0;

// ---------------------------------------------------------------------------
// OCEL 2.0 reader (minimal — only the fields process structure depends on)
// ---------------------------------------------------------------------------

/// One `event.relationship` / `object.relationship` entry: a qualified edge to
/// an object id.
#[derive(Debug, Clone, Deserialize)]
pub struct OcelRelationship {
    /// The referenced object's id.
    #[serde(rename = "objectId")]
    pub object_id: String,
    /// Qualifier describing the role of the reference (unused by the metrics).
    #[serde(default)]
    pub qualifier: String,
}

/// An OCEL 2.0 event: an activity occurrence at a time, related to objects.
#[derive(Debug, Clone, Deserialize)]
pub struct OcelEvent {
    /// Event id (unique within the log).
    #[serde(default)]
    pub id: String,
    /// The activity name (OCEL `type`).
    #[serde(rename = "type", default)]
    pub activity: String,
    /// ISO-8601 timestamp; lexicographic order is chronological for UTC `Z`.
    #[serde(default)]
    pub time: String,
    /// Objects this event is related to.
    #[serde(default)]
    pub relationships: Vec<OcelRelationship>,
}

/// An OCEL 2.0 object: a typed entity with object-to-object relationships.
#[derive(Debug, Clone, Deserialize)]
pub struct OcelObject {
    /// Object id (unique within the log).
    #[serde(default)]
    pub id: String,
    /// The object type name (OCEL `type`).
    #[serde(rename = "type", default)]
    pub object_type: String,
    /// Object-to-object relationships (unused by the current metrics).
    #[serde(default)]
    pub relationships: Vec<OcelRelationship>,
}

/// A minimal OCEL 2.0 log: the events and objects the metrics read.
#[derive(Debug, Clone, Deserialize)]
pub struct OcelLog {
    /// All events in the log.
    #[serde(default)]
    pub events: Vec<OcelEvent>,
    /// All objects in the log.
    #[serde(default)]
    pub objects: Vec<OcelObject>,
}

/// Parse an OCEL 2.0 log from JSON text.
///
/// Returns `None` when the text is not valid OCEL JSON. An otherwise-valid log
/// with zero events parses to `Some` with an empty `events` vector; callers that
/// require process structure should check `events.is_empty()`.
pub fn parse_ocel_log(text: &str) -> Option<OcelLog> {
    serde_json::from_str::<OcelLog>(text).ok()
}

/// Read and parse an OCEL 2.0 log from a filesystem path.
///
/// Returns `None` when the file is absent, unreadable, or not valid OCEL JSON —
/// the negative control: an absent observation source is never fabricated into a
/// log.
pub fn read_ocel_log(path: &str) -> Option<OcelLog> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_ocel_log(&text)
}

// ---------------------------------------------------------------------------
// Object-centric structural profile (van der Aalst OCPM signals)
// ---------------------------------------------------------------------------

/// A bounded structural fingerprint of an OCEL log. Every field is in `[0.0, 1.0]`
/// where higher means "more of that structural property is present". The fields
/// name the object-centric process-mining notions they proxy.
#[derive(Debug, Clone, PartialEq)]
pub struct LogProfile {
    /// Distinct activities present, normalized by [`ACTIVITY_CAP`].
    pub activity_variety: f64,
    /// Distinct object types present, normalized by [`OBJECT_TYPE_CAP`].
    pub object_type_spread: f64,
    /// Fraction of object traces with at least two events — the demand for
    /// sequential / temporal reasoning over an object's lifecycle.
    pub temporal_density: f64,
    /// Object-centric **divergence**: fraction of object traces in which some
    /// activity occurs more than once (a per-object repeated activity).
    pub divergence: f64,
    /// Object-centric **convergence**: fraction of events related to two or more
    /// objects of the *same* object type (one event over many like objects).
    pub convergence: f64,
    /// Distinct directly-follows pairs over the OC-DFG, normalized by the square
    /// of the activity count — a footprint-style relational density.
    pub df_density: f64,
}

impl LogProfile {
    /// The all-zero profile, used for an empty log (no observable structure).
    pub fn empty() -> Self {
        Self {
            activity_variety: 0.0,
            object_type_spread: 0.0,
            temporal_density: 0.0,
            divergence: 0.0,
            convergence: 0.0,
            df_density: 0.0,
        }
    }

    /// Derive the structural profile from an OCEL log by building the OC-DFG.
    ///
    /// Each object's events are ordered by timestamp to form its trace; directly
    /// follows pairs, per-object repeated activities (divergence), and
    /// same-type event sharing (convergence) are read off those traces.
    pub fn from_log(log: &OcelLog) -> Self {
        if log.events.is_empty() {
            return Self::empty();
        }

        let obj_type: HashMap<&str, &str> = log
            .objects
            .iter()
            .map(|o| (o.id.as_str(), o.object_type.as_str()))
            .collect();

        let activities: HashSet<&str> = log.events.iter().map(|e| e.activity.as_str()).collect();

        // Object type spread draws on declared objects and, defensively, on any
        // type reachable through event relationships.
        let mut object_types: HashSet<&str> =
            log.objects.iter().map(|o| o.object_type.as_str()).collect();

        // Per-object ordered traces of (time, activity), plus convergence count.
        let mut traces: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
        let mut convergent_events = 0usize;
        for ev in &log.events {
            let mut per_type: HashMap<&str, usize> = HashMap::new();
            for rel in &ev.relationships {
                let oid = rel.object_id.as_str();
                traces
                    .entry(oid)
                    .or_default()
                    .push((ev.time.as_str(), ev.activity.as_str()));
                if let Some(t) = obj_type.get(oid) {
                    object_types.insert(t);
                    *per_type.entry(*t).or_default() += 1;
                }
            }
            if per_type.values().any(|&c| c >= 2) {
                convergent_events += 1;
            }
        }

        let mut df_pairs: HashSet<(&str, &str)> = HashSet::new();
        let mut traces_multi = 0usize;
        let mut traces_divergent = 0usize;
        let total_traces = traces.len().max(1);
        for trace in traces.values_mut() {
            trace.sort_by(|a, b| a.0.cmp(b.0));
            if trace.len() >= 2 {
                traces_multi += 1;
            }
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for (_, act) in trace.iter() {
                *counts.entry(*act).or_default() += 1;
            }
            if counts.values().any(|&c| c >= 2) {
                traces_divergent += 1;
            }
            for win in trace.windows(2) {
                df_pairs.insert((win[0].1, win[1].1));
            }
        }

        let act_count = activities.len().max(1) as f64;
        let df_density = (df_pairs.len() as f64 / (act_count * act_count)).clamp(0.0, 1.0);

        Self {
            activity_variety: (activities.len() as f64 / ACTIVITY_CAP).clamp(0.0, 1.0),
            object_type_spread: (object_types.len() as f64 / OBJECT_TYPE_CAP).clamp(0.0, 1.0),
            temporal_density: (traces_multi as f64 / total_traces as f64).clamp(0.0, 1.0),
            divergence: (traces_divergent as f64 / total_traces as f64).clamp(0.0, 1.0),
            convergence: (convergent_events as f64 / log.events.len() as f64).clamp(0.0, 1.0),
            df_density,
        }
    }

    /// Distinct breed categories present in `breeds`, in `[0.0, 1.0]`.
    fn coverage(breeds: &[String]) -> (f64, bool) {
        let mut cats: Vec<BreedCategory> = Vec::new();
        for b in breeds {
            let c = breed_category(b);
            if !cats.contains(&c) {
                cats.push(c);
            }
        }
        let has_temporal = cats.contains(&BreedCategory::Temporal);
        (
            (cats.len() as f64 / CATEGORY_COUNT).clamp(0.0, 1.0),
            has_temporal,
        )
    }

    /// Score, in `[0.0, 1.0]`, how well a breed pipeline meets this log's demands.
    ///
    /// The score rewards a pipeline whose cognitive coverage matches what the log
    /// structurally requires: a Temporal breed when the log is sequential or
    /// divergent, and broad category coverage when the log is object-centrically
    /// complex (spread / convergence / dense directly-follows). An empty pipeline
    /// scores `0.0`.
    pub fn demand_match(&self, breeds: &[String]) -> f64 {
        if breeds.is_empty() {
            return 0.0;
        }
        let (coverage, has_temporal) = Self::coverage(breeds);

        let temporal_demand = ((self.temporal_density + self.divergence) / 2.0).clamp(0.0, 1.0);
        let breadth_demand =
            ((self.object_type_spread + self.convergence + self.df_density) / 3.0).clamp(0.0, 1.0);
        let variety_demand = self.activity_variety;

        // Meeting a demand scores 1.0; missing it costs the shortfall only.
        let temporal_term = if has_temporal {
            1.0
        } else {
            1.0 - temporal_demand
        };
        let breadth_term = 1.0 - (breadth_demand - coverage).max(0.0);
        let variety_term = 1.0 - (variety_demand - coverage).max(0.0);

        (0.4 * temporal_term + 0.4 * breadth_term + 0.2 * variety_term).clamp(0.0, 1.0)
    }
}

/// A [`BreedFitnessEvaluator`] that scores breeds against a fixed [`LogProfile`].
///
/// Built by `auto_evaluator` when an OCEL log is present but the wasm4pm engine
/// is not: it grounds fitness in the log's object-centric structure rather than
/// ignoring the log. It is a structural proxy, never an admitted verdict.
#[derive(Debug)]
pub struct LogGroundedFitnessEvaluator {
    /// The structural profile the evaluator scores against.
    pub profile: LogProfile,
}

impl BreedFitnessEvaluator for LogGroundedFitnessEvaluator {
    fn evaluate(&self, breeds: &[String]) -> f64 {
        self.profile.demand_match(breeds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A compact OCEL 2.0 log mirroring tests/fixtures/tpot2/sample.ocel.json:
    // order o1 is picked twice (divergence) and runs place->pick->pick->deliver.
    const SAMPLE: &str = r#"{
      "events": [
        {"id":"e1","type":"place order","time":"2026-06-01T08:00:00Z","relationships":[{"objectId":"o1","qualifier":"order"},{"objectId":"c1","qualifier":"customer"}]},
        {"id":"e2","type":"pick item","time":"2026-06-01T09:15:00Z","relationships":[{"objectId":"o1","qualifier":"order"},{"objectId":"i1","qualifier":"item"}]},
        {"id":"e3","type":"pick item","time":"2026-06-01T09:42:00Z","relationships":[{"objectId":"o1","qualifier":"order"},{"objectId":"i2","qualifier":"item"}]},
        {"id":"e4","type":"deliver order","time":"2026-06-02T11:30:00Z","relationships":[{"objectId":"o1","qualifier":"order"},{"objectId":"c1","qualifier":"customer"}]}
      ],
      "objects": [
        {"id":"o1","type":"Order"},
        {"id":"i1","type":"Item"},
        {"id":"i2","type":"Item"},
        {"id":"c1","type":"Customer"}
      ]
    }"#;

    fn sample() -> OcelLog {
        parse_ocel_log(SAMPLE).expect("sample OCEL must parse")
    }

    #[test]
    fn parses_events_and_objects() {
        let log = sample();
        assert_eq!(log.events.len(), 4);
        assert_eq!(log.objects.len(), 4);
        assert_eq!(log.events[0].activity, "place order");
    }

    // NEGATIVE CONTROL: garbage and absent sources must not fabricate a log.
    #[test]
    fn garbage_and_absent_sources_are_none() {
        assert!(parse_ocel_log("not json at all").is_none());
        assert!(read_ocel_log("/no/such/path/at/all.ocel.json").is_none());
    }

    #[test]
    fn empty_log_yields_zero_profile() {
        let log = parse_ocel_log(r#"{"events":[],"objects":[]}"#).unwrap();
        assert_eq!(LogProfile::from_log(&log), LogProfile::empty());
    }

    #[test]
    fn profile_fields_are_bounded() {
        let p = LogProfile::from_log(&sample());
        for v in [
            p.activity_variety,
            p.object_type_spread,
            p.temporal_density,
            p.divergence,
            p.convergence,
            p.df_density,
        ] {
            assert!((0.0..=1.0).contains(&v), "signal {v} out of [0,1]");
        }
    }

    #[test]
    fn divergence_detects_repeated_activity_per_object() {
        // Order o1 has two "pick item" events -> one of four object traces is
        // divergent (o1); i1, i2, c1 are not.
        let p = LogProfile::from_log(&sample());
        assert!(
            p.divergence > 0.0,
            "repeated per-object activity must register divergence"
        );
        assert!(
            p.temporal_density > 0.0,
            "multi-event traces must register temporal density"
        );
    }

    #[test]
    fn temporal_breed_helps_when_log_is_temporal() {
        // Same coverage size, one with a Temporal breed and one without; the
        // temporal pipeline must score at least as high on a temporal log.
        let p = LogProfile::from_log(&sample());
        let without = vec!["asp".to_string(), "cbr".to_string()];
        let with = vec!["asp".to_string(), "ltl_monitor".to_string()];
        assert!(
            p.demand_match(&with) > p.demand_match(&without),
            "a Temporal breed must raise fitness on a temporally-demanding log"
        );
    }

    #[test]
    fn broader_coverage_helps_on_complex_log() {
        let p = LogProfile::from_log(&sample());
        let narrow = vec!["asp".to_string()];
        let broad = vec![
            "asp".to_string(),
            "cbr".to_string(),
            "bayesian_network".to_string(),
            "strips".to_string(),
        ];
        assert!(
            p.demand_match(&broad) >= p.demand_match(&narrow),
            "broader category coverage must not reduce fitness on a complex log"
        );
    }

    #[test]
    fn empty_pipeline_scores_zero() {
        let p = LogProfile::from_log(&sample());
        assert_eq!(p.demand_match(&[]), 0.0);
    }

    #[test]
    fn read_round_trips_through_a_temp_file() {
        let mut path = std::env::temp_dir();
        path.push(format!("tpot2-ocel-{}.json", std::process::id()));
        std::fs::write(&path, SAMPLE).unwrap();
        let log = read_ocel_log(path.to_str().unwrap()).expect("temp OCEL must read");
        assert_eq!(log.events.len(), 4);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn evaluator_matches_profile_demand() {
        let profile = LogProfile::from_log(&sample());
        let breeds = vec!["asp".to_string(), "ltl_monitor".to_string()];
        let evaluator = LogGroundedFitnessEvaluator {
            profile: profile.clone(),
        };
        assert_eq!(evaluator.evaluate(&breeds), profile.demand_match(&breeds));
    }
}
