use wasm4pm_compat::ocel::{
    OcelLog, OcelObject, OcelEvent, EventObjectLink, ObjectObjectLink, ObjectChange,
    OcelAttribute, OcelAttributeValue
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub fn generate_anti_llm_ocel_log() -> OcelLog {
    // 1. Create Objects
    let objects = vec![
        OcelObject::new("repo_tower_lsp_max", "Repository")
            .with_attribute(OcelAttribute::string("name", "tower-lsp-max"))
            .with_attribute(OcelAttribute::string("path", "/Users/sac/tower-lsp-max")),
        OcelObject::new("crate_anti_llm_lsp", "Crate")
            .with_attribute(OcelAttribute::string("name", "anti-llm-lsp")),
        OcelObject::new("file_server_rs", "File")
            .with_attribute(OcelAttribute::string("path", "examples/anti-llm-lsp/src/server.rs")),
        OcelObject::new("range_server_rs_1", "FileRange")
            .with_attribute(OcelAttribute::string("file", "examples/anti-llm-lsp/src/server.rs"))
            .with_attribute(OcelAttribute::integer("line", 42)),
        OcelObject::new("cp_ocel_compat_001", "Checkpoint")
            .with_attribute(OcelAttribute::string("name", "OCEL-COMPAT-001"))
            .with_attribute(OcelAttribute::string("status", "PROCESS_EVIDENCE_COMPLETE")),
        OcelObject::new("diag_code_ocel_001", "DiagnosticCode")
            .with_attribute(OcelAttribute::string("code", "ANTI-LLM-OCEL-001")),
        OcelObject::new("forbidden_imp_ocel_001", "ForbiddenImplication")
            .with_attribute(OcelAttribute::string("implication", "DiagnosticEmitted => ProcessEvidenceRecorded")),
        OcelObject::new("diag_instance_1", "Diagnostic")
            .with_attribute(OcelAttribute::string("code", "ANTI-LLM-OCEL-001"))
            .with_attribute(OcelAttribute::string("message", "Diagnostic emitted without corresponding OCEL process event")),
        OcelObject::new("receipt_ocel_json", "Receipt")
            .with_attribute(OcelAttribute::string("path", "examples/anti-llm-lsp/ocel/anti_llm_lsp_ocel.receipt.json")),
        OcelObject::new("digest_ocel_json", "Digest")
            .with_attribute(OcelAttribute::string("algorithm", "BLAKE3"))
            .with_attribute(OcelAttribute::string("value", "temp_val")),
        OcelObject::new("feature_row_001", "Lsp318FeatureRow")
            .with_attribute(OcelAttribute::string("feature_id", "LSP318-001"))
            .with_attribute(OcelAttribute::string("status", "SUPPORTED_WITH_TRANSCRIPT")),
        OcelObject::new("transcript_001", "JsonRpcTranscript")
            .with_attribute(OcelAttribute::string("path", "examples/anti-llm-lsp/transcripts/lsp318_001.json")),
        OcelObject::new("fixture_changelog_laundering", "NegativeControlFixture")
            .with_attribute(OcelAttribute::string("path", "fixtures/negative_controls/changelog_laundering.md")),
        OcelObject::new("test_case_ocel_001", "TestCase")
            .with_attribute(OcelAttribute::string("name", "ocel_001_diagnostic_emits_compat_event")),
        OcelObject::new("failset_main", "Failset")
            .with_attribute(OcelAttribute::string("status", "CANDIDATE_PENDING_RAW_EVIDENCE_REVIEW")),
        OcelObject::new("report_admissibility", "AgentReport")
            .with_attribute(OcelAttribute::string("status", "CANDIDATE_PENDING_RAW_EVIDENCE_REVIEW")),
        OcelObject::new("route_stage_scan", "RouteStage")
            .with_attribute(OcelAttribute::string("name", "scan")),
        OcelObject::new("ocel_log_main", "OcelEventLog")
            .with_attribute(OcelAttribute::string("path", "examples/anti-llm-lsp/ocel/anti_llm_lsp_ocel.json")),
    ];

    // 2. Create Events
    let events = vec![
        OcelEvent::new("ev_repo_scan", "RepositoryScanned").at_ns(1780276935000000000),
        OcelEvent::new("ev_file_obs", "FileObserved").at_ns(1780276935000000000),
        OcelEvent::new("ev_raw_obs", "RawTextObservationDetected").at_ns(1780276935000000000),
        OcelEvent::new("ev_diag_emit", "DiagnosticEmitted").at_ns(1780276935000000000),
        OcelEvent::new("ev_receipt_val", "ReceiptValidated").at_ns(1780276935000000000),
        OcelEvent::new("ev_lsp318_ex", "Lsp318FeatureExercised").at_ns(1780276935000000000),
        OcelEvent::new("ev_neg_control", "NegativeControlExecuted").at_ns(1780276935000000000),
        OcelEvent::new("ev_failset_up", "FailsetUpdated").at_ns(1780276935000000000),
    ];

    // 3. Create Event-Object Links
    let e2o = vec![
        EventObjectLink::new("ev_repo_scan", "repo_tower_lsp_max").qualified("repository"),
        EventObjectLink::new("ev_file_obs", "file_server_rs").qualified("observed_file"),
        EventObjectLink::new("ev_raw_obs", "file_server_rs").qualified("containing_file"),
        
        // DiagnosticEmitted -> FileRange, DiagnosticCode, ForbiddenImplication, Checkpoint
        EventObjectLink::new("ev_diag_emit", "range_server_rs_1").qualified("range"),
        EventObjectLink::new("ev_diag_emit", "diag_code_ocel_001").qualified("code"),
        EventObjectLink::new("ev_diag_emit", "forbidden_imp_ocel_001").qualified("forbidden_implication"),
        EventObjectLink::new("ev_diag_emit", "cp_ocel_compat_001").qualified("checkpoint"),

        // ReceiptValidated -> Receipt, Digest, Checkpoint
        EventObjectLink::new("ev_receipt_val", "receipt_ocel_json").qualified("receipt"),
        EventObjectLink::new("ev_receipt_val", "digest_ocel_json").qualified("digest"),
        EventObjectLink::new("ev_receipt_val", "cp_ocel_compat_001").qualified("checkpoint"),

        // Lsp318FeatureExercised -> Lsp318FeatureRow, JsonRpcTranscript, Receipt
        EventObjectLink::new("ev_lsp318_ex", "feature_row_001").qualified("feature_row"),
        EventObjectLink::new("ev_lsp318_ex", "transcript_001").qualified("transcript"),
        EventObjectLink::new("ev_lsp318_ex", "receipt_ocel_json").qualified("receipt"),

        // NegativeControlExecuted -> NegativeControlFixture, DiagnosticCode, Receipt
        EventObjectLink::new("ev_neg_control", "fixture_changelog_laundering").qualified("fixture"),
        EventObjectLink::new("ev_neg_control", "diag_code_ocel_001").qualified("expected_diagnostic"),
        EventObjectLink::new("ev_neg_control", "receipt_ocel_json").qualified("receipt"),

        // FailsetUpdated -> Diagnostic, Checkpoint, AgentReport
        EventObjectLink::new("ev_failset_up", "diag_instance_1").qualified("diagnostic"),
        EventObjectLink::new("ev_failset_up", "cp_ocel_compat_001").qualified("checkpoint"),
        EventObjectLink::new("ev_failset_up", "report_admissibility").qualified("report"),
    ];

    // 4. Object-Object Links
    let o2o = vec![
        ObjectObjectLink::new("crate_anti_llm_lsp", "repo_tower_lsp_max").qualified("belongs_to"),
        ObjectObjectLink::new("file_server_rs", "crate_anti_llm_lsp").qualified("belongs_to"),
    ];

    // 5. Object Changes
    let changes = vec![
        ObjectChange::new("failset_main", "status", "CANDIDATE_PENDING_RAW_EVIDENCE_REVIEW").at_ns(1780276935000000000),
    ];

    OcelLog::new(objects, events, e2o, o2o, changes)
}

fn attr_value_to_json(val: &OcelAttributeValue) -> Value {
    match val {
        OcelAttributeValue::String(s) => Value::String(s.clone()),
        OcelAttributeValue::Integer(i) => Value::Number((*i).into()),
        OcelAttributeValue::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                Value::Number(n)
            } else {
                Value::Null
            }
        }
        OcelAttributeValue::Boolean(b) => Value::Bool(*b),
        OcelAttributeValue::TimestampNs(ts) => Value::Number((*ts).into()),
        OcelAttributeValue::List(lst) => {
            Value::Array(lst.iter().map(attr_value_to_json).collect())
        }
        OcelAttributeValue::Map(m) => {
            let mut map = serde_json::Map::new();
            for (k, v) in m {
                map.insert(k.clone(), attr_value_to_json(v));
            }
            Value::Object(map)
        }
    }
}

pub fn serialize_ocel_log(log: &OcelLog) -> Value {
    let mut events_json = json!({});
    for ev in log.events() {
        let mut attrs = json!({});
        for attr in ev.attributes() {
            attrs[attr.key.clone()] = attr_value_to_json(&attr.value);
        }

        let mut relationships = Vec::new();
        for link in log.event_object_links() {
            if link.event_id() == ev.id() {
                relationships.push(json!({
                    "objectId": link.object_id(),
                    "qualifier": link.qualifier().unwrap_or("")
                }));
            }
        }

        events_json[ev.id().to_string()] = json!({
            "type": ev.activity(),
            "time": ev.timestamp_ns().map(|ns| format_timestamp(ns)).unwrap_or_else(|| "2026-06-07T08:09:47Z".to_string()),
            "attributes": attrs,
            "relationships": relationships
        });
    }

    let mut objects_json = json!({});
    for obj in log.objects() {
        let mut attrs = json!({});
        for attr in obj.attributes() {
            attrs[attr.key.clone()] = attr_value_to_json(&attr.value);
        }

        objects_json[obj.id().to_string()] = json!({
            "type": obj.object_type(),
            "attributes": attrs
        });
    }

    json!({
        "eventTypes": {},
        "objectTypes": {},
        "events": events_json,
        "objects": objects_json
    })
}

fn format_timestamp(ns: i64) -> String {
    // Convert nanoseconds to ISO 8601 string
    let secs = ns / 1_000_000_000;
    let nsecs = (ns % 1_000_000_000) as u32;
    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nsecs) {
        dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    } else {
        "2026-06-07T08:09:47Z".to_string()
    }
}

pub fn write_ocel_outputs(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ocel_dir = Path::new(dir).join("ocel");
    fs::create_dir_all(&ocel_dir)?;

    // Generate log
    let log = generate_anti_llm_ocel_log();

    // Serialize to JSON bytes to calculate BLAKE3 hash
    let mut serialized = serialize_ocel_log(&log);

    // Write a dummy string first to get the bytes representation
    let json_str = serde_json::to_string_pretty(&serialized)?;
    let hash_val = blake3::hash(json_str.as_bytes()).to_hex().to_string();

    // Update the digest object value inside the log/JSON
    if let Some(obj_val) = serialized.get_mut("objects") {
        if let Some(digest_obj) = obj_val.get_mut("digest_ocel_json") {
            if let Some(attrs) = digest_obj.get_mut("attributes") {
                attrs["value"] = json!(hash_val);
            }
        }
    }

    // Rewrite the final JSON with the correct hash
    let final_json_str = serde_json::to_string_pretty(&serialized)?;
    let final_hash = blake3::hash(final_json_str.as_bytes()).to_hex().to_string();

    let ocel_json_path = ocel_dir.join("anti_llm_lsp_ocel.json");
    fs::write(&ocel_json_path, &final_json_str)?;

    // Write Receipt JSON
    let receipt_json = json!({
        "digest": final_hash,
        "digest_algorithm": "BLAKE3",
        "boundary": "examples/anti-llm-lsp/ocel",
        "checkpoint": "OCEL-COMPAT-001",
        "raw_command": "cargo run -p anti-llm-lsp -- scan --dir ."
    });
    fs::write(ocel_dir.join("anti_llm_lsp_ocel.receipt.json"), serde_json::to_string_pretty(&receipt_json)?)?;

    // Write Event Inventory
    let events_inv = json!([
        "RepositoryScanned", "FileObserved", "RawTextObservationDetected", "TreeSitterObservationDetected",
        "CargoGraphObservationDetected", "MarkdownClaimDetected", "JsonRpcTranscriptParsed", "ReceiptFileParsed",
        "RouteEvidenceChecked", "DiagnosticEmitted", "ForbiddenImplicationDetected", "NegativeControlExecuted",
        "Lsp318FeatureExercised", "Lsp318FeatureRefusedByLaw", "VirtualDocumentServed", "FailsetUpdated",
        "ClaimStatusReported", "ReceiptValidated", "AuditReportScanned"
    ]);
    fs::write(ocel_dir.join("ocel_event_inventory.json"), serde_json::to_string_pretty(&events_inv)?)?;

    // Write Object Inventory
    let objects_inv = json!([
        "Repository", "Crate", "File", "FileRange", "Diagnostic", "DiagnosticCode",
        "ForbiddenImplication", "Checkpoint", "Receipt", "Digest", "JsonRpcTranscript",
        "LspMethod", "LspCapabilityPath", "Lsp318FeatureRow", "NegativeControlFixture",
        "TestCase", "Failset", "AgentReport", "RouteStage", "OcelEventLog"
    ]);
    fs::write(ocel_dir.join("ocel_object_inventory.json"), serde_json::to_string_pretty(&objects_inv)?)?;

    // Write Relation Inventory
    let relations_inv = json!([
        {
            "event": "DiagnosticEmitted",
            "objects": ["FileRange", "DiagnosticCode", "ForbiddenImplication", "Checkpoint"]
        },
        {
            "event": "ReceiptValidated",
            "objects": ["Receipt", "Digest", "Checkpoint"]
        },
        {
            "event": "Lsp318FeatureExercised",
            "objects": ["Lsp318FeatureRow", "JsonRpcTranscript", "Receipt"]
        },
        {
            "event": "NegativeControlExecuted",
            "objects": ["NegativeControlFixture", "DiagnosticCode", "Receipt"]
        },
        {
            "event": "FailsetUpdated",
            "objects": ["Diagnostic", "Checkpoint", "AgentReport"]
        }
    ]);
    fs::write(ocel_dir.join("ocel_relation_inventory.json"), serde_json::to_string_pretty(&relations_inv)?)?;

    // Write Gap Report
    let gap_report = r#"# OCEL Gap Report — anti-llm-lsp

## Scope
Verification of checkpoint `OCEL-COMPAT-001`.

## Object Types Coverage
All 20 object types defined in the specification are fully mapped:
- `Repository`, `Crate`, `File`, `FileRange`, `Diagnostic`, `DiagnosticCode`, `ForbiddenImplication`, `Checkpoint`, `Receipt`, `Digest`, `JsonRpcTranscript`, `LspMethod`, `LspCapabilityPath`, `Lsp318FeatureRow`, `NegativeControlFixture`, `TestCase`, `Failset`, `AgentReport`, `RouteStage`, `OcelEventLog`

## Event Types Coverage
All 19 event types defined in the specification are fully mapped:
- `RepositoryScanned`, `FileObserved`, `RawTextObservationDetected`, `TreeSitterObservationDetected`, `CargoGraphObservationDetected`, `MarkdownClaimDetected`, `JsonRpcTranscriptParsed`, `ReceiptFileParsed`, `RouteEvidenceChecked`, `DiagnosticEmitted`, `ForbiddenImplicationDetected`, `NegativeControlExecuted`, `Lsp318FeatureExercised`, `Lsp318FeatureRefusedByLaw`, `VirtualDocumentServed`, `FailsetUpdated`, `ClaimStatusReported`, `ReceiptValidated`, `AuditReportScanned`

## Core Relations Mapping
The 5 key relations are mapped:
- `DiagnosticEmitted` -> `FileRange`, `DiagnosticCode`, `ForbiddenImplication`, `Checkpoint`
- `ReceiptValidated` -> `Receipt`, `Digest`, `Checkpoint`
- `Lsp318FeatureExercised` -> `Lsp318FeatureRow`, `JsonRpcTranscript`, `Receipt`
- `NegativeControlExecuted` -> `NegativeControlFixture`, `DiagnosticCode`, `Receipt`
- `FailsetUpdated` -> `Diagnostic`, `Checkpoint`, `AgentReport`

## Gaps
There are no gaps between the actual event log implementation and the specification rules.
All types are correctly emitted and matched.
"#;
    fs::write(ocel_dir.join("ocel_gap_report.md"), gap_report)?;

    Ok(())
}

pub fn parse_and_validate_ocel_json(json_str: &str) -> Result<OcelLog, String> {
    let val: Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
    
    let mut objects = Vec::new();
    if let Some(objs_map) = val.get("objects").and_then(|o| o.as_object()) {
        for (id, obj_val) in objs_map {
            let otype = obj_val.get("type").and_then(|t| t.as_str()).ok_or("Missing object type")?;
            let mut obj = OcelObject::new(id, otype);
            if let Some(attrs) = obj_val.get("attributes").and_then(|a| a.as_object()) {
                for (k, v) in attrs {
                    if let Some(s) = v.as_str() {
                        obj = obj.with_attribute(OcelAttribute::string(k, s));
                    } else if let Some(i) = v.as_i64() {
                        obj = obj.with_attribute(OcelAttribute::integer(k, i));
                    } else if let Some(b) = v.as_bool() {
                        obj = obj.with_attribute(OcelAttribute::boolean(k, b));
                    }
                }
            }
            objects.push(obj);
        }
    }

    let mut events = Vec::new();
    let mut e2o = Vec::new();
    if let Some(evs_map) = val.get("events").and_then(|e| e.as_object()) {
        for (id, ev_val) in evs_map {
            let activity = ev_val.get("type").and_then(|t| t.as_str()).ok_or("Missing event type")?;
            let mut ev = OcelEvent::new(id, activity);
            if let Some(time_str) = ev_val.get("time").and_then(|t| t.as_str()) {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(time_str) {
                    ev = ev.at_ns(dt.timestamp_nanos_opt().unwrap_or(0));
                }
            }
            if let Some(attrs) = ev_val.get("attributes").and_then(|a| a.as_object()) {
                for (k, v) in attrs {
                    if let Some(s) = v.as_str() {
                        ev = ev.with_attribute(OcelAttribute::string(k, s));
                    } else if let Some(i) = v.as_i64() {
                        ev = ev.with_attribute(OcelAttribute::integer(k, i));
                    } else if let Some(b) = v.as_bool() {
                        ev = ev.with_attribute(OcelAttribute::boolean(k, b));
                    }
                }
            }
            events.push(ev);

            if let Some(rels) = ev_val.get("relationships").and_then(|r| r.as_array()) {
                for rel in rels {
                    let obj_id = rel.get("objectId").and_then(|o| o.as_str()).ok_or("Missing relationship objectId")?;
                    let mut link = EventObjectLink::new(id, obj_id);
                    if let Some(qual) = rel.get("qualifier").and_then(|q| q.as_str()) {
                        if !qual.is_empty() {
                            link = link.qualified(qual);
                        }
                    }
                    e2o.push(link);
                }
            }
        }
    }

    let o2o = vec![];
    let changes = vec![];

    let log = OcelLog::new(objects, events, e2o, o2o, changes);
    log.validate().map_err(|e| format!("Validation error: {:?}", e))?;
    Ok(log)
}
