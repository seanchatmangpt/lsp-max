use lsp_max_protocol::{LawAxis, MaxDiagnostic, Repairability, Terminality};
use serde::{Deserialize, Serialize};

mod helpers;
pub mod queries;

use helpers::*;
pub use queries::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub is_success: bool,
    pub diagnostics: Vec<MaxDiagnostic>,
    pub execution_time_ms: u64,
}

pub fn verify_invariants(store: &oxigraph::store::Store) -> VerificationReport {
    let start_time = std::time::Instant::now();
    let mut diagnostics = Vec::new();

    // Invariant 1: Orphan LSIF
    if let Ok(true) = run_ask(store, QUERY_INVARIANT_1) {
        if let Ok(solutions) = run_select(store, QUERY_INVARIANT_1_SELECT) {
            for (idx, sol) in solutions.iter().enumerate() {
                let s = sol.get("s").map(term_to_string).unwrap_or_default();
                let p = sol.get("p").map(term_to_string).unwrap_or_default();
                let o = sol.get("o").map(term_to_string).unwrap_or_default();
                let g = sol.get("g").map(term_to_string);

                let message = format!(
                    "LSIF edge <{}> -{}--> <{}> points to non-existent node (orphan relation).",
                    s, p, o
                );

                let observed = serde_json::json!({
                    "subject": s,
                    "predicate": p,
                    "object": o,
                    "graph": g,
                });
                let expected = serde_json::json!({
                    "object_exists": true
                });

                diagnostics.push(create_diagnostic_detailed(
                    &format!("diag-invariant-1-{}", idx),
                    &message,
                    "INVARIANT_1",
                    LawAxis::Protocol,
                    observed,
                    expected,
                    Repairability::Repairable,
                    Terminality::NonTerminal,
                ));
            }
        }
        if diagnostics.is_empty() {
            diagnostics.push(create_diagnostic_detailed(
                "diag-invariant-1",
                "LSIF edges point to non-existent nodes (orphan relations).",
                "INVARIANT_1",
                LawAxis::Protocol,
                serde_json::Value::Null,
                serde_json::Value::Null,
                Repairability::Repairable,
                Terminality::NonTerminal,
            ));
        }
    }

    // Invariant 2: Unreceipted consequences
    if let Ok(true) = run_ask(store, QUERY_INVARIANT_2) {
        if let Ok(solutions) = run_select(store, QUERY_INVARIANT_2_SELECT) {
            for (idx, sol) in solutions.iter().enumerate() {
                let artifact = sol.get("artifact").map(term_to_string).unwrap_or_default();
                let g = sol.get("g").map(term_to_string);

                let message = format!(
                    "Artifact or Diagnostic <{}> lacks cryptographic provenance receipt.",
                    artifact
                );

                let observed = serde_json::json!({
                    "artifact": artifact,
                    "has_receipt": false,
                    "graph": g,
                });
                let expected = serde_json::json!({
                    "has_receipt": true
                });

                diagnostics.push(create_diagnostic_detailed(
                    &format!("diag-invariant-2-{}", idx),
                    &message,
                    "INVARIANT_2",
                    LawAxis::Receipt,
                    observed,
                    expected,
                    Repairability::Repairable,
                    Terminality::NonTerminal,
                ));
            }
        }
        if diagnostics
            .iter()
            .filter(|d| d.violated_invariant == "INVARIANT_2")
            .count()
            == 0
        {
            diagnostics.push(create_diagnostic_detailed(
                "diag-invariant-2",
                "Found diagnostics/artifacts lacking cryptographic provenance receipt.",
                "INVARIANT_2",
                LawAxis::Receipt,
                serde_json::Value::Null,
                serde_json::Value::Null,
                Repairability::Repairable,
                Terminality::NonTerminal,
            ));
        }
    }

    // Invariant 3: Missing pre-computed projection
    if let Ok(true) = run_ask(store, QUERY_INVARIANT_3) {
        if let Ok(solutions) = run_select(store, QUERY_INVARIANT_3_SELECT) {
            for (idx, sol) in solutions.iter().enumerate() {
                let range = sol.get("range").map(term_to_string).unwrap_or_default();
                let g = sol.get("g").map(term_to_string);

                let message = format!(
                    "Definition-mapped range <{}> lacks pre-computed projection.",
                    range
                );

                let observed = serde_json::json!({
                    "range": range,
                    "has_projection": false,
                    "graph": g,
                });
                let expected = serde_json::json!({
                    "has_projection": true
                });

                diagnostics.push(create_diagnostic_detailed(
                    &format!("diag-invariant-3-{}", idx),
                    &message,
                    "INVARIANT_3",
                    LawAxis::Type,
                    observed,
                    expected,
                    Repairability::Repairable,
                    Terminality::NonTerminal,
                ));
            }
        }
        if diagnostics
            .iter()
            .filter(|d| d.violated_invariant == "INVARIANT_3")
            .count()
            == 0
        {
            diagnostics.push(create_diagnostic_detailed(
                "diag-invariant-3",
                "Definition-mapped range lacks pre-computed projection.",
                "INVARIANT_3",
                LawAxis::Type,
                serde_json::Value::Null,
                serde_json::Value::Null,
                Repairability::Repairable,
                Terminality::NonTerminal,
            ));
        }
    }

    // Invariant 4: Ontology laundering
    if let Ok(true) = run_ask(store, QUERY_INVARIANT_4) {
        if let Ok(solutions) = run_select(store, QUERY_INVARIANT_4_SELECT) {
            for (idx, sol) in solutions.iter().enumerate() {
                let s = sol.get("s").map(term_to_string).unwrap_or_default();
                let p = sol.get("p").map(term_to_string).unwrap_or_default();
                let o = sol.get("o").map(term_to_string).unwrap_or_default();
                let g = sol.get("g").map(term_to_string);

                let message = format!(
                    "Ontology laundering detected: unwhitelisted LSIF predicate <{}> used on subject <{}>.",
                    p, s
                );

                let observed = serde_json::json!({
                    "subject": s,
                    "predicate": p,
                    "object": o,
                    "graph": g,
                });
                let expected = serde_json::json!({
                    "whitelisted_predicate": true
                });

                diagnostics.push(create_diagnostic_detailed(
                    &format!("diag-invariant-4-{}", idx),
                    &message,
                    "INVARIANT_4",
                    LawAxis::Security,
                    observed,
                    expected,
                    Repairability::NotRepairable,
                    Terminality::Terminal,
                ));
            }
        }
        if diagnostics
            .iter()
            .filter(|d| d.violated_invariant == "INVARIANT_4")
            .count()
            == 0
        {
            diagnostics.push(create_diagnostic_detailed(
                "diag-invariant-4",
                "Ontology laundering detected: unwhitelisted LSIF predicate used.",
                "INVARIANT_4",
                LawAxis::Security,
                serde_json::Value::Null,
                serde_json::Value::Null,
                Repairability::NotRepairable,
                Terminality::Terminal,
            ));
        }
    }

    // Invariant 5: Replay mismatch
    if let Ok(true) = run_ask(store, QUERY_INVARIANT_5) {
        if let Ok(solutions) = run_select(store, QUERY_INVARIANT_5_SELECT) {
            for (idx, sol) in solutions.iter().enumerate() {
                let receipt = sol.get("receipt").map(term_to_string).unwrap_or_default();
                let expected_hash = sol
                    .get("expectedResultHash")
                    .map(term_to_string)
                    .unwrap_or_default();
                let actual_hash = sol
                    .get("actualResultHash")
                    .map(term_to_string)
                    .unwrap_or_default();
                let g = sol.get("g").map(term_to_string);

                let message = format!(
                    "Replay mismatch: expected result hash <{}> and actual result hash <{}> differ (false ALIVE) for receipt <{}>.",
                    expected_hash, actual_hash, receipt
                );

                let observed = serde_json::json!({
                    "receipt": receipt,
                    "expected_result_hash": expected_hash,
                    "actual_result_hash": actual_hash,
                    "graph": g,
                });
                let expected = serde_json::json!({
                    "result_hashes_equal": true
                });

                diagnostics.push(create_diagnostic_detailed(
                    &format!("diag-invariant-5-{}", idx),
                    &message,
                    "INVARIANT_5",
                    LawAxis::Autopoiesis,
                    observed,
                    expected,
                    Repairability::NotRepairable,
                    Terminality::Terminal,
                ));
            }
        }
        if diagnostics
            .iter()
            .filter(|d| d.violated_invariant == "INVARIANT_5")
            .count()
            == 0
        {
            diagnostics.push(create_diagnostic_detailed(
                "diag-invariant-5",
                "Replay mismatch: expected and actual result hashes differ (false ALIVE).",
                "INVARIANT_5",
                LawAxis::Autopoiesis,
                serde_json::Value::Null,
                serde_json::Value::Null,
                Repairability::NotRepairable,
                Terminality::Terminal,
            ));
        }
    }

    let is_success = diagnostics.is_empty();
    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    VerificationReport {
        is_success,
        diagnostics,
        execution_time_ms,
    }
}

#[cfg(test)]
mod tests;
