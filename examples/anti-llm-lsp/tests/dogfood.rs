use anti_llm_lsp::engine;
use anti_llm_lsp::rules::lsp318::{get_feature_matrix, Lsp318Feature};
use std::fs;
use std::path::PathBuf;

fn find_file_path(suffix: &str) -> PathBuf {
    let p1 = PathBuf::from(suffix);
    if p1.exists() {
        return p1;
    }
    let p2 = PathBuf::from("examples/anti-llm-lsp").join(suffix);
    if p2.exists() {
        return p2;
    }
    panic!("Could not locate file path: {}", suffix);
}

fn check_diag_code(diags: &[anti_llm_lsp::diagnostics::AntiLlmDiagnostic], expected: &str) {
    let mut found = false;
    for d in diags {
        if d.code == expected {
            found = true;
            break;
        }
    }
    if !found {
        panic!("Expected diagnostic code {} was not found!", expected);
    }
}

// -------------------------------------------------------------
// 11 V0 Tests
// -------------------------------------------------------------

#[test]
fn no_tower_lsp_lock() {
    let path = find_file_path("src/server.rs");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    for d in &diags {
        if d.code == "ANTI-LLM-SURFACE-001" {
            panic!("Found plain tower-lsp diagnostic in clean file server.rs");
        }
    }
}

#[test]
fn detects_tower_lsp_negative_fixture() {
    let path = find_file_path("fixtures/negative_controls/tower_lsp_dependency/Cargo.toml");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-SURFACE-001");
}

#[test]
fn detects_fake_clap_abstraction() {
    let path = find_file_path("fixtures/negative_controls/fake_clap_report.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-AUTH-002");
}

#[test]
fn detects_route_log_not_route_proof() {
    let path = find_file_path("fixtures/negative_controls/route_log_not_route_proof.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-ROUTE-001");
}

#[test]
fn detects_test_stdout_not_receipt() {
    let path = find_file_path("fixtures/negative_controls/test_stdout_not_receipt.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-RECEIPT-001");
}

#[test]
fn detects_debug_diagnostic_leak() {
    let path = find_file_path("fixtures/negative_controls/debug_diagnostic_leak.rs");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-STRANGE-001");
    check_diag_code(&diags, "ANTI-LLM-STRANGE-002");
    check_diag_code(&diags, "ANTI-LLM-STRANGE-003");
}

#[test]
fn detects_string_authority() {
    let path = find_file_path("fixtures/negative_controls/string_authority.rs");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-STRANGE-007");
}

#[test]
fn detects_workspace_edit_receipt_binding() {
    let path = find_file_path("fixtures/negative_controls/workspace_edit_receipt_binding.rs");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-MUT-002");
}

#[test]
fn detects_victory_language_with_failset() {
    let path_vic = find_file_path("fixtures/negative_controls/victory_language_with_failset.md");
    let path_dep = find_file_path("fixtures/negative_controls/tower_lsp_dependency/Cargo.toml");

    let mut obs = engine::scan_file(&path_vic.to_string_lossy());
    obs.extend(engine::scan_file(&path_dep.to_string_lossy()));

    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-CLAIM-004");
}

#[test]
fn detects_bad_version() {
    let path = find_file_path("fixtures/negative_controls/bad_version/Cargo.toml");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-VERSION-001");
}

#[test]
fn detects_static_scan_as_route_proof() {
    let path = find_file_path("fixtures/negative_controls/static_scan_as_route_proof.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-ROUTE-008");
}

// -------------------------------------------------------------
// 19 LSP 3.18 Delta Changelog Coverage Proof Tests
// -------------------------------------------------------------

fn verify_lsp318_feature(feature_id: &str, expected_status: &str) -> Lsp318Feature {
    let matrix = get_feature_matrix();
    for f in matrix {
        if f.feature_id == feature_id {
            if f.status != expected_status {
                panic!(
                    "Feature {} status expected {}, got {}",
                    feature_id, expected_status, f.status
                );
            }
            return f;
        }
    }
    panic!("Feature {} not found in matrix", feature_id);
}

#[test]
fn lsp318_001_inline_completion_safe_status() {
    let f = verify_lsp318_feature("LSP318-001", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_002_dynamic_text_document_content_virtual_docs() {
    let f = verify_lsp318_feature("LSP318-002", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_003_folding_range_refresh_failset() {
    let f = verify_lsp318_feature("LSP318-003", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_004_multi_range_formatting_failset() {
    let f = verify_lsp318_feature("LSP318-004", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_005_workspace_edit_snippets_non_authority() {
    let f = verify_lsp318_feature("LSP318-005", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_006_relative_pattern_document_filters() {
    let f = verify_lsp318_feature("LSP318-006", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_007_relative_pattern_notebook_filters_or_refusal() {
    let f = verify_lsp318_feature("LSP318-007", "REFUSED_BY_LAW_WITH_RECEIPT");
    let receipt_path = find_file_path(&f.receipt_path);
    let content = fs::read_to_string(receipt_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    let status = val.get("status").and_then(|s| s.as_str()).unwrap();
    if status != "REFUSED" {
        panic!("Expected status REFUSED, got {}", status);
    }
}

#[test]
fn lsp318_008_code_action_kind_documentation() {
    let f = verify_lsp318_feature("LSP318-008", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_009_nullable_active_parameter() {
    let f = verify_lsp318_feature("LSP318-009", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_010_command_tooltips() {
    let f = verify_lsp318_feature("LSP318-010", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_011_workspace_edit_metadata() {
    let f = verify_lsp318_feature("LSP318-011", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_012_text_document_edit_snippets() {
    let f = verify_lsp318_feature("LSP318-012", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_013_debug_message_kind_protocol_trace() {
    let f = verify_lsp318_feature("LSP318-013", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_014_code_lens_resolvable_properties() {
    let f = verify_lsp318_feature("LSP318-014", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_015_completion_list_apply_kind() {
    let f = verify_lsp318_feature("LSP318-015", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let content = fs::read_to_string(transcript_path).unwrap();
    if content.is_empty() {
        panic!("Transcript is empty");
    }
}

#[test]
fn lsp318_all_features_matrix_complete() {
    let matrix = get_feature_matrix();
    if matrix.len() != 15 {
        panic!("Expected 15 features in matrix, got {}", matrix.len());
    }
}

#[test]
fn lsp318_initialize_capability_transcript() {
    // Check that one of the transcripts parses properly
    let f = verify_lsp318_feature("LSP318-001", "SUPPORTED_WITH_TRANSCRIPT");
    let transcript_path = find_file_path(&f.positive_transcript_path);
    let _content = fs::read_to_string(transcript_path).unwrap();
    let obs = engine::scan_file(&f.positive_transcript_path);
    for o in &obs {
        if o.construct == "initialize without 3.18 caps" {
            panic!("Positive transcript triggered capability warning");
        }
    }
}

#[test]
fn lsp318_no_plain_lsp_fallback() {
    let manifest_path = find_file_path("Cargo.toml");
    let content = fs::read_to_string(manifest_path).unwrap();
    if content.contains("tower-lsp =") || content.contains("tower_lsp") {
        panic!("Crate depends on plain tower-lsp");
    }
}

#[test]
fn lsp318_no_basic_lsp_substitution() {
    // Verify each feature has a valid receipt with standard fields
    let matrix = get_feature_matrix();
    for f in matrix {
        let receipt_path = find_file_path(&f.receipt_path);
        let content = fs::read_to_string(&receipt_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&content).unwrap();

        let digest = val.get("digest").and_then(|d| d.as_str()).unwrap();
        if digest != f.digest {
            panic!("Receipt digest mismatch for feature {}", f.feature_id);
        }

        if val
            .get("digest_algorithm")
            .and_then(|a| a.as_str())
            .unwrap()
            != "BLAKE3"
        {
            panic!(
                "Expected digest algorithm BLAKE3 for feature {}",
                f.feature_id
            );
        }

        if val.get("boundary").and_then(|b| b.as_str()).is_none() {
            panic!("Missing boundary for feature {}", f.feature_id);
        }

        if val.get("checkpoint").and_then(|c| c.as_str()).is_none() {
            panic!("Missing checkpoint for feature {}", f.feature_id);
        }

        if val.get("raw_command").and_then(|r| r.as_str()).is_none() {
            panic!("Missing raw_command for feature {}", f.feature_id);
        }
    }
}

#[test]
fn detects_changelog_only_laundering() {
    let path = find_file_path("fixtures/negative_controls/changelog_laundering.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-LSP318-COMB-001");
}

#[test]
fn ocel_001_diagnostic_emits_compat_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&ocel_content).unwrap();
    let events = val.get("events").unwrap().as_array().unwrap();
    assert!(events
        .iter()
        .any(|ev| ev.get("type").and_then(|t| t.as_str()) == Some("DiagnosticEmitted")));

    // Verify negative control
    let path = find_file_path("fixtures/negative_controls/ocel_no_event.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-OCEL-001");
}

#[test]
fn ocel_002_receipt_binds_to_ocel_object() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&ocel_content).unwrap();
    let events = val.get("events").unwrap().as_array().unwrap();

    let receipt_val_event = events
        .iter()
        .find(|ev| ev.get("type").and_then(|t| t.as_str()) == Some("ReceiptValidated"))
        .expect("Should have ReceiptValidated event");

    let rels = receipt_val_event
        .get("relationships")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(rels
        .iter()
        .any(|r| r.get("objectId").and_then(|o| o.as_str()) == Some("receipt_ocel_json")));

    // Verify negative control
    let path = find_file_path("fixtures/negative_controls/ocel_no_binding.md");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-OCEL-002");
}

#[test]
fn ocel_003_lsp318_feature_row_binds_to_ocel_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&ocel_content).unwrap();
    let events = val.get("events").unwrap().as_array().unwrap();

    let feature_ex_event = events
        .iter()
        .find(|ev| ev.get("type").and_then(|t| t.as_str()) == Some("Lsp318FeatureExercised"))
        .expect("Should have Lsp318FeatureExercised event");

    let rels = feature_ex_event
        .get("relationships")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(rels
        .iter()
        .any(|r| r.get("objectId").and_then(|o| o.as_str()) == Some("feature_row_001")));
}

#[test]
fn ocel_004_negative_control_binds_to_ocel_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&ocel_content).unwrap();
    let events = val.get("events").unwrap().as_array().unwrap();

    let neg_control_event = events
        .iter()
        .find(|ev| ev.get("type").and_then(|t| t.as_str()) == Some("NegativeControlExecuted"))
        .expect("Should have NegativeControlExecuted event");

    let rels = neg_control_event
        .get("relationships")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(rels.iter().any(
        |r| r.get("objectId").and_then(|o| o.as_str()) == Some("fixture_changelog_laundering")
    ));
}

#[test]
fn ocel_005_failset_update_binds_to_ocel_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&ocel_content).unwrap();
    let events = val.get("events").unwrap().as_array().unwrap();

    assert!(events
        .iter()
        .any(|ev| ev.get("type").and_then(|t| t.as_str()) == Some("FailsetUpdated")));
}

#[test]
fn ocel_006_ocel_export_uses_wasm4pm_compat_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();

    // Proves external JSON shape parses and validates successfully via the wasm4pm-compat boundary structure check
    let parsed_log = anti_llm_lsp::ocel::parse_and_validate_ocel_json(&ocel_content);
    assert!(parsed_log.is_ok());
}

#[test]
fn ocel_007_rejects_json_shape_without_compat_boundary() {
    let path = find_file_path("fixtures/negative_controls/ocel_no_compat.json");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-OCEL-003");
}

#[test]
fn ocel_008_rejects_full_wasm4pm_authority_in_compat_checkpoint() {
    let path = find_file_path("fixtures/negative_controls/ocel_full_wasm4pm.rs");
    let obs = engine::scan_file(&path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);
    check_diag_code(&diags, "ANTI-LLM-OCEL-004");
}

#[test]
fn ocel_009_generates_ocel_gap_report() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let gap_report_path = tmp.path().join("ocel/ocel_gap_report.md");
    let content = fs::read_to_string(&gap_report_path).unwrap();
    assert!(!content.is_empty());
    assert!(!content.contains("victory"));
    assert!(!content.contains("Victory"));
}

#[test]
fn ocel_010_receipts_ocel_export_digest() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp_path = tmp.path().to_string_lossy().to_string();
    anti_llm_lsp::ocel::write_ocel_outputs(&tmp_path).unwrap();
    let ocel_json_path = tmp.path().join("ocel/anti_llm_lsp_ocel.json");
    let ocel_content = fs::read_to_string(&ocel_json_path).unwrap();
    let expected_hash = blake3::hash(ocel_content.as_bytes()).to_hex().to_string();

    let receipt_path = tmp.path().join("ocel/anti_llm_lsp_ocel.receipt.json");
    let receipt_content = fs::read_to_string(&receipt_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&receipt_content).unwrap();

    let digest = val.get("digest").unwrap().as_str().unwrap();
    assert_eq!(digest, expected_hash);
    assert_eq!(
        val.get("digest_algorithm").unwrap().as_str().unwrap(),
        "BLAKE3"
    );
    assert_eq!(
        val.get("boundary").unwrap().as_str().unwrap(),
        "examples/anti-llm-lsp/ocel"
    );
    assert_eq!(
        val.get("checkpoint").unwrap().as_str().unwrap(),
        "OCEL-COMPAT-001"
    );
}

#[test]
fn test_typescript_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let file_path = tmp.path().join("test_file.ts");
    let content = r#"
        // Forbidden words and leaks test
        const x: any = 10;
        const y = x as any; // ts_smell: as any
        // TODO: finish this stub // ts_smell: TODO
        
        // Naming leak
        const frameworkName = "Nitro LSP"; // ts_leak: Nitro LSP
        
        // Vocabulary leak
        const method = "GALL"; // ts_leak: GALL
        
        // Claim
        const message = "We achieved a total victory!"; // ts_claim: victory
    "#;
    fs::write(&file_path, content).unwrap();

    let obs = engine::scan_file(&file_path.to_string_lossy());
    let diags = engine::evaluate_diagnostics(&obs);

    check_diag_code(&diags, "ANTI-LLM-STRANGE-009");
    check_diag_code(&diags, "ANTI-LLM-CLAIM-005");
    check_diag_code(&diags, "ANTI-LLM-CLAIM-006");
}

// -------------------------------------------------------------
// Spec-graph inventory reconciliation
// -------------------------------------------------------------

/// Assert that every method name in the feature matrix has a corresponding
/// entry in the generated lsp318_message_inventory.json.
///
/// This closes the loop: the feature matrix declares coverage for a set of LSP
/// methods; the inventory is derived from the canonical metaModel.json.  Any
/// method in the matrix that is absent from the inventory indicates either a
/// typo in the matrix or a method that was removed from the spec.
#[test]
fn feature_matrix_methods_present_in_inventory() {
    use std::collections::HashSet;

    #[derive(serde::Deserialize)]
    struct InventoryEntry {
        method: String,
    }

    let inv_path = find_file_path("generated/lsp318_message_inventory.json");
    let raw = fs::read_to_string(&inv_path)
        .unwrap_or_else(|e| panic!("failed to read message inventory: {e}"));
    let inventory: Vec<InventoryEntry> =
        serde_json::from_str(&raw).expect("failed to parse lsp318_message_inventory.json");

    let inventory_methods: HashSet<String> = inventory.into_iter().map(|e| e.method).collect();

    let matrix = get_feature_matrix();
    let mut missing: Vec<String> = Vec::new();
    for feature in &matrix {
        // Skip placeholder entries that have no real method name.
        if feature.request_method.is_empty() || feature.request_method == "none" {
            continue;
        }
        if !inventory_methods.contains(&feature.request_method) {
            missing.push(format!(
                "{} ({} — method: {})",
                feature.feature_id, feature.feature, feature.request_method
            ));
        }
    }

    if !missing.is_empty() {
        panic!(
            "{} feature matrix method(s) not found in lsp318_message_inventory.json:\n  {}",
            missing.len(),
            missing.join("\n  ")
        );
    }
}
