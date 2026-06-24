use lsp_max_lsif::salsa_db::{LsifSemanticDatabase, SemanticDb};
use std::time::Instant;

#[test]
fn test_salsa_incremental_stress() {
    let mut db = LsifSemanticDatabase::default();
    let uri = "file:///stress_test.rs".to_string();
    
    // 1. Initial Load: 10,000 functions
    let mut text = String::new();
    for i in 0..10_000 {
        text.push_str(&format!("fn_huge_{} {{ }}\n", i));
    }
    
    let t0 = Instant::now();
    db.set_document_text(uri.clone(), text.clone());
    let initial_hash = db.cryptographic_standing(uri.clone());
    let load_time = t0.elapsed();
    
    println!("Initial load & semantic parse + BLAKE3 hash (10k items): {:?}", load_time);
    assert!(!initial_hash.is_empty());

    // 2. Exact same input (pure cache hit)
    let t1 = Instant::now();
    let cached_hash = db.cryptographic_standing(uri.clone());
    let cache_time = t1.elapsed();
    
    println!("Identical re-query (full cache hit): {:?}", cache_time);
    assert_eq!(initial_hash, cached_hash);
    
    // 3. Mutate one line (incremental update)
    let t2 = Instant::now();
    let mut new_text = text.clone();
    new_text.push_str("fn_one_more { }\n");
    
    db.set_document_text(uri.clone(), new_text);
    let mutated_hash = db.cryptographic_standing(uri.clone());
    let incremental_time = t2.elapsed();
    
    println!("Incremental mutation (10k + 1 items): {:?}", incremental_time);
    assert_ne!(initial_hash, mutated_hash);
    
    // Validate performance: cache should be virtually instant (sub-millisecond)
    assert!(cache_time.as_micros() < 500, "Cache hit took too long!");
}
