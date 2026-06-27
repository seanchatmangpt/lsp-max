use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max::max_runtime::{AutonomicMesh, Receipt};
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// Receipt is re-exported from lsp_max_runtime and derives Serialize.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for querying instance receipts.
pub struct ReceiptService {
    state_path: String,
}

impl ReceiptService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn list(&self, instance_id: &str) -> std::result::Result<Vec<Receipt>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let inst = mesh
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        Ok(inst.receipts.clone())
    }

    pub fn verify(&self, instance_id: &str) -> std::result::Result<(usize, bool), String> {
        let receipts = self.list(instance_id)?;
        let count = receipts.len();
        let chain_valid = !receipts.is_empty()
            && receipts
                .iter()
                .all(|r| !r.receipt_id.is_empty() && !r.hash.is_empty());
        Ok((count, chain_valid))
    }
}

impl Default for ReceiptService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct ReceiptListResult {
    pub receipts: Vec<Receipt>,
    pub count: usize,
}

/// List all receipts for the given instance.
#[verb("list")]
pub fn list(instance_id: String) -> Result<ReceiptListResult> {
    let service = ReceiptService::new();
    let receipts = service
        .list(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    let count = receipts.len();
    Ok(ReceiptListResult { receipts, count })
}

#[derive(Serialize)]
pub struct ReceiptVerifyResult {
    pub count: usize,
    pub chain_valid: bool,
}

/// Verify the receipt chain for an instance (all receipts have non-empty ids and hashes).
#[verb("verify")]
pub fn verify(instance_id: String) -> Result<ReceiptVerifyResult> {
    let service = ReceiptService::new();
    let (count, chain_valid) = service
        .verify(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(ReceiptVerifyResult { count, chain_valid })
}

#[derive(Serialize)]
pub struct VerifyLedgerResult {
    pub instance_id: String,
    pub raw: serde_json::Value,
}

/// Verify the receipt ledger for an instance via the `max/verifyLedger` RPC.
#[verb("verify-ledger")]
pub fn verify_ledger(instance_id: String) -> Result<VerifyLedgerResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/verifyLedger", serde_json::Value::Null)
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(VerifyLedgerResult { instance_id, raw })
}

#[derive(Serialize)]
pub struct LedgerReportResult {
    pub instance_id: String,
    pub raw: serde_json::Value,
}

/// Generate a ledger report for an instance via the `max/ledgerReport` RPC.
#[verb("ledger-report")]
pub fn ledger_report(instance_id: String) -> Result<LedgerReportResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/ledgerReport", serde_json::Value::Null)
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(LedgerReportResult { instance_id, raw })
}

#[derive(Serialize)]
pub struct ReceiptWalkEntry {
    pub index: usize,
    pub receipt_id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Serialize)]
pub struct ReceiptWalkResult {
    pub instance_id: String,
    pub overall: String,
    pub entries: Vec<ReceiptWalkEntry>,
    pub total: usize,
}

/// Walk the receipt chain for an instance, reporting per-receipt admission status.
#[verb("walk")]
pub fn walk(instance_id: String) -> Result<ReceiptWalkResult> {
    let service = ReceiptService::new();
    let receipts = service
        .list(&instance_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    let total = receipts.len();
    let mut entries = Vec::new();
    let mut overall = if total == 0 { "UNKNOWN" } else { "ADMITTED" }.to_string();

    for (idx, r) in receipts.iter().enumerate() {
        let (status, detail) = if r.receipt_id.is_empty() || r.hash.is_empty() {
            overall = "REFUSED".to_string();
            (
                "REFUSED".to_string(),
                format!("empty receipt_id or hash at index {}", idx),
            )
        } else {
            (
                "ADMITTED".to_string(),
                format!(
                    "id={} hash={}...",
                    r.receipt_id,
                    &r.hash[..8.min(r.hash.len())]
                ),
            )
        };
        entries.push(ReceiptWalkEntry {
            index: idx,
            receipt_id: r.receipt_id.clone(),
            status,
            detail,
        });
    }

    Ok(ReceiptWalkResult {
        instance_id,
        overall,
        entries,
        total,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max::max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_mesh() -> (tempfile::NamedTempFile, ReceiptService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ReceiptService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    #[test]
    fn list_known_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.list("inst-1").is_ok());
    }

    #[test]
    fn list_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.list("no-such");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("not found"),
            "error should name the instance"
        );
    }

    #[test]
    fn list_new_instance_has_no_receipts() {
        let (_f, svc) = make_temp_mesh();
        let receipts = svc.list("inst-1").unwrap();
        assert!(receipts.is_empty(), "fresh instance must have no receipts");
    }

    #[test]
    fn list_fails_on_missing_state_file() {
        let svc = ReceiptService {
            state_path: "/tmp/no-such-dir-lsp-max/receipt/state.json".to_string(),
        };
        assert!(svc.list("inst-1").is_err());
    }

    #[test]
    fn verify_empty_receipt_chain_is_not_valid() {
        let (_f, svc) = make_temp_mesh();
        let (count, chain_valid) = svc.verify("inst-1").unwrap();
        assert_eq!(count, 0);
        assert!(!chain_valid, "empty receipt list must not be chain-valid");
    }

    #[test]
    fn verify_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.verify("no-such").is_err());
    }

    fn with_mesh_state<F: FnOnce()>(f: F) {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        mesh.add_instance(LspInstance::new("inst-empty"));
        // Seed a genesis receipt: a non-LSP_1 ledger verifies once it is non-empty
        // and every receipt carries a non-empty id and hash.
        if let Some(inst) = mesh.instances.get_mut("inst-1") {
            inst.receipts.push(Receipt {
                receipt_id: "rcpt-genesis".to_string(),
                hash: "genesis-hash".to_string(),
                prev_receipt_hash: None,
            });
        }
        let tmpf = tempfile::NamedTempFile::new().unwrap();
        let path = tmpf.path().to_str().unwrap().to_string();
        mesh.save_to_file(&path).unwrap();
        let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: under TEST_ENV_LOCK.
        unsafe { std::env::set_var("LSP_MAX_STATE_PATH", &path) };
        f();
        // SAFETY: restoring env under TEST_ENV_LOCK.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                None => std::env::remove_var("LSP_MAX_STATE_PATH"),
            }
        }
    }

    #[test]
    fn verify_ledger_returns_err_on_empty_receipt_chain() {
        // Counterfactual: verify_instance_ledger returns Err("Ledger is empty") for a
        // freshly-created instance that has no receipts.  The verb must propagate that Err.
        with_mesh_state(|| {
            let result = verify_ledger("inst-empty".to_string());
            assert!(result.is_err(), "expected Err for empty receipt chain");
        });
    }

    #[test]
    fn verify_ledger_unknown_instance_via_dispatch_returns_err() {
        // Counterfactual: an instance not in the mesh must yield Err before even reaching
        // the ledger verification step.
        with_mesh_state(|| {
            let result = verify_ledger("no-such-instance".to_string());
            assert!(result.is_err(), "expected Err for unknown instance");
        });
    }

    #[test]
    fn verify_ledger_unknown_instance_returns_err() {
        // Counterfactual: unknown instance must yield Err.
        with_mesh_state(|| {
            assert!(verify_ledger("no-such-instance".to_string()).is_err());
        });
    }

    #[test]
    fn ledger_report_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            assert!(ledger_report("inst-1".to_string()).is_ok());
        });
    }

    #[test]
    fn ledger_report_result_carries_instance_id() {
        // Falsification: the returned struct must echo back the queried instance_id.
        with_mesh_state(|| {
            let result = ledger_report("inst-1".to_string()).unwrap();
            assert_eq!(result.instance_id, "inst-1");
        });
    }

    #[test]
    fn ledger_report_unknown_instance_returns_err() {
        // Counterfactual: unknown instance must yield Err.
        with_mesh_state(|| {
            assert!(ledger_report("no-such-instance".to_string()).is_err());
        });
    }

    // walk verb ---------------------------------------------------------------

    #[test]
    fn walk_empty_receipt_chain_reports_unknown_overall() {
        // Success: walk returns Ok for a fresh instance with zero receipts.
        let (_f, svc) = make_temp_mesh();
        let receipts = svc.list("inst-1").unwrap();
        let result = run_walk_logic("inst-1", receipts);
        // Falsification: zero receipts → overall must be "UNKNOWN".
        assert_eq!(result.total, 0);
        assert_eq!(result.overall, "UNKNOWN");
    }

    #[test]
    fn walk_admitted_receipt_chain_reports_admitted_overall() {
        // Build a mesh whose instance carries a well-formed receipt.
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-r"));
        if let Some(inst) = mesh.instances.get_mut("inst-r") {
            inst.receipts.push(lsp_max::max_runtime::Receipt {
                receipt_id: "rcpt-001".to_string(),
                hash: "abc123hash".to_string(),
                prev_receipt_hash: None,
            });
        }
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ReceiptService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        let receipts = svc.list("inst-r").unwrap();
        let result = run_walk_logic("inst-r", receipts);
        // Success: one receipt present.
        assert_eq!(result.total, 1);
        // Falsification: well-formed receipt → overall is "ADMITTED".
        assert_eq!(result.overall, "ADMITTED");
        assert_eq!(result.entries[0].status, "ADMITTED");
        assert_eq!(result.entries[0].receipt_id, "rcpt-001");
    }

    #[test]
    fn walk_empty_hash_produces_refused_overall() {
        // A receipt with an empty hash triggers REFUSED.
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-bad"));
        if let Some(inst) = mesh.instances.get_mut("inst-bad") {
            inst.receipts.push(lsp_max::max_runtime::Receipt {
                receipt_id: "rcpt-bad".to_string(),
                hash: "".to_string(), // intentionally empty to trigger REFUSED
                prev_receipt_hash: None,
            });
        }
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = ReceiptService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        let receipts = svc.list("inst-bad").unwrap();
        let result = run_walk_logic("inst-bad", receipts);
        // Falsification: empty hash → overall "REFUSED".
        assert_eq!(result.overall, "REFUSED");
        assert_eq!(result.entries[0].status, "REFUSED");
    }

    #[test]
    fn walk_unknown_instance_returns_err() {
        // Counterfactual: unknown instance must return Err from the service list call.
        let (_f, svc) = make_temp_mesh();
        assert!(svc.list("no-such").is_err());
    }

    /// Re-implements the walk verb's core logic for service-layer testing
    /// without routing through the env-var verb entrypoint.
    fn run_walk_logic(
        instance_id: &str,
        receipts: Vec<lsp_max::max_runtime::Receipt>,
    ) -> ReceiptWalkResult {
        let total = receipts.len();
        let mut entries = Vec::new();
        let mut overall = if total == 0 { "UNKNOWN" } else { "ADMITTED" }.to_string();
        for (idx, r) in receipts.iter().enumerate() {
            let (status, detail) = if r.receipt_id.is_empty() || r.hash.is_empty() {
                overall = "REFUSED".to_string();
                (
                    "REFUSED".to_string(),
                    format!("empty receipt_id or hash at index {}", idx),
                )
            } else {
                (
                    "ADMITTED".to_string(),
                    format!(
                        "id={} hash={}...",
                        r.receipt_id,
                        &r.hash[..8.min(r.hash.len())]
                    ),
                )
            };
            entries.push(ReceiptWalkEntry {
                index: idx,
                receipt_id: r.receipt_id.clone(),
                status,
                detail,
            });
        }
        ReceiptWalkResult {
            instance_id: instance_id.to_string(),
            overall,
            entries,
            total,
        }
    }
}
