use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_protocol::explain::{explain_code, explain_method_status, ExplainReceiptResult};
use serde::Serialize;
use std::fs;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Serialize)]
pub struct ExplainCodeOutput {
    pub code: String,
    pub law_status: String,
    pub summary: String,
    pub resolution_steps: Vec<String>,
    pub law_axes: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ExplainMethodOutput {
    pub method: String,
    pub law_status: String,
    pub explanation: String,
    pub can_promote: bool,
    pub promotion_blockers: Vec<String>,
    pub law_axes: Vec<serde_json::Value>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

fn trace_to_json(a: &lsp_max_protocol::explain::LawAxisTrace) -> serde_json::Value {
    serde_json::json!({
        "axis": a.axis,
        "status": a.status,
        "description": a.description,
        "resolution": a.resolution,
    })
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Explain a known diagnostic code — returns law-axis trace and resolution steps.
#[verb("code")]
pub fn code(diagnostic_code: String) -> Result<ExplainCodeOutput> {
    let r = explain_code(&diagnostic_code);
    Ok(ExplainCodeOutput {
        code: r.diagnostic_code,
        law_status: r.law_status,
        summary: r.summary,
        resolution_steps: r.resolution_steps,
        law_axes: r.law_axes.iter().map(trace_to_json).collect(),
    })
}

/// Explain the law-axis status of a method name. `--law-status` defaults to CANDIDATE.
#[verb("method")]
pub fn method(method_name: String, law_status: Option<String>) -> Result<ExplainMethodOutput> {
    let status = law_status.unwrap_or_else(|| "CANDIDATE".to_string());
    let r = explain_method_status(&method_name, &status);
    Ok(ExplainMethodOutput {
        method: r.method,
        law_status: r.law_status,
        explanation: r.explanation,
        can_promote: r.can_promote,
        promotion_blockers: r.promotion_blockers,
        law_axes: r.law_axes.iter().map(trace_to_json).collect(),
    })
}

/// Explain a receipt file — validates BEGIN/END markers, digest, and checkpoint fields.
#[verb("receipt")]
pub fn receipt(receipt_path: String) -> Result<ExplainReceiptResult> {
    let content = fs::read_to_string(&receipt_path).unwrap_or_default();
    let has_begin = content.contains("-----BEGIN RECEIPT-----");
    let has_end = content.contains("-----END RECEIPT-----");
    let has_digest = content.contains("transcript_digest");
    let has_checkpoint = content.contains("checkpoint");
    let mut issues = vec![];
    if !has_begin {
        issues.push("OPEN: missing BEGIN RECEIPT marker".into());
    }
    if !has_end {
        issues.push("OPEN: missing END RECEIPT marker".into());
    }
    if !has_digest {
        issues.push("OPEN: missing transcript_digest field".into());
    }
    if !has_checkpoint {
        issues.push("OPEN: missing checkpoint field".into());
    }

    Ok(ExplainReceiptResult {
        receipt_path,
        valid: issues.is_empty(),
        has_begin_marker: has_begin,
        has_end_marker: has_end,
        has_digest,
        has_checkpoint,
        law_status: if issues.is_empty() {
            "CANDIDATE".into()
        } else {
            "BLOCKED".into()
        },
        issues,
    })
}
