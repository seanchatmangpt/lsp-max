use lsp_types_max::DiagnosticSeverity;

/// A single law specification entry, encoding all literals for one diagnostic+gate pair.
pub struct LawSpec {
    pub diag_id: &'static str,
    pub law_id: &'static str,
    pub severity: DiagnosticSeverity,
    pub message: &'static str,
    pub gate_id: &'static str,
    pub receipt_file: &'static str,
    pub receipt_token: &'static str,
    pub repair_action_id: &'static str,
    pub repair_description: &'static str,
    pub condition: fn(&crate::ServerRegistry) -> bool,
}

/// Returns the canonical law table with all three diagnostic laws.
///
/// `condition` returns `true` when the diagnostic should be **emitted** (law is violated).
/// The table is allocated exactly once and returned as a static slice on every call.
pub fn law_table() -> &'static [LawSpec] {
    static TABLE: std::sync::OnceLock<Vec<LawSpec>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| vec![
        LawSpec {
            diag_id: "diag-uninitialized-admission",
            law_id: "LAW-001",
            severity: DiagnosticSeverity::ERROR,
            message: "Server state violates lifecycle machine match: initialize must transition to InitializingState.",
            gate_id: "gate-state-check",
            receipt_file: "admission.receipt",
            receipt_token: "rcpt-uninitialized",
            repair_action_id: "repair-state-sync",
            repair_description: "Synchronize machine state with semantic state",
            condition: |registry| {
                let gate_state_check_active =
                    registry.gates.get("gate-state-check") == Some(&true)
                        || registry.current_state != crate::service::State::Uninitialized;
                !gate_state_check_active
            },
        },
        LawSpec {
            diag_id: "diag-missing-receipt",
            law_id: "LAW-003",
            severity: DiagnosticSeverity::WARNING,
            message: "Missing validation receipt for secure admission.",
            gate_id: "gate-receipt-check",
            receipt_file: "security.receipt",
            receipt_token: "rcpt-security-auth",
            repair_action_id: "repair-apply-security-patch",
            repair_description: "Apply cryptographic admission repair",
            condition: |registry| !registry.receipts.contains_key("rcpt-security-auth"),
        },
        LawSpec {
            diag_id: "diag-auth-generator",
            law_id: "LAW-003",
            severity: DiagnosticSeverity::INFORMATION,
            message: "Generate security authorization receipt.",
            gate_id: "gate-auth-check",
            receipt_file: "auth.receipt",
            receipt_token: "generated-rcpt-security-auth",
            repair_action_id: "repair-generate-auth",
            repair_description: "Generate security authorization receipt",
            condition: |registry| !registry.receipts.contains_key("rcpt-security-auth"),
        },
    ])
}
