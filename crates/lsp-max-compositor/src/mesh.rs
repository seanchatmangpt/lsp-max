use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::routing::{RoutingDecision, RoutingStrategy, RoutingTable, ServerCapabilityDecl};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshNode {
    pub server_id: String,
    pub display_name: String,
    pub capabilities: Vec<String>,
    pub law_status: String,
    pub transport: MeshTransport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshTransport {
    Stdio,
    Tcp { host: String, port: u16 },
    InProcess,
}

#[derive(Debug, Default)]
pub struct MeshTopology {
    nodes: HashMap<String, MeshNode>,
    routing_table: RoutingTable,
}

impl MeshTopology {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            routing_table: RoutingTable::new(RoutingStrategy::PriorityAdmitted),
        }
    }

    pub fn register_node(&mut self, node: MeshNode) {
        let decl = ServerCapabilityDecl {
            server_id: node.server_id.clone(),
            methods: node.capabilities.clone(),
            law_status: node.law_status.clone(),
            priority: 0,
        };
        self.routing_table.register(&decl);
        self.nodes.insert(node.server_id.clone(), node);
    }

    pub fn route(&self, method: &str) -> RoutingDecision {
        self.routing_table.resolve(method)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn routing_summary(&self) -> HashMap<String, Vec<String>> {
        self.routing_table.summary()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &MeshNode> {
        self.nodes.values()
    }
}
