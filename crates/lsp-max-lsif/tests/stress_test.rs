use lsp_max_lsif::{
    lsif_builder::LsifBuilder,
    lsif::*,
};
use std::io::Sink;
use std::time::Instant;
use oxigraph::sparql::QueryResults;

#[test]
fn test_lsif_stress_bench() {
    println!("Starting LSIF / Oxigraph Stress Test...");
    let mut builder = LsifBuilder::new(std::io::sink()).with_store().unwrap();

    let start_time = Instant::now();
    
    // Emit Metadata first!
    builder.emit_metadata("0.6.0", "file:///test/project", ToolInfo {
        name: "test_tool".to_string(),
        version: Some("1.0.0".to_string()),
        args: None,
    }).unwrap();

    // Create 1 Project
    let proj_id = builder.emit_project(Some("rust"), None).unwrap();

    let num_docs = 1_000;
    let refs_per_doc = 100;
    
    let mut doc_ids = Vec::with_capacity(num_docs);

    // Emit 1,000 documents
    for i in 0..num_docs {
        let doc_id = builder.emit_document(&format!("file:///test/project/doc_{}.rs", i), "rust").unwrap();
        doc_ids.push(doc_id.clone());

        // Emit 100 references per document
        let mut ref_ids = Vec::with_capacity(refs_per_doc);
        for j in 0..refs_per_doc {
            let range_id = builder.next_id();
            builder.emit(Element::Vertex(Vertex::Range {
                id: range_id.clone(),
                type_: VertexType::Vertex,
                start: lsp_types_max::Position::new(j as u32, 0),
                end: lsp_types_max::Position::new(j as u32, 5),
                tag: None,
            })).unwrap();
            ref_ids.push(range_id);
        }

        // Link document to references via contains edge
        let contains_id = builder.next_id();
        builder.emit(Element::Edge(Edge::Contains {
            id: contains_id,
            type_: EdgeType::Edge,
            out_v: doc_id.clone(),
            in_vs: ref_ids,
        })).unwrap();
    }

    // Link project to documents
    let proj_contains_id = builder.next_id();
    builder.emit(Element::Edge(Edge::Contains {
        id: proj_contains_id,
        type_: EdgeType::Edge,
        out_v: proj_id.clone(),
        in_vs: doc_ids,
    })).unwrap();

    let emit_duration = start_time.elapsed();
    println!("Emitted 1 Project, {} Documents, and {} References (Total elements ~{}) in {:?}", 
        num_docs, num_docs * refs_per_doc, 
        1 + num_docs + (num_docs * refs_per_doc) + num_docs + 1, 
        emit_duration
    );

    let store = builder.store.as_ref().unwrap();

    // Bench query: Count all ranges in all documents
    let query_start = Instant::now();
    let query = "
        PREFIX lsif: <lsif:>
        SELECT (COUNT(?range) AS ?count) WHERE {
            ?doc a lsif:document .
            ?doc lsif:contains ?range .
            ?range a lsif:range .
        }
    ";
    
    let results = store.query(query).unwrap();
    let mut count = 0;
    if let QueryResults::Solutions(solutions) = results {
        for sol in solutions {
            let s = sol.unwrap();
            let val = s.get("count").unwrap().to_string();
            // the literal is e.g. "100000"^^<http://www.w3.org/2001/XMLSchema#integer>
            let num_str = val.split('"').nth(1).unwrap_or("0");
            count = num_str.parse::<usize>().unwrap_or(0);
        }
    }
    let query_duration = query_start.elapsed();

    println!("SPARQL Graph Traversal: found {} ranges. Query time: {:?}", count, query_duration);
    
    assert_eq!(count, num_docs * refs_per_doc);
}
