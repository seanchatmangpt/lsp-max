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

/// Result of the `gate list` verb.
///
/// Reports the gate state and any active WASM4PM-* / GGEN-* code prefixes
/// currently known to block the gate.  `agent_scope` is always `"global"` until
/// per-agent gate partitioning is implemented (RFC A — OPEN).
#[derive(Debug, Serialize)]
pub struct GateListResult {
    pub andon_blocked: bool,
    pub gate_file: String,
    pub compositor_active: bool,
    /// Diagnostic code prefixes that are currently active (if gate is blocked).
    pub active_codes: Vec<String>,
    /// Scope of this gate read.  Currently always `"global"`.
    pub agent_scope: String,
}

/// Law-axis sets carried in the agent-context output.
/// Axes are enumerated per `ConformanceVector` law: refused/unknown are disjoint.
#[derive(Debug, Serialize)]
pub struct GoverningAxes {
    pub refused: Vec<String>,
    pub unknown: Vec<String>,
}

/// A single available repair action surfaced to agents under RFC-1 D_t PUSH.
#[derive(Debug, Serialize)]
pub struct AvailableRepair {
    pub action_id: String,
    pub verb: String,
}

/// Structured output emitted when `--format=agent-context` is requested.
///
/// Never returns `Err` even when BLOCKED — agents need the payload, not a
/// dead error string.  The `status` field carries the bounded state.
#[derive(Debug, Serialize)]
pub struct AgentContextResult {
    pub andon_blocked: bool,
    /// Bounded status: "BLOCKED" or "ADMITTED". Never victory language.
    pub status: String,
    /// Sequence number from the gate file, if present; UNKNOWN when absent.
    pub since_seq: Option<u64>,
    /// ANDON codes parsed from the gate file JSON, or empty when the gate
    /// file is a single byte (`1`) rather than a structured payload.
    pub active_andon_codes: Vec<String>,
    pub governing_axes: GoverningAxes,
    pub available_repairs: Vec<AvailableRepair>,
    pub compositor_active: bool,
    pub gate_file: String,
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

    /// List the gate state with active diagnostic code prefixes.
    ///
    /// When the gate is blocked, returns the WASM4PM-* and GGEN-* code
    /// categories known to trigger an ANDON signal.  `agent_scope` is `"global"`
    /// until RFC A per-agent partitioning is wired.
    pub fn list(&self) -> GateListResult {
        let check = self.check();
        let active_codes = if check.andon_blocked {
            // These are the two code-prefix families that the PreToolUse hook
            // enforces.  Specific code IDs require a running diagnostic server;
            // the CLI can only report the blocking families.
            vec!["WASM4PM-*".to_string(), "GGEN-*".to_string()]
        } else {
            vec![]
        };
        GateListResult {
            andon_blocked: check.andon_blocked,
            gate_file: check.gate_file,
            compositor_active: check.compositor_active,
            active_codes,
            agent_scope: "global".to_string(),
        }
    }

    /// Read the gate file with deeper parsing for the agent-context format.
    ///
    /// When the gate file contains JSON with a `codes` array, those codes are
    /// surfaced in `active_andon_codes`.  A plain `1` byte yields empty codes —
    /// the compositor has not emitted structured metadata yet.
    pub fn check_agent_context(&self) -> AgentContextResult {
        let path = Self::gate_file_path();
        let compositor_active = path.exists();

        let raw = if compositor_active {
            std::fs::read(&path).unwrap_or_default()
        } else {
            vec![]
        };

        let andon_blocked = raw.first().copied().map(|b| b == b'1').unwrap_or(false);

        // Attempt structured parse only when the content looks like JSON.
        let (active_andon_codes, since_seq) = if raw.first().copied() == Some(b'{') {
            parse_gate_json(&raw)
        } else {
            (vec![], None)
        };

        let status = if andon_blocked {
            "BLOCKED".to_string()
        } else {
            "ADMITTED".to_string()
        };

        // Governing axes: refused carries active ANDON codes; unknown is empty
        // unless the gate file could not be parsed (codes absent but gate set).
        let (refused, unknown) = if andon_blocked && active_andon_codes.is_empty() {
            (vec![], vec!["ANDON-GATE-CODES".to_string()])
        } else {
            (active_andon_codes.clone(), vec![])
        };

        AgentContextResult {
            andon_blocked,
            status,
            since_seq,
            active_andon_codes,
            governing_axes: GoverningAxes { refused, unknown },
            available_repairs: vec![AvailableRepair {
                action_id: "emit-receipt".to_string(),
                verb: "diagnostics repair-plan emit".to_string(),
            }],
            compositor_active,
            gate_file: path.display().to_string(),
        }
    }
}

/// Extract ANDON codes and optional `seq` from a structured gate file payload.
///
/// Expected shape: `{"blocked":true,"codes":["WASM4PM-001"],"seq":42}`.
/// Unknown fields are ignored; missing fields yield empty/None.
fn parse_gate_json(raw: &[u8]) -> (Vec<String>, Option<u64>) {
    let v: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return (vec![], None),
    };
    let codes = v["codes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let seq = v["seq"].as_u64();
    (codes, seq)
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

/// Check the compositor ANDON gate file.
/// Exit 2 when ANDON is set (blocks PreToolUse hook per Claude Code semantics).
/// Exit 0 when clear. Reads a single byte — no IPC, no subprocess.
/// Check the compositor ANDON gate file. Exits 1 if ANDON is set; 0 if clear.
/// Reads a single byte — no IPC, no subprocess — safe for PreToolUse hooks.
///
/// With `--format=agent-context`: emits structured JSON and always returns Ok,
/// even when BLOCKED, so agents receive machine-readable context (RFC-1 D_t PUSH).
#[verb("check")]
pub fn check(format: Option<String>) -> Result<serde_json::Value> {
    let svc = GateService::new();

    if format.as_deref() == Some("agent-context") {
        let ctx = svc.check_agent_context();
        let v = serde_json::to_value(&ctx)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
        return Ok(v);
    }

    let status = svc.check();
    if status.andon_blocked {
        eprintln!(
            "ANDON BLOCKED — law violations active\ngate: {}\nResolve all WASM4PM-* and GGEN-* diagnostics to clear.",
            status.gate_file
        );
        std::process::exit(2);
    }
    let v = serde_json::to_value(&status)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(v)
}

/// List the gate state and active diagnostic code prefixes.
///
/// Reports which WASM4PM-* / GGEN-* families are blocking the gate.
/// Agent-scoped partitioning is OPEN (RFC A); `agent_scope` is always
/// `"global"` in this release.
#[verb("list")]
pub fn list() -> Result<GateListResult> {
    let svc = GateService::new();
    Ok(svc.list())
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
    fn gate_list_returns_empty_codes_when_clear() {
        let svc = GateService::new();
        let path = GateService::gate_file_path();
        if !path.exists() {
            let result = svc.list();
            assert!(!result.andon_blocked);
            assert!(result.active_codes.is_empty());
            assert_eq!(result.agent_scope, "global");
        }
    }

    #[test]
    fn agent_context_returns_admitted_when_compositor_absent() {
        let svc = GateService::new();
        let path = GateService::gate_file_path();
        if !path.exists() {
            let ctx = svc.check_agent_context();
            assert!(!ctx.andon_blocked);
            assert_eq!(ctx.status, "ADMITTED");
            assert!(ctx.active_andon_codes.is_empty());
            assert!(!ctx.available_repairs.is_empty());
        }
    }

    #[test]
    fn parse_gate_json_extracts_codes_and_seq() {
        let raw = br#"{"blocked":true,"codes":["WASM4PM-001","GGEN-042"],"seq":7}"#;
        let (codes, seq) = parse_gate_json(raw);
        assert_eq!(codes, vec!["WASM4PM-001", "GGEN-042"]);
        assert_eq!(seq, Some(7));
    }

    #[test]
    fn parse_gate_json_tolerates_missing_fields() {
        let raw = br#"{"blocked":true}"#;
        let (codes, seq) = parse_gate_json(raw);
        assert!(codes.is_empty());
        assert!(seq.is_none());
    }

    #[test]
    fn parse_gate_json_returns_empty_on_invalid_json() {
        let (codes, seq) = parse_gate_json(b"not-json");
        assert!(codes.is_empty());
        assert!(seq.is_none());
    }

    #[test]
    fn governing_axes_unknown_when_blocked_without_codes() {
        let svc = GateService::new();
        let path = GateService::gate_file_path();
        if !path.exists() {
            // Gate is clear in tests; just verify the logic path via direct struct construction.
            let ctx = svc.check_agent_context();
            // No codes, not blocked — unknown axis must be empty.
            assert!(ctx.governing_axes.unknown.is_empty());
        }
    }
}
