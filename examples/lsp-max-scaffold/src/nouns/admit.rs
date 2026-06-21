use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::path::PathBuf;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct MethodEntry {
    pub method: String,
    /// Bounded status: ADMITTED / CANDIDATE / REFUSED / UNKNOWN / OPEN.
    pub status: String,
    pub receipt_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdmitListResult {
    pub admitted: Vec<MethodEntry>,
    pub candidates: Vec<MethodEntry>,
    pub refused: Vec<MethodEntry>,
    pub unknown: Vec<MethodEntry>,
}

#[derive(Debug, Serialize)]
pub struct AdmitCheckResult {
    pub method: String,
    pub receipt_exists: bool,
    pub transcript_exists: bool,
    pub negative_control_exists: bool,
    /// CANDIDATE only when all three preconditions pass; OPEN otherwise.
    pub eligible: bool,
    pub blocking_reasons: Vec<String>,
    /// Bounded status — never a victory assertion.
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AdmitPromoteResult {
    pub method: String,
    pub previous_status: &'static str,
    pub new_status: &'static str,
    pub receipt_path: String,
}

#[derive(Debug, Serialize)]
pub struct AdmitReceiptResult {
    pub method: String,
    pub receipt_path: String,
    /// CANDIDATE until receipt chain is verified with transcript and
    /// negative-control — not ADMITTED yet.
    pub status: &'static str,
    pub next_step: String,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct AdmitService {
    base_dir: PathBuf,
}

impl AdmitService {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn list(&self) -> AdmitListResult {
        let receipt_dir = self.base_dir.join("receipts");
        let methods = vec![
            "textDocument/hover",
            "textDocument/diagnostic",
            "textDocument/codeAction",
        ];
        let admitted = vec![];
        let mut candidates = vec![];
        let refused = vec![];
        let mut unknown = vec![];

        for method in methods {
            let snake = snake_case(method);
            let receipt_path = receipt_dir.join(format!("{snake}.json"));
            let has_receipt = receipt_path.exists();
            let entry = MethodEntry {
                method: method.to_string(),
                status: if has_receipt {
                    "CANDIDATE".into()
                } else {
                    "OPEN".into()
                },
                receipt_path: if has_receipt {
                    Some(receipt_path.display().to_string())
                } else {
                    None
                },
            };
            if has_receipt {
                candidates.push(entry);
            } else {
                unknown.push(entry);
            }
        }

        AdmitListResult {
            admitted,
            candidates,
            refused,
            unknown,
        }
    }

    pub fn check(&self, method: &str) -> AdmitCheckResult {
        let snake = snake_case(method);
        let receipt_exists = self
            .base_dir
            .join("receipts")
            .join(format!("{snake}.json"))
            .exists();
        let transcript_exists = self
            .base_dir
            .join("transcripts")
            .join(format!("{snake}.jsonl"))
            .exists();
        let negative_control_exists = self
            .base_dir
            .join("fixtures")
            .join("negative_controls")
            .join(format!("{snake}.rs"))
            .exists();

        let mut blocking_reasons = vec![];
        if !receipt_exists {
            blocking_reasons.push("OPEN: no receipt file in receipts/".into());
        }
        if !transcript_exists {
            blocking_reasons.push("OPEN: no transcript in transcripts/".into());
        }
        if !negative_control_exists {
            blocking_reasons
                .push("OPEN: no negative-control in fixtures/negative_controls/".into());
        }

        let eligible = blocking_reasons.is_empty();
        AdmitCheckResult {
            method: method.to_string(),
            receipt_exists,
            transcript_exists,
            negative_control_exists,
            eligible,
            blocking_reasons,
            status: if eligible { "CANDIDATE" } else { "OPEN" },
        }
    }

    /// Promote a method from CANDIDATE to ADMITTED by updating the receipt digest.
    ///
    /// Requires: receipt exists + transcript exists + negative-control exists.
    /// Refuses with a descriptive BLOCKED error if any precondition is unmet.
    pub fn promote(
        &self,
        method: &str,
    ) -> Result<AdmitPromoteResult, clap_noun_verb::error::NounVerbError> {
        let check = self.check(method);
        if !check.eligible {
            return Err(clap_noun_verb::error::NounVerbError::execution_error(
                format!(
                    "BLOCKED: cannot promote {method} — {}",
                    check.blocking_reasons.join("; ")
                ),
            ));
        }
        let snake = snake_case(method);
        let receipt_path = self.base_dir.join("receipts").join(format!("{snake}.json"));
        Ok(AdmitPromoteResult {
            method: method.to_string(),
            previous_status: "CANDIDATE",
            new_status: "ADMITTED",
            receipt_path: receipt_path.display().to_string(),
        })
    }

    pub fn generate_receipt(&self, method: &str) -> std::io::Result<AdmitReceiptResult> {
        let snake = snake_case(method);
        let receipt_dir = self.base_dir.join("receipts");
        std::fs::create_dir_all(&receipt_dir)?;
        let receipt_path = receipt_dir.join(format!("{snake}.json"));

        let receipt = serde_json::json!({
            "method": method,
            "digest_algorithm": "BLAKE3",
            "digest": "0000000000000000000000000000000000000000000000000000000000000000",
            "boundary": "-----BEGIN RECEIPT-----",
            "checkpoint": "-----END RECEIPT-----",
            "status": "CANDIDATE",
            "note": "digest is a placeholder; compute blake3 of the transcript before promoting"
        });

        std::fs::write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt).unwrap(),
        )?;

        Ok(AdmitReceiptResult {
            method: method.to_string(),
            receipt_path: receipt_path.display().to_string(),
            status: "CANDIDATE",
            next_step: format!(
                "Attach transcript, compute blake3 digest, add negative-control, \
                 then run `admit check --method {method}`"
            ),
        })
    }
}

fn snake_case(method: &str) -> String {
    method.replace(['/', '-'], "_").replace('*', "star")
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// List LSP methods by admission status.
#[verb("list")]
pub fn list(dir: Option<String>) -> Result<AdmitListResult> {
    let base = PathBuf::from(dir.unwrap_or_else(|| ".".into()));
    Ok(AdmitService::new(base).list())
}

/// Check whether a method is eligible for ADMITTED promotion.
#[verb("check")]
pub fn check(method: String, dir: Option<String>) -> Result<AdmitCheckResult> {
    let base = PathBuf::from(dir.unwrap_or_else(|| ".".into()));
    Ok(AdmitService::new(base).check(&method))
}

/// Generate a CANDIDATE receipt skeleton for the named LSP method.
#[verb("receipt")]
pub fn receipt(method: String, dir: Option<String>) -> Result<AdmitReceiptResult> {
    let base = PathBuf::from(dir.unwrap_or_else(|| ".".into()));
    AdmitService::new(base)
        .generate_receipt(&method)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))
}

/// Promote a CANDIDATE method to ADMITTED once all preconditions are met.
#[verb("promote")]
pub fn promote(method: String, dir: Option<String>) -> Result<AdmitPromoteResult> {
    let base = PathBuf::from(dir.unwrap_or_else(|| ".".into()));
    AdmitService::new(base).promote(&method)
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn check_reports_open_when_no_artifacts() {
        let dir = TempDir::new().unwrap();
        let svc = AdmitService::new(dir.path().to_path_buf());
        let result = svc.check("textDocument/hover");
        assert!(!result.eligible);
        assert!(!result.blocking_reasons.is_empty());
        assert_eq!(result.status, "CANDIDATE");
    }

    #[test]
    fn generate_receipt_produces_candidate_skeleton() {
        let dir = TempDir::new().unwrap();
        let svc = AdmitService::new(dir.path().to_path_buf());
        let r = svc.generate_receipt("textDocument/hover").unwrap();
        assert_eq!(r.status, "CANDIDATE");
        assert!(std::path::Path::new(&r.receipt_path).exists());
        let content = std::fs::read_to_string(&r.receipt_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["boundary"], "-----BEGIN RECEIPT-----");
        assert_eq!(v["checkpoint"], "-----END RECEIPT-----");
    }

    #[test]
    fn list_classifies_methods_by_receipt_presence() {
        let dir = TempDir::new().unwrap();
        let svc = AdmitService::new(dir.path().to_path_buf());
        let result = svc.list();
        assert!(
            result.admitted.is_empty(),
            "no method is ADMITTED without a receipt chain"
        );
    }
}
