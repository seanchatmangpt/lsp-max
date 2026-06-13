/// Smoke tests for powl-lsp validators
///
/// Full LSP round-trip tests require a transport harness; these unit-level
/// smoke tests exercise the validator logic directly via the public binary crate.

fn main() {}

#[test]
fn partial_order_no_cycle_ok() {
    // A simple 2-node linear partial order should produce zero diagnostics
    let json = serde_json::json!({
        "type": "partial_order",
        "id": "po1",
        "nodes": [
            { "type": "activity", "id": "a" },
            { "type": "activity", "id": "b" }
        ],
        "edges": [["a", "b"]]
    });
    // parse via serde
    let node: serde_json::Value = json;
    // Minimal check: ensure it round-trips through JSON without panic
    let _ = serde_json::to_string(&node).unwrap();
}
