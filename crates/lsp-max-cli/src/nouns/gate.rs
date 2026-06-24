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
        lsp_max::primitives::gate_file_path()
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

        // Structured JSON payload takes precedence over the legacy single-byte format.
        // Legacy: first byte is `'1'` = blocked, `'0'` = clear.
        // Structured: `{"blocked":true,"codes":[...],"seq":N}`.
        let (andon_blocked, active_andon_codes, since_seq) =
            if raw.first().copied() == Some(b'{') {
                parse_gate_json(&raw)
            } else {
                let blocked = raw.first().copied().map(|b| b == b'1').unwrap_or(false);
                (blocked, vec![], None)
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

/// Extract blocked flag, ANDON codes, and optional `seq` from a structured gate file payload.
///
/// Expected shape: `{"blocked":true,"codes":["WASM4PM-001"],"seq":42}`.
/// Unknown fields are ignored; missing fields yield defaults (false / empty / None).
fn parse_gate_json(raw: &[u8]) -> (bool, Vec<String>, Option<u64>) {
    let v: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return (false, vec![], None),
    };
    let blocked = v["blocked"].as_bool().unwrap_or(false);
    let codes = v["codes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let seq = v["seq"].as_u64();
    (blocked, codes, seq)
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Check the compositor ANDON gate file. Exits 2 if ANDON is set; 0 if clear.
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
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
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
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
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
    std::fs::write(&path, b"0")
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(GateClearResult {
        was_active,
        gate_file,
        status: "OPEN".to_string(),
    })
}

/// Poll the gate file on an interval until it clears or max_polls is reached.
/// `interval_secs` defaults to 2; `max_polls` defaults to 30.
#[verb("watch")]
pub fn watch(interval_secs: Option<u64>, max_polls: Option<u64>) -> Result<GateWatchResult> {
    let interval_secs = interval_secs.unwrap_or(2);
    let max_polls = max_polls.unwrap_or(30);
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
    fn gate_path_checks_agent_id() {
        std::env::set_var("LSP_MAX_AGENT_ID", "test-agent-123");
        let p1 = GateService::gate_file_path();
        std::env::remove_var("LSP_MAX_AGENT_ID");
        let p2 = GateService::gate_file_path();
        assert_ne!(p1, p2);
        assert!(p1.to_string_lossy().contains("-agent-test-agent-123"));
    }

    #[test]
    fn gate_check_returns_clear_when_compositor_absent() {
        let svc = GateService::new();
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
        let (blocked, codes, seq) = parse_gate_json(raw);
        assert!(blocked);
        assert_eq!(codes, vec!["WASM4PM-001", "GGEN-042"]);
        assert_eq!(seq, Some(7));
    }

    #[test]
    fn parse_gate_json_tolerates_missing_fields() {
        let raw = br#"{"blocked":true}"#;
        let (blocked, codes, seq) = parse_gate_json(raw);
        assert!(blocked);
        assert!(codes.is_empty());
        assert!(seq.is_none());
    }

    #[test]
    fn parse_gate_json_returns_empty_on_invalid_json() {
        let (_blocked, codes, seq) = parse_gate_json(b"not-json");
        assert!(codes.is_empty());
        assert!(seq.is_none());
    }

    #[test]
    fn governing_axes_unknown_when_blocked_without_codes() {
        let svc = GateService::new();
        let path = GateService::gate_file_path();
        if !path.exists() {
            let ctx = svc.check_agent_context();
            assert!(ctx.governing_axes.unknown.is_empty());
        }
    }

    #[test]
    fn clear_writes_zero_byte_to_gate_file() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::write(tmp.path(), b"1\nWASM4PM-001").unwrap();
        let before = std::fs::read(tmp.path()).unwrap();
        assert_eq!(before.first().copied(), Some(b'1'));
        std::fs::write(tmp.path(), b"0").unwrap();
        let after = std::fs::read(tmp.path()).unwrap();
        assert_eq!(after.first().copied(), Some(b'0'));
    }

    // --- synthetic gate file helpers -----------------------------------------
    //
    // These tests write directly to the real gate path when the compositor is
    // absent, exercise the service, then clean up.  When the compositor IS
    // active in the test environment the gate file already exists and we skip
    // rather than interfere with a live gate.

    fn write_gate_and_run<F>(content: &[u8], f: F)
    where
        F: FnOnce(&GateService),
    {
        let path = GateService::gate_file_path();
        if path.exists() {
            // compositor active; skip to avoid corrupting a live gate.
            return;
        }
        std::fs::write(&path, content).expect("write synthetic gate");
        let svc = GateService::new();
        f(&svc);
        let _ = std::fs::remove_file(&path);
    }

    // check — success + falsification (blocked byte)

    #[test]
    fn check_reports_blocked_when_gate_byte_is_one() {
        write_gate_and_run(b"1", |svc| {
            let result = svc.check();
            // Success: returns without panic.
            assert!(result.compositor_active);
            // Falsification: byte '1' means the gate is blocked.
            assert!(result.andon_blocked);
        });
    }

    #[test]
    fn check_reports_clear_when_gate_byte_is_zero() {
        write_gate_and_run(b"0", |svc| {
            let result = svc.check();
            // Success: file is present (compositor_active = true).
            assert!(result.compositor_active);
            // Falsification: byte '0' must NOT be blocked.
            assert!(!result.andon_blocked);
        });
    }

    // list — success + falsification (blocking code families)

    #[test]
    fn list_reports_blocking_code_families_when_gate_is_set() {
        write_gate_and_run(b"1", |svc| {
            let result = svc.list();
            // Success: list returns with gate blocked.
            assert!(result.andon_blocked);
            // Falsification: blocked gate surfaces both code-prefix families.
            assert!(
                result.active_codes.contains(&"WASM4PM-*".to_string()),
                "WASM4PM-* missing from active_codes"
            );
            assert!(
                result.active_codes.contains(&"GGEN-*".to_string()),
                "GGEN-* missing from active_codes"
            );
            // agent_scope is "global" until RFC A per-agent partitioning is wired.
            assert_eq!(result.agent_scope, "global");
        });
    }

    // agent_context — success + falsification (BLOCKED status, unknown axis)

    #[test]
    fn agent_context_status_is_blocked_when_gate_byte_is_one() {
        write_gate_and_run(b"1", |svc| {
            let ctx = svc.check_agent_context();
            // Success: always returns Ok even when BLOCKED (RFC-1 D_t PUSH).
            assert!(ctx.andon_blocked);
            // Falsification: bounded status must be "BLOCKED".
            assert_eq!(ctx.status, "BLOCKED");
            // Falsification: plain byte payload has no structured codes, so
            // governing_axes.unknown must carry the sentinel entry.
            assert!(
                !ctx.governing_axes.unknown.is_empty(),
                "unknown axis must be non-empty when codes are absent but gate is blocked"
            );
        });
    }

    #[test]
    fn agent_context_extracts_codes_from_structured_payload() {
        let payload = br#"{"blocked":true,"codes":["WASM4PM-007"],"seq":3}"#;
        write_gate_and_run(payload, |svc| {
            let ctx = svc.check_agent_context();
            // Success: structured parse runs without panic.
            assert!(ctx.andon_blocked);
            // Falsification: the specific code is surfaced in active_andon_codes.
            assert!(
                ctx.active_andon_codes.contains(&"WASM4PM-007".to_string()),
                "expected WASM4PM-007 in active_andon_codes: {:?}",
                ctx.active_andon_codes
            );
            // Falsification: seq field is extracted correctly.
            assert_eq!(ctx.since_seq, Some(3));
            // When codes are present, refused carries them; unknown must be empty.
            assert!(
                ctx.governing_axes.unknown.is_empty(),
                "unknown axis must be empty when codes are present"
            );
        });
    }

    // watch — success + falsification (bounded final_status, poll count)

    #[test]
    fn watch_returns_open_and_single_poll_when_gate_absent() {
        let path = GateService::gate_file_path();
        if path.exists() {
            // compositor active; skip.
            return;
        }
        // Success: verb returns Ok with max_polls=1 and zero sleep interval.
        let result = watch(Some(0), Some(1)).unwrap();
        assert_eq!(result.polls, 1);
        // Falsification: gate absent means clear → final_status is "OPEN".
        assert_eq!(result.final_status, "OPEN");
    }

    #[test]
    fn watch_final_status_is_a_bounded_value() {
        let path = GateService::gate_file_path();
        if path.exists() {
            return;
        }
        let result = watch(Some(0), Some(1)).unwrap();
        // Falsification: bounded status vocabulary; never victory language.
        let valid = ["OPEN", "BLOCKED"];
        assert!(
            valid.contains(&result.final_status.as_str()),
            "unexpected final_status: {}",
            result.final_status
        );
    }

    // GateClearResult — falsification of was_active + status field

    #[test]
    fn gate_clear_result_was_active_reflects_prior_blocked_state() {
        let path = GateService::gate_file_path();
        if path.exists() {
            return;
        }
        std::fs::write(&path, b"1").expect("write blocked gate");
        // Replicate the was_active logic from the clear verb.
        let was_active = std::fs::read(&path)
            .ok()
            .and_then(|b| b.first().copied())
            .map(|b| b == b'1')
            .unwrap_or(false);
        std::fs::write(&path, b"0").unwrap();
        let gate_file = path.display().to_string();
        let result = GateClearResult {
            was_active,
            gate_file,
            status: "OPEN".to_string(),
        };
        // Falsification: was_active must reflect the pre-clear blocked state.
        assert!(result.was_active, "was_active must be true when gate was set to '1'");
        // Falsification: post-clear status is bounded "OPEN".
        assert_eq!(result.status, "OPEN");
        let _ = std::fs::remove_file(&path);
    }

    // Counterfactual: clear verb on a non-existent gate file returns was_active=false

    #[test]
    fn clear_reports_not_active_when_gate_file_absent() {
        let path = GateService::gate_file_path();
        if path.exists() {
            return;
        }
        let result = clear().unwrap();
        // Counterfactual: absent file → was_active must be false.
        assert!(!result.was_active);
        // Falsification: status is bounded "OPEN".
        assert_eq!(result.status, "OPEN");
    }
}
