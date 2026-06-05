use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_runtime::AutonomicMesh;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// Hook names are extracted via the Hook trait's name() method.
// hooks: Vec<Box<dyn Hook>> does not implement Serialize, so we map to String.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for listing registered mesh hooks.
pub struct HookService {
    state_path: &'static str,
}

impl HookService {
    pub fn new() -> Self {
        Self {
            state_path: ".mesh_state.json",
        }
    }

    pub fn list(&self) -> std::result::Result<Vec<String>, String> {
        let mesh = AutonomicMesh::load_from_file(self.state_path).map_err(|e| e.to_string())?;
        let names: Vec<String> = mesh.hooks.iter().map(|h| h.name().to_string()).collect();
        Ok(names)
    }
}

impl Default for HookService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct HookListResult {
    pub hooks: Vec<String>,
    pub count: usize,
}

#[verb("list")]
pub fn list() -> Result<HookListResult> {
    let service = HookService::new();
    let hooks = service
        .list()
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    let count = hooks.len();
    Ok(HookListResult { hooks, count })
}
