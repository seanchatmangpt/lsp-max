use super::law_table::{law_table, LawSpec};
use crate::ServerRegistry;
use lsp_types_max::*;
use std::collections::HashMap;
use std::str::FromStr;

pub(crate) fn get_receipt_uri(root_path: &std::path::Path, filename: &str) -> lsp_types_max::Uri {
    let fallback = lsp_types_max::Uri::from_str("file:///")
        .expect("static URI 'file:///' is valid UTF-8 and always parses");
    url::Url::from_file_path(root_path.join(filename))
        .ok()
        .and_then(|u| lsp_types_max::Uri::from_str(u.as_str()).ok())
        .unwrap_or(fallback)
}

/// Build a `MaxDiagnostic` for a given `LawSpec`.
fn build_diagnostic(spec: &LawSpec) -> max_protocol::MaxDiagnostic {
    let diag_id = spec.diag_id.to_string();
    let lsp_diag = Diagnostic {
        range: Range::default(),
        severity: Some(spec.severity),
        code: None,
        code_description: None,
        source: Some("lsp-max".to_string()),
        message: spec.message.to_string(),
        related_information: None,
        tags: None,
        data: None,
    };

    // Law-specific fields that differ between entries.
    let (violated_axes, doc_routes, receipt_obligation) = match spec.diag_id {
        "diag-uninitialized-admission" => (
            vec!["LSP State Mapping".to_string()],
            vec![max_protocol::DocRoute {
                path: "/doc/lifecycle".to_string(),
            }],
            None,
        ),
        "diag-missing-receipt" => (
            vec!["Receipt Integrity".to_string()],
            vec![max_protocol::DocRoute {
                path: "/doc/receipts".to_string(),
            }],
            Some(max_protocol::ReceiptObligation {
                required_receipts: vec!["rcpt-security-auth".to_string()],
            }),
        ),
        "diag-auth-generator" => (vec!["Receipt Integrity".to_string()], vec![], None),
        _ => (vec![], vec![], None),
    };

    max_protocol::MaxDiagnostic {
        lsp: lsp_diag,
        diagnostic_id: diag_id,
        law_id: spec.law_id.to_string(),
        attempted_transition: None,
        violated_axes,
        doc_routes,
        repair_actions: vec![max_protocol::RepairAction {
            action_id: spec.repair_action_id.to_string(),
            description: spec.repair_description.to_string(),
        }],
        verification_gates: vec![max_protocol::GateId(spec.gate_id.to_string())],
        receipt_obligation,
        law_axis: max_protocol::LawAxis::default(),
        violated_invariant: String::new(),
        observed_state: serde_json::Value::Null,
        expected_state: serde_json::Value::Null,
        repairability: max_protocol::Repairability::default(),
        terminality: max_protocol::Terminality::default(),
    }
}

/// Build a `MaxCodeAction` for a given `LawSpec` and the already-built diagnostic.
fn build_action(
    spec: &LawSpec,
    diag: &max_protocol::MaxDiagnostic,
    uri: lsp_types_max::Uri,
) -> max_protocol::MaxCodeAction {
    let mut changes = HashMap::new();
    changes.insert(
        uri,
        vec![TextEdit {
            range: Range::default(),
            new_text: format!("{}\n", spec.receipt_token),
        }],
    );

    let lsp_action = CodeAction {
        title: spec.repair_description.to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.lsp.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
            ..Default::default()
        }),
        ..Default::default()
    };

    // Preconditions and receipt_plan differ per law entry.
    let (preconditions, expected_receipts) = match spec.diag_id {
        "diag-uninitialized-admission" => (
            vec![max_protocol::Precondition {
                condition: "State is Uninitialized".to_string(),
            }],
            vec![],
        ),
        "diag-missing-receipt" => (vec![], vec!["rcpt-security-auth".to_string()]),
        _ => (vec![], vec![]),
    };

    let rollback_strategy = match spec.diag_id {
        "diag-uninitialized-admission" => "Revert state to Uninitialized".to_string(),
        _ => "None".to_string(),
    };

    max_protocol::MaxCodeAction {
        action: lsp_action,
        preconditions,
        validation_plan: max_protocol::ValidationPlan {
            gates: vec![max_protocol::GateId(spec.gate_id.to_string())],
        },
        rollback_plan: max_protocol::RollbackPlan {
            strategy: rollback_strategy,
        },
        receipt_plan: max_protocol::ReceiptPlan { expected_receipts },
    }
}

pub(crate) fn update_diagnostics(registry: &mut ServerRegistry) {
    let root_path = registry.root_path.clone();

    for spec in law_table() {
        let diag_id = spec.diag_id.to_string();
        let cleared = registry.cleared_diagnostics.contains(&diag_id);
        let violated = (spec.condition)(registry);

        if violated && !cleared {
            let diag = build_diagnostic(spec);
            registry.diagnostics.insert(diag_id.clone(), diag.clone());
            let uri = get_receipt_uri(&root_path, spec.receipt_file);
            let action = build_action(spec, &diag, uri);
            registry.repair_plans.insert(diag_id, vec![action]);
        } else {
            registry.diagnostics.remove(&diag_id);
            registry.repair_plans.remove(&diag_id);
        }
    }
}
