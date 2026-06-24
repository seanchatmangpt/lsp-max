use lsp_max_lsif::{
    lsif_builder::LsifBuilder,
    lsif::*,
};
use std::io::Cursor;
use oxigraph::sparql::QueryResults;

#[test]
fn test_lsif_sparql_query() {
    let mut buffer = Cursor::new(Vec::new());
    let mut builder = LsifBuilder::new(&mut buffer).with_store().unwrap();

    let meta_id = builder.emit_metadata("0.6.0", "file:///test/project", ToolInfo {
        name: "test_tool".to_string(),
        version: Some("1.0.0".to_string()),
        args: None,
    }).unwrap();

    let proj_id = builder.emit_project(Some("rust"), None).unwrap();
    
    // Simulate a document
    let doc_id = builder.emit_document("file:///test/project/main.rs", "rust").unwrap();
    
    // Simulate finding a definition
    let range_id = builder.next_id();
    builder.emit(Element::Vertex(Vertex::Range {
        id: range_id.clone(),
        type_: VertexType::Vertex,
        start: lsp_types_max::Position::new(0, 0),
        end: lsp_types_max::Position::new(0, 5),
        tag: None,
    })).unwrap();

    // Contains edge: project -> document
    let contains_id = builder.next_id();
    builder.emit(Element::Edge(Edge::Contains {
        id: contains_id,
        type_: EdgeType::Edge,
        out_v: proj_id.clone(),
        in_vs: vec![doc_id.clone()],
    })).unwrap();
    
    let store = builder.store.as_ref().unwrap();

    // Query 1: Find all vertices of type 'project'
    let query = "
        PREFIX lsif: <lsif:>
        SELECT ?v WHERE {
            ?v a lsif:project .
        }
    ";
    
    let results = store.query(query).unwrap();
    if let QueryResults::Solutions(mut solutions) = results {
        let sol = solutions.next().unwrap().unwrap();
        let val = sol.get("v").unwrap().to_string();
        let expected = match proj_id {
            Id::Number(n) => format!("<urn:lsif:v:{}>", n),
            Id::String(s) => format!("<urn:lsif:v:{}>", s),
        };
        assert_eq!(val, expected);
    } else {
        panic!("Expected solutions");
    }

    // Query 2: Find document belonging to the project (traversing the contains edge)
    // Remember out_v -> label -> in_v
    let query2 = "
        PREFIX lsif: <lsif:>
        SELECT ?doc WHERE {
            ?proj a lsif:project .
            ?proj lsif:contains ?doc .
        }
    ";
    let results2 = store.query(query2).unwrap();
    if let QueryResults::Solutions(mut solutions) = results2 {
        let sol = solutions.next().unwrap().unwrap();
        let val = sol.get("doc").unwrap().to_string();
        let expected = match doc_id {
            Id::Number(n) => format!("<urn:lsif:v:{}>", n),
            Id::String(s) => format!("<urn:lsif:v:{}>", s),
        };
        assert_eq!(val, expected);
    } else {
        panic!("Expected solutions");
    }
}
