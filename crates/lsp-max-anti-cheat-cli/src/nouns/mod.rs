pub mod check;
pub mod config;
pub mod rules;
pub mod scan;

fn get_state_path() -> String {
    std::env::var("ANTI_CHEAT_STATE_PATH")
        .unwrap_or_else(|_| ".anti-cheat-state.json".to_string())
}
