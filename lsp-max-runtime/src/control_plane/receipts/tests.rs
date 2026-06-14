use super::*;
use tempfile::NamedTempFile;

#[test]
fn test_receipt_payload_hash_determinism() {
    let discipline_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let receipt1 = CryptographicReceipt {
        prev_hash: Blake3Hash([1u8; 32]),
        discipline_id,
        law_id,
        consequence_hash: Blake3Hash([2u8; 32]),
        sequence: 42,
        signature: [0u8; 64],
    };
    let receipt2 = CryptographicReceipt {
        prev_hash: Blake3Hash([1u8; 32]),
        discipline_id,
        law_id,
        consequence_hash: Blake3Hash([2u8; 32]),
        sequence: 42,
        signature: [3u8; 64],
    };
    assert_eq!(
        receipt1.compute_payload_hash(),
        receipt2.compute_payload_hash()
    );
}

#[test]
fn test_keystore_load_save() {
    let keystore = Keystore::generate();
    let file = NamedTempFile::new().unwrap();
    keystore.save_to_file(file.path()).unwrap();

    let loaded = Keystore::load_from_file(file.path()).unwrap();
    assert_eq!(keystore.to_bytes(), loaded.to_bytes());
    assert_eq!(keystore.verifying_key(), loaded.verifying_key());
}

#[test]
fn test_keystore_to_from_bytes() {
    let seed = [0x42u8; 32];
    let keystore = Keystore::from_seed(&seed);
    assert_eq!(keystore.to_bytes(), seed);

    let keystore2 = Keystore::from_bytes(&seed).unwrap();
    assert_eq!(keystore2.to_bytes(), seed);

    let err = Keystore::from_bytes(&[0u8; 31]);
    assert!(matches!(err, Err(KeyManagementError::KeyParse(_))));
}

#[test]
fn test_keystore_sign_verify_receipt() {
    let keystore = Keystore::generate();
    let discipline_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let mut receipt = CryptographicReceipt {
        prev_hash: Blake3Hash([0u8; 32]),
        discipline_id,
        law_id,
        consequence_hash: Blake3Hash([5u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };

    keystore.sign_receipt(&mut receipt);
    assert_ne!(receipt.signature, [0u8; 64]);

    // Verifying using the keystore directly
    assert!(keystore.verify_receipt(&receipt).is_ok());

    // Verifying with different key (should fail)
    let other_keystore = Keystore::generate();
    assert!(other_keystore.verify_receipt(&receipt).is_err());
}

#[test]
fn test_keystore_trusted_keys() {
    let keystore = Keystore::generate();
    let foreign_keystore = Keystore::generate();
    let foreign_discipline = Uuid::new_v4();

    keystore.register_trusted_key(foreign_discipline, foreign_keystore.verifying_key());
    assert_eq!(
        keystore.get_trusted_key(&foreign_discipline),
        Some(foreign_keystore.verifying_key())
    );

    let mut receipt = CryptographicReceipt {
        prev_hash: Blake3Hash([0u8; 32]),
        discipline_id: foreign_discipline,
        law_id: Uuid::new_v4(),
        consequence_hash: Blake3Hash([5u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };

    foreign_keystore.sign_receipt(&mut receipt);
    // Keystore has registered foreign verifying key for this discipline, so it should verify successfully
    assert!(keystore.verify_receipt(&receipt).is_ok());
}

#[test]
fn test_verify_receipt_chain_valid() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();

    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([9u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let mut r1 = CryptographicReceipt {
        prev_hash: r0.compute_payload_hash(),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([8u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r1);

    let chain = vec![r0, r1];
    assert!(verify_receipt_chain(&chain, &verifying_key, &genesis_hash).is_ok());
}

#[test]
fn test_verify_receipt_chain_invalid_chaining() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();

    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([9u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let mut r1 = CryptographicReceipt {
        prev_hash: Blake3Hash([9u8; 32]), // Invalid chain link
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([8u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r1);

    let chain = vec![r0, r1];
    assert_eq!(
        verify_receipt_chain(&chain, &verifying_key, &genesis_hash),
        Err(ChainValidationError::HashMismatch { index: 1 })
    );
}

#[test]
fn test_verify_receipt_chain_sequence_mismatch() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();

    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([9u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let mut r1 = CryptographicReceipt {
        prev_hash: r0.compute_payload_hash(),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([8u8; 32]),
        sequence: 2, // Sequence mismatch: expected 1, got 2
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r1);

    let chain = vec![r0, r1];
    assert_eq!(
        verify_receipt_chain(&chain, &verifying_key, &genesis_hash),
        Err(ChainValidationError::SequenceMismatch {
            index: 1,
            expected: 1,
            found: 2
        })
    );
}

#[test]
fn test_verify_receipt_chain_invalid_signature() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();

    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([9u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };

    let chain = vec![r0];
    assert_eq!(
        verify_receipt_chain(&chain, &verifying_key, &genesis_hash),
        Err(ChainValidationError::SignatureVerificationFailed { index: 0 })
    );
}

#[test]
fn test_replay_engine_success() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();

    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([100u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let mut r1 = CryptographicReceipt {
        prev_hash: r0.compute_payload_hash(),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([101u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r1);

    let chain = vec![r0, r1];
    let engine = ReplayEngine::new(genesis_hash, verifying_key);

    let res = engine.replay(&chain, |receipt| {
        if receipt.sequence == 0 {
            Blake3Hash([100u8; 32])
        } else {
            Blake3Hash([101u8; 32])
        }
    });

    assert!(res.is_ok());
}

#[test]
fn test_replay_engine_transition_mismatch() {
    let keystore = Keystore::generate();
    let verifying_key = keystore.verifying_key();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();

    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([100u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    keystore.sign_receipt(&mut r0);

    let chain = vec![r0];
    let engine = ReplayEngine::new(genesis_hash, verifying_key);

    let res = engine.replay(&chain, |_receipt| Blake3Hash([99u8; 32]));

    assert_eq!(res, Err(ChainValidationError::HashMismatch { index: 0 }));
}

#[test]
fn test_trace_attributes_and_ocel() {
    let disc_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let receipt = CryptographicReceipt {
        prev_hash: Blake3Hash([1u8; 32]),
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([2u8; 32]),
        sequence: 42,
        signature: [3u8; 64],
    };

    let attrs = receipt.trace_attributes();
    assert_eq!(attrs[0].0, "ostar.prev_hash");
    assert_eq!(
        attrs[0].1,
        "0101010101010101010101010101010101010101010101010101010101010101"
    );
    assert_eq!(attrs[1].0, "ostar.discipline_id");
    assert_eq!(attrs[1].1, disc_id.to_string());
    assert_eq!(attrs[2].0, "ostar.law_id");
    assert_eq!(attrs[2].1, law_id.to_string());
    assert_eq!(attrs[3].0, "ostar.consequence_hash");
    assert_eq!(
        attrs[3].1,
        "0202020202020202020202020202020202020202020202020202020202020202"
    );
    assert_eq!(attrs[4].0, "ostar.sequence");
    assert_eq!(attrs[4].1, "42");

    let ocel_event = receipt.to_ocel_event("e_99", "2026-06-05T14:49:14-07:00");
    assert_eq!(ocel_event["id"], "e_99");
    assert_eq!(ocel_event["attributes"]["sequence"], 42);

    let ocel_obj = receipt.to_ocel_object();
    assert_eq!(ocel_obj["id"], "receipt_42");
    assert_eq!(
        ocel_obj["attributes"]["prev_hash"],
        "0101010101010101010101010101010101010101010101010101010101010101"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-product #1 witness: moniker ↔ OCEL join
//
// These admit the join consumer added in this same change (rule 2: consumer and
// witness land together). The obligation is NOT mere re-run determinism — it is
// that the join key is the moniker CONTENT identity, stable under unrelated
// edits. A key derived from the LSIF numeric vertex id would pass a re-run check
// and silently break when a source file changed.
// ─────────────────────────────────────────────────────────────────────────────

/// The join key is a pure function of `(scheme, identifier)` — same inputs map
/// to the same OCEL objectId regardless of when or how often it is computed.
#[test]
fn moniker_join_key_is_pure_function_of_identity() {
    assert_eq!(
        moniker_object_id("rust", "a::alpha"),
        moniker_object_id("rust", "a::alpha"),
    );
    assert_eq!(
        moniker_object_id("rust", "a::alpha"),
        "moniker:rust:a::alpha"
    );
}

/// The join key derives ONLY from `(scheme, identifier)` — it has no access to
/// any allocation-ordered numeric id, so distinct identifiers map to distinct
/// keys. This is the formatting/key-shape contract of the join function.
///
/// NOTE: this is NOT the invariance witness. The function never sees other
/// symbols, so it cannot exercise the real property — that the INDEXER assigns
/// the same `identifier` after a symbol is inserted above. That indexer-level
/// invariant is pinned by the committed integration test
/// `moniker_identifiers_are_stable_under_symbol_inserted_above` in
/// `crates/lsif-rust/tests/moniker_stability.rs`.
#[test]
fn moniker_join_key_distinguishes_distinct_identifiers() {
    assert_ne!(
        moniker_object_id("rust", "a::alpha"),
        moniker_object_id("rust", "a::zzz_inserted_above"),
    );
}

/// A receipt event and the LSIF moniker vertex resolve to the SAME OCEL object
/// id — the load-bearing claim of cross-product #1. The CodeSymbol object's `id`
/// equals the `objectId` the event references under the `produced_symbol`
/// qualifier, so navigation and provenance traverse one identity.
#[test]
fn receipt_event_and_code_symbol_share_one_ocel_id() {
    let receipt = CryptographicReceipt {
        prev_hash: Blake3Hash([1u8; 32]),
        discipline_id: Uuid::new_v4(),
        law_id: Uuid::new_v4(),
        consequence_hash: Blake3Hash([2u8; 32]),
        sequence: 7,
        signature: [0u8; 64],
    };

    let event =
        receipt.to_ocel_event_for_symbol("e_1", "2026-06-13T00:00:00-07:00", "rust", "a::alpha");
    let symbol = moniker_to_ocel_object(
        "rust",
        "a::alpha",
        &MonikerKind::Export,
        &UniquenessLevel::Project,
    );

    // The symbol object's id is the join key.
    assert_eq!(symbol["id"], "moniker:rust:a::alpha");
    assert_eq!(symbol["type"], "CodeSymbol");

    // The event references that exact id under the produced_symbol qualifier.
    let rels = event["relationships"].as_array().unwrap();
    let produced = rels
        .iter()
        .find(|r| r["qualifier"] == "produced_symbol")
        .expect("event must relate to the produced symbol");
    assert_eq!(produced["objectId"], symbol["id"]);

    // The base relationships from to_ocel_event are preserved (the join augments,
    // it does not replace).
    assert!(rels.iter().any(|r| r["qualifier"] == "governing_law"));
}
