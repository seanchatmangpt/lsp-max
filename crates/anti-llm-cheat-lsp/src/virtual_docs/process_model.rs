//! `anti-llm://process-model` virtual document.
//!
//! Renders the live Directly-Follows Graph (DFG) and Declare conformance report
//! for the anti-llm detection pipeline, derived from the current set of
//! `AntiLlmDiagnostic` observations.
//!
//! Process mining theory: W.M.P. van der Aalst, "Process Mining: Data Science in
//! Action" (2nd ed., 2016). The DFG is the simplest discovery primitive — two
//! activities A and B are connected by an arc A → B with frequency count f(A→B).

use crate::diagnostics::AntiLlmDiagnostic;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Activity extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Map a diagnostic to its abstract activity name in the detection pipeline.
fn activity_of(d: &AntiLlmDiagnostic) -> &'static str {
    if d.code.starts_with("ANTI-LLM-VICTORY") || d.code.starts_with("ANTI-LLM-CLAIMS") {
        "VictoryLanguageDetected"
    } else if d.code.starts_with("ANTI-LLM-RECEIPT") {
        "FakeReceiptDetected"
    } else if d.code.starts_with("ANTI-LLM-ROUTE") {
        "FakeRouteDetected"
    } else if d.code.starts_with("ANTI-LLM-VERSION") {
        "VersionViolationDetected"
    } else if d.code.starts_with("ANTI-LLM-TOWER") || d.code.starts_with("ANTI-LLM-LSP") {
        "ForbiddenRefDetected"
    } else if d.code.starts_with("WASM4PM") {
        "ProcessViolationDetected"
    } else if d.code.starts_with("GGEN") {
        "GgenViolationDetected"
    } else {
        "CheatDetected"
    }
}

/// Build per-case traces from diagnostics.
/// Case = file_path.  Trace = sorted activities for that file, ending with
/// a synthetic `ScanComplete` event that every conformant trace must have.
fn build_traces(diagnostics: &[AntiLlmDiagnostic]) -> HashMap<String, Vec<String>> {
    let mut case_activities: HashMap<String, Vec<String>> = HashMap::new();
    for d in diagnostics {
        case_activities
            .entry(d.file_path.clone())
            .or_default()
            .push(activity_of(d).to_string());
    }
    // Append ScanComplete to every case — the normative terminal activity.
    for activities in case_activities.values_mut() {
        activities.push("ScanComplete".to_string());
    }
    // Synthetic catch-all case if no diagnostics.
    if case_activities.is_empty() {
        case_activities
            .entry("_workspace".to_string())
            .or_default()
            .push("ScanComplete".to_string());
    }
    case_activities
}

// ─────────────────────────────────────────────────────────────────────────────
// DFG (inline, no external crate dep)
// ─────────────────────────────────────────────────────────────────────────────

struct Dfg {
    nodes: HashMap<String, usize>,
    edges: HashMap<(String, String), usize>,
    start_activities: HashMap<String, usize>,
    end_activities: HashMap<String, usize>,
}

fn build_dfg(traces: &HashMap<String, Vec<String>>) -> Dfg {
    let mut nodes: HashMap<String, usize> = HashMap::new();
    let mut edges: HashMap<(String, String), usize> = HashMap::new();
    let mut start_activities: HashMap<String, usize> = HashMap::new();
    let mut end_activities: HashMap<String, usize> = HashMap::new();

    for trace in traces.values() {
        if trace.is_empty() {
            continue;
        }
        *start_activities.entry(trace[0].clone()).or_insert(0) += 1;
        *end_activities.entry(trace[trace.len() - 1].clone()).or_insert(0) += 1;
        for act in trace {
            *nodes.entry(act.clone()).or_insert(0) += 1;
        }
        for pair in trace.windows(2) {
            *edges
                .entry((pair[0].clone(), pair[1].clone()))
                .or_insert(0) += 1;
        }
    }
    Dfg {
        nodes,
        edges,
        start_activities,
        end_activities,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Declare conformance (inline)
// ─────────────────────────────────────────────────────────────────────────────

struct Violation {
    constraint: &'static str,
    case_id: String,
    detail: String,
}

fn check_conformance(traces: &HashMap<String, Vec<String>>) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (case_id, trace) in traces {
        // responded_existence(CheatDetected*, ScanComplete)
        let has_cheat = trace
            .iter()
            .any(|a| a.ends_with("Detected") && a != "ScanComplete");
        if has_cheat && !trace.contains(&"ScanComplete".to_string()) {
            violations.push(Violation {
                constraint: "responded_existence(Detected, ScanComplete)",
                case_id: case_id.clone(),
                detail: "detection event not followed by ScanComplete".to_string(),
            });
        }

        // absence(VictoryLanguageEmitted) — victory language is forbidden in output
        if trace.contains(&"VictoryLanguageDetected".to_string()) {
            violations.push(Violation {
                constraint: "absence(VictoryLanguageEmitted)",
                case_id: case_id.clone(),
                detail: "victory language detected in case".to_string(),
            });
        }
    }
    violations
}

// ─────────────────────────────────────────────────────────────────────────────
// Mermaid rendering
// ─────────────────────────────────────────────────────────────────────────────

fn mermaid_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn render_mermaid(dfg: &Dfg) -> String {
    let mut md = String::from("```mermaid\nflowchart LR\n");

    let mut nodes: Vec<(&String, &usize)> = dfg.nodes.iter().collect();
    nodes.sort_by_key(|(n, _)| n.as_str());
    for (name, count) in &nodes {
        md.push_str(&format!(
            "  {}[\"{}\\n(n={count})\"]\n",
            mermaid_id(name),
            name
        ));
    }

    let mut starts: Vec<(&String, &usize)> = dfg.start_activities.iter().collect();
    starts.sort_by_key(|(n, _)| n.as_str());
    for (act, freq) in &starts {
        md.push_str(&format!("  START((▶)) -->|{freq}| {}\n", mermaid_id(act)));
    }

    let mut ends: Vec<(&String, &usize)> = dfg.end_activities.iter().collect();
    ends.sort_by_key(|(n, _)| n.as_str());
    for (act, freq) in &ends {
        md.push_str(&format!("  {} -->|{freq}| END(((◼)))\n", mermaid_id(act)));
    }

    let mut edges: Vec<(&(String, String), &usize)> = dfg.edges.iter().collect();
    edges.sort_by_key(|((a, b), _)| (a.as_str(), b.as_str()));
    for ((from, to), freq) in &edges {
        md.push_str(&format!(
            "  {} -->|{freq}| {}\n",
            mermaid_id(from),
            mermaid_id(to)
        ));
    }

    md.push_str("```\n");
    md
}

// ─────────────────────────────────────────────────────────────────────────────
// Public render function
// ─────────────────────────────────────────────────────────────────────────────

/// Render the process model virtual document for `anti-llm://process-model`.
///
/// Returns a markdown document containing:
/// 1. DFG summary (nodes, edges, transitions)
/// 2. Mermaid flowchart of the DFG
/// 3. Declare conformance report
/// 4. Fitness score against the normative anti-llm detection model
pub fn render(diagnostics: &[AntiLlmDiagnostic]) -> String {
    let traces = build_traces(diagnostics);
    let dfg = build_dfg(&traces);
    let violations = check_conformance(&traces);

    let case_count = traces.len();
    let total_transitions: usize = dfg.edges.values().sum();

    // Fitness: fraction of cases with no violations.
    let violating_cases: std::collections::HashSet<&str> =
        violations.iter().map(|v| v.case_id.as_str()).collect();
    let conformant_cases = case_count.saturating_sub(violating_cases.len());
    let fitness = if case_count == 0 {
        1.0_f64
    } else {
        conformant_cases as f64 / case_count as f64
    };
    let fitness_pct = (fitness * 100.0).round() as u32;

    let conformance_status = if violations.is_empty() {
        "CANDIDATE"
    } else {
        "PARTIAL"
    };

    let mut md = format!(
        "# Anti-LLM Detection Process Model\n\n\
         **Status:** {conformance_status}  \n\
         **Cases (files):** {case_count}  \n\
         **Activities (nodes):** {}  \n\
         **Arcs (edges):** {}  \n\
         **Transitions:** {total_transitions}  \n\
         **Fitness:** {fitness_pct}%  \n\n\
         > DFG extracted from {} live diagnostic observations via Van der Aalst DFG algorithm.\n\n",
        dfg.nodes.len(),
        dfg.edges.len(),
        diagnostics.len()
    );

    md.push_str("## Directly-Follows Graph\n\n");
    md.push_str(&render_mermaid(&dfg));
    md.push('\n');

    md.push_str("## Declare Conformance Report\n\n");
    if violations.is_empty() {
        md.push_str("All traces conform to the normative detection model. Status: CANDIDATE\n\n");
    } else {
        md.push_str(&format!(
            "**{} violation(s) detected** across {} case(s). Status: PARTIAL\n\n",
            violations.len(),
            violating_cases.len()
        ));
        md.push_str("| Constraint | Case | Detail |\n");
        md.push_str("|---|---|---|\n");
        for v in &violations {
            let short_case = v.case_id.split('/').next_back().unwrap_or(&v.case_id);
            md.push_str(&format!(
                "| `{}` | `{}` | {} |\n",
                v.constraint, short_case, v.detail
            ));
        }
        md.push('\n');
    }

    md.push_str("## Normative Model (Declare)\n\n");
    md.push_str("The anti-llm detection pipeline must satisfy:\n\n");
    md.push_str("| # | Constraint | Meaning |\n");
    md.push_str("|---|---|---|\n");
    md.push_str("| 1 | `responded_existence(Detected, ScanComplete)` | Every detection must co-occur with a ScanComplete in the case |\n");
    md.push_str("| 2 | `absence(VictoryLanguageEmitted)` | Victory language is forbidden in all detection output |\n");
    md.push_str("| 3 | `init(ScanComplete)` | ScanComplete is the canonical terminal activity per case |\n");
    md.push('\n');

    md.push_str("## Activity Legend\n\n");
    md.push_str("| Activity | Source code prefix |\n");
    md.push_str("|---|---|\n");
    md.push_str("| `VictoryLanguageDetected` | `ANTI-LLM-VICTORY-*`, `ANTI-LLM-CLAIMS-*` |\n");
    md.push_str("| `FakeReceiptDetected` | `ANTI-LLM-RECEIPT-*` |\n");
    md.push_str("| `FakeRouteDetected` | `ANTI-LLM-ROUTE-*` |\n");
    md.push_str("| `VersionViolationDetected` | `ANTI-LLM-VERSION-*` |\n");
    md.push_str("| `ForbiddenRefDetected` | `ANTI-LLM-TOWER-*`, `ANTI-LLM-LSP-*` |\n");
    md.push_str("| `ProcessViolationDetected` | `WASM4PM-*` |\n");
    md.push_str("| `GgenViolationDetected` | `GGEN-*` |\n");
    md.push_str("| `CheatDetected` | (all other codes) |\n");
    md.push_str("| `ScanComplete` | Synthetic terminal — added to every case |\n");

    md
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_diag(code: &str, file_path: &str, line: usize) -> AntiLlmDiagnostic {
        AntiLlmDiagnostic {
            code: code.to_string(),
            category: "test-category".to_string(),
            file_path: file_path.to_string(),
            line,
            column: 1,
            message: "test message".to_string(),
            forbidden_implication: "none".to_string(),
            blocking: false,
            required_correction: "none".to_string(),
            required_next_proof: "none".to_string(),
        }
    }

    // ── activity_of mapping ──────────────────────────────────────────────────

    #[test]
    fn activity_of_victory_prefix() {
        let d = make_diag("ANTI-LLM-VICTORY-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "VictoryLanguageDetected");
    }

    #[test]
    fn activity_of_claims_prefix() {
        let d = make_diag("ANTI-LLM-CLAIMS-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "VictoryLanguageDetected");
    }

    #[test]
    fn activity_of_receipt_prefix() {
        let d = make_diag("ANTI-LLM-RECEIPT-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "FakeReceiptDetected");
    }

    #[test]
    fn activity_of_route_prefix() {
        let d = make_diag("ANTI-LLM-ROUTE-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "FakeRouteDetected");
    }

    #[test]
    fn activity_of_version_prefix() {
        let d = make_diag("ANTI-LLM-VERSION-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "VersionViolationDetected");
    }

    #[test]
    fn activity_of_tower_prefix() {
        let d = make_diag("ANTI-LLM-TOWER-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "ForbiddenRefDetected");
    }

    #[test]
    fn activity_of_lsp_prefix() {
        let d = make_diag("ANTI-LLM-LSP-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "ForbiddenRefDetected");
    }

    #[test]
    fn activity_of_wasm4pm_prefix() {
        let d = make_diag("WASM4PM-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "ProcessViolationDetected");
    }

    #[test]
    fn activity_of_ggen_prefix() {
        let d = make_diag("GGEN-001", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "GgenViolationDetected");
    }

    #[test]
    fn activity_of_unknown_falls_to_cheat() {
        let d = make_diag("SOME-UNKNOWN-CODE", "src/foo.rs", 1);
        assert_eq!(activity_of(&d), "CheatDetected");
    }

    // ── build_traces ─────────────────────────────────────────────────────────

    #[test]
    fn build_traces_empty_diagnostics_produces_workspace_case() {
        let traces = build_traces(&[]);
        assert!(traces.contains_key("_workspace"));
        let acts = &traces["_workspace"];
        assert_eq!(acts, &["ScanComplete"]);
    }

    #[test]
    fn build_traces_appends_scan_complete_to_every_case() {
        let diags = vec![
            make_diag("ANTI-LLM-VICTORY-001", "src/a.rs", 1),
            make_diag("ANTI-LLM-RECEIPT-001", "src/b.rs", 2),
        ];
        let traces = build_traces(&diags);
        for acts in traces.values() {
            assert_eq!(
                acts.last().map(|s| s.as_str()),
                Some("ScanComplete"),
                "every case must end with ScanComplete"
            );
        }
    }

    #[test]
    fn build_traces_groups_by_file_path() {
        let diags = vec![
            make_diag("ANTI-LLM-VICTORY-001", "src/a.rs", 1),
            make_diag("ANTI-LLM-RECEIPT-001", "src/a.rs", 5),
            make_diag("GGEN-001", "src/b.rs", 3),
        ];
        let traces = build_traces(&diags);
        assert_eq!(traces.len(), 2, "two distinct file paths must produce two cases");
        let a_trace = &traces["src/a.rs"];
        // two detection activities plus the appended ScanComplete
        assert_eq!(a_trace.len(), 3);
        assert_eq!(a_trace.last().map(|s| s.as_str()), Some("ScanComplete"));
    }

    // ── build_dfg ────────────────────────────────────────────────────────────

    #[test]
    fn build_dfg_single_activity_trace_has_no_edges() {
        let mut traces = HashMap::new();
        traces.insert("case1".to_string(), vec!["ScanComplete".to_string()]);
        let dfg = build_dfg(&traces);
        assert_eq!(dfg.nodes.len(), 1);
        assert!(dfg.edges.is_empty(), "single-node trace must have no edges");
        assert!(dfg.start_activities.contains_key("ScanComplete"));
        assert!(dfg.end_activities.contains_key("ScanComplete"));
    }

    #[test]
    fn build_dfg_two_activity_trace_produces_one_edge() {
        let mut traces = HashMap::new();
        traces.insert(
            "case1".to_string(),
            vec!["VictoryLanguageDetected".to_string(), "ScanComplete".to_string()],
        );
        let dfg = build_dfg(&traces);
        assert_eq!(dfg.nodes.len(), 2);
        assert_eq!(dfg.edges.len(), 1);
        let key = ("VictoryLanguageDetected".to_string(), "ScanComplete".to_string());
        assert_eq!(dfg.edges.get(&key), Some(&1));
    }

    // ── check_conformance ─────────────────────────────────────────────────────

    #[test]
    fn check_conformance_empty_traces_no_violations() {
        let traces = build_traces(&[]);
        let violations = check_conformance(&traces);
        assert!(violations.is_empty(), "synthetic _workspace case must be conformant");
    }

    #[test]
    fn check_conformance_victory_language_yields_absence_violation() {
        let diags = vec![make_diag("ANTI-LLM-VICTORY-001", "src/a.rs", 1)];
        let traces = build_traces(&diags);
        let violations = check_conformance(&traces);
        let has_absence = violations
            .iter()
            .any(|v| v.constraint.contains("absence(VictoryLanguageEmitted)"));
        assert!(
            has_absence,
            "VictoryLanguageDetected must trigger absence(VictoryLanguageEmitted) violation"
        );
    }

    #[test]
    fn check_conformance_receipt_detection_with_scan_complete_is_conformant() {
        // responded_existence is satisfied because build_traces appends ScanComplete.
        // No victory language → absence constraint is also satisfied.
        let diags = vec![make_diag("ANTI-LLM-RECEIPT-001", "src/a.rs", 1)];
        let traces = build_traces(&diags);
        let violations = check_conformance(&traces);
        let has_responded = violations
            .iter()
            .any(|v| v.constraint.contains("responded_existence"));
        assert!(
            !has_responded,
            "receipt detection with ScanComplete appended must not violate responded_existence"
        );
    }

    // ── render (public function) ───────────────────────────────────────────────

    #[test]
    fn render_empty_state_returns_non_empty_markdown() {
        let doc = render(&[]);
        assert!(!doc.is_empty());
    }

    #[test]
    fn render_empty_state_contains_required_sections() {
        let doc = render(&[]);
        assert!(doc.contains("# Anti-LLM Detection Process Model"), "missing H1");
        assert!(doc.contains("## Directly-Follows Graph"), "missing DFG section");
        assert!(
            doc.contains("## Declare Conformance Report"),
            "missing Declare section"
        );
        assert!(doc.contains("## Activity Legend"), "missing Activity Legend section");
    }

    #[test]
    fn render_empty_state_reports_candidate_status() {
        let doc = render(&[]);
        assert!(
            doc.contains("**Status:** CANDIDATE"),
            "empty state must report CANDIDATE status"
        );
    }

    #[test]
    fn render_mermaid_block_is_well_formed() {
        let doc = render(&[]);
        assert!(doc.contains("```mermaid"), "Mermaid opening fence must be present");
        assert!(doc.contains("flowchart LR"), "flowchart LR declaration must be present");
        // The closing fence ends the mermaid block
        assert!(doc.contains("```\n"), "Mermaid closing fence must be present");
    }

    #[test]
    fn render_with_victory_language_reports_partial_status() {
        let diags = vec![make_diag("ANTI-LLM-VICTORY-001", "src/foo.rs", 1)];
        let doc = render(&diags);
        assert!(
            doc.contains("**Status:** PARTIAL"),
            "victory language detection must produce PARTIAL status"
        );
    }

    #[test]
    fn render_with_diagnostics_includes_activity_nodes() {
        let diags = vec![
            make_diag("ANTI-LLM-RECEIPT-001", "src/a.rs", 1),
            make_diag("GGEN-001", "src/b.rs", 2),
        ];
        let doc = render(&diags);
        assert!(
            doc.contains("FakeReceiptDetected"),
            "FakeReceiptDetected activity must appear in output"
        );
        assert!(
            doc.contains("GgenViolationDetected"),
            "GgenViolationDetected activity must appear in output"
        );
        assert!(
            doc.contains("ScanComplete"),
            "ScanComplete terminal must appear in output"
        );
    }

    #[test]
    fn render_activity_legend_covers_all_categories() {
        let doc = render(&[]);
        for activity in &[
            "VictoryLanguageDetected",
            "FakeReceiptDetected",
            "FakeRouteDetected",
            "VersionViolationDetected",
            "ForbiddenRefDetected",
            "ProcessViolationDetected",
            "GgenViolationDetected",
            "CheatDetected",
            "ScanComplete",
        ] {
            assert!(
                doc.contains(activity),
                "Activity legend must include {activity}"
            );
        }
    }

    #[test]
    fn render_fitness_is_100_with_no_diagnostics() {
        let doc = render(&[]);
        assert!(
            doc.contains("**Fitness:** 100%"),
            "empty state must report 100% fitness"
        );
    }

    #[test]
    fn render_violation_table_lists_short_case_name() {
        let diags = vec![make_diag("ANTI-LLM-VICTORY-001", "src/cheat.rs", 3)];
        let doc = render(&diags);
        // next_back() on split('/') yields the last path segment
        assert!(
            doc.contains("cheat.rs"),
            "violation table must show the short file name (last path segment)"
        );
    }

    // ── mermaid_id ────────────────────────────────────────────────────────────

    #[test]
    fn mermaid_id_sanitizes_non_alphanumeric() {
        assert_eq!(mermaid_id("hello world"), "hello_world");
        assert_eq!(mermaid_id("anti-llm"), "anti_llm");
    }

    #[test]
    fn mermaid_id_preserves_alphanumeric_and_underscore() {
        assert_eq!(mermaid_id("ScanComplete_123"), "ScanComplete_123");
    }
}
