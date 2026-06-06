//! Integration tests for the wasm4pm-compat integration in tower-lsp-max.
//!
//! Verifies that control plane evidence is parsed, admitted, and graduated.
//! Configured for compilation and execution under nightly Rust.

#![feature(generic_const_exprs)]
#![feature(adt_const_params)]
#![feature(unsized_const_params)]
#![feature(const_trait_impl)]
#![feature(min_specialization)]
#![feature(portable_simd)]
#![allow(incomplete_features)]

use std::collections::HashSet;
use std::sync::Mutex;

use lsp_types::NumberOrString;
use lsp_types_max as lsp_types;
use tower_lsp_max_base::abstractions::RelationAdmitter;
use tower_lsp_max_lsif::lsif::{Element, PositionEncoding, Vertex, VertexType};
use tower_lsp_max_runtime::control_plane::admission::{AdmittedGraph, StoreFactory};
use tower_lsp_max_runtime::control_plane::receipts::{
    to_hex, Blake3Hash, CryptographicReceipt, Keystore, ReplayEngine,
};

use wasm4pm_compat::admission::{Admission, Admit, Refusal};
use wasm4pm_compat::evidence::Evidence;
use wasm4pm_compat::formats::{
    ExportFormat, FormatEnvelope, FormatExport, FormatKind, ImportFormat,
};
use wasm4pm_compat::loss::{LossPolicy, LossReport};
use wasm4pm_compat::ocel::{
    EventObjectLink, OcelAttribute, OcelEvent, OcelLog, OcelObject, OcelRefusal,
};
use wasm4pm_compat::receipt::{Digest, ProvenanceChain, ReceiptEnvelope, ReplayHint};
use wasm4pm_compat::state::Raw;
use wasm4pm_compat::strict::{ProcessBoundary, ProcessBoundaryKind, StrictCheck, StrictViolation};
use wasm4pm_compat::witness::Ocel20;
use wasm4pm_compat::ProjectionName;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Boundary admitter implementation to verify the type-level raw -> admitted lifecycle.
struct OcelLogAdmitter;

impl Admit for OcelLogAdmitter {
    type Raw = OcelLog;
    type Admitted = OcelLog;
    type Reason = OcelRefusal;
    type Witness = Ocel20;

    fn admit(
        raw: Evidence<Self::Raw, Raw, Self::Witness>,
    ) -> Result<Admission<Self::Admitted, Self::Witness>, Refusal<Self::Reason, Self::Witness>>
    {
        match raw.value.validate() {
            Ok(_) => Ok(Admission::new(raw.value)),
            Err(e) => Err(Refusal::new(e)),
        }
    }
}

/// Stand-in format importer implementation to verify the format import covenant.
struct CustomOcelImporter;

impl ImportFormat for CustomOcelImporter {
    type Admitted = OcelLog;
    type Reason = OcelRefusal;
    type Witness = Ocel20;

    fn import(
        env: FormatEnvelope<Self::Witness>,
    ) -> Result<Admission<Self::Admitted, Self::Witness>, Refusal<Self::Reason, Self::Witness>>
    {
        if env.is_empty() {
            return Err(Refusal::new(OcelRefusal::MissingObject));
        }
        if env.kind != FormatKind::OcelJson {
            return Err(Refusal::new(OcelRefusal::MissingEvent));
        }
        let content_str = String::from_utf8_lossy(&env.bytes);
        if content_str.contains("\"valid\":true") {
            let obj = OcelObject::new("o1", "type");
            let ev = OcelEvent::new("e1", "act");
            let link = EventObjectLink::new("e1", "o1");
            let log = OcelLog::new(vec![obj], vec![ev], vec![link], vec![], vec![]);
            Ok(Admission::new(log))
        } else {
            Err(Refusal::new(OcelRefusal::DanglingEventObjectLink))
        }
    }
}

/// Stand-in format exporter implementation to verify the lossy/lossless export covenant.
struct CustomOcelExporter;

#[derive(Debug, PartialEq, Eq)]
enum CustomExportRefusal {
    FlatteningLoss,
}

impl ExportFormat for CustomOcelExporter {
    type Source = OcelLog;
    type Reason = CustomExportRefusal;

    fn export(src: &Self::Source, policy: LossPolicy) -> Result<FormatExport, Self::Reason> {
        let types: HashSet<&str> = src.objects().iter().map(|o| o.object_type()).collect();
        let bytes = b"<flat-xes-log/>".to_vec();

        if types.len() > 1 {
            if policy == LossPolicy::RefuseLoss {
                return Err(CustomExportRefusal::FlatteningLoss);
            }
            let report = LossReport::<(), (), Vec<String>>::new(
                ProjectionName("ocel-to-xes-flattening"),
                policy,
                types
                    .iter()
                    .map(|t| format!("dropped_type={}", t))
                    .collect(),
            );
            Ok(FormatExport::lossy(FormatKind::XesXml, bytes, report))
        } else {
            Ok(FormatExport::lossless(FormatKind::XesXml, bytes))
        }
    }
}

#[test]
fn test_wasm4pm_ocel_validation() {
    let obj = OcelObject::new("ord-1", "order")
        .with_attribute(OcelAttribute::string("status", "open"))
        .with_attribute(OcelAttribute::integer("quantity", 5));
    let ev = OcelEvent::new("e1", "place_order")
        .at_ns(1_700_000_000_000_000_000)
        .with_attribute(OcelAttribute::float("price", 99.99));
    let link = EventObjectLink::new("e1", "ord-1").qualified("placed_by");

    let valid_log = OcelLog::new(vec![obj], vec![ev], vec![link], vec![], vec![]);

    assert!(valid_log.validate().is_ok());

    let dangling_link = EventObjectLink::new("e2", "ord-999");
    let invalid_log = OcelLog::new(
        vec![OcelObject::new("ord-1", "order")],
        vec![OcelEvent::new("e2", "ship")],
        vec![dangling_link],
        vec![],
        vec![],
    );

    let err = invalid_log.validate();
    assert_eq!(err, Err(OcelRefusal::DanglingEventObjectLink));
}

#[test]
fn test_wasm4pm_admission_lifecycle() {
    let obj = OcelObject::new("cust-1", "customer");
    let ev = OcelEvent::new("e-login", "login");
    let link = EventObjectLink::new("e-login", "cust-1");
    let log = OcelLog::new(vec![obj], vec![ev], vec![link], vec![], vec![]);

    let raw_evidence = Evidence::raw(log);
    let admission_res = OcelLogAdmitter::admit(raw_evidence);
    assert!(admission_res.is_ok());

    let admitted_evidence = admission_res.unwrap().into_evidence();
    assert_eq!(admitted_evidence.value.objects().len(), 1);

    let invalid_log = OcelLog::new(
        vec![],
        vec![OcelEvent::new("e1", "login")],
        vec![],
        vec![],
        vec![],
    );
    let raw_evidence_err = Evidence::raw(invalid_log);
    let admission_err = OcelLogAdmitter::admit(raw_evidence_err);
    assert!(admission_err.is_err());
    let refusal = admission_err.unwrap_err();
    assert_eq!(refusal.reason, OcelRefusal::MissingObject);
    assert_eq!(
        refusal.to_string(),
        "Refusal: OCEL refused by law: MissingObject"
    );
}

#[test]
fn test_wasm4pm_formats_import_export() {
    let envelope = FormatEnvelope::new(FormatKind::OcelJson, b"{\"valid\":true}".to_vec());
    let import_res = CustomOcelImporter::import(envelope);
    assert!(import_res.is_ok());
    let admitted_log = import_res.unwrap().value;
    assert_eq!(admitted_log.objects()[0].id(), "o1");

    let invalid_envelope = FormatEnvelope::new(FormatKind::OcelJson, b"{\"valid\":false}".to_vec());
    let import_err = CustomOcelImporter::import(invalid_envelope);
    assert!(import_err.is_err());
    assert_eq!(
        import_err.unwrap_err().reason,
        OcelRefusal::DanglingEventObjectLink
    );

    let single_type_log = OcelLog::new(
        vec![
            OcelObject::new("o1", "type-a"),
            OcelObject::new("o2", "type-a"),
        ],
        vec![OcelEvent::new("e1", "act")],
        vec![
            EventObjectLink::new("e1", "o1"),
            EventObjectLink::new("e1", "o2"),
        ],
        vec![],
        vec![],
    );
    let export_lossless =
        CustomOcelExporter::export(&single_type_log, LossPolicy::RefuseLoss).unwrap();
    assert!(!export_lossless.is_lossy());
    assert_eq!(export_lossless.kind, FormatKind::XesXml);

    let multi_type_log = OcelLog::new(
        vec![
            OcelObject::new("o1", "type-a"),
            OcelObject::new("o2", "type-b"),
        ],
        vec![OcelEvent::new("e1", "act")],
        vec![
            EventObjectLink::new("e1", "o1"),
            EventObjectLink::new("e1", "o2"),
        ],
        vec![],
        vec![],
    );
    let export_refused = CustomOcelExporter::export(&multi_type_log, LossPolicy::RefuseLoss);
    assert!(matches!(
        export_refused,
        Err(CustomExportRefusal::FlatteningLoss)
    ));

    let export_lossy =
        CustomOcelExporter::export(&multi_type_log, LossPolicy::AllowLossWithReport).unwrap();
    assert!(export_lossy.is_lossy());
    let report = export_lossy.loss.unwrap();
    assert_eq!(report.projection.0, "ocel-to-xes-flattening");
    assert_eq!(report.policy, LossPolicy::AllowLossWithReport);
    assert!(report.lost.iter().any(|s| s.contains("dropped_type=")));
}

#[test]
fn test_strict_boundary_declarations() {
    let valid_boundary =
        ProcessBoundary::fully_attested(ProcessBoundaryKind::ImportsFormat, "ocel-json-ingress");
    assert!(valid_boundary.check().is_ok());

    let mut invalid_boundary = valid_boundary.clone();
    invalid_boundary.has_witness = false;
    let check_err = invalid_boundary.check();
    assert!(check_err.is_err());
    let violations = check_err.unwrap_err();
    assert!(violations.contains(&StrictViolation::MissingWitness));

    let mut invalid_boundary_2 = valid_boundary.clone();
    invalid_boundary_2.has_round_trip_fixture = false;
    let violations_2 = invalid_boundary_2.check().unwrap_err();
    assert!(violations_2.contains(&StrictViolation::MissingRoundTripFixture));

    let mut conformance_boundary = ProcessBoundary::fully_attested(
        ProcessBoundaryKind::ClaimsConformance,
        "conformance-engine",
    );
    conformance_boundary.has_conformance_fields = false;
    let violations_3 = conformance_boundary.check().unwrap_err();
    assert!(violations_3.contains(&StrictViolation::MissingConformanceFields));
}

#[test]
fn test_cryptographic_receipt_conformance() {
    let keystore = Keystore::generate();
    let disc_id = uuid::Uuid::new_v4();
    let law_id = uuid::Uuid::new_v4();

    let mut receipt = CryptographicReceipt {
        prev_hash: Blake3Hash([0u8; 32]),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([7u8; 32]),
        sequence: 12,
        signature: [0u8; 64],
    };

    keystore.sign_receipt(&mut receipt);
    assert_ne!(receipt.signature, [0u8; 64]);
    assert!(keystore.verify_receipt(&receipt).is_ok());

    let event_val = receipt.to_ocel_event("event-12", "2026-06-05T15:12:48-07:00");
    let object_val = receipt.to_ocel_object();

    assert_eq!(event_val["id"], "event-12");
    assert_eq!(event_val["type"], "TransitionExecution");
    assert_eq!(event_val["time"], "2026-06-05T15:12:48-07:00");
    assert_eq!(event_val["attributes"]["sequence"], 12);

    let relationships = event_val["relationships"].as_array().unwrap();
    assert_eq!(relationships.len(), 3);
    assert_eq!(relationships[2]["qualifier"], "attestation");
    assert_eq!(relationships[2]["objectId"], "receipt_12");

    assert_eq!(object_val["id"], "receipt_12");
    assert_eq!(object_val["type"], "Receipt");
    assert!(object_val["attributes"]["prev_hash"].as_str().is_some());
    assert!(object_val["attributes"]["signature"].as_str().is_some());
}

#[test]
fn test_provenance_chain_graduation() {
    let keystore = Keystore::generate();
    let disc_id = uuid::Uuid::new_v4();
    let law_id = uuid::Uuid::new_v4();

    let mut r0 = CryptographicReceipt {
        prev_hash: Blake3Hash([0u8; 32]),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([1u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let payload_hash_0 = r0.compute_payload_hash();
    let mut r1 = CryptographicReceipt {
        prev_hash: payload_hash_0,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([2u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r1);

    let env0 = ReceiptEnvelope::new(
        format!("case-{}", r0.sequence),
        "tower-lsp-max-control-plane",
        Digest::new(format!("blake3:{}", to_hex(&r0.consequence_hash.0))),
        ReplayHint::new(format!("sequence:{}", r0.sequence)),
    );

    let env1 = ReceiptEnvelope::new(
        format!("case-{}", r1.sequence),
        "tower-lsp-max-control-plane",
        Digest::new(format!("blake3:{}", to_hex(&r1.consequence_hash.0))),
        ReplayHint::new(format!("sequence:{}", r1.sequence)),
    );

    assert_eq!(env0.subject, "case-0");
    assert_eq!(env1.subject, "case-1");
    assert_eq!(env0.witness, "tower-lsp-max-control-plane");
    assert_eq!(env1.digest.0, format!("blake3:{}", to_hex(&[2u8; 32])));

    let flat_chain = ProvenanceChain {
        input_hash: to_hex(&[0x11u8; 32]),
        config_hash: to_hex(&[0x22u8; 32]),
        plan_hash: to_hex(&[0x33u8; 32]),
        output_hash: to_hex(&r1.consequence_hash.0),
        combined_hash: to_hex(&r1.compute_payload_hash().0),
        algorithm_id: "lsif-rdf-admitter".to_string(),
        algorithm_version: "26.6.4".to_string(),
        backend_id: "oxigraph-sparql".to_string(),
        kernel_version: "26.6.4".to_string(),
        wasm_build_hash: "0".repeat(64),
    };

    assert_eq!(flat_chain.output_hash, to_hex(&[2u8; 32]));
    assert_eq!(flat_chain.algorithm_id, "lsif-rdf-admitter");
}

#[test]
fn test_deterministic_replay_audit() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let disc_id = uuid::Uuid::new_v4();
    let law_id = uuid::Uuid::new_v4();

    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([10u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let mut r1 = CryptographicReceipt {
        prev_hash: r0.compute_payload_hash(),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([20u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r1);

    let chain = vec![r0, r1];

    let engine = ReplayEngine::new(genesis_hash, verifying_key);

    let replay_res = engine.replay(&chain, |receipt| {
        if receipt.sequence == 0 {
            Blake3Hash([10u8; 32])
        } else {
            Blake3Hash([20u8; 32])
        }
    });
    assert!(replay_res.is_ok());

    let replay_err = engine.replay(&chain, |receipt| {
        if receipt.sequence == 0 {
            Blake3Hash([10u8; 32])
        } else {
            Blake3Hash([99u8; 32])
        }
    });
    assert!(replay_err.is_err());
    assert!(matches!(
        replay_err,
        Err(
            tower_lsp_max_runtime::control_plane::receipts::ChainValidationError::HashMismatch {
                index: 1
            }
        )
    ));
}

#[test]
fn test_admitted_graph_conformance() {
    let _lock = TEST_MUTEX.lock().unwrap();

    let temp_db = tempfile::tempdir().unwrap();
    std::env::set_var("TOWER_LSP_MAX_DB_PATH", temp_db.path().to_str().unwrap());
    let store = StoreFactory::open().unwrap();
    std::env::remove_var("TOWER_LSP_MAX_DB_PATH");

    let active_graph = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:project:local:snapshot:test-wasm4pm-compat").unwrap(),
    );

    let admitter = AdmittedGraph {
        store,
        active_graph,
    };

    let elements = vec![
        Element::Vertex(Vertex::MetaData {
            id: NumberOrString::Number(1),
            type_: VertexType::Vertex,
            version: "0.6.0".to_string(),
            project_root: "file:///".to_string(),
            position_encoding: PositionEncoding::Utf16,
            tool_info: None,
        }),
        Element::Vertex(Vertex::Project {
            id: NumberOrString::Number(2),
            type_: VertexType::Vertex,
            kind: Some("rust".to_string()),
            resource: None,
            contents: None,
        }),
    ];

    let admitted = admitter.admit(elements);
    assert!(admitted.is_ok());

    let admitted_graph = admitted.unwrap();
    assert_ne!(admitted_graph.receipt.signature, [0u8; 64]);
}
