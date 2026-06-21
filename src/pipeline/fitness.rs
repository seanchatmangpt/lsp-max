use crate::pipeline::catalog::{breed_category, BreedCategory};
use std::collections::HashSet;

/// Fitness evaluator for breed pipelines against OCEL event logs.
///
/// Architecture note: wasm4pm-cognition is not a direct dependency of lsp-max.
/// We evaluate fitness by either:
/// (a) invoking wasm4pm-cli as a subprocess (ADMITTED path)
/// (b) computing a heuristic score from breed composition (UNKNOWN path when CLI absent)
pub trait BreedFitnessEvaluator: Send + Sync {
    /// Score a sequence of breed names in [0.0, 1.0].
    fn evaluate(&self, breeds: &[String]) -> f64;
}

/// Subprocess-based evaluator: calls wasm4pm-cli to run each breed.
/// Returns the average conformance score across all breed executions.
///
/// wasm4pm-cli interface assumed: `wasm4pm breed run <breed-name> --ocel <path> --score-only`
/// Returns JSON: `{"fitness": 0.85, "status": "ADMITTED"}` on stdout.
pub struct SubprocessFitnessEvaluator {
    /// Path to the OCEL event log file passed to the CLI.
    pub ocel_path: Option<String>,
    /// Path or name of the wasm4pm-cli binary to invoke.
    pub wasm4pm_cli: String,
}

impl SubprocessFitnessEvaluator {
    /// Construct a new evaluator, auto-detecting the wasm4pm-cli binary.
    pub fn new(ocel_path: Option<String>) -> Self {
        let cli = which_wasm4pm_cli();
        Self {
            ocel_path,
            wasm4pm_cli: cli,
        }
    }

    fn run_breed(&self, breed: &str) -> Option<f64> {
        let mut cmd = std::process::Command::new(&self.wasm4pm_cli);
        cmd.arg("breed").arg("run").arg(breed).arg("--score-only");
        if let Some(ref path) = self.ocel_path {
            cmd.arg("--ocel").arg(path);
        }
        let out = cmd.output().ok()?;
        if !out.status.success() {
            return None;
        }
        let stdout = std::str::from_utf8(&out.stdout).ok()?;
        let val: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
        val.get("fitness")?.as_f64()
    }
}

impl std::fmt::Debug for SubprocessFitnessEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubprocessFitnessEvaluator")
            .field("ocel_path", &self.ocel_path)
            .field("wasm4pm_cli", &self.wasm4pm_cli)
            .finish()
    }
}

impl BreedFitnessEvaluator for SubprocessFitnessEvaluator {
    fn evaluate(&self, breeds: &[String]) -> f64 {
        if breeds.is_empty() {
            return 0.0;
        }
        let scores: Vec<f64> = breeds.iter().filter_map(|b| self.run_breed(b)).collect();
        if scores.is_empty() {
            // CLI absent or breed not recognized — fall back to heuristic
            return HeuristicFitnessEvaluator.evaluate(breeds);
        }
        scores.iter().sum::<f64>() / scores.len() as f64
    }
}

/// Heuristic fitness: scores breed pipelines by composition quality.
/// Used when wasm4pm-cli is not available. Bounded to [0.0, 1.0].
///
/// Heuristic rationale: diverse breed categories + appropriate pipeline length
/// correlates with good process mining coverage.
#[derive(Debug)]
pub struct HeuristicFitnessEvaluator;

/// Map a breed name to its lowercase category tag for heuristic scoring.
///
/// Delegates to the authoritative [`breed_category`] catalog so the heuristic and
/// the catalog can never disagree. A prior substring-matching implementation
/// misclassified breeds whose names contained a foreign category token (e.g.
/// `markov_logic` matched `"logic"` and was scored as a logic breed though the
/// catalog classifies it as rule-based); routing through [`breed_category`]
/// removes that divergence.
fn category_for(breed: &str) -> &'static str {
    match breed_category(breed) {
        BreedCategory::LogicBased => "logic",
        BreedCategory::RuleBased => "rule",
        BreedCategory::PlanningBased => "planning",
        BreedCategory::Probabilistic => "probabilistic",
        BreedCategory::Temporal => "temporal",
        BreedCategory::MemoryBased => "memory",
        BreedCategory::MetaBased => "meta",
    }
}

impl BreedFitnessEvaluator for HeuristicFitnessEvaluator {
    fn evaluate(&self, breeds: &[String]) -> f64 {
        if breeds.is_empty() {
            return 0.0;
        }

        // Category diversity: distinct catalog categories present, normalized by
        // the 7 categories in `BreedCategory`. Every breed maps to exactly one
        // category via `category_for`, so the count needs no "unknown" filter.
        let categories: HashSet<&str> = breeds.iter().map(|b| category_for(b.as_str())).collect();
        let diversity = (categories.len() as f64 / 7.0_f64).min(1.0);

        // Length preference: 2–4 nodes is optimal for process mining pipelines
        let length_score = match breeds.len() {
            0 => 0.0,
            1 => 0.3,
            2..=4 => 1.0,
            n => (4.0 / n as f64).min(1.0),
        };

        // Temporal breed bonus: temporal breeds are valuable for process mining
        let has_temporal = breeds.iter().any(|b| category_for(b) == "temporal");
        let temporal_bonus = if has_temporal { 0.1 } else { 0.0 };

        (diversity * 0.5 + length_score * 0.4 + temporal_bonus).min(1.0)
    }
}

fn which_wasm4pm_cli() -> String {
    for candidate in &[
        "wasm4pm",
        "../wasm4pm/target/debug/wasm4pm-cli",
        "../wasm4pm/target/release/wasm4pm-cli",
    ] {
        if std::process::Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok()
        {
            return (*candidate).to_string();
        }
    }
    "wasm4pm".to_string()
}

/// Auto-select a fitness evaluator, in descending order of grounding:
///
/// 1. `wasm4pm-cli` present -> [`SubprocessFitnessEvaluator`] (engine-backed).
/// 2. else an OCEL log present and carrying process structure ->
///    [`LogGroundedFitnessEvaluator`], scoring breeds against the log's own
///    object-centric structure instead of ignoring the log.
/// 3. else -> [`HeuristicFitnessEvaluator`] (log-blind composition heuristic).
///
/// The log-grounded path is a structural proxy, not engine-backed alignment
/// conformance; the caller's status mapping keeps an unverifiable outcome
/// UNKNOWN rather than coercing it to ADMITTED.
pub fn auto_evaluator(ocel_path: Option<String>) -> Box<dyn BreedFitnessEvaluator> {
    let cli = which_wasm4pm_cli();
    if std::process::Command::new(&cli)
        .arg("--version")
        .output()
        .is_ok()
    {
        return Box::new(SubprocessFitnessEvaluator {
            ocel_path,
            wasm4pm_cli: cli,
        });
    }
    if let Some(path) = ocel_path.as_deref() {
        if let Some(log) = crate::pipeline::ocel::read_ocel_log(path) {
            if !log.events.is_empty() {
                return Box::new(crate::pipeline::ocel::LogGroundedFitnessEvaluator {
                    profile: crate::pipeline::ocel::LogProfile::from_log(&log),
                });
            }
        }
    }
    Box::new(HeuristicFitnessEvaluator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_empty_breeds_is_zero() {
        assert_eq!(HeuristicFitnessEvaluator.evaluate(&[]), 0.0);
    }

    #[test]
    fn heuristic_single_breed_is_low() {
        let score = HeuristicFitnessEvaluator.evaluate(&["cbr".to_string()]);
        assert!(
            score > 0.0 && score < 0.5,
            "single breed score {} should be low",
            score
        );
    }

    #[test]
    fn heuristic_diverse_pipeline_scores_higher_than_homogeneous() {
        let diverse = vec![
            "cbr".to_string(),
            "ltl_monitor".to_string(),
            "asp".to_string(),
        ];
        let homogeneous = vec!["cbr".to_string(), "cbr".to_string(), "cbr".to_string()];
        let diverse_score = HeuristicFitnessEvaluator.evaluate(&diverse);
        let homo_score = HeuristicFitnessEvaluator.evaluate(&homogeneous);
        assert!(
            diverse_score > homo_score,
            "diverse ({}) should score higher than homogeneous ({})",
            diverse_score,
            homo_score
        );
    }

    #[test]
    fn heuristic_temporal_breed_gets_bonus() {
        let with_temporal = vec!["cbr".to_string(), "ltl_monitor".to_string()];
        let without_temporal = vec!["cbr".to_string(), "production_rules".to_string()];
        let with_score = HeuristicFitnessEvaluator.evaluate(&with_temporal);
        let without_score = HeuristicFitnessEvaluator.evaluate(&without_temporal);
        assert!(
            with_score >= without_score,
            "temporal bonus should not reduce score ({} vs {})",
            with_score,
            without_score
        );
    }

    #[test]
    fn heuristic_optimal_length_scores_above_threshold() {
        let optimal = vec![
            "cbr".to_string(),
            "ltl_monitor".to_string(),
            "asp".to_string(),
        ];
        let score = HeuristicFitnessEvaluator.evaluate(&optimal);
        assert!(
            score >= 0.5,
            "optimal length pipeline should score >= 0.5, got {}",
            score
        );
    }

    #[test]
    fn category_for_known_breeds() {
        assert_eq!(category_for("ltl_monitor"), "temporal");
        assert_eq!(category_for("asp"), "logic");
        assert_eq!(category_for("cbr"), "rule");
        assert_eq!(category_for("bayesian_network"), "probabilistic");
        assert_eq!(category_for("frame"), "memory");
        assert_eq!(category_for("production_rules"), "rule");
    }

    #[test]
    fn category_for_markov_logic_is_rule_not_logic() {
        // Regression: substring matching misfiled `markov_logic` under "logic"
        // because the name contains the token. The catalog classifies it as
        // rule-based; `category_for` now delegates to the catalog and must agree.
        assert_eq!(category_for("markov_logic"), "rule");
    }
}
