use serde::{Deserialize, Serialize};
use ocel_core::OCEL;
use wasm4pm_algos::gall::{check_gall_conformance, GallVerdict};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceIssue {
    pub severity: String,
    pub code: String,
    pub message: String,
}

pub fn analyze_ocel(content: &str) -> Vec<ConformanceIssue> {
    let mut issues = Vec::new();
    
    match serde_json::from_str::<OCEL>(content) {
        Ok(ocel) => {
            let verdict = check_gall_conformance(&ocel);

            let (severity, code, message) = match verdict {
                GallVerdict::Blocked { reason } => {
                    ("ERROR", "WASM4PM-VERDICT-BLOCKED", format!("Conformance Verdict: BLOCKED ({})", reason))
                }
                GallVerdict::Fit { fitness } => {
                    ("INFORMATION", "WASM4PM-VERDICT-FIT", format!("Conformance Verdict: FIT (Fitness: {:.1})", fitness))
                }
                GallVerdict::Deviation { fitness, missing } => {
                    ("ERROR", "WASM4PM-VERDICT-DEVIATION", format!("Conformance Verdict: DEVIATION (Fitness: {:.1}). Missing admission for: {}", fitness, missing.join(", ")))
                }
            };

            issues.push(ConformanceIssue {
                severity: severity.to_string(),
                code: code.to_string(),
                message,
            });
        }
        Err(e) => {
            issues.push(ConformanceIssue {
                severity: "ERROR".to_string(),
                code: "WASM4PM-PARSE-FAILED".to_string(),
                message: format!("Failed to parse OCEL: {}", e),
            });
        }
    }
    
    issues
}
