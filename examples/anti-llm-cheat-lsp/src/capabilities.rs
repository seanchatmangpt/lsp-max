//! Matrix-driven ServerCapabilities generator.
//!
//! This module derives ServerCapabilities from lsp318_coverage::surface_table,
//! ensuring advertised capabilities and handler wiring stay synchronized.
//! A capability is enabled only when its corresponding method has a Wired handler.

use crate::rules::lsp318_coverage::{full_surface, HandlerState};
use lsp_max::lsp_types::*;

/// Build ServerCapabilities by scanning the coverage matrix.
/// Only methods with Wired handlers get advertised capabilities.
pub fn build_capabilities() -> ServerCapabilities {
    let mut caps = ServerCapabilities::default();
    let surface = full_surface();

    for method in surface {
        // Only wire capabilities for Wired handlers
        if method.handler != HandlerState::Wired {
            continue;
        }

        match method.method {
            // ── Text document synchronization ─────────────────────────────────
            "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
                caps.text_document_sync =
                    Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));
            }

            // ── Navigation language features ──────────────────────────────────
            "textDocument/declaration" => {
                caps.declaration_provider = Some(DeclarationCapability::Simple(true));
            }
            "textDocument/definition" => {
                caps.definition_provider = Some(OneOf::Left(true));
            }
            "textDocument/typeDefinition" => {
                caps.type_definition_provider = Some(TypeDefinitionProviderCapability::Simple(true));
            }
            "textDocument/implementation" => {
                caps.implementation_provider =
                    Some(ImplementationProviderCapability::Simple(true));
            }
            "textDocument/references" => {
                caps.references_provider = Some(OneOf::Left(true));
            }
            "textDocument/hover" => {
                caps.hover_provider = Some(HoverProviderCapability::Simple(true));
            }

            // ── Completion / signature / lens / link / color / action ─────────
            "textDocument/completion" => {
                caps.completion_provider = Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec!["#".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    completion_item: None,
                });
            }
            "textDocument/signatureHelp" => {
                caps.signature_help_provider = Some(SignatureHelpOptions::default());
            }
            "textDocument/codeLens" => {
                caps.code_lens_provider = Some(CodeLensOptions {
                    resolve_provider: Some(true),
                });
            }
            "codeLens/resolve" => {
                // Already handled by textDocument/codeLens
            }
            "textDocument/codeAction" => {
                caps.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
            }

            // ── Formatting / folding / hints / inline / semantic / symbol ─────
            "textDocument/rangesFormatting" => {
                caps.document_range_formatting_provider = Some(OneOf::Left(true));
            }
            "textDocument/foldingRange" => {
                caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
            }
            "textDocument/inlineCompletion" => {
                caps.inline_completion_provider = Some(OneOf::Left(true));
            }
            "textDocument/documentSymbol" => {
                caps.document_symbol_provider = Some(OneOf::Left(true));
            }

            // ── Semantic tokens (AST-derived, Path B) ─────────────────────────
            // The `full`, `full/delta` and `range` rows collapse onto one
            // capability block; build it once when the first of them is seen.
            "textDocument/semanticTokens/full"
            | "textDocument/semanticTokens/full/delta"
            | "textDocument/semanticTokens/range" => {
                if caps.semantic_tokens_provider.is_none() {
                    caps.semantic_tokens_provider = Some(
                        SemanticTokensServerCapabilities::SemanticTokensOptions(
                            SemanticTokensOptions {
                                legend: crate::semantic::legend(),
                                full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                                range: Some(true),
                                work_done_progress_options: WorkDoneProgressOptions::default(),
                            },
                        ),
                    );
                }
            }

            // ── Pull diagnostics ──────────────────────────────────────────────
            "textDocument/diagnostic" => {
                caps.diagnostic_provider = Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                ));
            }

            // ── Workspace features ────────────────────────────────────────────
            "workspace/textDocumentContent" => {
                // No explicit capability field; handled by method availability
            }

            // ── Server-to-client refreshes ────────────────────────────────────
            "workspace/foldingRange/refresh" => {
                // Server-side refresh; no client capability needed
            }

            // ── Window features ───────────────────────────────────────────────
            "window/logMessage" => {
                // Server-side notification; no explicit capability
            }

            _ => {}
        }
    }

    caps
}
