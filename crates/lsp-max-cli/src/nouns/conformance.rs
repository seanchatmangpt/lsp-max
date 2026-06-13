use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_protocol::{ConformanceVector, LawAxis};
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Per-axis admitted/refused/unknown counts for a single LawAxis.
#[derive(Debug, Clone, Serialize)]
pub struct AxisBreakdown {
    pub axis: String,
    pub admitted: usize,
    pub refused: usize,
    pub unknown: usize,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct ConformanceService {
    state_path: String,
}

impl ConformanceService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Compute a ConformanceVector using LawAxis::all_named() for the instance.
    /// Axes with error-severity diagnostics are refused; axes with only
    /// non-error diagnostics are admitted; axes with no diagnostics are unknown.
    pub fn vector(&self, instance_id: &str) -> std::result::Result<ConformanceVector, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let inst = mesh
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;

        let mut has_error: std::collections::HashMap<LawAxis, bool> =
            std::collections::HashMap::new();
        let mut seen: std::collections::HashSet<LawAxis> = std::collections::HashSet::new();

        for diag in &inst.diagnostics {
            let is_error = matches!(
                diag.lsp.severity,
                Some(lsp_types_max::DiagnosticSeverity::ERROR)
            );
            let entry = has_error.entry(diag.law_axis.clone()).or_insert(false);
            if is_error {
                *entry = true;
            }
            seen.insert(diag.law_axis.clone());
        }

        let named = LawAxis::all_named();
        let mut admitted = Vec::new();
        let mut refused = Vec::new();
        let mut unknown = Vec::new();

        for axis in named {
            if let Some(&errored) = has_error.get(axis) {
                if errored {
                    refused.push(axis.clone());
                } else {
                    admitted.push(axis.clone());
                }
            } else {
                unknown.push(axis.clone());
            }
        }

        let total = admitted.len() + refused.len();
        let score = if total == 0 {
            None
        } else {
            Some(100.0 * admitted.len() as f64 / total as f64)
        };

        let mut cv = ConformanceVector {
            admitted,
            refused,
            unknown,
            score,
            strict_mode: true,
            ..Default::default()
        };
        cv.sync_bits_from_vecs();
        Ok(cv)
    }

    pub fn score(&self, instance_id: &str) -> std::result::Result<f64, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let inst = mesh
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        Ok(inst.conformance_score())
    }

    pub fn breakdown(&self, instance_id: &str) -> std::result::Result<Vec<AxisBreakdown>, String> {
        let vec = self.vector(instance_id)?;

        let named = LawAxis::all_named();
        let mut result = Vec::with_capacity(named.len());

        for axis in named {
            let admitted = vec.admitted.iter().filter(|a| *a == axis).count();
            let refused = vec.refused.iter().filter(|a| *a == axis).count();
            let unknown = vec.unknown.iter().filter(|a| *a == axis).count();
            result.push(AxisBreakdown {
                axis: format!("{:?}", axis),
                admitted,
                refused,
                unknown,
            });
        }

        Ok(result)
    }
}

impl Default for ConformanceService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct ConformanceScoreResult {
    pub instance_id: String,
    pub score: f64,
    pub conformance_vector: ConformanceVector,
}

#[verb("score")]
pub fn score(instance_id: String) -> Result<ConformanceScoreResult> {
    let svc = ConformanceService::new();
    let score = svc
        .score(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    let conformance_vector = svc
        .vector(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(ConformanceScoreResult {
        instance_id,
        score,
        conformance_vector,
    })
}

#[derive(Serialize)]
pub struct ConformanceBreakdownResult {
    pub instance_id: String,
    pub breakdown: Vec<AxisBreakdown>,
}

#[verb("breakdown")]
pub fn breakdown(instance_id: String) -> Result<ConformanceBreakdownResult> {
    let svc = ConformanceService::new();
    let breakdown = svc
        .breakdown(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(ConformanceBreakdownResult {
        instance_id,
        breakdown,
    })
}

/// Result type for the `vector` verb.
#[derive(Serialize)]
pub struct ConformanceVectorResult {
    pub admitted: Vec<String>,
    pub refused: Vec<String>,
    pub unknown: Vec<String>,
}

#[verb("vector")]
pub fn vector(instance_id: String) -> Result<ConformanceVectorResult> {
    let cv = ConformanceService::new()
        .vector(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(ConformanceVectorResult {
        admitted: cv.admitted.iter().map(|a| format!("{:?}", a)).collect(),
        refused: cv.refused.iter().map(|a| format!("{:?}", a)).collect(),
        unknown: cv.unknown.iter().map(|a| format!("{:?}", a)).collect(),
    })
}

/// Result type for the `vector-rpc` verb (dispatch_rpc-based).
#[derive(Serialize)]
pub struct ConformanceVectorRpcResult {
    pub instance_id: String,
    pub admitted: usize,
    pub refused: usize,
    pub unknown: usize,
    pub raw: serde_json::Value,
}

#[verb("vector-rpc")]
pub fn vector_rpc(instance_id: String) -> Result<ConformanceVectorRpcResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let response = mesh
        .dispatch_rpc(
            &instance_id,
            "max/conformanceVector",
            serde_json::Value::Null,
        )
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let admitted = response
        .get("admitted")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let refused = response
        .get("refused")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let unknown = response
        .get("unknown")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    Ok(ConformanceVectorRpcResult {
        instance_id,
        admitted,
        refused,
        unknown,
        raw: response,
    })
}

/// Result type for the  verb.
#[derive(Serialize)]
pub struct RunGateResult {
    pub instance_id: String,
    pub gate_id: String,
    pub passed: bool,
    pub raw: serde_json::Value,
}

#[verb("run-gate")]
pub fn run_gate(instance_id: String, gate_id: String) -> Result<RunGateResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let params = serde_json::json!({ "gate_id": gate_id });
    let response = mesh
        .dispatch_rpc(&instance_id, "max/runGate", params)
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let passed = response
        .as_bool()
        .or_else(|| response.get("passed").and_then(|v| v.as_bool()))
        .unwrap_or(false);
    Ok(RunGateResult {
        instance_id,
        gate_id,
        passed,
        raw: response,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_mesh() -> (tempfile::NamedTempFile, ConformanceService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("test-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ConformanceService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    #[test]
    fn score_known_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.score("test-inst");
        assert!(result.is_ok());
    }

    #[test]
    fn score_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.score("no-such").is_err());
    }

    #[test]
    fn breakdown_known_instance_returns_all_named_axes() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.breakdown("test-inst");
        assert!(result.is_ok());
        let axes = result.unwrap();
        assert_eq!(axes.len(), LawAxis::all_named().len());
    }

    #[test]
    fn breakdown_no_diagnostics_all_unknown() {
        let (_f, svc) = make_temp_mesh();
        let axes = svc.breakdown("test-inst").unwrap();
        for axis in &axes {
            assert_eq!(axis.admitted, 0);
            assert_eq!(axis.refused, 0);
            assert_eq!(axis.unknown, 1);
        }
    }
}

#[derive(Serialize)]
pub struct ConformanceDeltaResult {
    pub instance_id: String,
    pub raw: serde_json::Value,
}

#[verb("delta")]
pub fn delta(instance_id: String) -> Result<ConformanceDeltaResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(
            &instance_id,
            "max/conformanceDelta",
            serde_json::Value::Null,
        )
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(ConformanceDeltaResult { instance_id, raw })
}
