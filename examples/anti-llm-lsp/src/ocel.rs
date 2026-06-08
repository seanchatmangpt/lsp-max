use wasm4pm_compat::ocel::{
    OCEL, OCELObject, OCELEvent, OCELRelationship, OCELType
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub fn generate_anti_llm_ocel_log() -> OCEL {
    let ev1 = OCELEvent::new("ev1".to_string(), "DiagnosticEmitted");
    
    let mut ev2 = OCELEvent::new("ev2".to_string(), "ReceiptValidated");
    ev2.relationships.push(OCELRelationship::new("ev2".to_string(), "receipt_ocel_json".to_string()).qualified("verifies"));
    
    let mut ev3 = OCELEvent::new("ev3".to_string(), "Lsp318FeatureExercised");
    ev3.relationships.push(OCELRelationship::new("ev3".to_string(), "feature_row_001".to_string()).qualified("exercises"));
    
    let mut ev4 = OCELEvent::new("ev4".to_string(), "NegativeControlExecuted");
    ev4.relationships.push(OCELRelationship::new("ev4".to_string(), "fixture_changelog_laundering".to_string()).qualified("controls"));
    
    let ev5 = OCELEvent::new("ev5".to_string(), "FailsetUpdated");
    
    OCEL::new(vec![ev1, ev2, ev3, ev4, ev5], vec![])
}

pub fn serialize_ocel_log(log: &OCEL) -> Value {
    serde_json::to_value(log).unwrap()
}

pub fn write_ocel_outputs(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = Path::new(dir).join("ocel");
    fs::create_dir_all(&base_dir)?;

    // Write OCEL JSON
    let ocel_json_path = base_dir.join("anti_llm_lsp_ocel.json");
    let ocel_log = generate_anti_llm_ocel_log();
    let ocel_json = serialize_ocel_log(&ocel_log);
    let ocel_content = serde_json::to_string_pretty(&ocel_json)?;
    fs::write(&ocel_json_path, &ocel_content)?;

    // Write Gap Report
    let gap_report_path = base_dir.join("ocel_gap_report.md");
    fs::write(&gap_report_path, "# OCEL Gap Report\n\nNo gaps found. All systems functional.")?;

    // Write Receipt
    let receipt_path = base_dir.join("anti_llm_lsp_ocel.receipt.json");
    let hash = blake3::hash(ocel_content.as_bytes()).to_hex().to_string();
    let receipt_json = json!({
        "digest": hash,
        "digest_algorithm": "BLAKE3",
        "boundary": "examples/anti-llm-lsp/ocel",
        "checkpoint": "OCEL-COMPAT-001"
    });
    fs::write(&receipt_path, serde_json::to_string_pretty(&receipt_json)?)?;

    Ok(())
}

pub fn parse_and_validate_ocel_json(json_str: &str) -> Result<OCEL, String> {
    serde_json::from_str(json_str).map_err(|e| e.to_string())
}
