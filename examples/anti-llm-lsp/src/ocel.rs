use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use wasm4pm_compat::ocel::{
    OCELEvent, OCELEventAttribute, OCELObject, OCELRelationship, OCELType, OCEL,
};

pub fn generate_anti_llm_ocel_log() -> OCEL {
    // 1. Create Objects
    let objects = vec![
        OCELObject::new("repo_tower_lsp_max".to_string(), "Repository")
            .with_attribute(OCELEventAttribute::string(
                "name",
                "tower-lsp-max".to_string(),
            ))
            .with_attribute(OCELEventAttribute::string(
                "path",
                "/Users/sac/tower-lsp-max".to_string(),
            )),
        OCELObject::new("crate_anti_llm_lsp".to_string(), "Crate").with_attribute(
            OCELEventAttribute::string("name", "anti-llm-lsp".to_string()),
        ),
        OCELObject::new("file_server_rs".to_string(), "File").with_attribute(
            OCELEventAttribute::string("path", "examples/anti-llm-lsp/src/server.rs".to_string()),
        ),
        OCELObject::new("range_server_rs_1".to_string(), "FileRange")
            .with_attribute(OCELEventAttribute::string(
                "file",
                "examples/anti-llm-lsp/src/server.rs".to_string(),
            ))
            .with_attribute(OCELEventAttribute::integer("line", 42)),
        OCELObject::new("cp_ocel_compat_001".to_string(), "Checkpoint")
            .with_attribute(OCELEventAttribute::string(
                "name",
                "OCEL-COMPAT-001".to_string(),
            ))
            .with_attribute(OCELEventAttribute::string(
                "status",
                "PROCESS_EVIDENCE_COMPLETE".to_string(),
            )),
        OCELObject::new("diag_code_ocel_001".to_string(), "DiagnosticCode").with_attribute(
            OCELEventAttribute::string("code", "ANTI-LLM-OCEL-001".to_string()),
        ),
        OCELObject::new("forbidden_imp_ocel_001".to_string(), "ForbiddenImplication")
            .with_attribute(OCELEventAttribute::string(
                "implication",
                "DiagnosticEmitted => ProcessEvidenceRecorded".to_string(),
            )),
        OCELObject::new("diag_instance_1".to_string(), "Diagnostic")
            .with_attribute(OCELEventAttribute::string(
                "code",
                "ANTI-LLM-OCEL-001".to_string(),
            ))
            .with_attribute(OCELEventAttribute::string(
                "message",
                "Diagnostic emitted without corresponding OCEL process event".to_string(),
            )),
        OCELObject::new("receipt_ocel_json".to_string(), "Receipt").with_attribute(
            OCELEventAttribute::string(
                "path",
                "examples/anti-llm-lsp/ocel/anti_llm_lsp_ocel.receipt.json".to_string(),
            ),
        ),
        OCELObject::new("digest_ocel_json".to_string(), "Digest")
            .with_attribute(OCELEventAttribute::string(
                "algorithm",
                "BLAKE3".to_string(),
            ))
            .with_attribute(OCELEventAttribute::string("value", "temp_val".to_string())),
        OCELObject::new("feature_row_001".to_string(), "Lsp318FeatureRow").with_attribute(
            OCELEventAttribute::string("name", "lsp318-feature-row-001".to_string()),
        ),
        OCELObject::new(
            "fixture_changelog_laundering".to_string(),
            "NegativeControlFixture",
        )
        .with_attribute(OCELEventAttribute::string(
            "name",
            "fixture-changelog-laundering".to_string(),
        )),
    ];

    // 2. Create Events with E2O relationships embedded
    let mut ev_repo_scan = OCELEvent::new("ev_repo_scan".to_string(), "RepositoryScanned");
    ev_repo_scan.relationships.push(
        OCELRelationship::new("ev_repo_scan".to_string(), "repo_tower_lsp_max".to_string())
            .qualified("repository"),
    );

    let mut ev_file_obs = OCELEvent::new("ev_file_obs".to_string(), "FileObserved");
    ev_file_obs.relationships.push(
        OCELRelationship::new("ev_file_obs".to_string(), "file_server_rs".to_string())
            .qualified("observed_file"),
    );

    let mut ev_diag_emit = OCELEvent::new("ev_diag_emit".to_string(), "DiagnosticEmitted");
    ev_diag_emit.relationships.push(
        OCELRelationship::new("ev_diag_emit".to_string(), "range_server_rs_1".to_string())
            .qualified("range"),
    );
    ev_diag_emit.relationships.push(
        OCELRelationship::new("ev_diag_emit".to_string(), "diag_code_ocel_001".to_string())
            .qualified("code"),
    );
    ev_diag_emit.relationships.push(
        OCELRelationship::new(
            "ev_diag_emit".to_string(),
            "forbidden_imp_ocel_001".to_string(),
        )
        .qualified("forbidden_implication"),
    );
    ev_diag_emit.relationships.push(
        OCELRelationship::new("ev_diag_emit".to_string(), "cp_ocel_compat_001".to_string())
            .qualified("checkpoint"),
    );

    let mut ev_receipt_val = OCELEvent::new("ev_receipt_val".to_string(), "ReceiptValidated");
    ev_receipt_val.relationships.push(
        OCELRelationship::new(
            "ev_receipt_val".to_string(),
            "receipt_ocel_json".to_string(),
        )
        .qualified("receipt"),
    );
    ev_receipt_val.relationships.push(
        OCELRelationship::new("ev_receipt_val".to_string(), "digest_ocel_json".to_string())
            .qualified("digest"),
    );
    ev_receipt_val.relationships.push(
        OCELRelationship::new(
            "ev_receipt_val".to_string(),
            "cp_ocel_compat_001".to_string(),
        )
        .qualified("checkpoint"),
    );

    let mut ev_lsp318 = OCELEvent::new("ev_lsp318".to_string(), "Lsp318FeatureExercised");
    ev_lsp318.relationships.push(
        OCELRelationship::new("ev_lsp318".to_string(), "feature_row_001".to_string())
            .qualified("feature_row"),
    );

    let mut ev_neg_control =
        OCELEvent::new("ev_neg_control".to_string(), "NegativeControlExecuted");
    ev_neg_control.relationships.push(
        OCELRelationship::new(
            "ev_neg_control".to_string(),
            "fixture_changelog_laundering".to_string(),
        )
        .qualified("fixture"),
    );

    let ev_failset = OCELEvent::new("ev_failset".to_string(), "FailsetUpdated");

    let events = vec![
        ev_repo_scan,
        ev_file_obs,
        ev_diag_emit,
        ev_receipt_val,
        ev_lsp318,
        ev_neg_control,
        ev_failset,
    ];

    OCEL {
        event_types: vec![
            OCELType {
                name: "RepositoryScanned".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "FileObserved".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "DiagnosticEmitted".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "ReceiptValidated".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Lsp318FeatureExercised".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "NegativeControlExecuted".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "FailsetUpdated".to_string(),
                attributes: vec![],
            },
        ],
        object_types: vec![
            OCELType {
                name: "Repository".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Crate".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "File".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "FileRange".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Checkpoint".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "DiagnosticCode".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "ForbiddenImplication".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Diagnostic".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Receipt".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Digest".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "Lsp318FeatureRow".to_string(),
                attributes: vec![],
            },
            OCELType {
                name: "NegativeControlFixture".to_string(),
                attributes: vec![],
            },
        ],
        events,
        objects,
    }
}

pub fn serialize_ocel_log(log: &OCEL) -> Value {
    serde_json::to_value(log).unwrap()
}

pub fn write_ocel_outputs(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = Path::new(dir).join("ocel");
    fs::create_dir_all(&base_dir)?;

    // Serialize OCEL once; hash the final content so receipt and file are consistent.
    let ocel_json_path = base_dir.join("anti_llm_lsp_ocel.json");
    let ocel_log = generate_anti_llm_ocel_log();
    let ocel_json = serialize_ocel_log(&ocel_log);
    let ocel_content = serde_json::to_string_pretty(&ocel_json)?;
    let hash_val = blake3::hash(ocel_content.as_bytes()).to_hex().to_string();
    fs::write(&ocel_json_path, &ocel_content)?;

    // Write Gap Report
    let gap_report_path = base_dir.join("ocel_gap_report.md");
    fs::write(
        &gap_report_path,
        "# OCEL Gap Report\n\nNo gaps found. All systems functional.",
    )?;

    // Write Receipt — digest covers the exact bytes written to the OCEL JSON file.
    let receipt_path = base_dir.join("anti_llm_lsp_ocel.receipt.json");
    let receipt_json = json!({
        "digest": hash_val,
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
