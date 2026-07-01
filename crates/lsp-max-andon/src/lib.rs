// lsp-max-andon: ANDON layer stub
// Provides invariant registry, bus, analysis pipeline, LSP push adapter, and builder patterns.

pub mod core {
    pub struct InvariantRegistry {
        invariants: Vec<Box<dyn std::any::Any + Send>>,
    }

    impl InvariantRegistry {
        pub fn new() -> Self {
            Self {
                invariants: Vec::new(),
            }
        }

        pub fn register(&mut self, inv: impl std::any::Any + Send + 'static) {
            self.invariants.push(Box::new(inv));
        }
    }

    impl Default for InvariantRegistry {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod andon {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum AndonSeverity {
        Info,
        Warning,
        Error,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AndonEvent {
        pub id: String,
        pub message: String,
        pub severity: AndonSeverity,
        pub file: Option<String>,
        pub line: Option<u32>,
        #[serde(default)]
        pub blocking: bool,
        #[serde(default)]
        pub requires_ack: bool,
    }

    impl AndonEvent {
        pub fn new(
            id: impl Into<String>,
            message: impl Into<String>,
            severity: AndonSeverity,
        ) -> Self {
            Self {
                id: id.into(),
                message: message.into(),
                severity,
                file: None,
                line: None,
                blocking: false,
                requires_ack: false,
            }
        }

        pub fn with_file(mut self, file: impl Into<String>) -> Self {
            self.file = Some(file.into());
            self
        }

        pub fn with_line(mut self, line: u32) -> Self {
            self.line = Some(line);
            self
        }
    }

    pub struct AndonBus {
        events: Vec<AndonEvent>,
    }

    impl AndonBus {
        pub fn new() -> Self {
            Self { events: Vec::new() }
        }
        pub fn push(&mut self, event: AndonEvent) {
            self.events.push(event);
        }
        pub fn drain(&mut self) -> Vec<AndonEvent> {
            std::mem::take(&mut self.events)
        }
        pub fn events(&self) -> &[AndonEvent] {
            &self.events
        }
    }

    impl Default for AndonBus {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod analysis {
    use crate::andon::AndonEvent;
    use crate::core::InvariantRegistry;

    pub struct AnalysisPipeline;

    impl AnalysisPipeline {
        pub fn evaluate_registry(_registry: &InvariantRegistry) -> Vec<AndonEvent> {
            Vec::new()
        }
    }
}

pub mod lsp {
    use crate::andon::AndonEvent;
    use serde::{Deserialize, Serialize};

    /// Stub LSP Diagnostic type (avoids hard dep on lsp-types-max here)
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct StubDiagnostic {
        pub message: String,
        pub severity: u32,
    }

    pub struct LspPushAdapter;

    impl LspPushAdapter {
        pub fn event_to_diagnostic(event: &AndonEvent) -> StubDiagnostic {
            StubDiagnostic {
                message: event.message.clone(),
                severity: match event.severity {
                    crate::andon::AndonSeverity::Error => 1,
                    crate::andon::AndonSeverity::Warning => 2,
                    crate::andon::AndonSeverity::Info => 3,
                },
            }
        }
    }

    /// Notification type marker for LSP push
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LspMaxAndonRaised;

    impl lsp_types_max::notification::Notification for LspMaxAndonRaised {
        type Params = AndonEvent;
        const METHOD: &'static str = "lspMax/andonRaised";
    }
}

pub mod patterns {
    use crate::core::InvariantRegistry;

    pub struct Invariant {
        pub name: String,
        pub check: fn(&InvariantRegistry) -> bool,
    }

    pub fn build_empty_registry_invariant() -> Invariant {
        Invariant {
            name: "empty_registry".into(),
            check: |_| true,
        }
    }

    pub fn build_required_artifact_invariant(_path: &str) -> Invariant {
        Invariant {
            name: "required_artifact".into(),
            check: |_| true,
        }
    }

    pub fn build_marker_admission(_marker: &str) -> Invariant {
        Invariant {
            name: "marker_admission".into(),
            check: |_| true,
        }
    }

    pub fn build_need_n_invariant(_n: usize) -> Invariant {
        Invariant {
            name: "need_n".into(),
            check: |_| true,
        }
    }

    pub fn build_non_empty_check_set() -> Invariant {
        Invariant {
            name: "non_empty_check_set".into(),
            check: |_| true,
        }
    }

    pub fn build_brokered_command() -> Invariant {
        Invariant {
            name: "brokered_command".into(),
            check: |_| true,
        }
    }

    pub fn build_receipt_required() -> Invariant {
        Invariant {
            name: "receipt_required".into(),
            check: |_| true,
        }
    }
}
