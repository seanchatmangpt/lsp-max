use pattern_lsp::scanner::scan_document;
use lsp_types_max::Diagnostic;

#[test]
fn test_composite_source_merging() {
    let content = "let x = serde_json::Value::Null;\nfn main() { panic!(\"Intended error\"); }";
    let uri = "file:///fake/src/lib.rs";
    
    // Simulate other upstream source (e.g. rust-analyzer)
    let rust_analyzer_diagnostics = vec![
        Diagnostic {
            source: Some("rust-analyzer".into()),
            message: "variable does not need to be mutable".into(),
            ..Default::default()
        }
    ];
    
    // Simulate pattern-lsp upstream
    let pattern_findings = scan_document(uri, content).unwrap();
    let mut pattern_diagnostics: Vec<Diagnostic> = pattern_findings.into_iter().map(|f| {
        Diagnostic {
            source: Some("pattern-lsp".into()),
            message: f.matched_text,
            ..Default::default()
        }
    }).collect();
    
    // Test that the merged set includes both
    let mut composite = rust_analyzer_diagnostics.clone();
    composite.append(&mut pattern_diagnostics);
    
    let has_ra = composite.iter().any(|d| d.source.as_deref() == Some("rust-analyzer"));
    let has_pattern = composite.iter().any(|d| d.source.as_deref() == Some("pattern-lsp"));
    
    assert!(has_ra, "Composite should contain rust-analyzer diagnostics");
    assert!(has_pattern, "Composite should contain pattern-lsp diagnostics");
    
    // Simulate disabling pattern-lsp
    let composite_disabled = rust_analyzer_diagnostics.clone();
    assert_ne!(composite.len(), composite_disabled.len(), "Diagnostic count must change when pattern-lsp is disabled");
}
