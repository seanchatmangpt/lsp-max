use lsp_max_base::abstractions::RelationAdmitter;
use lsp_max_lsif::lsif::{Edge, Element, PositionEncoding, Vertex, VertexType};
use lsp_max_runtime::control_plane::admission::{resolve_db_path, AdmittedGraph, StoreFactory};
use lsp_types_max as lsp_types;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_rocksdb_config_and_path_resolution() {
    let _lock = TEST_MUTEX.lock().unwrap();

    // 1. Check TOWER_LSP_MAX_DB_PATH env var resolution
    let temp_dir_1 = tempfile::tempdir().unwrap();
    let db_path_1 = temp_dir_1.path().join("db1");
    std::env::set_var("TOWER_LSP_MAX_DB_PATH", db_path_1.to_str().unwrap());

    let resolved = resolve_db_path();
    assert_eq!(resolved, db_path_1);

    let store = StoreFactory::open().unwrap();
    assert!(db_path_1.exists());
    drop(store);

    std::env::remove_var("TOWER_LSP_MAX_DB_PATH");

    // 2. Check .lsp-max-config.json resolution
    let temp_dir_2 = tempfile::tempdir().unwrap();
    let config_file_path = temp_dir_2.path().join(".lsp-max-config.json");
    let db_path_2 = temp_dir_2.path().join("db2");

    let config_json = serde_json::json!({
        "database_path": db_path_2.to_str().unwrap()
    });
    std::fs::write(&config_file_path, config_json.to_string()).unwrap();

    std::env::set_var("TOWER_LSP_MAX_CONFIG", config_file_path.to_str().unwrap());

    let resolved = resolve_db_path();
    assert_eq!(resolved, db_path_2);

    let store = StoreFactory::open().unwrap();
    assert!(db_path_2.exists());
    drop(store);

    std::env::remove_var("TOWER_LSP_MAX_CONFIG");

    // 3. Check fallback in test mode (should contain lsp-max-db-)
    std::env::set_var("TOWER_LSP_MAX_TEST", "true");
    let resolved = resolve_db_path();
    assert!(resolved.to_str().unwrap().contains("lsp-max-db-"));

    let store = StoreFactory::open().unwrap();
    drop(store);
    std::env::remove_var("TOWER_LSP_MAX_TEST");
}

#[test]
fn test_named_graph_invariant_validation() {
    let _lock = TEST_MUTEX.lock().unwrap();

    let temp_db = tempfile::tempdir().unwrap();
    std::env::set_var("TOWER_LSP_MAX_DB_PATH", temp_db.path().to_str().unwrap());
    let store = StoreFactory::open().unwrap();
    std::env::remove_var("TOWER_LSP_MAX_DB_PATH");

    let active_graph = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:project:local:snapshot:snap-invariant-1").unwrap(),
    );

    let admitter = AdmittedGraph {
        store: store.clone(),
        active_graph,
    };

    // Construct an invalid LSIF node/edge pair containing an orphan relation
    let elements = vec![
        Element::Vertex(Vertex::MetaData {
            id: lsp_types::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            version: "0.6.0".to_string(),
            project_root: "file:///".to_string(),
            position_encoding: PositionEncoding::Utf16,
            tool_info: None,
        }),
        Element::Edge(Edge::Contains {
            id: lsp_types::NumberOrString::Number(2),
            type_: lsp_max_lsif::lsif::EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(1),
            in_vs: vec![lsp_types::NumberOrString::Number(999)], // Node 999 does not exist!
        }),
    ];

    let result = admitter.admit(elements);

    // Assert that the admission was refused with an invariant violation (INVARIANT_1)
    assert!(result.is_err());
    let report = match result {
        Err(r) => r,
        _ => unreachable!(),
    };
    assert!(!report.is_success);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.violated_invariant == "INVARIANT_1"));

    // Verify that the orphan triples were not written/committed to the store
    let contains_pred = oxigraph::model::NamedNode::new(
        "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains"
    ).unwrap();
    let has_contains = store
        .quads_for_pattern(None, Some(contains_pred.as_ref()), None, None)
        .next()
        .is_some();
    assert!(
        !has_contains,
        "Orphan relation was committed to the database despite failing validation!"
    );
}

#[test]
fn test_thread_safety_concurrent_reads_writes() {
    let _lock = TEST_MUTEX.lock().unwrap();

    let temp_db = tempfile::tempdir().unwrap();
    std::env::set_var("TOWER_LSP_MAX_DB_PATH", temp_db.path().to_str().unwrap());
    let store = StoreFactory::open().unwrap();
    std::env::remove_var("TOWER_LSP_MAX_DB_PATH");

    let num_threads = 10;
    let mut handles = Vec::new();
    let barrier = Arc::new(Barrier::new(num_threads));

    for i in 0..num_threads {
        let store_clone = store.clone();
        let barrier_clone = barrier.clone();

        let handle = std::thread::spawn(move || {
            barrier_clone.wait();

            let thread_subject = oxigraph::model::NamedOrBlankNode::NamedNode(
                oxigraph::model::NamedNode::new(format!("urn:test:subject-{}", i)).unwrap(),
            );
            let predicate = oxigraph::model::NamedNode::new("urn:test:predicate").unwrap();
            let object = oxigraph::model::Term::NamedNode(
                oxigraph::model::NamedNode::new(format!("urn:test:object-{}", i)).unwrap(),
            );

            // Write operation
            store_clone
                .insert(&oxigraph::model::Quad::new(
                    thread_subject.clone(),
                    predicate.clone(),
                    object.clone(),
                    oxigraph::model::GraphName::DefaultGraph,
                ))
                .unwrap();

            // Read operation
            let contains = store_clone
                .contains(&oxigraph::model::Quad::new(
                    thread_subject,
                    predicate,
                    object,
                    oxigraph::model::GraphName::DefaultGraph,
                ))
                .unwrap();
            assert!(contains);

            // Iteration
            let count = store_clone.iter().count();
            assert!(count > 0);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_transaction_isolation_dirty_reads_prevention() {
    let _lock = TEST_MUTEX.lock().unwrap();

    let temp_db = tempfile::tempdir().unwrap();
    std::env::set_var("TOWER_LSP_MAX_DB_PATH", temp_db.path().to_str().unwrap());
    let store = StoreFactory::open().unwrap();
    std::env::remove_var("TOWER_LSP_MAX_DB_PATH");

    let active_graph = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:project:local:snapshot:snap-isolation").unwrap(),
    );

    let admitter = AdmittedGraph {
        store: store.clone(),
        active_graph: active_graph.clone(),
    };

    // Case A: Admission fails validation. The candidate quads must NEVER be visible in the store.
    {
        let elements_invalid = vec![
            Element::Vertex(Vertex::MetaData {
                id: lsp_types::NumberOrString::Number(1),
                type_: VertexType::Vertex,
                version: "0.6.0".to_string(),
                project_root: "file:///".to_string(),
                position_encoding: PositionEncoding::Utf16,
                tool_info: None,
            }),
            Element::Edge(Edge::Contains {
                id: lsp_types::NumberOrString::Number(2),
                type_: lsp_max_lsif::lsif::EdgeType::Edge,
                out_v: lsp_types::NumberOrString::Number(1),
                in_vs: vec![lsp_types::NumberOrString::Number(999)], // orphan -> fails validation
            }),
        ];

        let store_clone = store.clone();
        let target_quad = oxigraph::model::Quad::new(
            oxigraph::model::NamedOrBlankNode::NamedNode(
                oxigraph::model::NamedNode::new("urn:project:local:lsif:1").unwrap(),
            ),
            oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                .unwrap(),
            oxigraph::model::Term::NamedNode(
                oxigraph::model::NamedNode::new("urn:lsp-max:core:Metadata").unwrap(),
            ),
            active_graph.clone(),
        );

        let reader_seen_dirty = Arc::new(AtomicBool::new(false));
        let reader_seen_dirty_clone = reader_seen_dirty.clone();
        let writer_done = Arc::new(AtomicBool::new(false));
        let writer_done_clone = writer_done.clone();
        let target_quad_clone = target_quad.clone();

        let reader_handle = std::thread::spawn(move || {
            while !writer_done_clone.load(Ordering::SeqCst) {
                if store_clone.contains(&target_quad_clone).unwrap() {
                    reader_seen_dirty_clone.store(true, Ordering::SeqCst);
                }
                std::thread::yield_now();
            }
        });

        let res = admitter.admit(elements_invalid);
        assert!(res.is_err()); // Should fail validation

        writer_done.store(true, Ordering::SeqCst);
        reader_handle.join().unwrap();

        assert!(!reader_seen_dirty.load(Ordering::SeqCst), "Unadmitted candidate quads were visible to concurrent reader during/after failed validation!");
        assert!(
            !store.contains(&target_quad).unwrap(),
            "Unadmitted candidate quads exist in the store after failed validation!"
        );
    }

    // Case B: Admission succeeds validation.
    {
        let elements_valid = vec![Element::Vertex(Vertex::MetaData {
            id: lsp_types::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            version: "0.6.0".to_string(),
            project_root: "file:///".to_string(),
            position_encoding: PositionEncoding::Utf16,
            tool_info: None,
        })];

        let target_quad = oxigraph::model::Quad::new(
            oxigraph::model::NamedOrBlankNode::NamedNode(
                oxigraph::model::NamedNode::new("urn:project:local:lsif:1").unwrap(),
            ),
            oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                .unwrap(),
            oxigraph::model::Term::NamedNode(
                oxigraph::model::NamedNode::new("urn:lsp-max:core:Metadata").unwrap(),
            ),
            active_graph,
        );

        let res = admitter.admit(elements_valid);
        assert!(res.is_ok());

        assert!(
            store.contains(&target_quad).unwrap(),
            "Candidate quads were not committed to the store on success!"
        );
    }
}
