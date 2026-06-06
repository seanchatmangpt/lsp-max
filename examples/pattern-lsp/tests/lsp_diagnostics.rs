use pattern_lsp::scanner::scan_document;

#[test]
fn test_lsp_diagnostics_raw_protocol() {
    let content = "let x = serde_json::Value::Null;";
    let uri = "file:///fake/src/lib.rs";
    
    let findings = scan_document(uri, content).expect("Scan failed");
    
    let mut found = false;
    for finding in findings {
        if finding.rule_id == "RAW-PROTOCOL-001" && finding.source == "pattern-lsp" {
            found = true;
            break;
        }
    }
    
    assert!(found, "Did not find RAW-PROTOCOL-001 from pattern-lsp");
}
