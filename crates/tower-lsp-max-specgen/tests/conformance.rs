/// Conformance check: verify that the checked-in lsp318_message_inventory.json
/// is consistent with the canonical metaModel.json.
///
/// Marked #[ignore] to avoid slowing CI on every run; invoke explicitly with:
///   cargo test -p tower-lsp-max-specgen -- --ignored
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct InventoryEntry {
    method: String,
}

fn workspace_root() -> std::path::PathBuf {
    // Walk up from the manifest dir until we find Cargo.toml at the root.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/tower-lsp-max-specgen -> workspace root is two levels up
    manifest
        .parent()
        .expect("parent of specgen")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
#[ignore]
fn message_inventory_matches_metamodel() {
    let root = workspace_root();

    let meta_path = root.join("vendors/vscode-languageserver-node/protocol/metaModel.json");
    assert!(
        meta_path.exists(),
        "metaModel.json not found at {}",
        meta_path.display()
    );
    let raw = fs::read_to_string(&meta_path)
        .unwrap_or_else(|e| panic!("failed to read metaModel.json: {e}"));

    #[derive(Debug, Deserialize)]
    struct MetaModelSlim {
        requests: Vec<MethodEntry>,
        notifications: Vec<MethodEntry>,
    }
    #[derive(Debug, Deserialize)]
    struct MethodEntry {
        method: String,
    }

    let meta: MetaModelSlim = serde_json::from_str(&raw).expect("failed to parse metaModel.json");

    let expected_count = meta.requests.len() + meta.notifications.len();
    let mut expected_methods: HashSet<String> = HashSet::new();
    for r in &meta.requests {
        expected_methods.insert(r.method.clone());
    }
    for n in &meta.notifications {
        expected_methods.insert(n.method.clone());
    }

    let inv_path = root.join("examples/anti-llm-lsp/generated/lsp318_message_inventory.json");
    assert!(
        inv_path.exists(),
        "lsp318_message_inventory.json not found at {}",
        inv_path.display()
    );
    let inv_raw = fs::read_to_string(&inv_path)
        .unwrap_or_else(|e| panic!("failed to read message inventory: {e}"));

    // The inventory may be enriched (more fields than just method/kind), so we
    // parse only what we need.
    let inventory: Vec<InventoryEntry> =
        serde_json::from_str(&inv_raw).expect("failed to parse lsp318_message_inventory.json");

    assert_eq!(
        inventory.len(),
        expected_count,
        "inventory has {} entries but metaModel has {} requests+notifications",
        inventory.len(),
        expected_count,
    );

    for entry in &inventory {
        assert!(
            expected_methods.contains(&entry.method),
            "inventory method '{}' not found in metaModel",
            entry.method
        );
    }

    // Also verify the spec graph file exists and has the same count.
    let sg_path = root.join("examples/anti-llm-lsp/generated/lsp318_spec_graph.json");
    if sg_path.exists() {
        let sg_raw = fs::read_to_string(&sg_path)
            .unwrap_or_else(|e| panic!("failed to read spec_graph: {e}"));
        let sg: Vec<InventoryEntry> =
            serde_json::from_str(&sg_raw).expect("failed to parse lsp318_spec_graph.json");
        assert_eq!(
            sg.len(),
            expected_count,
            "spec_graph has {} entries but metaModel has {}",
            sg.len(),
            expected_count,
        );
    }

    let _ = Path::new(""); // suppress unused import warning
}
