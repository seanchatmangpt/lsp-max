use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Bounded outcome of a doctor-repair invocation. The verdict is never
/// victorious: a clear run is `ADMITTED`, a run with at least one failed safe
/// action is `PARTIAL`, a detector-only surfacing is `OPEN`, and an
/// environment we cannot drive is `UNKNOWN` / `BLOCKED`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum RepairVerdict {
    /// Plan printed, or every applied safe action came back ADMITTED.
    Admitted,
    /// --apply ran but at least one safe action was BLOCKED/PARTIAL.
    Partial,
    /// Detector-only; nothing in scope to repair (plan mode default channel).
    Open,
    /// The repair driver could not be run or located.
    Blocked,
    /// Outcome could not be determined (e.g. non-numeric exit).
    Unknown,
}

impl RepairVerdict {
    fn from_plan_exit(code: Option<i32>) -> Self {
        match code {
            Some(0) => RepairVerdict::Admitted,
            Some(2) => RepairVerdict::Blocked,
            Some(_) => RepairVerdict::Partial,
            None => RepairVerdict::Unknown,
        }
    }

    fn from_apply_exit(code: Option<i32>) -> Self {
        match code {
            Some(0) => RepairVerdict::Admitted,
            Some(1) => RepairVerdict::Partial,
            Some(2) => RepairVerdict::Blocked,
            Some(_) => RepairVerdict::Unknown,
            None => RepairVerdict::Unknown,
        }
    }
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Drives `scripts/doctor-repair.sh` — the self-healing arm. The noun owns no
/// repair logic itself; the script is the single auditable implementation so the
/// CLI surface and a direct `bash scripts/doctor-repair.sh` invocation cannot
/// drift apart.
pub struct RepairService;

impl RepairService {
    pub fn new() -> Self {
        Self
    }

    /// Resolve the repair script path. Anchored on the git work-tree root so the
    /// noun behaves identically regardless of the caller's working directory.
    pub fn script_path() -> PathBuf {
        Self::repo_root().join("scripts/doctor-repair.sh")
    }

    fn repo_root() -> PathBuf {
        if let Ok(out) = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return PathBuf::from(s);
                }
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn run(&self, apply: bool) -> std::result::Result<RepairRun, String> {
        let script = Self::script_path();
        if !script.exists() {
            return Err(format!(
                "repair driver not found at {} (BLOCKED)",
                script.display()
            ));
        }

        let mut cmd = Command::new("bash");
        cmd.arg(&script);
        if apply {
            cmd.arg("--apply");
        }
        cmd.current_dir(Self::repo_root());

        let output = cmd
            .output()
            .map_err(|e| format!("failed to spawn repair driver: {e} (BLOCKED)"))?;

        let code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let verdict = if apply {
            RepairVerdict::from_apply_exit(code)
        } else {
            RepairVerdict::from_plan_exit(code)
        };

        // The receipt path is the only proof of an applied action; stdout is not.
        // The script prints `Receipt written: <path>` on the apply path.
        let receipt_path = if apply {
            stdout
                .lines()
                .find_map(|l| l.trim().strip_prefix("Receipt written:"))
                .map(|p| p.trim().to_string())
        } else {
            None
        };

        Ok(RepairRun {
            verdict,
            exit_code: code,
            receipt_path,
            stdout,
            stderr,
        })
    }

    pub fn plan(&self) -> std::result::Result<RepairRun, String> {
        self.run(false)
    }

    pub fn apply(&self) -> std::result::Result<RepairRun, String> {
        self.run(true)
    }
}

impl Default for RepairService {
    fn default() -> Self {
        Self::new()
    }
}

/// Raw result of driving the script once.
pub struct RepairRun {
    pub verdict: RepairVerdict,
    pub exit_code: Option<i32>,
    pub receipt_path: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

/// Result returned by `repair plan`.
#[derive(Serialize)]
pub struct RepairPlanResult {
    pub verdict: RepairVerdict,
    pub exit_code: Option<i32>,
    /// Bounded-status repair plan text emitted by the driver. This is a plan,
    /// not a receipt: it surfaces detected breakages, it does not prove action.
    pub plan: String,
}

/// Print the bounded-status REPAIR PLAN. Read-only: detects disk pressure,
/// tracked-but-ignored artifacts, committed conflict markers, and manifest
/// path/version drift, and mutates nothing.
#[verb("plan")]
pub fn plan() -> Result<RepairPlanResult> {
    let svc = RepairService::new();
    let run = svc.plan().map_err(NounVerbError::execution_error)?;
    Ok(RepairPlanResult {
        verdict: run.verdict,
        exit_code: run.exit_code,
        plan: run.stdout,
    })
}

/// Result returned by `repair apply`.
#[derive(Serialize)]
pub struct RepairApplyResult {
    pub verdict: RepairVerdict,
    pub exit_code: Option<i32>,
    /// Path to the receipt artifact produced by the apply run. `None` means the
    /// driver wrote no receipt — which is itself an UNKNOWN, never an admission.
    pub receipt_path: Option<String>,
    pub stdout: String,
}

/// Perform ONLY the safe filesystem / git-index repairs (prune regenerable build
/// dirs, untrack tracked-but-ignored runtime artifacts) and write a signed
/// receipt to receipts/. Source conflict markers and manifest drift are reported
/// by the plan but never auto-edited.
#[verb("apply")]
pub fn apply() -> Result<RepairApplyResult> {
    let svc = RepairService::new();
    let run = svc.apply().map_err(NounVerbError::execution_error)?;
    if run.verdict == RepairVerdict::Blocked {
        return Err(NounVerbError::execution_error(format!(
            "repair apply BLOCKED (exit {:?}): {}",
            run.exit_code,
            run.stderr.trim()
        )));
    }
    Ok(RepairApplyResult {
        verdict: run.verdict,
        exit_code: run.exit_code,
        receipt_path: run.receipt_path,
        stdout: run.stdout,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_path_ends_with_doctor_repair() {
        let p = RepairService::script_path();
        assert!(
            p.ends_with("scripts/doctor-repair.sh"),
            "unexpected script path: {}",
            p.display()
        );
    }

    #[test]
    fn plan_exit_maps_to_bounded_verdicts() {
        assert_eq!(RepairVerdict::from_plan_exit(Some(0)), RepairVerdict::Admitted);
        assert_eq!(RepairVerdict::from_plan_exit(Some(2)), RepairVerdict::Blocked);
        assert_eq!(RepairVerdict::from_plan_exit(Some(7)), RepairVerdict::Partial);
        assert_eq!(RepairVerdict::from_plan_exit(None), RepairVerdict::Unknown);
    }

    #[test]
    fn apply_exit_maps_to_bounded_verdicts() {
        assert_eq!(
            RepairVerdict::from_apply_exit(Some(0)),
            RepairVerdict::Admitted
        );
        assert_eq!(RepairVerdict::from_apply_exit(Some(1)), RepairVerdict::Partial);
        assert_eq!(RepairVerdict::from_apply_exit(Some(2)), RepairVerdict::Blocked);
        assert_eq!(RepairVerdict::from_apply_exit(None), RepairVerdict::Unknown);
    }

    #[test]
    fn unknown_verdict_is_distinct_from_admitted_and_refused_polarity() {
        // Unknown must never collapse into an ADMITTED/PARTIAL/BLOCKED polarity.
        assert_ne!(RepairVerdict::Unknown, RepairVerdict::Admitted);
        assert_ne!(RepairVerdict::Unknown, RepairVerdict::Partial);
        assert_ne!(RepairVerdict::Unknown, RepairVerdict::Blocked);
    }
}
