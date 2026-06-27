use dashmap::DashMap;
use lsp_types_max::notification::Notification;
use lsp_types_max::{Diagnostic, DiagnosticSeverity, Url};
use serde::{Deserialize, Serialize};
use crate::andon::AndonEvent;
use crate::core::Severity;

pub struct LspMaxAndonRaised;
impl Notification for LspMaxAndonRaised {
    type Params = AndonEvent;
    const METHOD: &'static str = "lspMax/andonRaised";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdmissionChangedParams {
    pub status: String,
}

pub struct LspMaxAdmissionChanged;
impl Notification for LspMaxAdmissionChanged {
    type Params = AdmissionChangedParams;
    const METHOD: &'static str = "lspMax/admissionChanged";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TruthTableChangedParams {
    pub uri: String,
}

pub struct LspMaxTruthTableChanged;
impl Notification for LspMaxTruthTableChanged {
    type Params = TruthTableChangedParams;
    const METHOD: &'static str = "lspMax/truthTableChanged";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CounterfactualFailedParams {
    pub invariant_id: String,
}

pub struct LspMaxCounterfactualFailed;
impl Notification for LspMaxCounterfactualFailed {
    type Params = CounterfactualFailedParams;
    const METHOD: &'static str = "lspMax/counterfactualFailed";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NextLawfulStepChangedParams {
    pub step: String,
}

pub struct LspMaxNextLawfulStepChanged;
impl Notification for LspMaxNextLawfulStepChanged {
    type Params = NextLawfulStepChangedParams;
    const METHOD: &'static str = "lspMax/nextLawfulStepChanged";
}

pub struct VirtualDocRegistry {
    docs: DashMap<String, String>,
}

impl VirtualDocRegistry {
    pub fn new() -> Self {
        let registry = Self {
            docs: DashMap::new(),
        };
        registry.docs.insert("lsp-max://truth/table".to_string(), String::new());
        registry.docs.insert("lsp-max://truth/true".to_string(), String::new());
        registry.docs.insert("lsp-max://truth/false".to_string(), String::new());
        registry.docs.insert("lsp-max://truth/counterfactuals".to_string(), String::new());
        registry.docs.insert("lsp-max://truth/andon".to_string(), String::new());
        registry.docs.insert("lsp-max://invariants".to_string(), String::new());
        registry.docs.insert("lsp-max://admission/gate".to_string(), String::new());
        registry.docs.insert("lsp-max://agent/next-step".to_string(), String::new());
        registry
    }

    pub fn update_doc(&self, uri: &str, content: String) {
        self.docs.insert(uri.to_string(), content);
    }

    pub fn get_doc(&self, uri: &str) -> Option<String> {
        self.docs.get(uri).map(|v| v.clone())
    }
}

pub struct LspPushAdapter;

impl LspPushAdapter {
    pub fn event_to_diagnostic(event: &AndonEvent) -> Diagnostic {
        let severity = match event.severity {
            Severity::Info => DiagnosticSeverity::INFORMATION,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Stop | Severity::Refuse => DiagnosticSeverity::ERROR,
        };

        Diagnostic {
            range: Default::default(),
            severity: Some(severity),
            code: Some(lsp_types_max::NumberOrString::String(event.code.clone())),
            source: Some("lsp-max-andon".to_string()),
            message: event.message.clone(),
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        }
    }
}
