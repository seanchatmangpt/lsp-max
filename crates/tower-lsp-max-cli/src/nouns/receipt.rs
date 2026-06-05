use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_runtime::{AutonomicMesh, Receipt};

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// Receipt is re-exported from tower_lsp_max_runtime and derives Serialize.

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

#[verb("list")]
pub fn list(instance_id: String) -> Result<ReceiptListResult> {
    let service = ReceiptService::new();
    let receipts = service
        .list(&instance_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    let count = receipts.len();
    Ok(ReceiptListResult { receipts, count })
}

#[derive(Serialize)]
pub struct ReceiptVerifyResult {
    pub count: usize,
    pub chain_valid: bool,
}

#[verb("verify")]
pub fn verify(instance_id: String) -> Result<ReceiptVerifyResult> {
    let service = ReceiptService::new();
    let (count, chain_valid) = service
        .verify(&instance_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ReceiptVerifyResult { count, chain_valid })
}

#[derive(Serialize)]
pub struct VerifyLedgerResult {
    pub instance_id: String,
    pub raw: serde_json::Value,
}

#[verb("verify-ledger")]
pub fn verify_ledger(instance_id: String) -> Result<VerifyLedgerResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/verifyLedger", serde_json::Value::Null)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(VerifyLedgerResult { instance_id, raw })
}

#[derive(Serialize)]
pub struct LedgerReportResult {
    pub instance_id: String,
    pub raw: serde_json::Value,
}

#[verb("ledger-report")]
pub fn ledger_report(instance_id: String) -> Result<LedgerReportResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/ledgerReport", serde_json::Value::Null)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(LedgerReportResult { instance_id, raw })
}
