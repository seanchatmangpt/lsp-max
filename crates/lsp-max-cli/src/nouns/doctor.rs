use crate::nouns::config::ConfigService;
use crate::nouns::gate::GateService;
use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    /// Bounded status: ADMITTED / PARTIAL / UNKNOWN / BLOCKED
    pub status: String,
    pub detail: String,
    pub fix: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorResult {
    /// Bounded overall status: ADMITTED / PARTIAL / UNKNOWN / BLOCKED.
    /// BLOCKED dominates; PARTIAL or UNKNOWN demote ADMITTED.
    pub overall: String,
    pub checks: Vec<DoctorCheck>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct DoctorService;

impl DoctorService {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> DoctorResult {
        let mut checks = Vec::new();

        checks.push(self.check_gate());
        self.check_config(&mut checks);
        checks.push(self.check_toolchain());
        checks.push(self.check_workspace_resolve());

        let overall = compute_overall(&checks);
        DoctorResult { overall, checks }
    }

    fn check_gate(&self) -> DoctorCheck {
        let svc = GateService::new();
        let gate = svc.check();
        if !gate.compositor_active {
            DoctorCheck {
                id: "gate".to_string(),
                status: "UNKNOWN".to_string(),
                detail: format!("compositor not active ({})", gate.gate_file),
                fix: "cargo build -p lsp-max-cli && lsp-max-cli gate check".to_string(),
            }
        } else if gate.andon_blocked {
            DoctorCheck {
                id: "gate".to_string(),
                status: "BLOCKED".to_string(),
                detail: format!("ANDON SET ({})", gate.gate_file),
                fix: "lsp-max-cli diagnostics snapshot".to_string(),
            }
        } else {
            DoctorCheck {
                id: "gate".to_string(),
                status: "ADMITTED".to_string(),
                detail: "ANDON CLEAR".to_string(),
                fix: String::new(),
            }
        }
    }

    fn check_config(&self, checks: &mut Vec<DoctorCheck>) {
        let svc = ConfigService::new();
        for key in &["api_base", "model"] {
            let check = if svc.view(key).is_some() {
                DoctorCheck {
                    id: format!("config:{key}"),
                    status: "ADMITTED".to_string(),
                    detail: format!("{key} is set"),
                    fix: String::new(),
                }
            } else {
                DoctorCheck {
                    id: format!("config:{key}"),
                    status: "PARTIAL".to_string(),
                    detail: "using built-in default".to_string(),
                    fix: format!("lsp-max-cli config set {key} <value>"),
                }
            };
            checks.push(check);
        }
    }

    fn check_toolchain(&self) -> DoctorCheck {
        // Read the pinned channel from rust-toolchain.toml in the workspace root.
        let pin = read_toolchain_pin();

        let output = std::process::Command::new("rustup")
            .args(["show", "active-toolchain"])
            .output();

        match output {
            Err(_) => DoctorCheck {
                id: "toolchain".to_string(),
                status: "UNKNOWN".to_string(),
                detail: "rustup not found".to_string(),
                fix: "install rustup from https://rustup.rs".to_string(),
            },
            Ok(out) => {
                let active = String::from_utf8_lossy(&out.stdout)
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();

                if active.is_empty() {
                    return DoctorCheck {
                        id: "toolchain".to_string(),
                        status: "UNKNOWN".to_string(),
                        detail: "rustup not reporting an active toolchain".to_string(),
                        fix: "rustup show".to_string(),
                    };
                }

                let pin_str = pin.as_deref().unwrap_or("");
                if !pin_str.is_empty() && active.contains(pin_str) {
                    DoctorCheck {
                        id: "toolchain".to_string(),
                        status: "ADMITTED".to_string(),
                        detail: active,
                        fix: String::new(),
                    }
                } else {
                    DoctorCheck {
                        id: "toolchain".to_string(),
                        status: "PARTIAL".to_string(),
                        detail: format!(
                            "active={active} pin={}",
                            if pin_str.is_empty() { "(unreadable)" } else { pin_str }
                        ),
                        fix: format!(
                            "rustup toolchain install {}",
                            if pin_str.is_empty() { "<channel>" } else { pin_str }
                        ),
                    }
                }
            }
        }
    }

    fn check_workspace_resolve(&self) -> DoctorCheck {
        let result = std::process::Command::new("cargo")
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .output();

        match result {
            Ok(out) if out.status.success() => DoctorCheck {
                id: "resolve".to_string(),
                status: "ADMITTED".to_string(),
                detail: "cargo metadata resolves the workspace".to_string(),
                fix: String::new(),
            },
            _ => DoctorCheck {
                id: "resolve".to_string(),
                status: "BLOCKED".to_string(),
                detail: "workspace does not resolve (siblings missing?)".to_string(),
                fix: "just setup".to_string(),
            },
        }
    }
}

impl Default for DoctorService {
    fn default() -> Self {
        Self::new()
    }
}

/// BLOCKED dominates; PARTIAL or UNKNOWN demote ADMITTED.
fn compute_overall(checks: &[DoctorCheck]) -> String {
    let mut overall = "ADMITTED".to_string();
    for check in checks {
        match check.status.as_str() {
            "BLOCKED" => return "BLOCKED".to_string(),
            "PARTIAL" | "UNKNOWN" if overall == "ADMITTED" => {
                overall = check.status.clone();
            }
            _ => {}
        }
    }
    overall
}

/// Read the `channel` field from `rust-toolchain.toml` in the workspace root.
/// Returns None if the file is absent or unparseable.
fn read_toolchain_pin() -> Option<String> {
    // Walk up from cwd looking for the file.
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("rust-toolchain.toml");
        if candidate.exists() {
            let text = std::fs::read_to_string(&candidate).ok()?;
            // Parse: channel = "nightly-2026-04-15"
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("channel") {
                    if let Some(val) = trimmed.split('"').nth(1) {
                        return Some(val.to_string());
                    }
                }
            }
            return None;
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Run all bounded-status health checks and emit a DoctorResult envelope.
/// Pass `--format agent-context` to receive Ok even when overall is BLOCKED.
#[verb("run")]
pub fn run(format: Option<String>) -> Result<DoctorResult> {
    let svc = DoctorService::new();
    let result = svc.run();

    if format.as_deref() == Some("agent-context") {
        return Ok(result);
    }

    if result.overall == "BLOCKED" {
        return Err(NounVerbError::execution_error(
            "doctor: BLOCKED — run with --format agent-context for details".to_string(),
        ));
    }

    Ok(result)
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_overall_blocked_dominates() {
        let checks = vec![
            DoctorCheck {
                id: "a".into(),
                status: "ADMITTED".into(),
                detail: String::new(),
                fix: String::new(),
            },
            DoctorCheck {
                id: "b".into(),
                status: "BLOCKED".into(),
                detail: String::new(),
                fix: String::new(),
            },
            DoctorCheck {
                id: "c".into(),
                status: "PARTIAL".into(),
                detail: String::new(),
                fix: String::new(),
            },
        ];
        assert_eq!(compute_overall(&checks), "BLOCKED");
    }

    #[test]
    fn compute_overall_partial_demotes_admitted() {
        let checks = vec![
            DoctorCheck {
                id: "a".into(),
                status: "ADMITTED".into(),
                detail: String::new(),
                fix: String::new(),
            },
            DoctorCheck {
                id: "b".into(),
                status: "PARTIAL".into(),
                detail: String::new(),
                fix: String::new(),
            },
        ];
        assert_eq!(compute_overall(&checks), "PARTIAL");
    }

    #[test]
    fn compute_overall_all_admitted() {
        let checks = vec![DoctorCheck {
            id: "a".into(),
            status: "ADMITTED".into(),
            detail: String::new(),
            fix: String::new(),
        }];
        assert_eq!(compute_overall(&checks), "ADMITTED");
    }

    #[test]
    fn doctor_service_run_returns_bounded_statuses() {
        let svc = DoctorService::new();
        let result = svc.run();
        let valid = ["ADMITTED", "PARTIAL", "UNKNOWN", "BLOCKED", "OPEN"];
        assert!(
            valid.contains(&result.overall.as_str()),
            "overall status '{}' is not bounded",
            result.overall
        );
        for check in &result.checks {
            assert!(
                valid.contains(&check.status.as_str()),
                "check '{}' has unbounded status '{}'",
                check.id,
                check.status
            );
        }
    }
}
