use crate::analyzer::DefaultAnalyzer;
use crate::law::AxisState;
use crate::verifiable::{verify_chain, verify_receipt, Receipt, VerifiableEngine};
use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Serialize)]
pub struct VerifyScanResult {
    pub file: String,
    /// Number of law violations detected in the file.
    pub findings: usize,
    /// Receipts whose witness replay reproduced the finding.
    pub admitted: usize,
    /// Receipts that failed replay (forged or tampered).
    pub refused: usize,
    /// Chain linkage verdict label (bounded).
    pub chain_verdict: &'static str,
    pub chain_head: String,
    /// Proof status — ADMITTED when every receipt verifies and the chain is
    /// intact. This certifies the *proofs*, not the absence of violations.
    pub proof_status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct VerifyChainResult {
    pub receipts: usize,
    pub chain_verdict: &'static str,
    pub chain_head: Option<String>,
    pub status: &'static str,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct VerifyService;

impl VerifyService {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(&self, path: &str) -> std::io::Result<VerifyScanResult> {
        let source = std::fs::read_to_string(path)?;
        let analyzer = DefaultAnalyzer::new();
        let mut engine = VerifiableEngine::new(&analyzer);
        let diags = engine.extend(&source);

        let mut admitted = 0;
        let mut refused = 0;
        let mut receipts = Vec::with_capacity(diags.len());
        for d in &diags {
            match verify_receipt(&d.receipt, &d.witness, &analyzer) {
                AxisState::Admitted => admitted += 1,
                _ => refused += 1,
            }
            receipts.push(d.receipt.clone());
        }

        let verdict = verify_chain(&receipts);
        let proof_status = if refused == 0 && verdict.is_intact() {
            "ADMITTED"
        } else {
            "REFUSED"
        };

        Ok(VerifyScanResult {
            file: path.to_string(),
            findings: diags.len(),
            admitted,
            refused,
            chain_verdict: verdict.label(),
            chain_head: engine.head().to_string(),
            proof_status,
        })
    }

    pub fn chain(&self, path: &str) -> std::io::Result<VerifyChainResult> {
        let raw = std::fs::read_to_string(path)?;
        let receipts: Vec<Receipt> = serde_json::from_str(&raw).map_err(std::io::Error::other)?;
        let verdict = verify_chain(&receipts);
        let head = if let crate::verifiable::ChainVerdict::Intact { head, .. } = &verdict {
            Some(head.clone())
        } else {
            None
        };
        Ok(VerifyChainResult {
            receipts: receipts.len(),
            chain_verdict: verdict.label(),
            chain_head: head,
            status: if verdict.is_intact() {
                "ADMITTED"
            } else {
                "REFUSED"
            },
        })
    }
}

impl Default for VerifyService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Scan a file, build a replay-verifiable diagnostic chain, and verify it.
/// Each finding is independently replayed from its witness; the result reports
/// how many proofs were ADMITTED vs REFUSED and whether the chain is intact.
#[verb("scan")]
pub fn scan(file: String) -> Result<VerifyScanResult> {
    VerifyService::new()
        .scan(&file)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))
}

/// Verify the hash-chain linkage of a persisted receipt array (JSON).
#[verb("chain")]
pub fn chain(file: String) -> Result<VerifyChainResult> {
    VerifyService::new()
        .chain(&file)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn scan_admits_honest_findings_and_intact_chain() {
        // Build a victory token at runtime so this test's source stays law-clean.
        let token: String = "enod".chars().rev().collect();
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "// status: {token}").unwrap();

        let result = VerifyService::new()
            .scan(f.path().to_str().unwrap())
            .unwrap();
        assert!(result.findings >= 1);
        assert_eq!(result.refused, 0);
        assert_eq!(result.chain_verdict, "ADMITTED");
        assert_eq!(result.proof_status, "ADMITTED");
    }

    #[test]
    fn scan_of_clean_file_has_empty_intact_chain() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "fn main() {{ let x = 1; }}").unwrap();

        let result = VerifyService::new()
            .scan(f.path().to_str().unwrap())
            .unwrap();
        assert_eq!(result.findings, 0);
        assert_eq!(result.proof_status, "ADMITTED");
        assert_eq!(result.chain_head, crate::verifiable::genesis_head());
    }
}
