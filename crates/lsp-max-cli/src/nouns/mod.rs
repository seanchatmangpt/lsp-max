pub mod admission;
pub mod admit;
pub mod agent;
pub mod client;
pub mod config;
pub mod conformance;
pub mod diagnostics;
pub mod event;
pub mod gate;
pub mod generate;
pub mod ggen;
pub mod hook;
pub mod mesh;
pub mod metamodel;
pub mod ontology;
pub mod pack;
pub mod plugin;
pub mod receipt;
pub mod rpc;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod stream;
pub mod telemetry;
pub mod template;
pub mod workspace;

pub fn get_state_path() -> String {
    std::env::var("LSP_MAX_STATE_PATH").unwrap_or_else(|_| ".mesh_state.json".to_string())
}

#[cfg(test)]
pub static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
