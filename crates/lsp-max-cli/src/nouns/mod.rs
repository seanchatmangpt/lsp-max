pub mod admission;
pub mod agent;
pub mod alias;
pub mod batch;
pub mod client;
pub mod config;
pub mod conformance;
pub mod diagnostics;
pub mod doctor;
pub mod event;
pub mod export;
pub mod gate;
pub mod history;
pub mod hook;
pub mod import;
pub mod logs;
pub mod metamodel;
pub mod metrics;
pub mod ocel;
pub mod plugin;
pub mod process;
pub mod receipt;
pub mod repair;
pub mod rpc;
pub mod server;
pub mod snapshot;
pub mod state;
pub mod swarm;
pub mod task;
pub mod telemetry;
pub mod workspace;

pub fn get_state_path() -> String {
    std::env::var("LSP_MAX_STATE_PATH").unwrap_or_else(|_| ".mesh_state.json".to_string())
}

#[cfg(test)]
pub static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
