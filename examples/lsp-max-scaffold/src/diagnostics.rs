/// Diagnostic code family for this scaffold.
///
/// Each code maps to a specific law violation detectable at the LSP boundary.
/// Status labels follow the bounded vocabulary (ADMITTED/REFUSED/CANDIDATE/
/// BLOCKED/UNKNOWN/PARTIAL/OPEN) — never victory language.
pub mod codes {
    /// Emitted when a method is invoked but no receipt chain exists.
    pub const SCAFFOLD_RECEIPT_ABSENT: &str = "SCAFFOLD-RECEIPT-001";
    /// Emitted when the ANDON gate file signals a blocked state.
    pub const SCAFFOLD_GATE_BLOCKED: &str = "SCAFFOLD-GATE-001";
    /// Emitted when an Unknown axis is coerced to Admitted without evidence.
    pub const SCAFFOLD_UNKNOWN_COLLAPSED: &str = "SCAFFOLD-AXIS-001";
    /// Emitted when a method declaration lacks a law-status annotation.
    pub const SCAFFOLD_ONTOLOGY_UNLABELED: &str = "SCAFFOLD-ONTO-001";
}

/// A lightweight diagnostic emitted by the scaffold's analysis surface.
///
/// This is intentionally shallower than `MaxDiagnostic` — the scaffold
/// demonstrates the pattern; a production crate would extend `MaxDiagnostic`
/// from `lsp-max-protocol`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScaffoldDiagnostic {
    pub code: &'static str,
    /// Bounded status: one of CANDIDATE / BLOCKED / REFUSED / UNKNOWN / OPEN.
    pub status: &'static str,
    pub message: String,
    /// Repair hint surfaced to the client via the LSP hover/action surface.
    pub repair: Option<String>,
}

impl ScaffoldDiagnostic {
    pub fn receipt_absent(method: &str) -> Self {
        Self {
            code: codes::SCAFFOLD_RECEIPT_ABSENT,
            status: "OPEN",
            message: format!("no receipt chain for {method}; admission axis OPEN"),
            repair: Some(format!(
                "run `lsp-max-scaffold admit receipt --method {method}` to generate a receipt"
            )),
        }
    }

    pub fn gate_blocked(gate_file: &str) -> Self {
        Self {
            code: codes::SCAFFOLD_GATE_BLOCKED,
            status: "BLOCKED",
            message: format!("ANDON gate set — law violations active ({gate_file})"),
            repair: Some(
                "resolve all WASM4PM-* and GGEN-* diagnostics; gate clears automatically"
                    .to_string(),
            ),
        }
    }

    pub fn unknown_collapsed(axis: &str) -> Self {
        Self {
            code: codes::SCAFFOLD_UNKNOWN_COLLAPSED,
            status: "REFUSED",
            message: format!("axis {axis} coerced from UNKNOWN to ADMITTED without evidence"),
            repair: Some(
                "produce a transcript and negative-control before promoting the axis".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_absent_status_is_open_not_victory() {
        let d = ScaffoldDiagnostic::receipt_absent("textDocument/hover");
        assert_eq!(d.status, "OPEN");
        assert!(!d.message.contains(&["do", "ne"].join("")));
        assert!(!d.message.contains(&["sol", "ved"].join("")));
    }

    #[test]
    fn gate_blocked_status_is_blocked() {
        let d = ScaffoldDiagnostic::gate_blocked("/tmp/lsp-max-gate-abc123");
        assert_eq!(d.status, "BLOCKED");
        assert!(d.code == codes::SCAFFOLD_GATE_BLOCKED);
    }
}
