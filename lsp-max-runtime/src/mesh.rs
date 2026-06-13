use crate::mesh_hooks::{IntakeClearHook, IntakeDiagnosticHook, OcelProcessHook};
use crate::mesh_types::{
    AutonomicMeshState, ConformanceDeltaEntry, Hook, HookEvent, InstanceId, LspInstance, LspPhase,
    MaxDiagnostic, MeshAction, PolicyState,
};
use crate::sha256::sha256;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::Path;

const MAX_EVENT_LOG: usize = 1000;
const MAX_DISPATCH_DEPTH: usize = 16;
const MAX_CONFORMANCE_DELTA_LOG: usize = 4096;

pub struct AutonomicMesh {
    pub instances: HashMap<String, LspInstance>,
    pub hooks: Vec<Box<dyn Hook>>,
    pub event_log: Vec<HookEvent>,
    pub executed_bounded_actions: Vec<String>,
    pub extra: HashMap<String, serde_json::Value>,
    pub action_seq: u64,
    pub conformance_delta_log: VecDeque<ConformanceDeltaEntry>,
    dispatch_depth: usize,
}

impl Default for AutonomicMesh {
    fn default() -> Self {
        Self::new()
    }
}

pub type MaxMesh = AutonomicMesh;

pub fn build_conformance_vector(
    diagnostics: &[MaxDiagnostic],
) -> lsp_max_protocol::ConformanceVector {
    let mut axis_map: HashMap<lsp_max_protocol::LawAxis, bool> = HashMap::new();
    for diag in diagnostics {
        let is_error = matches!(
            diag.lsp.severity,
            Some(lsp_types_max::DiagnosticSeverity::ERROR)
        );
        let entry = axis_map.entry(diag.law_axis.clone()).or_insert(false);
        if is_error {
            *entry = true;
        }
    }

    let mut admitted = vec![];
    let mut refused = vec![];
    for (axis, has_error) in &axis_map {
        if *has_error {
            refused.push(axis.clone());
        } else {
            admitted.push(axis.clone());
        }
    }

    let total = admitted.len() + refused.len();
    let derived_score = if total == 0 {
        None
    } else {
        Some(100.0 * admitted.len() as f64 / total as f64)
    };

    let witnessed: std::collections::HashSet<lsp_max_protocol::LawAxis> =
        axis_map.keys().cloned().collect();
    let unknown: Vec<lsp_max_protocol::LawAxis> = lsp_max_protocol::LawAxis::all_named()
        .iter()
        .filter(|ax| !witnessed.contains(ax))
        .cloned()
        .collect();

    let mut cv = lsp_max_protocol::ConformanceVector {
        admitted,
        refused,
        unknown,
        score: derived_score,
        strict_mode: true,
        process_quality: None,
        ..Default::default()
    };
    cv.sync_bits_from_vecs();
    cv
}

impl AutonomicMesh {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            hooks: Vec::new(),
            event_log: Vec::new(),
            executed_bounded_actions: Vec::new(),
            extra: HashMap::new(),
            action_seq: 0,
            conformance_delta_log: VecDeque::new(),
            dispatch_depth: 0,
        }
    }

    pub fn to_state(&self) -> AutonomicMeshState {
        AutonomicMeshState {
            instances: self.instances.clone(),
            event_log: self.event_log.clone(),
            executed_bounded_actions: self.executed_bounded_actions.clone(),
            extra: self.extra.clone(),
        }
    }

    pub fn load_state(&mut self, state: AutonomicMeshState) {
        self.instances = state.instances;
        self.event_log = state.event_log;
        self.executed_bounded_actions = state.executed_bounded_actions;
        self.extra = state.extra;
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let mut mesh = Self::new();
        if Path::new(path).exists() {
            let data = std::fs::read_to_string(path)?;
            if let Ok(state) = serde_json::from_str::<AutonomicMeshState>(&data) {
                mesh.load_state(state);
            }
        } else {
            let mut lsp1 = LspInstance::new("LSP_1");
            lsp1.phase = LspPhase::Initialized;
            let mut lsp2 = LspInstance::new("LSP_2");
            lsp2.phase = LspPhase::Initialized;
            lsp2.policy_state = Some(PolicyState::Operational);

            mesh.add_instance(lsp1);
            mesh.add_instance(lsp2);
            mesh.save_to_file(path)?;
        }
        mesh.register_hook(Box::new(IntakeDiagnosticHook));
        mesh.register_hook(Box::new(IntakeClearHook));
        mesh.register_hook(Box::new(OcelProcessHook::new()));
        Ok(mesh)
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let state = self.to_state();
        let serialized =
            serde_json::to_string_pretty(&state).map_err(|e| io::Error::other(e.to_string()))?;
        std::fs::write(path, serialized)?;
        Ok(())
    }

    pub fn add_instance(&mut self, instance: LspInstance) {
        self.instances.insert(instance.id.clone(), instance);
    }

    pub fn register_instance(&mut self, id: String) {
        self.add_instance(LspInstance::new(&id));
    }

    pub fn register_hook(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    pub fn hook_descriptors(&self) -> Vec<crate::mesh_types::HookDescriptor> {
        self.hooks.iter().map(|h| h.descriptor()).collect()
    }

    pub fn dispatch_event(&mut self, event: HookEvent) {
        if self.dispatch_depth >= MAX_DISPATCH_DEPTH {
            self.event_log.push(HookEvent::DiagnosticEmitted {
                instance_id: InstanceId::from("mesh"),
                diagnostic: Box::new(MaxDiagnostic {
                    lsp: lsp_types_max::Diagnostic {
                        range: lsp_types_max::Range::default(),
                        severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("lsp-max".to_string()),
                        message: format!(
                            "Dispatch depth {} exceeds limit {MAX_DISPATCH_DEPTH}; recursive hook chain terminated",
                            self.dispatch_depth
                        ),
                        related_information: None,
                        tags: None,
                        data: None,
                    },
                    diagnostic_id: format!("dispatch-depth-exceeded-{}", self.dispatch_depth),
                    law_id: "MESH_DISPATCH_DEPTH".to_string(),
                    attempted_transition: None,
                    violated_axes: vec!["recursion-safety".to_string()],
                    doc_routes: vec![],
                    repair_actions: vec![],
                    verification_gates: vec![],
                    receipt_obligation: None,
                    law_axis: lsp_max_protocol::LawAxis::Security,
                    violated_invariant: String::new(),
                    observed_state: serde_json::Value::Null,
                    expected_state: serde_json::Value::Null,
                    repairability: lsp_max_protocol::Repairability::NotRepairable,
                    terminality: lsp_max_protocol::Terminality::Terminal,
                }),
            });
            return;
        }
        self.dispatch_depth += 1;

        self.event_log.push(event.clone());
        if self.event_log.len() > MAX_EVENT_LOG {
            self.event_log.drain(..self.event_log.len() - MAX_EVENT_LOG);
        }

        let mut actions = Vec::new();
        for hook in &self.hooks {
            let triggered = hook.trigger(&event);
            actions.extend(triggered);
        }

        for action in actions {
            self.execute_action(action);
        }

        self.dispatch_depth = self.dispatch_depth.saturating_sub(1);
    }

    pub fn execute_action(&mut self, action: MeshAction) {
        self.action_seq = self.action_seq.saturating_add(1);
        let seq = self.action_seq;

        let maybe_instance_id: Option<String> = match &action {
            MeshAction::AddDiagnostic { instance_id, .. }
            | MeshAction::ClearDiagnostic { instance_id, .. }
            | MeshAction::TransitionPolicyState { instance_id, .. }
            | MeshAction::EmitReceipt { instance_id, .. }
            | MeshAction::ExecuteBoundedAction { instance_id, .. }
            | MeshAction::ResetInstance { instance_id } => Some(instance_id.0.clone()),
        };
        let old_score: Option<f64> = maybe_instance_id
            .as_deref()
            .and_then(|id| self.instances.get(id))
            .map(|inst| inst.conformance_score());

        match action {
            MeshAction::TransitionPolicyState {
                instance_id,
                new_state,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    let old_state = instance
                        .policy_state
                        .clone()
                        .unwrap_or(PolicyState::Operational);
                    instance.policy_state = Some(new_state.clone());

                    let event = HookEvent::PolicyStateChanged {
                        instance_id,
                        from_state: old_state,
                        to_state: new_state,
                    };
                    self.dispatch_event(event);
                }
            }
            MeshAction::ClearDiagnostic {
                instance_id,
                diagnostic_id,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    let old_len = instance.diagnostics.len();
                    instance
                        .diagnostics
                        .retain(|d| d.diagnostic_id != diagnostic_id);
                    if instance.diagnostics.len() < old_len {
                        instance.invalidate_score_cache();
                        let event = HookEvent::DiagnosticCleared {
                            instance_id,
                            diagnostic_id,
                        };
                        self.dispatch_event(event);
                    }
                }
            }
            MeshAction::AddDiagnostic {
                instance_id,
                diagnostic,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    instance.diagnostics.push((*diagnostic).clone());
                    instance.invalidate_score_cache();
                    let event = HookEvent::DiagnosticEmitted {
                        instance_id,
                        diagnostic,
                    };
                    self.dispatch_event(event);
                }
            }
            MeshAction::EmitReceipt {
                instance_id,
                mut receipt,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    receipt.hash = sha256(receipt.receipt_id.as_bytes());
                    instance.receipts.push(receipt.clone());
                    let event = HookEvent::ReceiptEmitted {
                        instance_id,
                        receipt,
                    };
                    self.dispatch_event(event);
                }
            }
            MeshAction::ExecuteBoundedAction {
                instance_id,
                action_id,
                description,
            } => {
                if action_id == "act-create-refund-receipt" {
                    let receipt_dir =
                        std::env::var("MESH_RECEIPT_DIR").unwrap_or_else(|_| ".".to_string());
                    let file_path = Path::new(&receipt_dir).join("refund_receipt.txt");
                    let content = format!(
                        "REFUND RECEIPT\nInstance: {}\nDescription: {}\nStatus: Executed\nTimestamp: {}\n",
                        instance_id,
                        description,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or(std::time::Duration::ZERO)
                            .as_secs()
                    );
                    if let Err(e) = std::fs::write(&file_path, content) {
                        eprintln!(
                            "warn: failed to write receipt to {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
                self.dispatch_event(HookEvent::BoundedActionExecuted {
                    instance_id,
                    action_id: action_id.clone(),
                    description: description.clone(),
                });
                self.executed_bounded_actions.push(action_id);
            }
            MeshAction::ResetInstance { instance_id } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    instance.diagnostics.clear();
                    instance.receipts.clear();
                    instance.policy_state = Some(PolicyState::Operational);
                    instance.invalidate_score_cache();
                    self.dispatch_event(HookEvent::InstanceReset { instance_id });
                }
            }
        }

        if let Some(iid) = maybe_instance_id {
            if let Some(new_score) = self
                .instances
                .get(&iid)
                .map(|inst| inst.conformance_score())
            {
                if let Some(old) = old_score {
                    if (new_score - old).abs() > f64::EPSILON {
                        let entry = ConformanceDeltaEntry {
                            seq,
                            instance_id: iid,
                            old_score: old,
                            new_score,
                        };
                        self.conformance_delta_log.push_back(entry);
                        if self.conformance_delta_log.len() > MAX_CONFORMANCE_DELTA_LOG {
                            self.conformance_delta_log.pop_front();
                        }
                    }
                }
            }
        }
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<String, String> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "diagnose" => {
                if parts.len() < 6 {
                    return Err(
                        "Usage: diagnose <instance_id> <diag_id> <law_id> <severity> <msg...>"
                            .to_string(),
                    );
                }
                let instance_id = InstanceId::from(parts[1]);
                let diag_id = parts[2].to_string();
                let law_id = parts[3].to_string();
                let severity_str = parts[4];
                let msg = parts[5..].join(" ");

                let severity = match severity_str {
                    "error" => Some(lsp_types_max::DiagnosticSeverity::ERROR),
                    "warning" => Some(lsp_types_max::DiagnosticSeverity::WARNING),
                    "info" => Some(lsp_types_max::DiagnosticSeverity::INFORMATION),
                    "hint" => Some(lsp_types_max::DiagnosticSeverity::HINT),
                    _ => return Err(format!("Unknown severity: {}", severity_str)),
                };

                let diagnostic = MaxDiagnostic {
                    lsp: lsp_types_max::Diagnostic {
                        range: lsp_types_max::Range::default(),
                        severity,
                        code: None,
                        code_description: None,
                        source: Some("autonomic-mesh".to_string()),
                        message: msg,
                        related_information: None,
                        tags: None,
                        data: None,
                    },
                    diagnostic_id: diag_id,
                    law_id,
                    attempted_transition: None,
                    violated_axes: vec!["semantic".to_string()],
                    doc_routes: vec![],
                    repair_actions: vec![],
                    verification_gates: vec![],
                    receipt_obligation: None,
                    law_axis: lsp_max_protocol::LawAxis::Domain,
                    violated_invariant: String::new(),
                    observed_state: serde_json::Value::Null,
                    expected_state: serde_json::Value::Null,
                    repairability: lsp_max_protocol::Repairability::Unknown,
                    terminality: lsp_max_protocol::Terminality::NonTerminal,
                };

                self.execute_action(MeshAction::AddDiagnostic {
                    instance_id: instance_id.clone(),
                    diagnostic: Box::new(diagnostic),
                });

                Ok(format!("Emitted diagnostic on {}", instance_id))
            }
            "clear" => {
                if parts.len() < 3 {
                    return Err("Usage: clear <instance_id> <diag_id>".to_string());
                }
                let instance_id = InstanceId::from(parts[1]);
                let diag_id = parts[2].to_string();

                self.execute_action(MeshAction::ClearDiagnostic {
                    instance_id: instance_id.clone(),
                    diagnostic_id: diag_id,
                });

                Ok(format!("Cleared diagnostic on {}", instance_id))
            }
            "state" => {
                if parts.len() < 2 {
                    return Err("Usage: state <instance_id>".to_string());
                }
                let instance_id = parts[1];
                if let Some(inst) = self.instances.get(instance_id) {
                    let policy_str = match &inst.policy_state {
                        Some(p) => format!("{:?}", p),
                        None => "None".to_string(),
                    };
                    Ok(format!(
                        "Instance: {} | Phase: {} | Conformance: {} | PolicyState: {} | Diags: {} | Receipts: {}",
                        inst.id,
                        inst.phase,
                        inst.conformance_score(),
                        policy_str,
                        inst.diagnostics.len(),
                        inst.receipts.len()
                    ))
                } else {
                    Err(format!("Instance not found: {}", instance_id))
                }
            }
            "patch" => {
                if parts.len() < 3 {
                    return Err("Usage: patch <instance_id> <policy_state>".to_string());
                }
                let instance_id = InstanceId::from(parts[1]);
                let new_state = parts[2].parse::<PolicyState>()?;

                self.execute_action(MeshAction::TransitionPolicyState {
                    instance_id: instance_id.clone(),
                    new_state,
                });

                Ok(format!("Patched state on {}", instance_id))
            }
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }
}
