use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================
//
// The `report` noun renders the project-health surface: ONE call that shows
// where this law-state runtime stands across its governing axes (gate, law
// compliance, doc coverage, LSP 3.18 surface, workspace crates, siblings). It
// drives `scripts/status-report.sh`, which owns the aggregation and is the sole
// source of the bounded statuses. This noun is READ-ONLY: it observes the
// script's output and reports it; it mutates no tracked file. Each axis carries
// a bounded status (ADMITTED / CANDIDATE / BLOCKED / REFUSED / UNKNOWN /
// PARTIAL / OPEN); UNKNOWN is never collapsed into a polarity.

/// One axis of the status surface, mirroring a `metrics.*` block emitted by
/// `scripts/status-report.sh --json`.
#[derive(Debug, Clone, Serialize)]
pub struct StatusAxis {
    pub axis: String,
    pub status: String,
    pub detail: String,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Locates and drives the read-only status-report script.
pub struct ReportService {
    script_path: PathBuf,
}

impl ReportService {
    pub fn new() -> Self {
        Self {
            script_path: locate_status_script(),
        }
    }

    /// Run `scripts/status-report.sh --json` and return its parsed JSON block.
    /// The script is the authority for the bounded statuses; this method does
    /// not re-derive them.
    pub fn status_json(&self) -> std::result::Result<serde_json::Value, String> {
        let raw = self.run(&["--json"])?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("status-report.sh did not emit valid JSON: {e}"))
    }

    /// Run `scripts/status-report.sh` and return its rendered human table as
    /// captured stdout text (ANSI color codes preserved as the script writes).
    pub fn status_table(&self) -> std::result::Result<String, String> {
        self.run(&[])
    }

    fn run(&self, args: &[&str]) -> std::result::Result<String, String> {
        if !self.script_path.exists() {
            return Err(format!(
                "status-report.sh not located at {}",
                self.script_path.display()
            ));
        }
        let output = Command::new("bash")
            .arg(&self.script_path)
            .args(args)
            .output()
            .map_err(|e| format!("failed to invoke status-report.sh: {e}"))?;

        // The script exits 1 when posture is BLOCKED; that is a bounded signal,
        // not an invocation failure. Stdout still carries the full report, so we
        // surface it rather than treating a BLOCKED posture as an error.
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if stdout.trim().is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "status-report.sh produced no output (stderr: {})",
                stderr.trim()
            ));
        }
        Ok(stdout)
    }
}

impl Default for ReportService {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve `scripts/status-report.sh` from the current working directory,
/// walking up parent directories so the noun works whether invoked from the
/// workspace root or a crate subdirectory.
fn locate_status_script() -> PathBuf {
    let rel = PathBuf::from("scripts/status-report.sh");
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir: Option<&std::path::Path> = Some(cwd.as_path());
        while let Some(d) = dir {
            let candidate = d.join(&rel);
            if candidate.exists() {
                return candidate;
            }
            dir = d.parent();
        }
    }
    rel
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

/// Result of `report status`. In `--json` mode `surface` carries the full
/// machine block emitted by the script; in human mode `rendered` carries the
/// captured table and `surface` is `None`.
#[derive(Serialize)]
pub struct ReportStatusResult {
    /// Bounded overall posture (e.g. ADMITTED / PARTIAL / BLOCKED). UNKNOWN when
    /// the surface could not be observed; never a victory claim.
    pub posture: String,
    /// Per-axis bounded statuses (populated in `--json` mode).
    pub axes: Vec<StatusAxis>,
    /// The raw machine block from `status-report.sh --json` (only in `--json` mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<serde_json::Value>,
    /// The captured human table (only when `--json` is not set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered: Option<String>,
}

/// Render the project-health surface for the law-state runtime.
///
/// `--json` emits the machine block (axes + posture + sibling versions) for
/// agents and CI; without it, the captured human table is returned. The verb is
/// read-only — it drives `scripts/status-report.sh` and reports its bounded
/// output, mutating nothing.
#[verb("status")]
pub fn status(json: Option<bool>) -> Result<ReportStatusResult> {
    let svc = ReportService::new();
    let want_json = json.unwrap_or(false);

    if want_json {
        let surface = svc.status_json().map_err(NounVerbError::execution_error)?;
        let posture = surface
            .get("posture")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        let axes = extract_axes(&surface);
        Ok(ReportStatusResult {
            posture,
            axes,
            surface: Some(surface),
            rendered: None,
        })
    } else {
        let rendered = svc.status_table().map_err(NounVerbError::execution_error)?;
        // Recover the posture token from the rendered table so the structured
        // result still carries a bounded posture even in human mode.
        let posture = parse_posture_from_table(&rendered);
        Ok(ReportStatusResult {
            posture,
            axes: Vec::new(),
            surface: None,
            rendered: Some(rendered),
        })
    }
}

/// Pull each `metrics.*` block out of the script's JSON into typed axes.
fn extract_axes(surface: &serde_json::Value) -> Vec<StatusAxis> {
    let Some(metrics) = surface.get("metrics").and_then(|m| m.as_object()) else {
        return Vec::new();
    };
    let mut axes = Vec::with_capacity(metrics.len());
    for (name, block) in metrics {
        let status = block
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        let detail = block
            .get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        axes.push(StatusAxis {
            axis: name.clone(),
            status,
            detail,
        });
    }
    axes
}

/// Best-effort recovery of the bounded posture token from the rendered table.
/// Returns UNKNOWN when no bounded token is found (never fabricates a polarity).
fn parse_posture_from_table(table: &str) -> String {
    for line in table.lines() {
        if line.contains("POSTURE:") {
            for token in ["BLOCKED", "REFUSED", "PARTIAL", "CANDIDATE", "OPEN", "ADMITTED"] {
                if line.contains(token) {
                    return token.to_string();
                }
            }
        }
    }
    "UNKNOWN".to_string()
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_axes_reads_metrics_blocks() {
        let surface = serde_json::json!({
            "metrics": {
                "gate": { "status": "UNKNOWN", "detail": "not observed" },
                "law_compliance": { "status": "ADMITTED", "detail": "no violations" }
            }
        });
        let axes = extract_axes(&surface);
        assert_eq!(axes.len(), 2);
        assert!(axes.iter().any(|a| a.axis == "gate" && a.status == "UNKNOWN"));
        assert!(axes
            .iter()
            .any(|a| a.axis == "law_compliance" && a.status == "ADMITTED"));
    }

    #[test]
    fn extract_axes_missing_metrics_is_empty_not_panic() {
        let surface = serde_json::json!({ "posture": "PARTIAL" });
        assert!(extract_axes(&surface).is_empty());
    }

    #[test]
    fn extract_axes_defaults_absent_status_to_unknown() {
        let surface = serde_json::json!({
            "metrics": { "doc_coverage": { "detail": "narrative only" } }
        });
        let axes = extract_axes(&surface);
        assert_eq!(axes.len(), 1);
        assert_eq!(axes[0].status, "UNKNOWN");
    }

    #[test]
    fn parse_posture_recovers_bounded_token() {
        let table = "  POSTURE: \x1b[1;33mPARTIAL\x1b[0m\n";
        assert_eq!(parse_posture_from_table(table), "PARTIAL");
    }

    #[test]
    fn parse_posture_unknown_when_absent() {
        assert_eq!(parse_posture_from_table("no posture line here"), "UNKNOWN");
    }

    #[test]
    fn parse_posture_blocked_takes_precedence_in_line() {
        // A line naming both tokens resolves to the stronger gap signal first.
        let table = "  POSTURE: BLOCKED (was ADMITTED before regression)\n";
        assert_eq!(parse_posture_from_table(table), "BLOCKED");
    }

    #[test]
    fn locate_script_returns_relative_fallback_when_absent() {
        // Pure path logic: the fallback is the relative script path. We assert
        // the tail rather than existence to avoid coupling to the test cwd.
        let p = locate_status_script();
        assert!(p.ends_with("scripts/status-report.sh"));
    }
}
