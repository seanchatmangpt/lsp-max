use serde::{Deserialize, Serialize};

pub const EXPLAIN_DIAGNOSTIC: &str = "max/explain.diagnostic";
pub const EXPLAIN_STATUS: &str = "max/explain.status";
pub const EXPLAIN_RECEIPT: &str = "max/explain.receipt";

/// Request: explain a specific diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainDiagnosticParams {
    /// Serialised document URI
    pub uri: String,
    pub position: ExplainPosition,
    /// Optional: diagnostic code to explain (e.g. "ANTI-LLM-META-003")
    pub diagnostic_code: Option<String>,
}

/// Minimal position type for explain requests (row/column, 0-based).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainPosition {
    pub line: u32,
    pub character: u32,
}

/// A single law-axis trace step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawAxisTrace {
    pub axis: String,               // e.g. "transcript", "negative_control", "receipt"
    pub status: String,             // ADMITTED | CANDIDATE | REFUSED | UNKNOWN | OPEN
    pub description: String,        // why this axis is in this state
    pub resolution: Option<String>, // what would clear this axis
}

/// Full explanation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainDiagnosticResult {
    pub diagnostic_code: String,
    pub law_status: String,        // overall status
    pub summary: String,           // one-sentence explanation
    pub law_axes: Vec<LawAxisTrace>,
    pub resolution_steps: Vec<String>, // ordered steps to resolve
    pub related_receipts: Vec<String>, // receipt paths if any
    pub related_docs: Vec<String>,     // doc links
}

/// Request: explain the status of a method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainStatusParams {
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainStatusResult {
    pub method: String,
    pub law_status: String,
    pub explanation: String,
    pub law_axes: Vec<LawAxisTrace>,
    pub can_promote: bool,
    pub promotion_blockers: Vec<String>,
}

/// Request: explain a receipt chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainReceiptParams {
    pub receipt_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainReceiptResult {
    pub receipt_path: String,
    pub valid: bool,
    pub has_begin_marker: bool,
    pub has_end_marker: bool,
    pub has_digest: bool,
    pub has_checkpoint: bool,
    pub issues: Vec<String>,
    pub law_status: String,
}

/// Well-known diagnostic code explanations
pub fn explain_code(code: &str) -> ExplainDiagnosticResult {
    match code {
        "ANTI-LLM-META-001" => ExplainDiagnosticResult {
            diagnostic_code: code.to_string(),
            law_status: "REFUSED".into(),
            summary: "Forbidden plain tower-lsp reference detected".into(),
            law_axes: vec![LawAxisTrace {
                axis: "naming_law".into(),
                status: "REFUSED".into(),
                description: "All references must use lsp-max, not tower-lsp or tower_lsp".into(),
                resolution: Some("Replace with lsp-max or lsp_max".into()),
            }],
            resolution_steps: vec![
                "Search for tower-lsp / tower_lsp using scripts/check-law-compliance.sh".into(),
                "Replace all occurrences with lsp-max (crate name) or lsp_max (Rust identifier)".into(),
                "Verify with `just dx-verify`".into(),
            ],
            related_receipts: vec![],
            related_docs: vec!["CLAUDE.md#naming-law".into()],
        },
        "ANTI-LLM-META-002" => ExplainDiagnosticResult {
            diagnostic_code: code.to_string(),
            law_status: "REFUSED".into(),
            summary: "Victory language detected — use bounded status".into(),
            law_axes: vec![LawAxisTrace {
                axis: "language_law".into(),
                status: "REFUSED".into(),
                description: "Terms like 'done', 'complete', 'solved' are forbidden; they assert certainty that cannot be receipted".into(),
                resolution: Some("Replace with ADMITTED, CANDIDATE, OPEN, PARTIAL, or BLOCKED".into()),
            }],
            resolution_steps: vec![
                "Replace 'done' → 'ADMITTED' (if receipt exists) or 'CANDIDATE'".into(),
                "Replace 'complete' → 'PARTIAL' or 'CANDIDATE'".into(),
                "Replace 'solved' → 'ADMITTED' (if receipt exists) or 'CANDIDATE'".into(),
            ],
            related_receipts: vec![],
            related_docs: vec!["CLAUDE.md#victory-language".into()],
        },
        "ANTI-LLM-META-003" => ExplainDiagnosticResult {
            diagnostic_code: code.to_string(),
            law_status: "BLOCKED".into(),
            summary: "law:ADMITTED claimed without law:receipt — receipt chain OPEN".into(),
            law_axes: vec![
                LawAxisTrace {
                    axis: "receipt".into(),
                    status: "OPEN".into(),
                    description: "ADMITTED status requires a receipt artifact (BEGIN/END markers + SHA256 digest)".into(),
                    resolution: Some("Run `lsp-max admit receipt <method>` to generate a receipt template".into()),
                },
                LawAxisTrace {
                    axis: "transcript".into(),
                    status: "UNKNOWN".into(),
                    description: "No transcript present to receipt".into(),
                    resolution: Some("Run the dogfood test for this method and capture output as transcript".into()),
                },
            ],
            resolution_steps: vec![
                "Generate receipt: `lsp-max admit receipt <method>`".into(),
                "Attach transcript to the receipt file".into(),
                "Add negative-control test in tests/negative/".into(),
                "Run `lsp-max admit promote <method>` once all axes are present".into(),
            ],
            related_receipts: vec![],
            related_docs: vec!["CLAUDE.md#receipt-chain".into()],
        },
        "ANTI-LLM-META-004" => ExplainDiagnosticResult {
            diagnostic_code: code.to_string(),
            law_status: "UNKNOWN".into(),
            summary: "lsp:Request without law:status — defaults to UNKNOWN".into(),
            law_axes: vec![LawAxisTrace {
                axis: "law_status".into(),
                status: "UNKNOWN".into(),
                description: "Every lsp:Request must have an explicit law:status assertion".into(),
                resolution: Some("Add law:status law:CANDIDATE to the method declaration".into()),
            }],
            resolution_steps: vec![
                "Add `law:status law:CANDIDATE ;` to the method's TTL block".into(),
                "UNKNOWN must not collapse to ADMITTED without tracing".into(),
            ],
            related_receipts: vec![],
            related_docs: vec!["CLAUDE.md#unknown-status".into()],
        },
        _ => ExplainDiagnosticResult {
            diagnostic_code: code.to_string(),
            law_status: "UNKNOWN".into(),
            summary: format!("Unknown diagnostic code: {code}"),
            law_axes: vec![],
            resolution_steps: vec!["Check CLAUDE.md for law documentation".into()],
            related_receipts: vec![],
            related_docs: vec![],
        },
    }
}

pub fn explain_method_status(method: &str, law_status: &str) -> ExplainStatusResult {
    let (explanation, axes, can_promote, blockers) = match law_status {
        "ADMITTED" => (
            format!("{method} is ADMITTED — receipt chain is closed"),
            vec![
                LawAxisTrace {
                    axis: "transcript".into(),
                    status: "PRESENT".into(),
                    description: "Transcript file attached".into(),
                    resolution: None,
                },
                LawAxisTrace {
                    axis: "negative_control".into(),
                    status: "PRESENT".into(),
                    description: "Negative-control test present".into(),
                    resolution: None,
                },
                LawAxisTrace {
                    axis: "receipt".into(),
                    status: "PRESENT".into(),
                    description: "Receipt artifact with digest".into(),
                    resolution: None,
                },
            ],
            false,
            vec![],
        ),
        "REFUSED" => (
            format!("{method} is REFUSED — law-blocked by ontology assertion"),
            vec![LawAxisTrace {
                axis: "policy".into(),
                status: "REFUSED".into(),
                description: "Ontology law:REFUSED assertion present".into(),
                resolution: Some("Edit domain.ttl law:reason to understand the policy".into()),
            }],
            false,
            vec!["REFUSED methods cannot be promoted — update ontology policy to change".into()],
        ),
        "UNKNOWN" => (
            format!("{method} status is UNKNOWN — not yet traced"),
            vec![LawAxisTrace {
                axis: "tracing".into(),
                status: "UNKNOWN".into(),
                description: "Law-axis tracing not initiated".into(),
                resolution: Some("Add law:status law:CANDIDATE to begin tracing".into()),
            }],
            false,
            vec!["UNKNOWN must not collapse to ADMITTED without explicit tracing".into()],
        ),
        _ => (
            format!("{method} is CANDIDATE — receipt chain OPEN"),
            vec![
                LawAxisTrace {
                    axis: "transcript".into(),
                    status: "OPEN".into(),
                    description: "No transcript attached yet".into(),
                    resolution: Some("Run dogfood test and capture output".into()),
                },
                LawAxisTrace {
                    axis: "negative_control".into(),
                    status: "OPEN".into(),
                    description: "No negative-control test yet".into(),
                    resolution: Some("Create tests/negative/<method>.rs".into()),
                },
                LawAxisTrace {
                    axis: "receipt".into(),
                    status: "OPEN".into(),
                    description: "No receipt artifact yet".into(),
                    resolution: Some("Run `lsp-max admit receipt <method>`".into()),
                },
            ],
            true,
            vec![
                "transcript OPEN".into(),
                "negative_control OPEN".into(),
                "receipt OPEN".into(),
            ],
        ),
    };

    ExplainStatusResult {
        method: method.to_string(),
        law_status: law_status.to_string(),
        explanation,
        law_axes: axes,
        can_promote,
        promotion_blockers: blockers,
    }
}
