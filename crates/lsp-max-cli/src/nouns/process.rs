use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier — Van der Aalst process mining primitives
// ==============================================================================

/// A directed edge in the directly-follows graph.
#[derive(Debug, Clone, Serialize)]
pub struct DfgEdge {
    pub source: String,
    pub target: String,
    pub frequency: usize,
}

/// Directly-follows graph mined from the mesh event log.
#[derive(Debug, Clone, Serialize)]
pub struct DirectlyFollowsGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<DfgEdge>,
    pub start_activities: Vec<(String, usize)>,
    pub end_activities: Vec<(String, usize)>,
    pub case_count: usize,
    pub event_count: usize,
}

/// A process variant — a unique ordered sequence of activities observed in one or more cases.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessVariant {
    pub trace: Vec<String>,
    pub frequency: usize,
    pub cases: Vec<String>,
    /// Fraction of all cases that follow this variant.
    pub relative_frequency: f64,
}

/// Per-pair causal relation in Van der Aalst's causal footprint.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CausalRelation {
    /// A directly precedes B (A → B only).
    Causes,
    /// B directly precedes A (B → A only).
    CausedBy,
    /// Both A→B and B→A appear: parallel/concurrent.
    Parallel,
    /// Neither A→B nor B→A: mutually exclusive.
    Exclusive,
}

#[derive(Debug, Clone, Serialize)]
pub struct CausalEntry {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// Replay fitness score for a single case trace against the DFG model.
#[derive(Debug, Clone, Serialize)]
pub struct FitnessScore {
    pub instance_id: String,
    pub trace_length: usize,
    pub fitted_transitions: usize,
    pub total_transitions: usize,
    pub fitness: f64,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct ProcessMiningService {
    state_path: String,
}

impl ProcessMiningService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Deserialise the event log to JSON and extract all events.
    fn load_event_jsons(&self) -> std::result::Result<Vec<serde_json::Value>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        mesh.event_log
            .iter()
            .map(|ev| serde_json::to_value(ev).map_err(|e| e.to_string()))
            .collect()
    }

    /// Extract (activity_name, case_id) from a serialised HookEvent.
    /// HookEvent serialises as `{"VariantName": {"instance_id": "...", ...}}`.
    fn parse_event(event: &serde_json::Value) -> Option<(String, String)> {
        let obj = event.as_object()?;
        for (activity, payload) in obj {
            let instance_id = payload
                .as_object()
                .and_then(|p| p.get("instance_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(id) = instance_id {
                return Some((activity.clone(), id));
            }
        }
        None
    }

    /// Build ordered per-case activity traces from the event log.
    fn build_traces(
        events: &[serde_json::Value],
    ) -> HashMap<String, Vec<String>> {
        let mut traces: HashMap<String, Vec<String>> = HashMap::new();
        for ev in events {
            if let Some((activity, case_id)) = Self::parse_event(ev) {
                traces.entry(case_id).or_default().push(activity);
            }
        }
        traces
    }

    /// Mine a directly-follows graph from the mesh event log.
    pub fn dfg(&self) -> std::result::Result<DirectlyFollowsGraph, String> {
        let events = self.load_event_jsons()?;
        let traces = Self::build_traces(&events);

        let mut node_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut edge_map: HashMap<(String, String), usize> = HashMap::new();
        let mut start_map: HashMap<String, usize> = HashMap::new();
        let mut end_map: HashMap<String, usize> = HashMap::new();
        let event_count = events.len();
        let case_count = traces.len();

        for trace in traces.values() {
            for a in trace {
                node_set.insert(a.clone());
            }
            if let Some(first) = trace.first() {
                *start_map.entry(first.clone()).or_insert(0) += 1;
            }
            if let Some(last) = trace.last() {
                *end_map.entry(last.clone()).or_insert(0) += 1;
            }
            for window in trace.windows(2) {
                *edge_map
                    .entry((window[0].clone(), window[1].clone()))
                    .or_insert(0) += 1;
            }
        }

        let mut nodes: Vec<String> = node_set.into_iter().collect();
        nodes.sort();

        let mut edges: Vec<DfgEdge> = edge_map
            .into_iter()
            .map(|((source, target), frequency)| DfgEdge { source, target, frequency })
            .collect();
        edges.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.source.cmp(&b.source)));

        let mut start_activities: Vec<(String, usize)> = start_map.into_iter().collect();
        start_activities.sort_by(|a, b| b.1.cmp(&a.1));
        let mut end_activities: Vec<(String, usize)> = end_map.into_iter().collect();
        end_activities.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(DirectlyFollowsGraph {
            nodes,
            edges,
            start_activities,
            end_activities,
            case_count,
            event_count,
        })
    }

    /// Extract process variants sorted by descending frequency.
    pub fn variants(
        &self,
    ) -> std::result::Result<Vec<ProcessVariant>, String> {
        let events = self.load_event_jsons()?;
        let traces = Self::build_traces(&events);
        let total_cases = traces.len();

        let mut variant_map: HashMap<Vec<String>, (usize, Vec<String>)> = HashMap::new();
        for (case_id, trace) in &traces {
            let entry = variant_map.entry(trace.clone()).or_insert((0, vec![]));
            entry.0 += 1;
            entry.1.push(case_id.clone());
        }

        let mut variants: Vec<ProcessVariant> = variant_map
            .into_iter()
            .map(|(trace, (frequency, cases))| ProcessVariant {
                relative_frequency: if total_cases > 0 {
                    frequency as f64 / total_cases as f64
                } else {
                    0.0
                },
                trace,
                frequency,
                cases,
            })
            .collect();
        variants.sort_by(|a, b| {
            b.frequency
                .cmp(&a.frequency)
                .then(a.trace.first().unwrap_or(&String::new()).cmp(b.trace.first().unwrap_or(&String::new())))
        });
        Ok(variants)
    }

    /// Replay fitness of a single instance trace against the DFG.
    /// Returns 1.0 for traces with fewer than 2 events (vacuously fit).
    pub fn fitness(
        &self,
        instance_id: &str,
    ) -> std::result::Result<FitnessScore, String> {
        let events = self.load_event_jsons()?;
        let traces = Self::build_traces(&events);
        let dfg = self.dfg()?;

        let edge_set: std::collections::HashSet<(String, String)> = dfg
            .edges
            .iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();

        let trace = traces.get(instance_id).cloned().unwrap_or_default();
        let trace_length = trace.len();
        let total_transitions = trace_length.saturating_sub(1);

        let fitted_transitions = if total_transitions == 0 {
            0
        } else {
            trace
                .windows(2)
                .filter(|w| edge_set.contains(&(w[0].clone(), w[1].clone())))
                .count()
        };

        let fitness = if total_transitions == 0 {
            1.0
        } else {
            fitted_transitions as f64 / total_transitions as f64
        };

        Ok(FitnessScore {
            instance_id: instance_id.to_string(),
            trace_length,
            fitted_transitions,
            total_transitions,
            fitness,
        })
    }

    /// Build Van der Aalst's causal footprint for all activity pairs in the log.
    pub fn causal(&self) -> std::result::Result<Vec<CausalEntry>, String> {
        let events = self.load_event_jsons()?;
        let traces = Self::build_traces(&events);

        // Collect all directly-follows pairs in both directions.
        let mut ab: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        let mut activities: std::collections::HashSet<String> = std::collections::HashSet::new();

        for trace in traces.values() {
            for a in trace {
                activities.insert(a.clone());
            }
            for w in trace.windows(2) {
                ab.insert((w[0].clone(), w[1].clone()));
            }
        }

        let mut result = Vec::new();
        let mut act_list: Vec<String> = activities.into_iter().collect();
        act_list.sort();

        for a in &act_list {
            for b in &act_list {
                if a == b {
                    continue;
                }
                let a_to_b = ab.contains(&(a.clone(), b.clone()));
                let b_to_a = ab.contains(&(b.clone(), a.clone()));
                let relation = match (a_to_b, b_to_a) {
                    (true, false) => CausalRelation::Causes,
                    (false, true) => CausalRelation::CausedBy,
                    (true, true) => CausalRelation::Parallel,
                    (false, false) => CausalRelation::Exclusive,
                };
                result.push(CausalEntry {
                    from: a.clone(),
                    to: b.clone(),
                    relation: format!("{:?}", relation),
                });
            }
        }
        Ok(result)
    }
}

impl Default for ProcessMiningService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct DfgResult {
    pub graph: DirectlyFollowsGraph,
    pub status: String,
}

/// Mine a directly-follows graph (DFG) from the mesh event log.
#[verb("dfg")]
pub fn dfg() -> Result<DfgResult> {
    let svc = ProcessMiningService::new();
    let graph = svc.dfg().map_err(NounVerbError::execution_error)?;
    let status = if graph.case_count == 0 {
        "UNKNOWN".to_string()
    } else {
        "ADMITTED".to_string()
    };
    Ok(DfgResult { graph, status })
}

#[derive(Serialize)]
pub struct VariantsResult {
    pub variants: Vec<ProcessVariant>,
    pub total_variants: usize,
    pub total_cases: usize,
}

/// Extract all unique process variants (trace sequences) from the event log.
#[verb("variants")]
pub fn variants() -> Result<VariantsResult> {
    let svc = ProcessMiningService::new();
    let variants = svc.variants().map_err(NounVerbError::execution_error)?;
    let total_cases = variants.iter().map(|v| v.frequency).sum();
    let total_variants = variants.len();
    Ok(VariantsResult { variants, total_variants, total_cases })
}

#[derive(Serialize)]
pub struct FitnessResult {
    pub score: FitnessScore,
    pub status: String,
}

/// Compute the replay fitness of one instance's trace against the mined DFG model.
#[verb("fitness")]
pub fn fitness(instance_id: String) -> Result<FitnessResult> {
    let svc = ProcessMiningService::new();
    let score = svc
        .fitness(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    let status = if score.fitness >= 0.8 {
        "ADMITTED".to_string()
    } else if score.fitness >= 0.5 {
        "PARTIAL".to_string()
    } else {
        "REFUSED".to_string()
    };
    Ok(FitnessResult { score, status })
}

#[derive(Serialize)]
pub struct CausalResult {
    pub footprint: Vec<CausalEntry>,
    pub activity_count: usize,
    pub pair_count: usize,
}

/// Compute Van der Aalst's causal footprint matrix for all activity pairs in the log.
#[verb("causal")]
pub fn causal() -> Result<CausalResult> {
    let svc = ProcessMiningService::new();
    let footprint = svc.causal().map_err(NounVerbError::execution_error)?;
    let pair_count = footprint.len();
    let activity_count = {
        let mut acts: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in &footprint {
            acts.insert(e.from.clone());
            acts.insert(e.to.clone());
        }
        acts.len()
    };
    Ok(CausalResult { footprint, activity_count, pair_count })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_svc() -> (tempfile::NamedTempFile, ProcessMiningService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("case-1"));
        mesh.add_instance(LspInstance::new("case-2"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ProcessMiningService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- dfg ---

    #[test]
    fn dfg_on_empty_event_log_returns_ok_with_no_edges() {
        let (_f, svc) = make_temp_svc();
        let graph = svc.dfg().unwrap();
        assert!(graph.edges.is_empty(), "fresh mesh has no events → no DFG edges");
        assert_eq!(graph.event_count, 0);
    }

    #[test]
    fn dfg_fails_on_missing_state_file() {
        let svc = ProcessMiningService {
            state_path: "/tmp/no-such-dir-lsp-max/process/state.json".to_string(),
        };
        assert!(svc.dfg().is_err());
    }

    // --- variants ---

    #[test]
    fn variants_empty_log_returns_ok_with_no_variants() {
        let (_f, svc) = make_temp_svc();
        let vars = svc.variants().unwrap();
        assert!(vars.is_empty(), "no events → no variants");
    }

    #[test]
    fn variants_relative_frequency_sums_to_one() {
        let (_f, svc) = make_temp_svc();
        let vars = svc.variants().unwrap();
        if !vars.is_empty() {
            let total: f64 = vars.iter().map(|v| v.relative_frequency).sum();
            assert!((total - 1.0).abs() < 1e-9, "relative frequencies must sum to 1.0");
        }
    }

    // --- fitness ---

    #[test]
    fn fitness_unknown_instance_returns_1_0_vacuously() {
        let (_f, svc) = make_temp_svc();
        let score = svc.fitness("no-events-inst").unwrap();
        assert_eq!(score.trace_length, 0);
        assert_eq!(score.total_transitions, 0);
        assert_eq!(score.fitness, 1.0, "empty trace is vacuously fit");
    }

    #[test]
    fn fitness_fails_on_missing_state_file() {
        let svc = ProcessMiningService {
            state_path: "/tmp/no-such-dir-lsp-max/fitness/state.json".to_string(),
        };
        assert!(svc.fitness("any-inst").is_err());
    }

    // --- causal ---

    #[test]
    fn causal_empty_log_returns_ok_with_no_entries() {
        let (_f, svc) = make_temp_svc();
        let footprint = svc.causal().unwrap();
        assert!(footprint.is_empty(), "no events → no causal pairs");
    }

    #[test]
    fn causal_relation_causes_and_exclusive_logic() {
        type Pairs = std::collections::HashSet<(String, String)>;
        let classify = |df: &Pairs, a: &str, b: &str| {
            match (df.contains(&(a.to_string(), b.to_string())), df.contains(&(b.to_string(), a.to_string()))) {
                (true, false) => CausalRelation::Causes,
                (false, true) => CausalRelation::CausedBy,
                (true, true) => CausalRelation::Parallel,
                (false, false) => CausalRelation::Exclusive,
            }
        };
        let mut df: Pairs = Pairs::new();
        df.insert(("A".to_string(), "B".to_string()));
        assert_eq!(classify(&df, "A", "B"), CausalRelation::Causes);
        assert_eq!(classify(&Pairs::new(), "A", "B"), CausalRelation::Exclusive);
    }
}
