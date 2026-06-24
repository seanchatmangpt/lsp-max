// Dogfood coverage for three API-reconciliation changes:
//
// A) capabilities.rs — type_hierarchy_provider no-op arms
//    (lsp_types_max ServerCapabilities has no type_hierarchy_provider field;
//    the three typeHierarchy methods are now explicit no-op match arms.)
//
// B) virtual_docs/process_model.rs — .next_back() clippy fix
//    (case-id shortening now calls Iterator::next_back() on a Split iterator
//    rather than .last(), which consumed the full iterator unnecessarily.)
//
// C) diagnostic detection regression — victory language, forbidden ref, fake receipt
//    (these are exercised inline via tempfile to confirm the engine paths are
//    reachable and produce the expected diagnostic codes.)

use anti_llm_cheat_lsp::{capabilities, diagnostics::AntiLlmDiagnostic, engine, virtual_docs};
use std::io::Write as _;

// ─────────────────────────────────────────────────────────────────────────────
// Group A — capabilities: type_hierarchy no-op arms
// ─────────────────────────────────────────────────────────────────────────────

/// `build_capabilities()` must not panic; the three typeHierarchy methods are
/// no-op arms in the match.  The only observable invariant is that the function
/// returns without panic AND does not set a call_hierarchy_provider for the
/// type-hierarchy slot (which has no typed field in this lsp-types-max version).
#[test]
fn type_hierarchy_arms_do_not_panic() {
    // Calling build_capabilities exercises every arm in the matrix loop,
    // including the three no-op typeHierarchy arms.
    let caps = capabilities::build_capabilities();

    // The call-hierarchy provider (a distinct field) must still be wired —
    // its arms are active and the type_hierarchy arms must not have clobbered it.
    assert!(
        caps.call_hierarchy_provider.is_some(),
        "call_hierarchy_provider must be Some after build_capabilities (typeHierarchy no-ops must not clobber it)"
    );
}

/// After build_capabilities the text_document_sync must be present —
/// this confirms the loop ran to completion past the typeHierarchy no-op arms.
#[test]
fn capabilities_loop_runs_past_type_hierarchy_no_ops() {
    let caps = capabilities::build_capabilities();
    assert!(
        caps.text_document_sync.is_some(),
        "text_document_sync must be advertised: loop must have run past typeHierarchy no-op arms"
    );
    assert!(
        caps.diagnostic_provider.is_some(),
        "diagnostic_provider must be advertised: loop must have processed all arms"
    );
}

/// The three typeHierarchy method strings must appear in the coverage surface
/// exactly as the no-op arms in capabilities.rs expect.
#[test]
fn type_hierarchy_methods_present_in_coverage_surface() {
    use anti_llm_cheat_lsp::rules::lsp318_coverage::full_surface;

    let surface = full_surface();
    let methods: Vec<&str> = surface.iter().map(|m| m.method).collect();

    for required in &[
        "textDocument/prepareTypeHierarchy",
        "typeHierarchy/supertypes",
        "typeHierarchy/subtypes",
    ] {
        assert!(
            methods.contains(required),
            "method '{}' must be in the coverage surface so the no-op arm is reachable",
            required
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Group B — process_model virtual doc: .next_back() smoke test
// ─────────────────────────────────────────────────────────────────────────────

/// Render with an empty diagnostic slice — the synthetic _workspace case must
/// appear, and the function must not panic on .next_back() with an empty path.
#[test]
fn process_model_renders_with_empty_diagnostics() {
    let result = virtual_docs::process_model::render(&[]);
    assert!(
        !result.is_empty(),
        "render must return a non-empty string for empty diagnostics"
    );
    assert!(
        result.contains("ScanComplete"),
        "synthetic terminal ScanComplete must appear in the rendered document"
    );
    assert!(
        result.contains("CANDIDATE"),
        "conformance status must be CANDIDATE when there are no violations"
    );
}

/// Render with a multi-segment file path that exercises the .next_back() split.
///
/// The .next_back() call (the clippy fix from .last()) shortens the case_id in the
/// Declare violation table.  To force a violation we need a code that maps to
/// VictoryLanguageDetected, which requires a prefix of "ANTI-LLM-VICTORY" or
/// "ANTI-LLM-CLAIMS".  With a violation present, the table row is rendered and
/// we can verify the filename segment is extracted correctly.
#[test]
fn process_model_renders_with_slash_path_case_id() {
    let diag = AntiLlmDiagnostic {
        // ANTI-LLM-VICTORY prefix → VictoryLanguageDetected → absence violation fires
        code: "ANTI-LLM-VICTORY-001".to_string(),
        category: "claims".to_string(),
        // Multi-segment path — .next_back() shortens this to "Cargo.toml" in the table.
        file_path: "crates/anti-llm-cheat-lsp/Cargo.toml".to_string(),
        line: 1,
        column: 1,
        message: "victory language detected".to_string(),
        forbidden_implication: "victory => BLOCKED".to_string(),
        blocking: false,
        required_correction: "use bounded status".to_string(),
        required_next_proof: "verify no victory language".to_string(),
    };

    let result = virtual_docs::process_model::render(std::slice::from_ref(&diag));
    assert!(
        !result.is_empty(),
        "render must return content for a single diagnostic"
    );
    assert!(
        result.contains("VictoryLanguageDetected"),
        "ANTI-LLM-VICTORY-001 must map to VictoryLanguageDetected activity"
    );
    // The case shortening via .next_back() must produce the filename segment in the
    // violation table (the table only appears when violations are present).
    assert!(
        result.contains("Cargo.toml"),
        "case-id shortening via .next_back() must yield the filename segment in the violation table"
    );
}

/// Render with a victory-language diagnostic — the Declare absence constraint
/// must fire and the document status must be PARTIAL, not CANDIDATE.
///
/// `activity_of` maps codes with prefix "ANTI-LLM-VICTORY" or "ANTI-LLM-CLAIMS"
/// to `VictoryLanguageDetected`; the absence constraint then fires.
#[test]
fn process_model_partial_status_when_victory_language_detected() {
    let diag = AntiLlmDiagnostic {
        // Must start with "ANTI-LLM-VICTORY" to map to VictoryLanguageDetected.
        code: "ANTI-LLM-VICTORY-001".to_string(),
        category: "claims".to_string(),
        file_path: "report.md".to_string(),
        line: 2,
        column: 1,
        message: "victory language detected".to_string(),
        forbidden_implication: "victory => BLOCKED".to_string(),
        blocking: false,
        required_correction: "use bounded status".to_string(),
        required_next_proof: "receipt required".to_string(),
    };

    let result = virtual_docs::process_model::render(std::slice::from_ref(&diag));
    assert!(
        result.contains("PARTIAL"),
        "status must be PARTIAL when a VictoryLanguageDetected violation is present"
    );
    assert!(
        result.contains("absence(VictoryLanguageEmitted)"),
        "Declare absence constraint must appear in the conformance table"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Group C — diagnostic detection: victory language, forbidden ref, fake receipt
// ─────────────────────────────────────────────────────────────────────────────
//
// These use tempfile to exercise the engine::scan_file → evaluate_diagnostics
// pipeline with inline content.  They are negative-control confirmations that
// the detection paths remain reachable after the API reconciliation pass.

fn check_diag_code(diags: &[anti_llm_cheat_lsp::diagnostics::AntiLlmDiagnostic], code: &str) {
    assert!(
        diags.iter().any(|d| d.code == code),
        "expected diagnostic code '{}' not found in {:?}",
        code,
        diags.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

/// A file containing "all clean" must produce ANTI-LLM-CLAIM-004.
/// This is the inline negative-control confirming the victory-language path is active.
#[test]
fn detects_victory_language_inline() {
    let mut f = tempfile::Builder::new()
        .suffix(".md")
        .tempfile()
        .expect("tempfile creation failed");
    writeln!(f, "# Status\n\nall clean — BLOCKED items resolved.").unwrap();

    let obs = engine::scan_file(&f.path().to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-CLAIM-004");
}

/// A Cargo.toml file with a `tower-lsp` dependency must produce ANTI-LLM-SURFACE-001.
/// This confirms the forbidden-reference detection path survives the API change.
#[test]
fn detects_forbidden_ref_inline() {
    let mut f = tempfile::Builder::new()
        .suffix(".toml")
        .tempfile()
        .expect("tempfile creation failed");
    writeln!(
        f,
        "[package]\nname = \"test\"\nversion = \"26.6.21\"\n\n[dependencies]\ntower-lsp = \"0.20\""
    )
    .unwrap();

    let obs = engine::scan_file(&f.path().to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-SURFACE-001");
}

/// A file with `test result: ok` (fake stdout-as-receipt pattern) must produce
/// ANTI-LLM-RECEIPT-001, confirming the receipt detection path is active.
#[test]
fn detects_fake_receipt_inline() {
    let mut f = tempfile::Builder::new()
        .suffix(".md")
        .tempfile()
        .expect("tempfile creation failed");
    writeln!(
        f,
        "# Test run\n\ntest result: ok. 3 passed; 0 failed; finished in 0.01s\n\nThis proves admission."
    )
    .unwrap();

    let obs = engine::scan_file(&f.path().to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-RECEIPT-001");
}
