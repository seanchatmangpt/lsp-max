use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::metamodel::{MessageDirection, MetaModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecGraphNode {
    pub method: String,
    pub kind: String,
    pub direction: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capability_paths: Vec<String>,
}

fn capability_paths(client_cap: &Option<String>, server_cap: &Option<String>) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    if let Some(c) = client_cap {
        paths.push(c.clone());
    }
    if let Some(s) = server_cap {
        paths.push(s.clone());
    }
    paths.sort();
    paths
}

fn direction_str(dir: &MessageDirection) -> String {
    match dir {
        MessageDirection::ClientToServer => "clientToServer".to_string(),
        MessageDirection::ServerToClient => "serverToClient".to_string(),
        MessageDirection::Both => "both".to_string(),
    }
}

/// Emit `lsp318_spec_graph.json` — graph of all LSP 3.18 requests and notifications
/// with their capability paths.
///
/// Output is sorted deterministically by method name so regeneration is
/// `git diff --exit-code`-able.
pub fn emit_spec_graph(model: &MetaModel) -> Result<String> {
    let mut map: BTreeMap<String, SpecGraphNode> = BTreeMap::new();

    for r in &model.requests {
        map.insert(
            r.method.clone(),
            SpecGraphNode {
                method: r.method.clone(),
                kind: "request".to_string(),
                direction: direction_str(&r.message_direction),
                capability_paths: capability_paths(&r.client_capability, &r.server_capability),
            },
        );
    }

    for n in &model.notifications {
        map.insert(
            n.method.clone(),
            SpecGraphNode {
                method: n.method.clone(),
                kind: "notification".to_string(),
                direction: direction_str(&n.message_direction),
                capability_paths: capability_paths(&n.client_capability, &n.server_capability),
            },
        );
    }

    let nodes: Vec<&SpecGraphNode> = map.values().collect();
    Ok(serde_json::to_string_pretty(&nodes)?)
}
