use serde::{Deserialize, Serialize};

pub const INTENT_DECLARE: &str = "max/intent.declare";
pub const INTENT_VALIDATE: &str = "max/intent.validate";
pub const INTENT_REVOKE: &str = "max/intent.revoke";
pub const INTENT_LIST: &str = "max/intent.list";

/// Declared action categories for intent pre-flight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentKind {
    /// Agent intends to write/mutate a file
    FileWrite { uri: String },
    /// Agent intends to run a shell command
    ShellExec { command: String },
    /// Agent intends to call an LSP method
    LspCall { method: String },
    /// Agent intends to promote a CANDIDATE to ADMITTED
    AdmissionPromotion { method: String },
    /// Agent intends to push to git remote
    GitPush { branch: String },
    /// Agent intends to create a receipt artifact
    ReceiptGeneration { receipt_id: String },
    /// Custom intent for extension methods
    Custom {
        kind: String,
        payload: serde_json::Value,
    },
}

/// Pre-flight validation outcome for a declared intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentOutcome {
    /// Intent is clear — action may proceed
    Cleared,
    /// Intent is blocked — action must not proceed
    Blocked { reason: String },
    /// Intent needs clarification before proceeding
    ClarificationRequired { question: String },
    /// Intent is deferred — retry after condition is met
    Deferred { condition: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeclareParams {
    /// Unique intent ID (client-generated)
    pub intent_id: String,
    /// The declared action
    pub kind: IntentKind,
    /// Human-readable description of why this action is being taken
    pub rationale: String,
    /// Optional: related document context (serialised URI string)
    pub context_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeclareResult {
    pub intent_id: String,
    pub outcome: IntentOutcome,
    /// CANDIDATE | ADMITTED | REFUSED
    pub law_status: String,
    pub gate_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentValidateParams {
    pub intent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentValidateResult {
    pub intent_id: String,
    pub valid: bool,
    pub outcome: IntentOutcome,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRevokeParams {
    pub intent_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRevokeResult {
    pub intent_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentListParams {
    /// "Cleared" | "Blocked" | "ClarificationRequired"
    pub filter_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentListResult {
    pub intents: Vec<IntentSummary>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSummary {
    pub intent_id: String,
    pub kind_label: String,
    pub outcome: String,
    pub law_status: String,
    pub gate_open: bool,
}

/// In-process intent registry for the current session.
pub struct IntentRegistry {
    intents: std::collections::HashMap<String, (IntentDeclareParams, IntentDeclareResult)>,
}

impl IntentRegistry {
    pub fn new() -> Self {
        Self {
            intents: std::collections::HashMap::new(),
        }
    }

    pub fn declare(&mut self, params: IntentDeclareParams) -> IntentDeclareResult {
        let outcome = self.validate_intent(&params);
        let gate_open = matches!(outcome, IntentOutcome::Cleared);
        let law_status = if gate_open {
            "CANDIDATE".into()
        } else {
            "REFUSED".into()
        };
        let result = IntentDeclareResult {
            intent_id: params.intent_id.clone(),
            outcome: outcome.clone(),
            law_status,
            gate_open,
        };
        self.intents
            .insert(params.intent_id.clone(), (params, result.clone()));
        result
    }

    fn validate_intent(&self, params: &IntentDeclareParams) -> IntentOutcome {
        match &params.kind {
            IntentKind::FileWrite { uri }
                if uri.contains("tower-lsp") || uri.contains("tower_lsp") =>
            {
                IntentOutcome::Blocked {
                    reason: "LawViolation: uri contains forbidden tower-lsp reference".into(),
                }
            }
            IntentKind::ShellExec { command } if command.contains("--no-verify") => {
                IntentOutcome::Blocked {
                    reason: "LawViolation: --no-verify bypasses law compliance hooks".into(),
                }
            }
            IntentKind::AdmissionPromotion { method } if method.is_empty() => {
                IntentOutcome::ClarificationRequired {
                    question: "Which method is being promoted?".into(),
                }
            }
            _ => IntentOutcome::Cleared,
        }
    }

    pub fn revoke(&mut self, id: &str) -> Option<IntentRevokeResult> {
        self.intents.remove(id).map(|_| IntentRevokeResult {
            intent_id: id.to_string(),
            status: "REVOKED".into(),
        })
    }

    pub fn list(&self, filter: Option<&str>) -> IntentListResult {
        let intents: Vec<IntentSummary> = self
            .intents
            .values()
            .filter(|(_, r)| filter.map(|f| r.outcome.label() == f).unwrap_or(true))
            .map(|(p, r)| IntentSummary {
                intent_id: p.intent_id.clone(),
                kind_label: p.kind.label(),
                outcome: r.outcome.label().to_string(),
                law_status: r.law_status.clone(),
                gate_open: r.gate_open,
            })
            .collect();
        let total = intents.len();
        IntentListResult { intents, total }
    }
}

impl Default for IntentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentKind {
    pub fn label(&self) -> String {
        match self {
            Self::FileWrite { uri } => format!("FileWrite:{uri}"),
            Self::ShellExec { command } => format!("ShellExec:{command}"),
            Self::LspCall { method } => format!("LspCall:{method}"),
            Self::AdmissionPromotion { method } => format!("AdmissionPromotion:{method}"),
            Self::GitPush { branch } => format!("GitPush:{branch}"),
            Self::ReceiptGeneration { receipt_id } => {
                format!("ReceiptGeneration:{receipt_id}")
            }
            Self::Custom { kind, .. } => format!("Custom:{kind}"),
        }
    }
}

impl IntentOutcome {
    pub fn label(&self) -> &str {
        match self {
            Self::Cleared => "Cleared",
            Self::Blocked { .. } => "Blocked",
            Self::ClarificationRequired { .. } => "ClarificationRequired",
            Self::Deferred { .. } => "Deferred",
        }
    }
}
