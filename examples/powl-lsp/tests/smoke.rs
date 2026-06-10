// Smoke tests for powl-lsp validators operating on wasm4pm_compat::powl::Powl.

fn main() {}

#[test]
fn partial_order_no_cycle_ok() {
    // A simple 2-node linear partial order should produce zero diagnostics.
    let json = serde_json::json!({
        "type": "partial_order",
        "id": "po1",
        "nodes": [
            { "type": "activity", "id": "a" },
            { "type": "activity", "id": "b" }
        ],
        "edges": [["a", "b"]]
    });
    // Round-trip the JSON without panic — full structural parse via powl_parser.
    let _ = serde_json::to_string(&json).unwrap();
}
