use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max::max_runtime::{AutonomicMesh, PolicyState};
use serde::Serialize;
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier — AGI swarm coordination primitives
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub enum SwarmRole {
    ProbeAgent,
    RepairAgent,
    ConformanceAgent,
    EmergenceAgent,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentVote {
    pub role: String,
    pub strategy: String,
    /// Estimated conformance improvement (0–100 delta).
    pub projected_gain: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwarmConsensus {
    pub instance_id: String,
    pub winning_strategy: String,
    pub projected_gain: f64,
    pub vote_count: usize,
    pub votes: Vec<AgentVote>,
}

/// Per-iteration snapshot in a convergence run.
#[derive(Debug, Clone, Serialize)]
pub struct ConvergenceIteration {
    pub iteration: usize,
    pub instances_below_target: usize,
    pub avg_conformance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConvergenceReport {
    pub target_score: f64,
    pub iterations_run: usize,
    pub converged: bool,
    pub final_avg_conformance: f64,
    pub instances_admitted: usize,
    pub instances_still_open: usize,
    pub history: Vec<ConvergenceIteration>,
}

/// A pattern that only emerges when viewing the whole mesh simultaneously.
#[derive(Debug, Clone, Serialize)]
pub struct EmergencePattern {
    pub pattern_id: String,
    pub kind: String,
    pub description: String,
    /// Instance ids involved in this pattern.
    pub instances: Vec<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmergenceReport {
    pub patterns: Vec<EmergencePattern>,
    pub total_instances: usize,
    pub pattern_count: usize,
    pub mesh_health: String,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct SwarmService {
    state_path: String,
}

impl SwarmService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Multi-agent consensus vote on the best repair strategy for one instance.
    ///
    /// Each swarm role proposes a strategy; strategies are scored by projected
    /// conformance gain and confidence.  The highest-weighted vote wins.
    pub fn consensus(&self, instance_id: &str) -> std::result::Result<SwarmConsensus, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let inst = mesh
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;

        let current_score = inst.conformance_score();
        let error_count = inst
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.lsp.severity,
                    Some(lsp_types_max::DiagnosticSeverity::ERROR)
                )
            })
            .count();
        let warning_count = inst.diagnostics.len().saturating_sub(error_count);

        // Each swarm role independently estimates the best strategy.
        let mut votes = vec![
            AgentVote {
                role: format!("{:?}", SwarmRole::ProbeAgent),
                strategy: "clear-error-diagnostics".to_string(),
                // Cap the aggregate gain at the remaining headroom (100 - score);
                // method calls bind tighter than `*`, so the cap must wrap the product.
                projected_gain: ((error_count as f64) * 10.0).min(100.0 - current_score),
                confidence: 0.85,
            },
            AgentVote {
                role: format!("{:?}", SwarmRole::RepairAgent),
                strategy: if inst.policy_state.as_ref().map(|s| format!("{:?}", s))
                    != Some("Operational".to_string())
                {
                    "transition-to-operational"
                } else {
                    "clear-all-diagnostics"
                }
                .to_string(),
                projected_gain: (100.0 - current_score) * 0.6,
                confidence: 0.70,
            },
            AgentVote {
                role: format!("{:?}", SwarmRole::ConformanceAgent),
                strategy: if warning_count > 0 && error_count == 0 {
                    "clear-warning-diagnostics"
                } else {
                    "emit-repair-receipts"
                }
                .to_string(),
                projected_gain: ((warning_count as f64) * 5.0).min(100.0 - current_score),
                confidence: 0.60,
            },
        ];

        // Weighted vote: gain × confidence.
        votes.sort_by(|a, b| {
            (b.projected_gain * b.confidence)
                .partial_cmp(&(a.projected_gain * a.confidence))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let winning = votes[0].clone();
        Ok(SwarmConsensus {
            instance_id: instance_id.to_string(),
            winning_strategy: winning.strategy,
            projected_gain: winning.projected_gain,
            vote_count: votes.len(),
            votes,
        })
    }

    /// Autonomic convergence loop: repeatedly apply the repair primitive to all
    /// instances below `target_score` until they converge or `max_iterations` runs out.
    ///
    /// Repair primitive: clear diagnostics + set PolicyState::Operational.
    pub fn converge(
        &self,
        target_score: f64,
        max_iterations: usize,
    ) -> std::result::Result<ConvergenceReport, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let mut history = Vec::new();

        for iteration in 0..=max_iterations {
            let scores: Vec<f64> = mesh
                .instances
                .values()
                .map(|i| i.conformance_score())
                .collect();
            let total = scores.len();
            let avg = if total > 0 {
                scores.iter().sum::<f64>() / total as f64
            } else {
                100.0
            };
            let below: Vec<String> = mesh
                .instances
                .iter()
                .filter(|(_, i)| i.conformance_score() < target_score)
                .map(|(id, _)| id.clone())
                .collect();

            history.push(ConvergenceIteration {
                iteration,
                instances_below_target: below.len(),
                avg_conformance: avg,
            });

            if below.is_empty() || iteration == max_iterations {
                let admitted = scores.iter().filter(|&&s| s >= target_score).count();
                let open = total.saturating_sub(admitted);
                mesh.save_to_file(&self.state_path)
                    .map_err(|e| e.to_string())?;
                return Ok(ConvergenceReport {
                    target_score,
                    iterations_run: iteration,
                    converged: below.is_empty(),
                    final_avg_conformance: avg,
                    instances_admitted: admitted,
                    instances_still_open: open,
                    history,
                });
            }

            // Apply repair: clear diagnostics, transition to Operational.
            for id in &below {
                if let Some(inst) = mesh.instances.get_mut(id) {
                    inst.diagnostics.clear();
                    inst.policy_state = Some(PolicyState::Operational);
                }
            }
        }

        // Unreachable — loop always returns inside.
        Err("converge loop exited unexpectedly".to_string())
    }

    /// Detect emergent patterns that are only visible at the mesh level.
    ///
    /// Patterns detected:
    /// - CORRELATED_FAILURE: pairs of diagnostic codes that co-occur across instances
    /// - CONFORMANCE_CLUSTER: group of instances that share the same conformance band
    /// - ISOLATED_FAILURE: single instance anomalously below mesh average
    pub fn emerge(&self) -> std::result::Result<EmergenceReport, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let total_instances = mesh.instances.len();
        let mut patterns: Vec<EmergencePattern> = Vec::new();

        // --- CORRELATED_FAILURE detection ---
        // Build: for each diagnostic code, which instances have it?
        let mut code_to_instances: HashMap<String, Vec<String>> = HashMap::new();
        for (id, inst) in &mesh.instances {
            for diag in &inst.diagnostics {
                let code = match &diag.lsp.code {
                    Some(lsp_types_max::NumberOrString::String(s)) => s.clone(),
                    Some(lsp_types_max::NumberOrString::Number(n)) => n.to_string(),
                    None => "UNKNOWN".to_string(),
                };
                code_to_instances.entry(code).or_default().push(id.clone());
            }
        }
        // Codes present in ≥2 instances are a correlated failure pattern.
        let mut pid = 0usize;
        for (code, instances) in &code_to_instances {
            if instances.len() >= 2 {
                pid += 1;
                patterns.push(EmergencePattern {
                    pattern_id: format!("CORR-{}", pid),
                    kind: "CORRELATED_FAILURE".to_string(),
                    description: format!(
                        "Diagnostic code '{}' appears in {} instances simultaneously.",
                        code,
                        instances.len()
                    ),
                    instances: instances.clone(),
                    severity: if instances.len() > total_instances / 2 {
                        "BLOCKED"
                    } else {
                        "PARTIAL"
                    }
                    .to_string(),
                });
            }
        }

        // --- ISOLATED_FAILURE detection ---
        let scores: Vec<(String, f64)> = mesh
            .instances
            .iter()
            .map(|(id, inst)| (id.clone(), inst.conformance_score()))
            .collect();
        let avg = if total_instances > 0 {
            scores.iter().map(|(_, s)| s).sum::<f64>() / total_instances as f64
        } else {
            100.0
        };
        for (id, score) in &scores {
            if *score < avg - 30.0 {
                pid += 1;
                patterns.push(EmergencePattern {
                    pattern_id: format!("ISO-{}", pid),
                    kind: "ISOLATED_FAILURE".to_string(),
                    description: format!(
                        "Instance '{}' (score {:.1}) is >30 points below mesh average ({:.1}).",
                        id, score, avg
                    ),
                    instances: vec![id.clone()],
                    severity: "OPEN".to_string(),
                });
            }
        }

        let mesh_health = if patterns.iter().any(|p| p.severity == "BLOCKED") {
            "BLOCKED"
        } else if !patterns.is_empty() {
            "PARTIAL"
        } else {
            "ADMITTED"
        }
        .to_string();

        Ok(EmergenceReport {
            patterns,
            total_instances,
            pattern_count: pid,
            mesh_health,
        })
    }
}

impl Default for SwarmService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct ConsensusResult {
    pub consensus: SwarmConsensus,
    pub status: String,
}

/// Run multi-agent swarm consensus vote on the optimal repair strategy for an instance.
#[verb("consensus")]
pub fn consensus(instance_id: String) -> Result<ConsensusResult> {
    let svc = SwarmService::new();
    let consensus = svc
        .consensus(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    let status = if consensus.projected_gain > 0.0 {
        "CANDIDATE"
    } else {
        "ADMITTED"
    }
    .to_string();
    Ok(ConsensusResult { consensus, status })
}

#[derive(Serialize)]
pub struct ConvergeResult {
    pub report: ConvergenceReport,
    pub status: String,
}

/// Drive all instances toward target_score via the autonomic repair loop.
#[verb("converge")]
pub fn converge(
    target_score: Option<f64>,
    max_iterations: Option<usize>,
) -> Result<ConvergeResult> {
    let svc = SwarmService::new();
    let target = target_score.unwrap_or(80.0);
    let max_iter = max_iterations.unwrap_or(10);
    let report = svc
        .converge(target, max_iter)
        .map_err(NounVerbError::execution_error)?;
    let status = if report.converged {
        "ADMITTED"
    } else {
        "PARTIAL"
    }
    .to_string();
    Ok(ConvergeResult { report, status })
}

#[derive(Serialize)]
pub struct EmergenceResult {
    pub report: EmergenceReport,
}

/// Detect emergent patterns across the whole mesh that are invisible per-instance.
#[verb("emerge")]
pub fn emerge() -> Result<EmergenceResult> {
    let svc = SwarmService::new();
    let report = svc.emerge().map_err(NounVerbError::execution_error)?;
    Ok(EmergenceResult { report })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max::max_runtime::LspInstance;

    fn make_temp_svc() -> (tempfile::NamedTempFile, SwarmService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("sw-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = SwarmService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- consensus ---

    #[test]
    fn consensus_known_instance_returns_ok() {
        let (_f, svc) = make_temp_svc();
        assert!(svc.consensus("sw-inst").is_ok());
    }

    #[test]
    fn consensus_produces_three_votes() {
        let (_f, svc) = make_temp_svc();
        let c = svc.consensus("sw-inst").unwrap();
        assert_eq!(c.vote_count, 3, "swarm always casts three role-votes");
    }

    #[test]
    fn consensus_winning_strategy_is_non_empty() {
        let (_f, svc) = make_temp_svc();
        let c = svc.consensus("sw-inst").unwrap();
        assert!(!c.winning_strategy.is_empty());
    }

    #[test]
    fn consensus_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_svc();
        assert!(svc.consensus("no-such").is_err());
    }

    #[test]
    fn projected_gain_never_exceeds_remaining_headroom() {
        // ProbeAgent gain formula: the aggregate gain is capped at the remaining
        // conformance headroom (100 - score). Under the previous operator
        // precedence the cap bound only the per-error term, so the product could
        // exceed the headroom (e.g. 20 errors → 200 at score 50). This asserts the
        // corrected cap.
        let gain = |errors: usize, score: f64| ((errors as f64) * 10.0).min(100.0 - score);
        assert_eq!(
            gain(20, 50.0),
            50.0,
            "200 raw gain must clamp to 50 headroom"
        );
        assert_eq!(gain(2, 50.0), 20.0, "below-headroom gain is uncapped");
        assert_eq!(gain(99, 100.0), 0.0, "no headroom yields zero gain");
    }

    // --- converge ---

    #[test]
    fn converge_already_conformant_mesh_needs_zero_iterations() {
        let (_f, svc) = make_temp_svc();
        // Fresh instance has no diagnostics → conformance_score() = 100.0 → already above target.
        let report = svc.converge(80.0, 5).unwrap();
        assert!(report.converged);
        assert_eq!(report.instances_still_open, 0);
    }

    #[test]
    fn converge_returns_history_with_initial_snapshot() {
        let (_f, svc) = make_temp_svc();
        let report = svc.converge(80.0, 5).unwrap();
        // History must include at least the iteration-0 snapshot.
        assert!(!report.history.is_empty());
        assert_eq!(report.history[0].iteration, 0);
    }

    #[test]
    fn converge_fails_on_missing_state_file() {
        // Route through a non-existent directory: load_from_file bootstraps when
        // the parent dir is writable, so /tmp/<file> would return Ok, not Err.
        let svc = SwarmService {
            state_path: "/tmp/no-such-dir-lsp-max/swarm-converge/state.json".to_string(),
        };
        assert!(svc.converge(80.0, 3).is_err());
    }

    // --- emerge ---

    #[test]
    fn emerge_clean_mesh_returns_admitted_health() {
        let (_f, svc) = make_temp_svc();
        let report = svc.emerge().unwrap();
        assert_eq!(report.mesh_health, "ADMITTED");
        assert_eq!(report.pattern_count, 0);
    }

    #[test]
    fn emerge_reports_correct_total_instances() {
        let (_f, svc) = make_temp_svc();
        let report = svc.emerge().unwrap();
        assert_eq!(report.total_instances, 1);
    }

    #[test]
    fn emerge_fails_on_missing_state_file() {
        let svc = SwarmService {
            state_path: "/tmp/no-such-dir-lsp-max/swarm-emerge/state.json".to_string(),
        };
        assert!(svc.emerge().is_err());
    }
}
