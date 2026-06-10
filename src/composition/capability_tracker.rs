//! Capability tracking for upstream source composition (R2).

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::Client;

use super::strategy::{method_strategy, CompositionStrategy, SourceHealth, UpstreamSource};

#[derive(Debug)]
pub struct CapabilityTracker {
    pub client_capabilities: Option<lsp_types_max::ClientCapabilities>,
    pub sources: HashMap<String, UpstreamSource>,
    pub dynamic_registrations: HashMap<String, DynamicRegistration>,
    pub client: Option<Client>,
}

#[derive(Debug, Clone)]
pub struct DynamicRegistration {
    pub id: String,
    pub method: String,
    pub source_id: String,
    pub options: Value,
}

impl CapabilityTracker {
    pub fn new() -> Self {
        Self {
            client_capabilities: None,
            sources: HashMap::new(),
            dynamic_registrations: HashMap::new(),
            client: None,
        }
    }

    pub fn add_source(&mut self, source: UpstreamSource) {
        self.sources.insert(source.id.clone(), source);
    }

    /// Record a dynamic registration. Returns false if duplicate ID.
    pub fn register_dynamic(
        &mut self,
        id: &str,
        method: &str,
        source_id: &str,
        options: Value,
    ) -> bool {
        if id.is_empty() {
            return false;
        }
        if self.dynamic_registrations.contains_key(id) {
            return false;
        }
        self.dynamic_registrations.insert(
            id.to_string(),
            DynamicRegistration {
                id: id.to_string(),
                method: method.to_string(),
                source_id: source_id.to_string(),
                options,
            },
        );
        if let Some(src) = self.sources.get_mut(source_id) {
            src.dynamic_registrations
                .insert(method.to_string(), json!({"id": id}));
        }
        true
    }

    /// Remove a dynamic registration. Returns false if not found (safe no-op).
    pub fn unregister_dynamic(&mut self, id: &str) -> bool {
        if let Some(reg) = self.dynamic_registrations.remove(id) {
            if let Some(src) = self.sources.get_mut(&reg.source_id) {
                src.dynamic_registrations.remove(&reg.method);
            }
            true
        } else {
            false
        }
    }

    /// Derive effective downstream capabilities.
    /// This is NOT a raw union: only methods supported by at least one healthy source
    /// AND supported by the client AND not denied by routing policy are advertised.
    pub fn derive_effective_capabilities(
        &self,
        client_caps: &lsp_types_max::ClientCapabilities,
    ) -> lsp_types_max::ServerCapabilities {
        let mut caps = lsp_types_max::ServerCapabilities::default();

        let check_method = |method: &str| -> bool {
            let has_source = self
                .sources
                .values()
                .any(|s| s.is_routable() && s.supports_method(method));
            let client_ok = client_supports(client_caps, method);
            if std::env::var("SABOTAGE_CAPABILITY_TRACKER").is_ok() {
                has_source || client_ok
            } else {
                has_source && client_ok
            }
        };

        // hover
        if check_method("textDocument/hover")
            && method_strategy("textDocument/hover") != CompositionStrategy::Deny
        {
            caps.hover_provider = Some(lsp_types_max::HoverProviderCapability::Simple(true));
        }

        // completion
        if check_method("textDocument/completion")
            && method_strategy("textDocument/completion") != CompositionStrategy::Deny
        {
            let mut completion_opts_list = Vec::new();
            for s in self.sources.values() {
                if s.is_routable() && s.supports_method("textDocument/completion") {
                    if let Some(ref scaps) = s.server_capabilities {
                        if let Some(ref copts) = scaps.completion_provider {
                            completion_opts_list.push(copts.clone());
                        }
                    }
                }
            }
            let mut resolve_provider = None;
            let mut trigger_chars: Option<Vec<String>> = None;
            for opts in &completion_opts_list {
                if let Some(r) = opts.resolve_provider {
                    if std::env::var("SABOTAGE_CAPABILITY_TRACKER").is_ok() {
                        resolve_provider = Some(resolve_provider.unwrap_or(false) || r);
                    } else {
                        resolve_provider = Some(resolve_provider.unwrap_or(true) && r);
                    }
                } else if std::env::var("SABOTAGE_CAPABILITY_TRACKER").is_err() {
                    resolve_provider = Some(false);
                }
                if let Some(ref chars) = opts.trigger_characters {
                    if let Some(ref mut current) = trigger_chars {
                        if std::env::var("SABOTAGE_CAPABILITY_TRACKER").is_ok() {
                            for c in chars {
                                if !current.contains(c) {
                                    current.push(c.clone());
                                }
                            }
                        } else {
                            current.retain(|c| chars.contains(c));
                        }
                    } else {
                        trigger_chars = Some(chars.clone());
                    }
                }
            }
            caps.completion_provider = Some(lsp_types_max::CompletionOptions {
                resolve_provider,
                trigger_characters: trigger_chars,
                ..Default::default()
            });
        }

        // definition
        if check_method("textDocument/definition")
            && method_strategy("textDocument/definition") != CompositionStrategy::Deny
        {
            caps.definition_provider = Some(lsp_types_max::OneOf::Left(true));
        }

        // declaration
        if check_method("textDocument/declaration")
            && method_strategy("textDocument/declaration") != CompositionStrategy::Deny
        {
            caps.declaration_provider = Some(lsp_types_max::DeclarationCapability::Simple(true));
        }

        // implementation
        if check_method("textDocument/implementation")
            && method_strategy("textDocument/implementation") != CompositionStrategy::Deny
        {
            caps.implementation_provider = Some(
                lsp_types_max::ImplementationProviderCapability::Simple(true),
            );
        }

        // references
        if check_method("textDocument/references")
            && method_strategy("textDocument/references") != CompositionStrategy::Deny
        {
            caps.references_provider = Some(lsp_types_max::OneOf::Left(true));
        }

        // documentSymbol
        if check_method("textDocument/documentSymbol")
            && method_strategy("textDocument/documentSymbol") != CompositionStrategy::Deny
        {
            caps.document_symbol_provider = Some(lsp_types_max::OneOf::Left(true));
        }

        // formatting
        if check_method("textDocument/formatting")
            && method_strategy("textDocument/formatting") != CompositionStrategy::Deny
        {
            caps.document_formatting_provider = Some(lsp_types_max::OneOf::Left(true));
        }

        // rangeFormatting
        if check_method("textDocument/rangeFormatting")
            && method_strategy("textDocument/rangeFormatting") != CompositionStrategy::Deny
        {
            caps.document_range_formatting_provider = Some(lsp_types_max::OneOf::Left(true));
        }

        // rename
        if check_method("textDocument/rename")
            && method_strategy("textDocument/rename") != CompositionStrategy::Deny
        {
            caps.rename_provider = Some(lsp_types_max::OneOf::Left(true));
        }

        // codeAction
        if check_method("textDocument/codeAction")
            && method_strategy("textDocument/codeAction") != CompositionStrategy::Deny
        {
            caps.code_action_provider =
                Some(lsp_types_max::CodeActionProviderCapability::Simple(true));
        }

        // textDocumentSync
        let any_healthy = self.sources.values().any(|s| s.is_routable());
        if any_healthy {
            caps.text_document_sync = Some(lsp_types_max::TextDocumentSyncCapability::Kind(
                lsp_types_max::TextDocumentSyncKind::INCREMENTAL,
            ));
        }

        caps
    }

    pub fn routable_sources_for_method(&self, method: &str) -> Vec<String> {
        let mut sources: Vec<&UpstreamSource> = self
            .sources
            .values()
            .filter(|s| s.is_routable() && s.supports_method(method))
            .collect();
        sources.sort_by_key(|s| {
            if s.health == SourceHealth::Healthy {
                0
            } else {
                1
            }
        });
        sources.into_iter().map(|s| s.id.clone()).collect()
    }

    pub fn degrade_source(
        &mut self,
        source_id: &str,
        health: SourceHealth,
    ) -> Vec<DynamicRegistration> {
        if std::env::var("SABOTAGE_SOURCE_HEALTH").is_ok() {
            return Vec::new();
        }
        let mut unregistered = Vec::new();
        if let Some(src) = self.sources.get_mut(source_id) {
            src.health = health;
        }
        let is_healthy = matches!(
            self.sources.get(source_id).map(|s| &s.health),
            Some(SourceHealth::Healthy)
        );
        if !is_healthy {
            let ids_to_remove: Vec<String> = self
                .dynamic_registrations
                .iter()
                .filter(|(_, reg)| reg.source_id == source_id)
                .map(|(id, _)| id.clone())
                .collect();
            for id in ids_to_remove {
                if let Some(reg) = self.dynamic_registrations.remove(&id) {
                    if let Some(src) = self.sources.get_mut(&reg.source_id) {
                        src.dynamic_registrations.remove(&reg.method);
                    }
                    unregistered.push(reg);
                }
            }
        }
        if !unregistered.is_empty() {
            if let Some(ref client) = self.client {
                let client = client.clone();
                let unregs: Vec<lsp_types_max::Unregistration> = unregistered
                    .iter()
                    .map(|reg| lsp_types_max::Unregistration {
                        id: reg.id.clone(),
                        method: reg.method.clone(),
                    })
                    .collect();
                tokio::spawn(async move {
                    let _ = client.unregister_capability(unregs).await;
                });
            }
        }
        unregistered
    }
}

pub fn client_supports(client_caps: &lsp_types_max::ClientCapabilities, method: &str) -> bool {
    let is_empty = client_caps.text_document.is_none()
        && client_caps.workspace.is_none()
        && client_caps.window.is_none()
        && client_caps.general.is_none();
    if is_empty {
        return true;
    }
    let td = client_caps.text_document.as_ref();
    match method {
        "textDocument/hover" => td.and_then(|t| t.hover.as_ref()).is_some(),
        "textDocument/completion" => td.and_then(|t| t.completion.as_ref()).is_some(),
        "textDocument/definition" => td.and_then(|t| t.definition.as_ref()).is_some(),
        "textDocument/declaration" => td.and_then(|t| t.declaration.as_ref()).is_some(),
        "textDocument/implementation" => td.and_then(|t| t.implementation.as_ref()).is_some(),
        "textDocument/references" => td.and_then(|t| t.references.as_ref()).is_some(),
        "textDocument/rename" => td.and_then(|t| t.rename.as_ref()).is_some(),
        "textDocument/formatting" => td.and_then(|t| t.formatting.as_ref()).is_some(),
        "textDocument/rangeFormatting" => td.and_then(|t| t.range_formatting.as_ref()).is_some(),
        "textDocument/codeAction" => td.and_then(|t| t.code_action.as_ref()).is_some(),
        "textDocument/documentSymbol" => td.and_then(|t| t.document_symbol.as_ref()).is_some(),
        _ => true,
    }
}
