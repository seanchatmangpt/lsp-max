use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::{AutonomicMesh, Receipt};
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

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

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

    // --- list ---

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
        assert!(result.unwrap_err().contains("not found"), "error should name the instance");
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
            state_path: "/tmp/nonexistent-receipt-test.json".to_string(),
        };
        assert!(svc.list("inst-1").is_err());
    }

    // --- verify ---

    #[test]
    fn verify_known_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.verify("inst-1").is_ok());
    }

    #[test]
    fn verify_empty_receipt_chain_is_not_valid() {
        let (_f, svc) = make_temp_mesh();
        let (count, chain_valid) = svc.verify("inst-1").unwrap();
        assert_eq!(count, 0);
        // Empty receipt list → chain cannot be valid (requires at least one receipt).
        assert!(!chain_valid, "empty receipt list must not be chain-valid");
    }

    #[test]
    fn verify_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.verify("no-such").is_err());
    }

    // --- RPC verbs ---

    fn with_mesh_state<F: FnOnce()>(f: F) {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
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
    fn verify_ledger_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            assert!(verify_ledger("inst-1".to_string()).is_ok());
        });
    }

    #[test]
    fn ledger_report_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            assert!(ledger_report("inst-1".to_string()).is_ok());
        });
    }
}
