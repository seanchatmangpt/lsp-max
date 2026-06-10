use super::*;
use crate::control_plane::receipts::{Blake3Hash, CryptographicReceipt};
use ed25519_dalek::Signer;
use oxigraph::model::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use rand_core::RngCore;

#[test]
fn test_xorshift_rng_determinism() {
    let mut rng1 = XorshiftRng::new(42);
    let mut rng2 = XorshiftRng::new(42);
    assert_eq!(rng1.next_u64(), rng2.next_u64());
    assert_eq!(rng1.next_u32(), rng2.next_u32());
}

#[test]
fn test_replay_clock_formatting() {
    let clock = ReplayClock::new(1772886586000);
    let iso = clock.now_iso();
    assert_eq!(iso, "2026-03-07T12:29:46Z");
}

#[test]
fn test_preprocess_query_now_rand() {
    let clock = ReplayClock::new(1772886586000);
    let mut entropy = ReplayEntropy::new(42);

    let query = "SELECT (now() AS ?time) (rand() AS ?r) WHERE { ?s ?p ?o }";
    let preprocessed = preprocess_query(query, &clock, &mut entropy);

    assert!(preprocessed
        .contains("\"2026-03-07T12:29:46Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime>"));
    assert!(!preprocessed.contains("rand()"));

    let query2 = "SELECT ?brand WHERE { ?s brand:now() ?o }";
    let preprocessed2 = preprocess_query(query2, &clock, &mut entropy);
    assert!(preprocessed2.contains("brand:now()"));
}

#[test]
fn test_hash_query_results_boolean() {
    let res_t = QueryResults::Boolean(true);
    let hash_t = hash_query_results(res_t).unwrap();
    let res_f = QueryResults::Boolean(false);
    let hash_f = hash_query_results(res_f).unwrap();
    assert_ne!(hash_t, hash_f);
}

#[test]
fn test_verifier_end_to_end() {
    let store = Store::new().unwrap();

    // 1. Setup a graph snapshot
    let snap_graph = GraphName::NamedNode(NamedNode::new("urn:project:local:snapshot:g1").unwrap());
    store
        .insert(&Quad::new(
            NamedOrBlankNode::NamedNode(NamedNode::new("urn:project:s1").unwrap()),
            NamedNode::new("urn:project:p1").unwrap(),
            Term::Literal(Literal::new_simple_literal("hello")),
            snap_graph.clone(),
        ))
        .unwrap();

    // 2. Setup a query registry
    let query_str = "SELECT ?val WHERE { ?s <urn:project:p1> ?val }";
    let query_hash = crate::sha256(query_str.as_bytes());

    // 3. Compute expected hash by running on an isolated store
    let isolated = Store::new().unwrap();
    isolated
        .insert(&Quad::new(
            NamedOrBlankNode::NamedNode(NamedNode::new("urn:project:s1").unwrap()),
            NamedNode::new("urn:project:p1").unwrap(),
            Term::Literal(Literal::new_simple_literal("hello")),
            GraphName::DefaultGraph,
        ))
        .unwrap();
    let parsed = SparqlEvaluator::new().parse_query(query_str).unwrap();
    let results = parsed.on_store(&isolated).execute().unwrap();
    let expected_hash = hash_query_results(results).unwrap();

    // 4. Record a receipt in the store
    let rcpt_node = NamedOrBlankNode::NamedNode(NamedNode::new("urn:receipt:1").unwrap());
    store
        .insert(&Quad::new(
            rcpt_node.clone(),
            NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
            Term::NamedNode(NamedNode::new("urn:tower-lsp-max:core:Receipt").unwrap()),
            GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            rcpt_node.clone(),
            NamedNode::new("urn:tower-lsp-max:core:resultHash").unwrap(),
            Term::Literal(Literal::new_simple_literal(expected_hash.clone())),
            GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            rcpt_node.clone(),
            NamedNode::new("urn:tower-lsp-max:core:queryHash").unwrap(),
            Term::Literal(Literal::new_simple_literal(query_hash.clone())),
            GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            rcpt_node.clone(),
            NamedNode::new("urn:tower-lsp-max:core:graphHash").unwrap(),
            Term::Literal(Literal::new_simple_literal("g1")),
            GraphName::DefaultGraph,
        ))
        .unwrap();

    // 5. Initialize verifier and run
    let mut verifier = QueryConsequenceReplayVerifier::new(1772886586000, 42);
    verifier.register_query(query_hash.clone(), query_str.to_string());

    let summary = verifier.verify_all(&store).unwrap();
    assert_eq!(summary.total_replayed, 1);
    assert_eq!(summary.total_success, 1);
    assert_eq!(summary.total_mismatch, 0);

    // Check that a max:Replay is recorded in the store
    let check_replay_q = "
        PREFIX max: <urn:tower-lsp-max:core:>
        ASK {
            ?r a max:Replay ;
               max:queryHash ?qHash ;
               max:graphHash ?gHash ;
               max:resultHash ?resHash .
        }
    ";
    let parsed_check = SparqlEvaluator::new().parse_query(check_replay_q).unwrap();
    let check_res = parsed_check.on_store(&store).execute().unwrap();
    if let QueryResults::Boolean(val) = check_res {
        assert!(val);
    } else {
        panic!("Expected boolean result");
    }
}

#[test]
fn test_verify_replay_engine() {
    use ed25519_dalek::SigningKey;
    use uuid::Uuid;

    let seed = [0u8; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let disc_id = Uuid::nil();
    let law_id = Uuid::nil();

    // 1. Genesis receipt (sequence 0)
    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([0u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    let payload_hash_0 = r0.compute_payload_hash();
    r0.signature = signing_key.sign(&payload_hash_0.0).to_bytes();

    // 2. Next receipt (sequence 1)
    let mut r1 = CryptographicReceipt {
        prev_hash: payload_hash_0,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([42u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    let payload_hash_1 = r1.compute_payload_hash();
    r1.signature = signing_key.sign(&payload_hash_1.0).to_bytes();

    let chain = vec![r0, r1];
    let res = verify_replay(&chain, &verifying_key, &genesis_hash);
    assert!(res.is_ok(), "Replay verification failed: {:?}", res);

    // Test failure on invalid hash
    let mut invalid_chain = chain.clone();
    invalid_chain[1].prev_hash = Blake3Hash([99u8; 32]);
    let res_invalid = verify_replay(&invalid_chain, &verifying_key, &genesis_hash);
    assert!(res_invalid.is_err());
}
