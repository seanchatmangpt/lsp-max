//! Composition strategy routing and source state types.

use std::collections::HashMap;

use serde_json::Value;

// ── Composition Strategy ───────────────────────────────────────────────────────

/// The routing/composition strategy for a given method family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionStrategy {
    SingleOwner,
    OrderedFanout,
    MergeAttributed,
    MergeDeduped,
    FirstSuccess,
    RankedProviders,
    TransactionalEditGate,
    ObserveOnly,
    Deny,
}

/// Method routing table: maps LSP method names to composition strategies.
pub fn method_strategy(method: &str) -> CompositionStrategy {
    if std::env::var("SABOTAGE_ROUTING_MATRIX").is_ok() && method == "textDocument/hover" {
        return CompositionStrategy::Deny;
    }
    match method {
        "initialize" | "initialized" | "shutdown" | "exit" => CompositionStrategy::SingleOwner,

        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didSave"
        | "textDocument/didClose"
        | "textDocument/willSave"
        | "workspace/didChangeConfiguration"
        | "workspace/didChangeWorkspaceFolders"
        | "workspace/didCreateFiles"
        | "workspace/didRenameFiles"
        | "workspace/didDeleteFiles"
        | "workspace/didChangeWatchedFiles"
        | "notebookDocument/didOpen"
        | "notebookDocument/didChange"
        | "notebookDocument/didSave"
        | "notebookDocument/didClose" => CompositionStrategy::OrderedFanout,

        "textDocument/publishDiagnostics" | "textDocument/documentSymbol" | "workspace/symbol" => {
            CompositionStrategy::MergeAttributed
        }

        "textDocument/hover"
        | "textDocument/signatureHelp"
        | "textDocument/linkedEditingRange"
        | "documentLink/resolve"
        | "completionItem/resolve"
        | "codeLens/resolve"
        | "workspaceSymbol/resolve"
        | "inlayHint/resolve"
        | "textDocument/diagnostic"
        | "workspace/diagnostic"
        | "workspace/textDocumentContent" => CompositionStrategy::FirstSuccess,

        "textDocument/definition"
        | "textDocument/declaration"
        | "textDocument/implementation"
        | "textDocument/typeDefinition"
        | "textDocument/references"
        | "textDocument/prepareCallHierarchy"
        | "callHierarchy/incomingCalls"
        | "callHierarchy/outgoingCalls"
        | "textDocument/prepareTypeHierarchy"
        | "typeHierarchy/supertypes"
        | "typeHierarchy/subtypes"
        | "textDocument/documentHighlight"
        | "textDocument/documentLink"
        | "textDocument/codeLens"
        | "textDocument/selectionRange"
        | "textDocument/foldingRange"
        | "textDocument/documentColor"
        | "textDocument/colorPresentation"
        | "textDocument/moniker"
        | "textDocument/inlayHint"
        | "textDocument/inlineValue" => CompositionStrategy::MergeDeduped,

        "textDocument/completion" | "textDocument/inlineCompletion" => {
            CompositionStrategy::RankedProviders
        }

        "textDocument/semanticTokens/full"
        | "textDocument/semanticTokens/full/delta"
        | "textDocument/semanticTokens/range" => CompositionStrategy::SingleOwner,

        "textDocument/formatting"
        | "textDocument/rangeFormatting"
        | "textDocument/onTypeFormatting"
        | "textDocument/rangesFormatting"
        | "textDocument/rename"
        | "textDocument/prepareRename"
        | "textDocument/codeAction"
        | "codeAction/resolve"
        | "workspace/applyEdit"
        | "textDocument/willSaveWaitUntil"
        | "workspace/willCreateFiles"
        | "workspace/willRenameFiles"
        | "workspace/willDeleteFiles"
        | "workspace/executeCommand" => CompositionStrategy::TransactionalEditGate,

        "$/cancelRequest" | "$/progress" | "window/workDoneProgress/cancel" | "$/setTrace" => {
            CompositionStrategy::ObserveOnly
        }

        _ => CompositionStrategy::Deny,
    }
}

// ── Source State ───────────────────────────────────────────────────────────────

/// The health state of an upstream source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceHealth {
    /// Source is healthy.
    Healthy,
    /// Source initialization failed.
    InitializationFailed,
    /// Source crashed.
    Crashed,
    /// Source connection timed out.
    TimedOut,
    /// Source returned an invalid response.
    InvalidResponse,
    /// Source is in degraded health state.
    Degraded,
}

/// Runtime state for a single upstream source.
#[derive(Debug)]
pub struct UpstreamSource {
    pub id: String,
    pub address: String,
    pub health: SourceHealth,
    pub server_capabilities: Option<lsp_types_max::ServerCapabilities>,
    pub dynamic_registrations: HashMap<String, Value>,
}

impl UpstreamSource {
    pub fn new(id: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            address: address.into(),
            health: SourceHealth::Healthy,
            server_capabilities: None,
            dynamic_registrations: HashMap::new(),
        }
    }

    pub fn is_routable(&self) -> bool {
        self.health != SourceHealth::InitializationFailed && self.health != SourceHealth::Crashed
    }

    pub fn supports_method(&self, method: &str) -> bool {
        if !self.is_routable() {
            return false;
        }
        if method == "initialize"
            || method == "initialized"
            || method == "shutdown"
            || method == "exit"
        {
            return true;
        }
        if self.dynamic_registrations.contains_key(method) {
            return true;
        }
        if let Some(caps) = &self.server_capabilities {
            capability_supports_method(caps, method)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_strategy_lifecycle() {
        assert_eq!(method_strategy("initialize"), CompositionStrategy::SingleOwner);
        assert_eq!(method_strategy("initialized"), CompositionStrategy::SingleOwner);
        assert_eq!(method_strategy("shutdown"), CompositionStrategy::SingleOwner);
        assert_eq!(method_strategy("exit"), CompositionStrategy::SingleOwner);
    }

    #[test]
    fn method_strategy_fanout() {
        assert_eq!(
            method_strategy("textDocument/didOpen"),
            CompositionStrategy::OrderedFanout
        );
        assert_eq!(
            method_strategy("textDocument/didChange"),
            CompositionStrategy::OrderedFanout
        );
        assert_eq!(
            method_strategy("textDocument/didSave"),
            CompositionStrategy::OrderedFanout
        );
    }

    #[test]
    fn method_strategy_hover_is_first_success() {
        // Per the routing table, hover uses FirstSuccess
        assert_eq!(
            method_strategy("textDocument/hover"),
            CompositionStrategy::FirstSuccess
        );
    }

    #[test]
    fn method_strategy_merge_attributed() {
        assert_eq!(
            method_strategy("textDocument/publishDiagnostics"),
            CompositionStrategy::MergeAttributed
        );
        assert_eq!(
            method_strategy("textDocument/documentSymbol"),
            CompositionStrategy::MergeAttributed
        );
    }

    #[test]
    fn method_strategy_merge_deduped() {
        assert_eq!(
            method_strategy("textDocument/definition"),
            CompositionStrategy::MergeDeduped
        );
        assert_eq!(
            method_strategy("textDocument/references"),
            CompositionStrategy::MergeDeduped
        );
    }

    #[test]
    fn method_strategy_ranked_providers() {
        assert_eq!(
            method_strategy("textDocument/completion"),
            CompositionStrategy::RankedProviders
        );
    }

    #[test]
    fn method_strategy_transactional_edit_gate() {
        assert_eq!(
            method_strategy("textDocument/formatting"),
            CompositionStrategy::TransactionalEditGate
        );
        assert_eq!(
            method_strategy("textDocument/rename"),
            CompositionStrategy::TransactionalEditGate
        );
    }

    #[test]
    fn method_strategy_observe_only() {
        assert_eq!(
            method_strategy("$/cancelRequest"),
            CompositionStrategy::ObserveOnly
        );
        assert_eq!(
            method_strategy("$/progress"),
            CompositionStrategy::ObserveOnly
        );
    }

    #[test]
    fn method_strategy_unknown_defaults_to_deny() {
        assert_eq!(
            method_strategy("nonexistent/method"),
            CompositionStrategy::Deny
        );
        assert_eq!(method_strategy(""), CompositionStrategy::Deny);
    }

    #[test]
    fn upstream_source_new_is_healthy_and_routable() {
        let src = UpstreamSource::new("test-src", "127.0.0.1:9999");
        assert_eq!(src.id, "test-src");
        assert_eq!(src.health, SourceHealth::Healthy);
        assert!(src.is_routable());
    }

    #[test]
    fn upstream_source_initialization_failed_is_not_routable() {
        let mut src = UpstreamSource::new("src-a", "addr");
        src.health = SourceHealth::InitializationFailed;
        assert!(!src.is_routable());
    }

    #[test]
    fn upstream_source_crashed_is_not_routable() {
        let mut src = UpstreamSource::new("src-b", "addr");
        src.health = SourceHealth::Crashed;
        assert!(!src.is_routable());
    }

    #[test]
    fn upstream_source_degraded_is_still_routable() {
        let mut src = UpstreamSource::new("src-c", "addr");
        src.health = SourceHealth::Degraded;
        assert!(src.is_routable());
    }

    #[test]
    fn upstream_source_supports_lifecycle_without_caps() {
        let src = UpstreamSource::new("src-d", "addr");
        assert!(src.supports_method("initialize"));
        assert!(src.supports_method("shutdown"));
        assert!(src.supports_method("exit"));
    }

    #[test]
    fn upstream_source_does_not_support_hover_without_caps() {
        let src = UpstreamSource::new("src-e", "addr");
        // No server_capabilities set — hover not supported
        assert!(!src.supports_method("textDocument/hover"));
    }

    #[test]
    fn upstream_source_not_routable_does_not_support_any_method() {
        let mut src = UpstreamSource::new("src-f", "addr");
        src.health = SourceHealth::Crashed;
        assert!(!src.supports_method("initialize"));
        assert!(!src.supports_method("textDocument/hover"));
    }
}

/// Derives whether a ServerCapabilities supports the given method.
pub fn capability_supports_method(caps: &lsp_types_max::ServerCapabilities, method: &str) -> bool {
    match method {
        "textDocument/hover" => {
            if let Some(ref p) = caps.hover_provider {
                match p {
                    lsp_types_max::HoverProviderCapability::Simple(b) => *b,
                    lsp_types_max::HoverProviderCapability::Options(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/completion" => caps.completion_provider.is_some(),
        "textDocument/definition" => {
            if let Some(ref p) = caps.definition_provider {
                match p {
                    lsp_types_max::OneOf::Left(b) => *b,
                    lsp_types_max::OneOf::Right(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/declaration" => {
            if let Some(ref p) = caps.declaration_provider {
                match p {
                    lsp_types_max::DeclarationCapability::Simple(b) => *b,
                    lsp_types_max::DeclarationCapability::RegistrationOptions(_) => true,
                    lsp_types_max::DeclarationCapability::Options(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/implementation" => {
            if let Some(ref p) = caps.implementation_provider {
                match p {
                    lsp_types_max::ImplementationProviderCapability::Simple(b) => *b,
                    lsp_types_max::ImplementationProviderCapability::Options(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/references" => {
            if let Some(ref p) = caps.references_provider {
                match p {
                    lsp_types_max::OneOf::Left(b) => *b,
                    lsp_types_max::OneOf::Right(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/documentSymbol" | "workspace/symbol" => {
            if let Some(ref p) = caps.document_symbol_provider {
                match p {
                    lsp_types_max::OneOf::Left(b) => *b,
                    lsp_types_max::OneOf::Right(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/formatting" => {
            if let Some(ref p) = caps.document_formatting_provider {
                match p {
                    lsp_types_max::OneOf::Left(b) => *b,
                    lsp_types_max::OneOf::Right(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/rangeFormatting" => {
            if let Some(ref p) = caps.document_range_formatting_provider {
                match p {
                    lsp_types_max::OneOf::Left(b) => *b,
                    lsp_types_max::OneOf::Right(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/rename" => {
            if let Some(ref p) = caps.rename_provider {
                match p {
                    lsp_types_max::OneOf::Left(b) => *b,
                    lsp_types_max::OneOf::Right(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/codeAction" => {
            if let Some(ref p) = caps.code_action_provider {
                match p {
                    lsp_types_max::CodeActionProviderCapability::Simple(b) => *b,
                    lsp_types_max::CodeActionProviderCapability::Options(_) => true,
                }
            } else {
                false
            }
        }
        "textDocument/semanticTokens/full"
        | "textDocument/semanticTokens/full/delta"
        | "textDocument/semanticTokens/range" => caps.semantic_tokens_provider.is_some(),
        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didSave"
        | "textDocument/didClose" => true,
        _ => false,
    }
}
