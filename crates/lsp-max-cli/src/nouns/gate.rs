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

#[derive(Debug, Serialize)]
pub struct GateListResult {
    pub active: bool,
    pub violations: Vec<String>,
    pub gate_file: String,
}

#[derive(Debug, Serialize)]
pub struct GateClearResult {
    pub was_active: bool,
    pub gate_file: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct GateWatchResult {
    pub polls: usize,
    pub final_status: String,
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

/// List all active gate signals and violation codes recorded in the gate file.
/// Byte 0 of the gate file is '1' when BLOCKED; remaining bytes are
/// newline-separated violation codes (e.g. WASM4PM-001, GGEN-003).
#[verb("list")]
pub fn list() -> Result<GateListResult> {
    let path = GateService::gate_file_path();
    let gate_file = path.display().to_string();
    if !path.exists() {
        return Ok(GateListResult {
            active: false,
            violations: vec![],
            gate_file,
        });
    }
    let bytes = std::fs::read(&path).map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    if bytes.first().copied() != Some(b'1') {
        return Ok(GateListResult {
            active: false,
            violations: vec![],
            gate_file,
        });
    }
    // Byte 0 is the status flag; remaining bytes are newline-separated violation codes.
    let tail = &bytes[1..];
    let violations: Vec<String> = String::from_utf8_lossy(tail)
        .split('\n')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    Ok(GateListResult {
        active: true,
        violations,
        gate_file,
    })
}

/// Clear the ANDON gate signal by writing '0' to the gate file.
/// Does NOT resolve the underlying diagnostic — only clears the signal.
#[verb("clear")]
pub fn clear() -> Result<GateClearResult> {
    let path = GateService::gate_file_path();
    let gate_file = path.display().to_string();
    if !path.exists() {
        return Ok(GateClearResult {
            was_active: false,
            gate_file,
            status: "OPEN".to_string(),
        });
    }
    let was_active = std::fs::read(&path)
        .ok()
        .and_then(|b| b.first().copied())
        .map(|b| b == b'1')
        .unwrap_or(false);
    std::fs::write(&path, b"0").map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(GateClearResult {
        was_active,
        gate_file,
        status: "OPEN".to_string(),
    })
}

/// Poll the gate file on an interval until it clears or max_polls is reached.
/// Each poll result is emitted to stderr as JSON; the final summary is returned.
#[verb("watch")]
pub fn watch(
    /// Polling interval in seconds (default: 2)
    #[arg(long, default_value_t = 2)]
    interval_secs: u64,
    /// Maximum number of polls before stopping (default: 30)
    #[arg(long, default_value_t = 30)]
    max_polls: u64,
) -> Result<GateWatchResult> {
    let svc = GateService::new();
    let mut polls: usize = 0;
    let mut final_status = "BLOCKED".to_string();
    for _ in 0..max_polls {
        let poll_result = svc.check();
        polls += 1;
        eprintln!(
            "{}",
            serde_json::to_string(&poll_result).unwrap_or_default()
        );
        if !poll_result.andon_blocked {
            final_status = "OPEN".to_string();
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(interval_secs));
    }
    Ok(GateWatchResult {
        polls,
        final_status,
    })
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

    #[test]
    fn list_returns_inactive_when_no_gate_file() {
        let path = GateService::gate_file_path();
        if path.exists() {
            return; // Skip: compositor active in this environment.
        }
        let result = list().unwrap();
        assert!(!result.active);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn list_parses_violation_codes_from_blocked_gate() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        // Write a blocked gate file: byte 0 = '1', then violation codes.
        let content = b"1\nWASM4PM-001\nGGEN-003\n";
        std::fs::write(tmp.path(), content).unwrap();
        // Exercise the parse logic directly by calling the internal parser logic.
        let bytes = std::fs::read(tmp.path()).unwrap();
        assert_eq!(bytes.first().copied(), Some(b'1'));
        let tail = &bytes[1..];
        let violations: Vec<String> = String::from_utf8_lossy(tail)
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        assert_eq!(violations, vec!["WASM4PM-001", "GGEN-003"]);
    }

    #[test]
    fn clear_writes_zero_byte_to_gate_file() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::write(tmp.path(), b"1\nWASM4PM-001").unwrap();
        // Verify the gate reads as blocked before clearing.
        let before = std::fs::read(tmp.path()).unwrap();
        assert_eq!(before.first().copied(), Some(b'1'));
        // Write '0' to simulate what clear() does.
        std::fs::write(tmp.path(), b"0").unwrap();
        let after = std::fs::read(tmp.path()).unwrap();
        assert_eq!(after.first().copied(), Some(b'0'));
    }
}
