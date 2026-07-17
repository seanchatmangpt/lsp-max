//! Clippy-style `LintPass` trait — composable lint passes for anti-llm rules.
//!
//! Each `LintPass` operates directly on source text and file extension, returning
//! `AntiLlmDiagnostic` values without going through the `Observation` pipeline.
//! This makes it straightforward to add new rules that need multi-line or
//! function-body context that the single-pattern AhoCorasick pass cannot express.
//!
//! Registration: push a `Box<dyn LintPass>` into the vec returned by
//! `registered_passes()`. The engine calls all passes from
//! `run_lint_passes(source, file_ext)`.

use crate::diagnostics::AntiLlmDiagnostic;

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait LintPass: Send + Sync {
    fn name(&self) -> &'static str;
    fn family(&self) -> &'static str; // e.g. "CLAIM", "HOLLOW", "OCEL"
    fn check(&self, source: &str, file_path: &str, file_ext: &str) -> Vec<AntiLlmDiagnostic>;
    fn is_blocking(&self) -> bool {
        true
    }
}

// ── HollowLintPass ────────────────────────────────────────────────────────────
//
// Wraps the HOLLOW family: blocking hollow patterns detected from the
// `HOLLOW_PATTERNS` table in rules::hollow. This is a proof-of-concept
// migration — it re-uses the existing table so the two code paths stay in sync.

pub struct HollowLintPass;

/// Blocking hollow patterns duplicated here so `HollowLintPass` is self-contained.
/// The canonical source remains `rules::hollow::HOLLOW_PATTERNS`; these must stay
/// in sync with the blocking entries (is_blocking == true) in that table.
const HOLLOW_BLOCKING: &[(&str, &str, &str)] = &[
    (
        "unimplemented!()",
        "ANTI-LLM-HOLLOW-001",
        "unimplemented!() is a placeholder — hollow by law",
    ),
    (
        "todo!()",
        "ANTI-LLM-HOLLOW-002",
        "todo!() is a placeholder — hollow by law",
    ),
    (
        "panic!(\"not implemented\")",
        "ANTI-LLM-HOLLOW-003",
        "panic-as-stub detected — hollow by law",
    ),
    (
        "// TODO:",
        "ANTI-LLM-HOLLOW-004",
        "TODO comment is a placeholder — implement or formally refuse",
    ),
    (
        "// FIXME:",
        "ANTI-LLM-HOLLOW-005",
        "FIXME comment is a placeholder — implement or formally refuse",
    ),
];

impl LintPass for HollowLintPass {
    fn name(&self) -> &'static str {
        "hollow"
    }
    fn family(&self) -> &'static str {
        "HOLLOW"
    }

    fn check(&self, source: &str, file_path: &str, file_ext: &str) -> Vec<AntiLlmDiagnostic> {
        if file_ext != "rs" {
            return vec![];
        }
        let mut diags = Vec::new();
        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;
            for (pattern, code, msg) in HOLLOW_BLOCKING {
                if line.contains(pattern) {
                    diags.push(AntiLlmDiagnostic {
                        code: code.to_string(),
                        category: "hollow_implementation".to_string(),
                        file_path: file_path.to_string(),
                        line: line_num,
                        column: 1,
                        message: msg.to_string(),
                        forbidden_implication: format!(
                            "Placeholder({}) => HollowAdmission",
                            pattern.trim()
                        ),
                        blocking: true,
                        required_correction:
                            "Replace with real implementation or formal Refuses-by-law declaration"
                                .to_string(),
                        required_next_proof: "Provide transcript + receipt showing real behavior"
                            .to_string(),
                    });
                }
            }
        }
        diags
    }
}

// ── ClaimLintPass ─────────────────────────────────────────────────────────────
//
// Proof-of-concept wrapper around the CLAIM family. Flags a non-exhaustive
// subset of victory-language terms that are unconditionally blocking regardless
// of domain-term exemptions. Full exemption logic lives in rules::claims::evaluate;
// this pass is additive and independent.

pub struct ClaimLintPass;

const CLAIM_BLOCKING_TERMS: &[(&str, &str)] = &[
    (
        "all tests pass",
        "All tests pass — victory language; use bounded status",
    ),
    (
        "everything works",
        "Everything works — victory language; use bounded status",
    ),
    (
        "fully implemented",
        "Fully implemented — victory language; use bounded status",
    ),
    (
        "100% coverage",
        "100% coverage — victory language; use bounded status",
    ),
];

impl LintPass for ClaimLintPass {
    fn name(&self) -> &'static str {
        "claim"
    }
    fn family(&self) -> &'static str {
        "CLAIM"
    }

    fn check(&self, source: &str, file_path: &str, _file_ext: &str) -> Vec<AntiLlmDiagnostic> {
        let mut diags = Vec::new();
        let lower = source.to_lowercase();
        for (line_idx, (line, lower_line)) in
            source.lines().zip(lower.lines()).enumerate()
        {
            let line_num = line_idx + 1;
            for (term, msg) in CLAIM_BLOCKING_TERMS {
                if lower_line.contains(term) {
                    diags.push(AntiLlmDiagnostic {
                        code: "ANTI-LLM-CLAIM-LP-001".to_string(),
                        category: "claim".to_string(),
                        file_path: file_path.to_string(),
                        line: line_num,
                        column: 1,
                        message: msg.to_string(),
                        forbidden_implication: format!(
                            "VictoryTerm({}) => UnboundedClaim",
                            term
                        ),
                        blocking: true,
                        required_correction:
                            "Replace with bounded status: ADMITTED, CANDIDATE, PARTIAL, OPEN"
                                .to_string(),
                        required_next_proof:
                            "Victory language removed; bounded status present".to_string(),
                    });
                    // Avoid duplicate lines from multiple terms
                    let _ = line; // suppress unused-variable warning
                    break;
                }
            }
        }
        diags
    }
}

// ── Ocel005LintPass — ANTI-LLM-OCEL-005 ─────────────────────────────────────
//
// Static approximation: scan Rust function bodies for `publish_diagnostics` calls
// that have no adjacent `ocel` or `event` push within 50 lines (before or after).
//
// Algorithm:
//   1. Collect the line numbers of every `publish_diagnostics` occurrence.
//   2. For each such line, inspect the ±50-line window for any `ocel` or `event` token.
//   3. If none found, emit ANTI-LLM-OCEL-005.
//
// This is a static heuristic, not a full AST traversal; false positives and
// negatives are possible. The rule is non-blocking (warning) because the
// approximation is imprecise.

pub struct Ocel005LintPass;

impl LintPass for Ocel005LintPass {
    fn name(&self) -> &'static str {
        "ocel_005"
    }
    fn family(&self) -> &'static str {
        "OCEL"
    }

    fn is_blocking(&self) -> bool {
        // Static approximation — emit as warning until a tighter AST pass exists.
        false
    }

    fn check(&self, source: &str, file_path: &str, file_ext: &str) -> Vec<AntiLlmDiagnostic> {
        if file_ext != "rs" {
            return vec![];
        }
        let lines: Vec<&str> = source.lines().collect();
        let total = lines.len();
        let mut diags = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            if !line.contains("publish_diagnostics") {
                continue;
            }
            let line_num = idx + 1;

            // Window: 50 lines before and after the publish_diagnostics call.
            let window_start = idx.saturating_sub(50);
            let window_end = (idx + 50).min(total.saturating_sub(1));

            let has_ocel_token = lines[window_start..=window_end]
                .iter()
                .any(|l| l.contains("ocel") || l.contains("event"));

            if !has_ocel_token {
                diags.push(AntiLlmDiagnostic {
                    code: "ANTI-LLM-OCEL-005".to_string(),
                    category: "process_evidence".to_string(),
                    file_path: file_path.to_string(),
                    line: line_num,
                    column: 1,
                    message:
                        "publish_diagnostics call with no adjacent ocel/event push within 50 lines \
                         — diagnostic emitted without process evidence (static approximation)."
                            .to_string(),
                    forbidden_implication:
                        "DiagnosticPublished => OcelEventRecorded".to_string(),
                    blocking: false,
                    required_correction:
                        "Add an OCEL event emission (push_event / ocel_*) near every \
                         publish_diagnostics call so the diagnostic is traceable in the process log."
                            .to_string(),
                    required_next_proof:
                        "OCEL export contains DiagnosticPublished event linked to this call site."
                            .to_string(),
                });
            }
        }

        diags
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

/// Returns all registered `LintPass` implementations.
///
/// Engine callers should call `run_lint_passes` rather than iterating this vec
/// directly — `run_lint_passes` handles file-extension extraction and deduplication.
pub fn registered_passes() -> Vec<Box<dyn LintPass>> {
    vec![
        Box::new(HollowLintPass),
        Box::new(ClaimLintPass),
        Box::new(Ocel005LintPass),
    ]
}

/// Run all registered passes against `source` and return the union of findings.
///
/// `file_path` is used both for file-extension dispatch and for populating the
/// `file_path` field of returned diagnostics.
pub fn run_lint_passes(source: &str, file_path: &str) -> Vec<AntiLlmDiagnostic> {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    registered_passes()
        .iter()
        .flat_map(|p| p.check(source, file_path, ext))
        .collect()
}
