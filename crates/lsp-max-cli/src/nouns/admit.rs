use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_gen::{GeneratorContext, GeneratorEngine, ReceiptGenerator};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct AdmitStatusEntry {
    pub method: String,
    pub law_status: String,
    pub has_receipt: bool,
    pub receipt_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdmitListResult {
    pub candidates: Vec<AdmitStatusEntry>,
    pub admitted: Vec<AdmitStatusEntry>,
    pub refused: Vec<AdmitStatusEntry>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdmitReceiptResult {
    pub method: String,
    pub receipt_path: String,
    // CANDIDATE — not ADMITTED until receipt chain is verified with transcript and negative control
    pub status: String,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdmitPromoteResult {
    pub method: String,
    pub previous_status: String,
    pub new_status: String,
    pub ontology_file: String,
    pub receipt_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdmitCheckResult {
    pub method: String,
    pub receipt_exists: bool,
    pub transcript_exists: bool,
    pub negative_control_exists: bool,
    pub eligible: bool,
    pub blocking_reasons: Vec<String>,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct AdmitService;

impl AdmitService {
    pub fn new() -> Self {
        Self
    }

    pub fn list(&self, dir: &str) -> AdmitListResult {
        let onto_dir = PathBuf::from(dir).join("ontology");
        let mut candidates = vec![];
        let mut admitted = vec![];
        let mut refused = vec![];

        if let Ok(entries) = fs::read_dir(&onto_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("ttl") {
                    continue;
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                for line in content.lines() {
                    if line.contains("lsp:methodName") {
                        let method =
                            line.split('"').nth(1).unwrap_or("unknown").to_string();
                        let snake =
                            method.replace(['/', '-'], "_").replace('*', "star");
                        let receipt_path = PathBuf::from(dir)
                            .join("receipts")
                            .join(format!("{snake}.json"));
                        let has_receipt = receipt_path.exists();
                        let entry = AdmitStatusEntry {
                            method: method.clone(),
                            law_status: "CANDIDATE".into(),
                            has_receipt,
                            receipt_path: if has_receipt {
                                Some(receipt_path.display().to_string())
                            } else {
                                None
                            },
                        };
                        candidates.push(entry);
                    }
                    if line.contains("law:ADMITTED") {
                        if let Some(last) = candidates.last().cloned() {
                            let mut e = last;
                            e.law_status = "ADMITTED".into();
                            admitted.push(e);
                            candidates.pop();
                        }
                    }
                    if line.contains("law:REFUSED") {
                        if let Some(last) = candidates.last().cloned() {
                            let mut e = last;
                            e.law_status = "REFUSED".into();
                            refused.push(e);
                            candidates.pop();
                        }
                    }
                }
            }
        }

        AdmitListResult {
            candidates,
            admitted,
            refused,
            status: "CANDIDATE".into(),
        }
    }

    pub fn generate_receipt(
        &self,
        method: &str,
        dir: &str,
        transcript: Option<&str>,
    ) -> Result<AdmitReceiptResult> {
        let snake = method.replace(['/', '-'], "_").replace('*', "star");
        let output_dir = PathBuf::from(dir);
        let mut ctx = GeneratorContext::new(&snake, output_dir);
        ctx.extra = serde_json::json!({
            "method": method,
            "transcript": transcript.unwrap_or("OPEN"),
        });

        let engine = GeneratorEngine::new(vec![Box::new(ReceiptGenerator)]);
        let written = engine
            .run("receipt", &ctx)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

        let receipt_path = written
            .first()
            .map(|w| w.path.display().to_string())
            .unwrap_or_else(|| format!("receipts/{snake}.json"));

        Ok(AdmitReceiptResult {
            method: method.to_string(),
            receipt_path,
            status: "CANDIDATE".into(),
            next_step:
                "Attach transcript, verify negative-control, then run `admit promote`".into(),
        })
    }

    pub fn check(&self, method: &str, dir: &str) -> AdmitCheckResult {
        let snake = method.replace(['/', '-'], "_").replace('*', "star");
        let base = PathBuf::from(dir);
        let receipt_exists = base
            .join("receipts")
            .join(format!("{snake}.json"))
            .exists();
        let transcript_exists = base
            .join("transcripts")
            .join(format!("{snake}.txt"))
            .exists()
            || base
                .join("tests")
                .join("dogfood")
                .join(format!("{snake}.rs"))
                .exists();
        let negative_control_exists = base
            .join("tests")
            .join("negative")
            .join(format!("{snake}.rs"))
            .exists();

        let mut blocking_reasons = vec![];
        if !receipt_exists {
            blocking_reasons.push("OPEN: no receipt file in receipts/".into());
        }
        if !transcript_exists {
            blocking_reasons
                .push("OPEN: no transcript in transcripts/ or tests/dogfood/".into());
        }
        if !negative_control_exists {
            blocking_reasons
                .push("OPEN: no negative-control in tests/negative/".into());
        }

        AdmitCheckResult {
            method: method.to_string(),
            receipt_exists,
            transcript_exists,
            negative_control_exists,
            eligible: blocking_reasons.is_empty(),
            blocking_reasons,
            status: "CANDIDATE".into(),
        }
    }

    pub fn promote(
        &self,
        method: &str,
        dir: &str,
        ontology_file: &str,
    ) -> Result<AdmitPromoteResult> {
        let check = self.check(method, dir);
        if !check.eligible {
            return Err(NounVerbError::execution_error(format!(
                "BLOCKED: cannot promote {} — {}",
                method,
                check.blocking_reasons.join("; ")
            )));
        }

        let onto_path = PathBuf::from(dir).join(ontology_file);
        let content = fs::read_to_string(&onto_path)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

        let snake = method.replace(['/', '-'], "_").replace('*', "star");
        let receipt_path = format!("receipts/{snake}.json");
        let updated = content.replace(
            &format!(
                "\"{}\";\n    law:status law:CANDIDATE",
                method
            ),
            &format!(
                "\"{}\";\n    law:status law:ADMITTED ;\n    law:receipt <{}> ",
                method, receipt_path
            ),
        );

        if updated == content {
            return Err(NounVerbError::execution_error(format!(
                "BLOCKED: could not find CANDIDATE status for {} in {}",
                method, ontology_file
            )));
        }

        fs::write(&onto_path, updated)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

        Ok(AdmitPromoteResult {
            method: method.to_string(),
            previous_status: "CANDIDATE".into(),
            new_status: "ADMITTED".into(),
            ontology_file: ontology_file.to_string(),
            receipt_path,
        })
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

/// List LSP methods by admission status (CANDIDATE / ADMITTED / REFUSED).
#[verb("list")]
pub fn list(dir: Option<String>) -> Result<AdmitListResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(AdmitService::new().list(&dir))
}

/// Generate a receipt artifact for the named LSP method.
#[verb("receipt")]
pub fn receipt(
    method: String,
    dir: Option<String>,
    transcript: Option<String>,
) -> Result<AdmitReceiptResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    AdmitService::new().generate_receipt(&method, &dir, transcript.as_deref())
}

/// Check whether the named LSP method is eligible for ADMITTED promotion.
#[verb("check")]
pub fn check(method: String, dir: Option<String>) -> Result<AdmitCheckResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(AdmitService::new().check(&method, &dir))
}

/// Promote the named LSP method from CANDIDATE to ADMITTED in the ontology.
#[verb("promote")]
pub fn promote(
    method: String,
    dir: Option<String>,
    ontology_file: Option<String>,
) -> Result<AdmitPromoteResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let onto = ontology_file.unwrap_or_else(|| "ontology/domain.ttl".to_string());
    AdmitService::new().promote(&method, &dir, &onto)
}
