use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::path::PathBuf;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Serialize)]
pub struct GateCheckResult {
    pub andon_blocked: bool,
    pub gate_file: String,
    /// False when the compositor process has not written the gate file yet.
    pub compositor_active: bool,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct GateService;

impl GateService {
    pub fn new() -> Self {
        Self
    }

    /// Derive the workspace-specific gate file path.
    /// Formula must match lsp-max-compositor/src/gate_file.rs exactly.
    pub fn gate_file_path() -> PathBuf {
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let hash = fnv1a(workspace.to_string_lossy().as_bytes());
        let dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        dir.join(format!("lsp-max-gate-{hash:016x}"))
    }

    /// Read the gate file. One syscall; no IPC; no subprocess.
    pub fn check(&self) -> GateCheckResult {
        let path = Self::gate_file_path();
        let compositor_active = path.exists();
        let andon_blocked = compositor_active
            && std::fs::read(&path)
                .ok()
                .and_then(|b| b.first().copied())
                .map(|b| b == b'1')
                .unwrap_or(false);
        GateCheckResult {
            andon_blocked,
            gate_file: path.display().to_string(),
            compositor_active,
        }
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Check the compositor ANDON gate file. Exits 1 if ANDON is set; 0 if clear.
/// Reads a single byte — no IPC, no subprocess — safe for PreToolUse hooks.
#[verb("check")]
pub fn check() -> Result<GateCheckResult> {
    let svc = GateService::new();
    let status = svc.check();
    if status.andon_blocked {
        return Err(NounVerbError::execution_error(format!(
            "ANDON gate BLOCKED — law violations active ({})",
            status.gate_file
        )));
    }
    Ok(status)
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_path_is_deterministic() {
        let p1 = GateService::gate_file_path();
        let p2 = GateService::gate_file_path();
        assert_eq!(p1, p2);
    }

    #[test]
    fn gate_check_returns_clear_when_compositor_absent() {
        let svc = GateService::new();
        // The compositor is not running in unit tests; the gate file does not exist.
        let path = GateService::gate_file_path();
        if !path.exists() {
            let result = svc.check();
            assert!(!result.andon_blocked);
            assert!(!result.compositor_active);
        }
    }
}
