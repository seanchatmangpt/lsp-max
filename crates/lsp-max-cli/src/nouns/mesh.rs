use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;

#[derive(Debug, Clone, Serialize)]
pub struct MeshStatusResult {
    pub node_count: usize,
    pub method_count: usize,
    pub nodes: Vec<serde_json::Value>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeshRouteResult {
    pub method: String,
    pub decision: String,
    pub server_id: Option<String>,
    pub law_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeshRegisterResult {
    pub server_id: String,
    pub capabilities_count: usize,
    pub status: String,
}

pub struct MeshService;

impl MeshService {
    pub fn new() -> Self {
        Self
    }

    fn state_path() -> String {
        std::env::var("LSP_MAX_STATE_PATH").unwrap_or_else(|_| ".mesh_state.json".to_string())
    }

    fn load_state() -> serde_json::Value {
        let path = Self::state_path();
        if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        }
    }

    fn save_state(val: &serde_json::Value) -> std::result::Result<(), String> {
        let content = serde_json::to_string_pretty(val).map_err(|e| e.to_string())?;
        fs::write(Self::state_path(), content).map_err(|e| e.to_string())
    }

    pub fn status(&self) -> MeshStatusResult {
        let state = Self::load_state();
        let nodes: Vec<serde_json::Value> = state
            .get("mesh_nodes")
            .and_then(|n| n.as_object())
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        let method_count: usize = nodes
            .iter()
            .filter_map(|n| {
                n.get("capabilities")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
            })
            .sum();
        MeshStatusResult {
            node_count: nodes.len(),
            method_count,
            nodes,
            status: "CANDIDATE".into(),
        }
    }

    pub fn register(
        &self,
        id: &str,
        capabilities: Vec<String>,
        transport: &str,
    ) -> Result<MeshRegisterResult> {
        let mut state = Self::load_state();
        let nodes = state["mesh_nodes"]
            .as_object_mut()
            .cloned()
            .unwrap_or_default();
        let mut new_nodes = nodes;
        new_nodes.insert(
            id.to_string(),
            serde_json::json!({
                "server_id": id,
                "capabilities": capabilities,
                "transport": transport,
                "law_status": "CANDIDATE",
            }),
        );
        state["mesh_nodes"] = serde_json::Value::Object(new_nodes);
        Self::save_state(&state).map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
        Ok(MeshRegisterResult {
            server_id: id.to_string(),
            capabilities_count: capabilities.len(),
            status: "CANDIDATE".into(),
        })
    }

    pub fn route(&self, method: &str) -> MeshRouteResult {
        let state = Self::load_state();
        let nodes = state.get("mesh_nodes").and_then(|n| n.as_object());
        if let Some(nodes) = nodes {
            for (id, node) in nodes {
                let caps = node.get("capabilities").and_then(|c| c.as_array());
                if let Some(caps) = caps {
                    if caps.iter().any(|c| c.as_str() == Some(method)) {
                        let law_status = node
                            .get("law_status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("CANDIDATE")
                            .to_string();
                        return MeshRouteResult {
                            method: method.to_string(),
                            decision: "Route".into(),
                            server_id: Some(id.clone()),
                            law_status,
                        };
                    }
                }
            }
        }
        MeshRouteResult {
            method: method.to_string(),
            decision: "Unroutable".into(),
            server_id: None,
            law_status: "REFUSED".into(),
        }
    }
}

impl Default for MeshService {
    fn default() -> Self {
        Self::new()
    }
}

#[verb("status")]
pub fn status() -> Result<MeshStatusResult> {
    Ok(MeshService::new().status())
}

#[verb("register")]
pub fn register(
    id: String,
    capabilities: Option<String>,
    transport: Option<String>,
) -> Result<MeshRegisterResult> {
    let caps = capabilities
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect();
    let transport = transport.unwrap_or_else(|| "stdio".to_string());
    MeshService::new().register(&id, caps, &transport)
}

#[verb("route")]
pub fn route(method: String) -> Result<MeshRouteResult> {
    Ok(MeshService::new().route(&method))
}
