use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A server's capability declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilityDecl {
    pub server_id: String,
    pub methods: Vec<String>,
    pub law_status: String,
    pub priority: u8,
}

/// Routing strategy when multiple servers claim the same method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum RoutingStrategy {
    /// Use highest-priority (lowest priority number) ADMITTED server first
    #[default]
    PriorityAdmitted,
    /// Fan out to ALL servers claiming the method; merge responses
    Fanout,
    /// Use first-registered server (registration order)
    FirstRegistered,
}

/// The routing table: method → ordered list of candidate servers.
#[derive(Debug, Default)]
pub struct RoutingTable {
    /// method → vec of (priority, server_id, law_status)
    routes: HashMap<String, Vec<(u8, String, String)>>,
    strategy: RoutingStrategy,
}

impl RoutingTable {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            routes: HashMap::new(),
            strategy,
        }
    }

    /// Register a server's capability declarations into the routing table.
    pub fn register(&mut self, decl: &ServerCapabilityDecl) {
        for method in &decl.methods {
            let entry = self.routes.entry(method.clone()).or_default();
            entry.push((decl.priority, decl.server_id.clone(), decl.law_status.clone()));
            // Sort: ADMITTED first, then by priority number (lower = higher priority)
            entry.sort_by(|a, b| {
                let admitted_a = a.2 == "ADMITTED";
                let admitted_b = b.2 == "ADMITTED";
                admitted_b
                    .cmp(&admitted_a)
                    .then(a.0.cmp(&b.0))
            });
        }
    }

    /// Resolve which server(s) should handle a given method.
    pub fn resolve(&self, method: &str) -> RoutingDecision {
        match self.routes.get(method) {
            None => RoutingDecision::Unroutable {
                method: method.to_string(),
            },
            Some(servers) => match self.strategy {
                RoutingStrategy::Fanout => RoutingDecision::Fanout {
                    method: method.to_string(),
                    server_ids: servers.iter().map(|(_, id, _)| id.clone()).collect(),
                },
                _ => {
                    // Find first ADMITTED, or fall back to CANDIDATE, then any.
                    let best = servers
                        .iter()
                        .find(|(_, _, status)| status == "ADMITTED")
                        .or_else(|| {
                            servers
                                .iter()
                                .find(|(_, _, status)| status == "CANDIDATE")
                        })
                        .or_else(|| servers.first());
                    match best {
                        Some((_, id, status)) => RoutingDecision::Route {
                            method: method.to_string(),
                            server_id: id.clone(),
                            law_status: status.clone(),
                        },
                        None => RoutingDecision::Unroutable {
                            method: method.to_string(),
                        },
                    }
                }
            },
        }
    }

    /// List all registered routes as a summary map.
    pub fn summary(&self) -> HashMap<String, Vec<String>> {
        self.routes
            .iter()
            .map(|(method, servers)| {
                let ids: Vec<String> = servers
                    .iter()
                    .map(|(_, id, status)| format!("{id}({status})"))
                    .collect();
                (method.clone(), ids)
            })
            .collect()
    }

    pub fn method_count(&self) -> usize {
        self.routes.len()
    }

    pub fn server_count(&self) -> usize {
        let mut ids = std::collections::HashSet::new();
        for servers in self.routes.values() {
            for (_, id, _) in servers {
                ids.insert(id.clone());
            }
        }
        ids.len()
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum RoutingDecision {
    Route {
        method: String,
        server_id: String,
        law_status: String,
    },
    Fanout {
        method: String,
        server_ids: Vec<String>,
    },
    Unroutable {
        method: String,
    },
}

impl RoutingDecision {
    pub fn status(&self) -> &str {
        match self {
            Self::Route { law_status, .. } => law_status,
            Self::Fanout { .. } => "CANDIDATE",
            Self::Unroutable { .. } => "REFUSED",
        }
    }
}
