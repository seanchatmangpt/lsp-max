use super::*;
use lsp_max_lsif::lsif::{Element, PositionEncoding, Vertex, VertexType};
use oxigraph::store::Store;

fn test_element() -> Element {
    Element::Vertex(Vertex::MetaData {
        id: lsp_types_max::NumberOrString::Number(1),
        type_: VertexType::Vertex,
        version: "0.6.0".to_string(),
        position_encoding: PositionEncoding::Utf16,
        project_root: "file:///".to_string(),
        tool_info: None,
    })
}

#[test]
fn test_kernel_transitions() {
    let store = Store::new().unwrap();

    // 1. RAW -> CANDIDATE
    let raw_machine = Machine::new(
        RAW,
        RawData {
            elements: vec![test_element()],
        },
    );
    let raw_receipt = raw_machine.receipt();
    assert_eq!(raw_receipt.sequence, 0);

    let candidate_machine = raw_machine.admit("snap-1".to_string()).unwrap();
    assert_eq!(candidate_machine.phase, CANDIDATE);

    // Check receipt on CANDIDATE
    let candidate_receipt = candidate_machine.receipt();
    assert_eq!(candidate_receipt.sequence, 1);

    // 2. CANDIDATE -> ADMITTED
    let mut admit_receipt = CryptographicReceipt {
        prev_hash: candidate_receipt.compute_payload_hash(),
        discipline_id: uuid::Uuid::new_v4(),
        law_id: uuid::Uuid::new_v4(),
        consequence_hash: Blake3Hash([0u8; 32]),
        sequence: 2,
        signature: [0u8; 64],
    };
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
    let p_hash = admit_receipt.compute_payload_hash();
    admit_receipt.signature = signing_key.sign(&p_hash.0).to_bytes();

    let active_graph = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:project:local:snapshot:snap-1").unwrap(),
    );

    let input = CandidateAdmitInput {
        store: store.clone(),
        graph_name: active_graph.clone(),
        receipt: admit_receipt.clone(),
    };

    let admitted_machine = candidate_machine.admit(input).unwrap();
    assert_eq!(admitted_machine.phase, ADMITTED);

    // Check receipt on ADMITTED matches the one we admitted it with
    let admitted_receipt = admitted_machine.receipt();
    assert_eq!(admitted_receipt.sequence, 2);

    // 3. ADMITTED -> SUPERSEDED
    let superseded_by = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:project:local:snapshot:snap-2").unwrap(),
    );
    let superseded_machine = admitted_machine.admit(superseded_by.clone()).unwrap();
    assert_eq!(superseded_machine.phase, SUPERSEDED);
}

#[test]
fn test_kernel_replay() {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);

    // R0: RAW
    let mut r0 = CryptographicReceipt {
        prev_hash: Blake3Hash([0u8; 32]),
        discipline_id: uuid::Uuid::nil(),
        law_id: uuid::Uuid::nil(),
        consequence_hash: Blake3Hash([0u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    r0.signature = signing_key.sign(&r0.compute_payload_hash().0).to_bytes();

    // R1: CANDIDATE
    let mut r1 = CryptographicReceipt {
        prev_hash: r0.compute_payload_hash(),
        discipline_id: uuid::Uuid::nil(),
        law_id: uuid::Uuid::nil(),
        consequence_hash: Blake3Hash([5u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    r1.signature = signing_key.sign(&r1.compute_payload_hash().0).to_bytes();

    // R2: ADMITTED
    let mut r2 = CryptographicReceipt {
        prev_hash: r1.compute_payload_hash(),
        discipline_id: uuid::Uuid::nil(),
        law_id: uuid::Uuid::nil(),
        consequence_hash: Blake3Hash([9u8; 32]),
        sequence: 2,
        signature: [0u8; 64],
    };
    r2.signature = signing_key.sign(&r2.compute_payload_hash().0).to_bytes();

    let history = [r0, r1, r2];

    // Replay RAW
    let replayed_raw =
        Machine::<GraphAdmissionLaw, RAW, RawData>::replay(history[..1].to_vec()).unwrap();
    assert_eq!(replayed_raw.phase, RAW);

    // Replay CANDIDATE
    let replayed_candidate =
        Machine::<GraphAdmissionLaw, CANDIDATE, CandidateData>::replay(history[..2].to_vec())
            .unwrap();
    assert_eq!(replayed_candidate.phase, CANDIDATE);

    // Replay ADMITTED
    let replayed_admitted =
        Machine::<GraphAdmissionLaw, ADMITTED, AdmittedData>::replay(history[..3].to_vec())
            .unwrap();
    assert_eq!(replayed_admitted.phase, ADMITTED);
}
