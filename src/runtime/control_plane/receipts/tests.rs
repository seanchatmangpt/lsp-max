use super::*;

fn hash(bytes: &[u8]) -> Blake3Hash {
    blake3::hash(bytes).into()
}

fn signed_receipt(
    keystore: &Keystore,
    prev_hash: Blake3Hash,
    sequence: u64,
    consequence: &[u8],
) -> CryptographicReceipt {
    let mut receipt = CryptographicReceipt {
        prev_hash,
        discipline_id: Uuid::from_u128(0x11111111_1111_1111_1111_111111111111),
        law_id: Uuid::from_u128(0x22222222_2222_2222_2222_222222222222),
        consequence_hash: hash(consequence),
        sequence,
        signature: [0; 64],
    };
    keystore.sign_receipt(&mut receipt);
    receipt
}

fn two_receipt_chain() -> (Keystore, Blake3Hash, Vec<CryptographicReceipt>) {
    let keystore = Keystore::from_seed(&[7; 32]);
    let genesis = hash(b"admitted-genesis");
    let first = signed_receipt(&keystore, genesis, 41, b"first consequence");
    let second = signed_receipt(
        &keystore,
        first.compute_payload_hash(),
        42,
        b"second consequence",
    );
    (keystore, genesis, vec![first, second])
}

#[test]
fn signed_chain_verifies_and_tampering_is_refused() {
    let (keystore, genesis, chain) = two_receipt_chain();

    assert_eq!(
        verify_receipt_chain(&chain, &keystore.verifying_key(), &genesis),
        Ok(())
    );

    let mut tampered = chain;
    tampered[1].consequence_hash = hash(b"tampered consequence");
    assert_eq!(
        verify_receipt_chain(&tampered, &keystore.verifying_key(), &genesis),
        Err(ChainValidationError::SignatureVerificationFailed { index: 1 })
    );
}

#[test]
fn broken_link_is_refused_even_when_resigned() {
    let (keystore, genesis, mut chain) = two_receipt_chain();
    chain[1].prev_hash = hash(b"unrelated predecessor");
    keystore.sign_receipt(&mut chain[1]);

    assert_eq!(
        verify_receipt_chain(&chain, &keystore.verifying_key(), &genesis),
        Err(ChainValidationError::HashMismatch { index: 1 })
    );
}

#[test]
fn sequence_gap_is_refused() {
    let (keystore, genesis, mut chain) = two_receipt_chain();
    chain[1].sequence = 43;
    keystore.sign_receipt(&mut chain[1]);

    assert_eq!(
        verify_receipt_chain(&chain, &keystore.verifying_key(), &genesis),
        Err(ChainValidationError::SequenceMismatch {
            index: 1,
            expected: 42,
            found: 43,
        })
    );
}

#[test]
fn replay_requires_the_observed_consequence() {
    let (keystore, genesis, chain) = two_receipt_chain();
    let replay = ReplayEngine::new(genesis, keystore.verifying_key());

    assert_eq!(replay.replay(&chain, |receipt| receipt.consequence_hash), Ok(()));
    assert_eq!(
        replay.replay(&chain, |receipt| {
            if receipt.sequence == 42 {
                hash(b"different replay result")
            } else {
                receipt.consequence_hash
            }
        }),
        Err(ChainValidationError::HashMismatch { index: 1 })
    );
}

#[test]
fn receipt_serde_round_trip_preserves_signature_and_identity() {
    let (_, _, chain) = two_receipt_chain();
    let encoded = serde_json::to_vec(&chain[0]).expect("serialize receipt");
    let decoded: CryptographicReceipt =
        serde_json::from_slice(&encoded).expect("deserialize receipt");

    assert_eq!(decoded, chain[0]);
    assert_eq!(decoded.compute_payload_hash(), chain[0].compute_payload_hash());
}
