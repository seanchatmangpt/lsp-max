use crate::andon::core::{AndonInvariant, Severity};
use crate::andon::core::RepairAction;

pub fn build_empty_registry_invariant() -> AndonInvariant {
    AndonInvariant {
        id: "LSPMAX-INVARIANT-EMPTY-REGISTRY".to_string(),
        statement: "InvariantRegistry.empty() implies ANDON".to_string(),
        scope: "system".to_string(),
        true_probe: Some("registry_has_invariants".to_string()),
        false_probe: Some("registry_empty".to_string()),
        counterfactual_probe: Some("clear_registry_fails".to_string()),
        witness_rule: Some("registry_state".to_string()),
        repair_rule: Some(RepairAction {
            id: "add_invariant".to_string(),
            title: "Add Project Invariant".to_string(),
            next_lawful_step: Some("define_invariant".to_string()),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Stop,
        blocks: true,
    }
}

pub fn build_required_artifact_invariant(path: &str) -> AndonInvariant {
    AndonInvariant {
        id: format!("RequiredArtifact:{}", path),
        statement: format!("File {} must exist.", path),
        scope: "file".to_string(),
        true_probe: Some("file_exists".to_string()),
        false_probe: Some("file_missing".to_string()),
        counterfactual_probe: Some("remove_file_fails".to_string()),
        witness_rule: Some("file_digest".to_string()),
        repair_rule: Some(RepairAction {
            id: "create_artifact".to_string(),
            title: format!("Create {}", path),
            next_lawful_step: Some(format!("create_file_{}", path)),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Stop,
        blocks: true,
    }
}

pub fn build_marker_admission(marker: &str) -> AndonInvariant {
    AndonInvariant {
        id: format!("MarkerAdmission:{}", marker),
        statement: format!("Must have {} marker to be admitted", marker),
        scope: "marker".to_string(),
        true_probe: Some("marker_present".to_string()),
        false_probe: Some("marker_missing".to_string()),
        counterfactual_probe: Some("remove_marker_fails".to_string()),
        witness_rule: Some("marker_location".to_string()),
        repair_rule: Some(RepairAction {
            id: "add_marker".to_string(),
            title: format!("Add {} marker", marker),
            next_lawful_step: Some("insert_marker".to_string()),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Stop,
        blocks: true,
    }
}

pub fn build_need_n_invariant(n: usize) -> AndonInvariant {
    AndonInvariant {
        id: format!("Need{}", n),
        statement: format!("Work unit size <= {}", n),
        scope: "decomposition".to_string(),
        true_probe: Some(format!("size_leq_{}", n)),
        false_probe: Some(format!("size_gt_{}", n)),
        counterfactual_probe: Some("add_nth_item_fails".to_string()),
        witness_rule: Some("task_count".to_string()),
        repair_rule: Some(RepairAction {
            id: format!("split_need_{}", n),
            title: "Split work unit".to_string(),
            next_lawful_step: Some(format!("split_need_{}", n)),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Refuse,
        blocks: true,
    }
}

pub fn build_non_empty_check_set() -> AndonInvariant {
    AndonInvariant {
        id: "NonEmptyCheckSet".to_string(),
        statement: "Empty checks_run is ANDON.".to_string(),
        scope: "validation".to_string(),
        true_probe: Some("checks_run_not_empty".to_string()),
        false_probe: Some("checks_run_empty".to_string()),
        counterfactual_probe: Some("disable_checker_fails".to_string()),
        witness_rule: Some("checks_report".to_string()),
        repair_rule: Some(RepairAction {
            id: "implement_check_lifecycle".to_string(),
            title: "Implement Check Lifecycle".to_string(),
            next_lawful_step: Some("implement_check_lifecycle_domain".to_string()),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Stop,
        blocks: true,
    }
}

pub fn build_brokered_command() -> AndonInvariant {
    AndonInvariant {
        id: "BrokeredCommand".to_string(),
        statement: "Heavy command requires build slot.".to_string(),
        scope: "execution".to_string(),
        true_probe: Some("has_build_slot".to_string()),
        false_probe: Some("no_build_slot".to_string()),
        counterfactual_probe: Some("direct_heavy_command_fails".to_string()),
        witness_rule: Some("build_slot_receipt".to_string()),
        repair_rule: Some(RepairAction {
            id: "request_build_slot".to_string(),
            title: "Request Build Slot".to_string(),
            next_lawful_step: Some("request_build_slot".to_string()),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Refuse,
        blocks: true,
    }
}

pub fn build_receipt_required() -> AndonInvariant {
    AndonInvariant {
        id: "ReceiptRequired".to_string(),
        statement: "Test output is not a receipt.".to_string(),
        scope: "evidence".to_string(),
        true_probe: Some("has_receipt".to_string()),
        false_probe: Some("missing_receipt".to_string()),
        counterfactual_probe: Some("hide_receipt_fails".to_string()),
        witness_rule: Some("cryptographic_receipt".to_string()),
        repair_rule: Some(RepairAction {
            id: "execute_route".to_string(),
            title: "Execute Admitted Route".to_string(),
            next_lawful_step: Some("execute_route_for_receipt".to_string()),
            command: None,
            code_action: None,
            virtual_doc_uri: None,
        }),
        severity: Severity::Stop,
        blocks: true,
    }
}
