pub mod admission;
pub mod agent;
pub mod client;
pub mod config;
pub mod diagnostics;
pub mod event;
pub mod hook;
pub mod metamodel;
pub mod plugin;
pub mod receipt;
pub mod rpc;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod telemetry;
pub mod workspace;

pub fn get_state_path() -> String {
    std::env::var("TOWER_LSP_MAX_STATE_PATH").unwrap_or_else(|_| ".mesh_state.json".to_string())
}

#[cfg(test)]
pub static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
