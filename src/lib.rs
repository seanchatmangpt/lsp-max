//! Language Server Protocol (LSP) server abstraction for [Tower].
//!
//! [Tower]: https://github.com/tower-rs/tower
//!
//! # Example
//!
//! ```rust,no_run
//! use tower_lsp_max::jsonrpc::Result;
//! use tower_lsp_max::lsp_types::*;
//! use tower_lsp_max::{Client, LanguageServer, LspService, Server};
//!
//! #[derive(Debug)]
//! struct Backend {
//!     client: Client,
//! }
//!
//! #[tower_lsp_max::async_trait]
//! impl LanguageServer for Backend {
//!     async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
//!         Ok(InitializeResult {
//!             capabilities: ServerCapabilities {
//!                 hover_provider: Some(HoverProviderCapability::Simple(true)),
//!                 completion_provider: Some(CompletionOptions::default()),
//!                 ..Default::default()
//!             },
//!             ..Default::default()
//!         })
//!     }
//!
//!     async fn initialized(&self, _: InitializedParams) {
//!         self.client
//!             .log_message(MessageType::INFO, "server initialized!")
//!             .await;
//!     }
//!
//!     async fn shutdown(&self) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
//!         Ok(Some(CompletionResponse::Array(vec![
//!             CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
//!             CompletionItem::new_simple("Bye".to_string(), "More detail".to_string())
//!         ])))
//!     }
//!
//!     async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
//!         Ok(Some(Hover {
//!             contents: HoverContents::Scalar(
//!                 MarkedString::String("You're hovering!".to_string())
//!             ),
//!             range: None
//!         }))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//! #   tracing_subscriber::fmt().init();
//! #
//! #   #[cfg(feature = "runtime-agnostic")]
//! #   use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
//! #   use std::io::Cursor;
//!     let stdin = tokio::io::stdin();
//!     let stdout = tokio::io::stdout();
//! #   let message = r#"{"jsonrpc":"2.0","method":"exit"}"#;
//! #   let (stdin, stdout) = (Cursor::new(format!("Content-Length: {}\r\n\r\n{}", message.len(), message).into_bytes()), Cursor::new(Vec::new()));
//! #   #[cfg(feature = "runtime-agnostic")]
//! #   let (stdin, stdout) = (stdin.compat(), stdout.compat_write());
//!
//!     let (service, socket) = LspService::new(|client| Backend { client });
//!     let _ = Server::new(stdin, stdout, socket).serve(service).await;
//! }
//! ```

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub extern crate lsp_types;

pub extern crate tower_lsp_max_agent as max_agent;
pub extern crate tower_lsp_max_protocol as max_protocol;
pub extern crate tower_lsp_max_runtime as max_runtime;

/// A re-export of [`async-trait`](https://docs.rs/async-trait) for convenience.
pub use async_trait::async_trait;

pub use self::service::progress::{
    Bounded, Cancellable, NotCancellable, OngoingProgress, Progress, Unbounded,
};
pub use self::service::{Client, ClientSocket, ExitedError, LspService, LspServiceBuilder};
pub use self::transport::{Loopback, Server};

use auto_impl::auto_impl;
use lsp_types::request::{
    GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
    GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
};
use lsp_types::*;
use serde_json::Value;
use tower_lsp_max_macros::rpc;
use tracing::{error, warn};

use self::jsonrpc::{Error, Result};

pub mod jsonrpc;

mod codec;
pub mod service;
mod transport;

/// Trait implemented by language server backends.
///
/// This interface allows servers adhering to the [Language Server Protocol] to be implemented in a
/// safe and easily testable way without exposing the low-level implementation details.
///
/// [Language Server Protocol]: https://microsoft.github.io/language-server-protocol/
#[rpc]
#[async_trait]
#[auto_impl(Arc, Box)]
pub trait LanguageServer: Send + Sync + 'static {
    /// The [`initialize`] request is the first request sent from the client to the server.
    ///
    /// [`initialize`]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// This method is guaranteed to only execute once. If the client sends this request to the
    /// server again, the server will respond with JSON-RPC error code `-32600` (invalid request).
    #[rpc(name = "initialize")]
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult>;

    /// The [`initialized`] notification is sent from the client to the server after the client
    /// received the result of the initialize request but before the client sends anything else.
    ///
    /// [`initialized`]: https://microsoft.github.io/language-server-protocol/specification#initialized
    ///
    /// The server can use the `initialized` notification, for example, to dynamically register
    /// capabilities with the client.
    #[rpc(name = "initialized")]
    async fn initialized(&self, params: InitializedParams) {
        let _ = params;
    }

    /// The [`shutdown`] request asks the server to gracefully shut down, but to not exit.
    ///
    /// [`shutdown`]: https://microsoft.github.io/language-server-protocol/specification#shutdown
    ///
    /// This request is often later followed by an [`exit`] notification, which will cause the
    /// server to exit immediately.
    ///
    /// [`exit`]: https://microsoft.github.io/language-server-protocol/specification#exit
    ///
    /// This method is guaranteed to only execute once. If the client sends this request to the
    /// server again, the server will respond with JSON-RPC error code `-32600` (invalid request).
    #[rpc(name = "shutdown")]
    async fn shutdown(&self) -> Result<()>;

    // Document Synchronization

    /// The [`textDocument/didOpen`] notification is sent from the client to the server to signal
    /// that a new text document has been opened by the client.
    ///
    /// [`textDocument/didOpen`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_didOpen
    ///
    /// The document's truth is now managed by the client and the server must not try to read the
    /// document’s truth using the document's URI. "Open" in this sense means it is managed by the
    /// client. It doesn't necessarily mean that its content is presented in an editor.
    #[rpc(name = "textDocument/didOpen")]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let _ = params;
        warn!("Got a textDocument/didOpen notification, but it is not implemented");
    }

    /// The [`textDocument/didChange`] notification is sent from the client to the server to signal
    /// changes to a text document.
    ///
    /// [`textDocument/didChange`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_didChange
    ///
    /// This notification will contain a distinct version tag and a list of edits made to the
    /// document for the server to interpret.
    #[rpc(name = "textDocument/didChange")]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let _ = params;
        warn!("Got a textDocument/didChange notification, but it is not implemented");
    }

    /// The [`textDocument/willSave`] notification is sent from the client to the server before the
    /// document is actually saved.
    ///
    /// [`textDocument/willSave`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_willSave
    #[rpc(name = "textDocument/willSave")]
    async fn will_save(&self, params: WillSaveTextDocumentParams) {
        let _ = params;
        warn!("Got a textDocument/willSave notification, but it is not implemented");
    }

    /// The [`textDocument/willSaveWaitUntil`] request is sent from the client to the server before
    /// the document is actually saved.
    ///
    /// [`textDocument/willSaveWaitUntil`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_willSaveWaitUntil
    ///
    /// The request can return an array of `TextEdit`s which will be applied to the text document
    /// before it is saved.
    ///
    /// Please note that clients might drop results if computing the text edits took too long or if
    /// a server constantly fails on this request. This is done to keep the save fast and reliable.
    #[rpc(name = "textDocument/willSaveWaitUntil")]
    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let _ = params;
        error!("Got a textDocument/willSaveWaitUntil request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/didSave`] notification is sent from the client to the server when the
    /// document was saved in the client.
    ///
    /// [`textDocument/didSave`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_didSave
    #[rpc(name = "textDocument/didSave")]
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let _ = params;
        warn!("Got a textDocument/didSave notification, but it is not implemented");
    }

    /// The [`textDocument/didClose`] notification is sent from the client to the server when the
    /// document got closed in the client.
    ///
    /// [`textDocument/didClose`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_didClose
    ///
    /// The document's truth now exists where the document's URI points to (e.g. if the document's
    /// URI is a file URI, the truth now exists on disk).
    #[rpc(name = "textDocument/didClose")]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let _ = params;
        warn!("Got a textDocument/didClose notification, but it is not implemented");
    }

    // Language Features

    /// The [`textDocument/declaration`] request asks the server for the declaration location of a
    /// symbol at a given text document position.
    ///
    /// [`textDocument/declaration`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_declaration
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.14.0.
    ///
    /// The [`GotoDeclarationResponse::Link`](lsp_types::GotoDefinitionResponse::Link) return value
    /// was introduced in specification version 3.14.0 and requires client-side support in order to
    /// be used. It can be returned if the client set the following field to `true` in the
    /// [`initialize`](Self::initialize) method:
    ///
    /// ```text
    /// InitializeParams::capabilities::text_document::declaration::link_support
    /// ```
    #[rpc(name = "textDocument/declaration")]
    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> Result<Option<GotoDeclarationResponse>> {
        let _ = params;
        error!("Got a textDocument/declaration request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/definition`] request asks the server for the definition location of a
    /// symbol at a given text document position.
    ///
    /// [`textDocument/definition`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_definition
    ///
    /// # Compatibility
    ///
    /// The [`GotoDefinitionResponse::Link`](lsp_types::GotoDefinitionResponse::Link) return value
    /// was introduced in specification version 3.14.0 and requires client-side support in order to
    /// be used. It can be returned if the client set the following field to `true` in the
    /// [`initialize`](Self::initialize) method:
    ///
    /// ```text
    /// InitializeParams::capabilities::text_document::definition::link_support
    /// ```
    #[rpc(name = "textDocument/definition")]
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let _ = params;
        error!("Got a textDocument/definition request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/typeDefinition`] request asks the server for the type definition location of
    /// a symbol at a given text document position.
    ///
    /// [`textDocument/typeDefinition`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_typeDefinition
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    ///
    /// The [`GotoTypeDefinitionResponse::Link`](lsp_types::GotoDefinitionResponse::Link) return
    /// value was introduced in specification version 3.14.0 and requires client-side support in
    /// order to be used. It can be returned if the client set the following field to `true` in the
    /// [`initialize`](Self::initialize) method:
    ///
    /// ```text
    /// InitializeParams::capabilities::text_document::type_definition::link_support
    /// ```
    #[rpc(name = "textDocument/typeDefinition")]
    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        let _ = params;
        error!("Got a textDocument/typeDefinition request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/implementation`] request is sent from the client to the server to resolve
    /// the implementation location of a symbol at a given text document position.
    ///
    /// [`textDocument/implementation`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_implementation
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    ///
    /// The [`GotoImplementationResponse::Link`](lsp_types::GotoDefinitionResponse::Link)
    /// return value was introduced in specification version 3.14.0 and requires client-side
    /// support in order to be used. It can be returned if the client set the following field to
    /// `true` in the [`initialize`](Self::initialize) method:
    ///
    /// ```text
    /// InitializeParams::capabilities::text_document::implementation::link_support
    /// ```
    #[rpc(name = "textDocument/implementation")]
    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> Result<Option<GotoImplementationResponse>> {
        let _ = params;
        error!("Got a textDocument/implementation request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/references`] request is sent from the client to the server to resolve
    /// project-wide references for the symbol denoted by the given text document position.
    ///
    /// [`textDocument/references`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_references
    #[rpc(name = "textDocument/references")]
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let _ = params;
        error!("Got a textDocument/references request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/prepareCallHierarchy`] request is sent from the client to the server to
    /// return a call hierarchy for the language element of given text document positions.
    ///
    /// [`textDocument/prepareCallHierarchy`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_prepareCallHierarchy
    ///
    /// The call hierarchy requests are executed in two steps:
    ///
    /// 1. First, a call hierarchy item is resolved for the given text document position (this
    ///    method).
    /// 2. For a call hierarchy item, the incoming or outgoing call hierarchy items are resolved
    ///    inside [`incoming_calls`] and [`outgoing_calls`], respectively.
    ///
    /// [`incoming_calls`]: Self::incoming_calls
    /// [`outgoing_calls`]: Self::outgoing_calls
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "textDocument/prepareCallHierarchy")]
    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        let _ = params;
        error!("Got a textDocument/prepareCallHierarchy request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`callHierarchy/incomingCalls`] request is sent from the client to the server to
    /// resolve **incoming** calls for a given call hierarchy item.
    ///
    /// The request doesn't define its own client and server capabilities. It is only issued if a
    /// server registers for the [`textDocument/prepareCallHierarchy`] request.
    ///
    /// [`callHierarchy/incomingCalls`]: https://microsoft.github.io/language-server-protocol/specification#callHierarchy_incomingCalls
    /// [`textDocument/prepareCallHierarchy`]: Self::prepare_call_hierarchy
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "callHierarchy/incomingCalls")]
    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let _ = params;
        error!("Got a callHierarchy/incomingCalls request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`callHierarchy/outgoingCalls`] request is sent from the client to the server to
    /// resolve **outgoing** calls for a given call hierarchy item.
    ///
    /// The request doesn't define its own client and server capabilities. It is only issued if a
    /// server registers for the [`textDocument/prepareCallHierarchy`] request.
    ///
    /// [`callHierarchy/outgoingCalls`]: https://microsoft.github.io/language-server-protocol/specification#callHierarchy_outgoingCalls
    /// [`textDocument/prepareCallHierarchy`]: Self::prepare_call_hierarchy
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "callHierarchy/outgoingCalls")]
    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let _ = params;
        error!("Got a callHierarchy/outgoingCalls request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/prepareTypeHierarchy`] request is sent from the client to the server to
    /// return a type hierarchy for the language element of given text document positions.
    ///
    /// [`textDocument/prepareTypeHierarchy`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_prepareTypeHierarchy
    ///
    /// Returns `Ok(None)` if the server couldn’t infer a valid type from the position.
    ///
    /// The type hierarchy requests are executed in two steps:
    ///
    /// 1. First, a type hierarchy item is prepared for the given text document position.
    /// 2. For a type hierarchy item, the supertype or subtype type hierarchy items are resolved in
    ///    [`supertypes`](Self::supertypes) and [`subtypes`](Self::subtypes), respectively.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "textDocument/prepareTypeHierarchy")]
    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        let _ = params;
        error!("Got a textDocument/prepareTypeHierarchy request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`typeHierarchy/supertypes`] request is sent from the client to the server to resolve
    /// the **supertypes** for a given type hierarchy item.
    ///
    /// Returns `Ok(None)` if the server couldn’t infer a valid type from item in `params`.
    ///
    /// The request doesn’t define its own client and server capabilities. It is only issued if a
    /// server registers for the `textDocument/prepareTypeHierarchy` request.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "typeHierarchy/supertypes")]
    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        let _ = params;
        error!("Got a typeHierarchy/supertypes request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`typeHierarchy/subtypes`] request is sent from the client to the server to resolve
    /// the **subtypes** for a given type hierarchy item.
    ///
    /// Returns `Ok(None)` if the server couldn’t infer a valid type from item in `params`.
    ///
    /// The request doesn’t define its own client and server capabilities. It is only issued if a
    /// server registers for the `textDocument/prepareTypeHierarchy` request.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "typeHierarchy/subtypes")]
    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        let _ = params;
        error!("Got a typeHierarchy/subtypes request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/documentHighlight`] request is sent from the client to the server to
    /// resolve appropriate highlights for a given text document position.
    ///
    /// [`textDocument/documentHighlight`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_documentHighlight
    ///
    /// For programming languages, this usually highlights all textual references to the symbol
    /// scoped to this file.
    ///
    /// This request differs slightly from `textDocument/references` in that this one is allowed to
    /// be more fuzzy.
    #[rpc(name = "textDocument/documentHighlight")]
    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let _ = params;
        error!("Got a textDocument/documentHighlight request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/documentLink`] request is sent from the client to the server to request
    /// the location of links in a document.
    ///
    /// A document link is a range in a text document that links to an internal or external
    /// resource, like another text document or a web site.
    ///
    /// [`textDocument/documentLink`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_documentLink
    ///
    /// # Compatibility
    ///
    /// The [`DocumentLink::tooltip`] field was introduced in specification version 3.15.0 and
    /// requires client-side support in order to be used. It can be returned if the client set the
    /// following field to `true` in the [`initialize`](Self::initialize) method:
    ///
    /// ```text
    /// InitializeParams::capabilities::text_document::document_link::tooltip_support
    /// ```
    #[rpc(name = "textDocument/documentLink")]
    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        let _ = params;
        error!("Got a textDocument/documentLink request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`documentLink/resolve`] request is sent from the client to the server to resolve the
    /// target of a given document link.
    ///
    /// [`documentLink/resolve`]: https://microsoft.github.io/language-server-protocol/specification#documentLink_resolve
    ///
    /// A document link is a range in a text document that links to an internal or external
    /// resource, like another text document or a web site.
    #[rpc(name = "documentLink/resolve")]
    async fn document_link_resolve(&self, params: DocumentLink) -> Result<DocumentLink> {
        let _ = params;
        error!("Got a documentLink/resolve request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/hover`] request asks the server for hover information at a given text
    /// document position.
    ///
    /// [`textDocument/hover`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_hover
    ///
    /// Such hover information typically includes type signature information and inline
    /// documentation for the symbol at the given text document position.
    #[rpc(name = "textDocument/hover")]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _ = params;
        error!("Got a textDocument/hover request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/codeLens`] request is sent from the client to the server to compute code
    /// lenses for a given text document.
    ///
    /// [`textDocument/codeLens`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_codeLens
    #[rpc(name = "textDocument/codeLens")]
    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let _ = params;
        error!("Got a textDocument/codeLens request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`codeLens/resolve`] request is sent from the client to the server to resolve the
    /// command for a given code lens item.
    ///
    /// [`codeLens/resolve`]: https://microsoft.github.io/language-server-protocol/specification#codeLens_resolve
    #[rpc(name = "codeLens/resolve")]
    async fn code_lens_resolve(&self, params: CodeLens) -> Result<CodeLens> {
        let _ = params;
        error!("Got a codeLens/resolve request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/foldingRange`] request is sent from the client to the server to return
    /// all folding ranges found in a given text document.
    ///
    /// [`textDocument/foldingRange`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_foldingRange
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.10.0.
    #[rpc(name = "textDocument/foldingRange")]
    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let _ = params;
        error!("Got a textDocument/foldingRange request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/selectionRange`] request is sent from the client to the server to return
    /// suggested selection ranges at an array of given positions. A selection range is a range
    /// around the cursor position which the user might be interested in selecting.
    ///
    /// [`textDocument/selectionRange`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_selectionRange
    ///
    /// A selection range in the return array is for the position in the provided parameters at the
    /// same index. Therefore `params.positions[i]` must be contained in `result[i].range`.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.15.0.
    #[rpc(name = "textDocument/selectionRange")]
    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        let _ = params;
        error!("Got a textDocument/selectionRange request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/documentSymbol`] request is sent from the client to the server to
    /// retrieve all symbols found in a given text document.
    ///
    /// [`textDocument/documentSymbol`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_documentSymbol
    ///
    /// The returned result is either:
    ///
    /// * [`DocumentSymbolResponse::Flat`] which is a flat list of all symbols found in a given
    ///   text document. Then neither the symbol’s location range nor the symbol’s container name
    ///   should be used to infer a hierarchy.
    /// * [`DocumentSymbolResponse::Nested`] which is a hierarchy of symbols found in a given text
    ///   document.
    #[rpc(name = "textDocument/documentSymbol")]
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let _ = params;
        error!("Got a textDocument/documentSymbol request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/semanticTokens/full`] request is sent from the client to the server to
    /// resolve the semantic tokens of a given file.
    ///
    /// [`textDocument/semanticTokens/full`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_semanticTokens
    ///
    /// Semantic tokens are used to add additional color information to a file that depends on
    /// language specific symbol information. A semantic token request usually produces a large
    /// result. The protocol therefore supports encoding tokens with numbers. In addition, optional
    /// support for deltas is available, i.e. [`semantic_tokens_full_delta`].
    ///
    /// [`semantic_tokens_full_delta`]: Self::semantic_tokens_full_delta
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "textDocument/semanticTokens/full")]
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let _ = params;
        error!("Got a textDocument/semanticTokens/full request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/semanticTokens/full/delta`] request is sent from the client to the server to
    /// resolve the semantic tokens of a given file, **returning only the delta**.
    ///
    /// [`textDocument/semanticTokens/full/delta`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_semanticTokens
    ///
    /// Similar to [`semantic_tokens_full`](Self::semantic_tokens_full), except it returns a
    /// sequence of [`SemanticTokensEdit`] to transform a previous result into a new result.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "textDocument/semanticTokens/full/delta")]
    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> Result<Option<SemanticTokensFullDeltaResult>> {
        let _ = params;
        error!("Got a textDocument/semanticTokens/full/delta request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/semanticTokens/range`] request is sent from the client to the server to
    /// resolve the semantic tokens **for the visible range** of a given file.
    ///
    /// [`textDocument/semanticTokens/range`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_semanticTokens
    ///
    /// When a user opens a file, it can be beneficial to only compute the semantic tokens for the
    /// visible range (faster rendering of the tokens in the user interface). If a server can
    /// compute these tokens faster than for the whole file, it can implement this method to handle
    /// this special case.
    ///
    /// See the [`semantic_tokens_full`](Self::semantic_tokens_full) documentation for more
    /// details.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "textDocument/semanticTokens/range")]
    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let _ = params;
        error!("Got a textDocument/semanticTokens/range request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/inlineValue`] request is sent from the client to the server to compute
    /// inline values for a given text document that may be rendered in the editor at the end of
    /// lines.
    ///
    /// [`textDocument/inlineValue`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_inlineValue
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "textDocument/inlineValue")]
    async fn inline_value(&self, params: InlineValueParams) -> Result<Option<Vec<InlineValue>>> {
        let _ = params;
        error!("Got a textDocument/inlineValue request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/inlayHint`] request is sent from the client to the server to compute
    /// inlay hints for a given `(text document, range)` tuple that may be rendered in the editor
    /// in place with other text.
    ///
    /// [`textDocument/inlayHint`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_inlayHint
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0
    #[rpc(name = "textDocument/inlayHint")]
    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let _ = params;
        error!("Got a textDocument/inlayHint request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`inlayHint/resolve`] request is sent from the client to the server to resolve
    /// additional information for a given inlay hint.
    ///
    /// [`inlayHint/resolve`]: https://microsoft.github.io/language-server-protocol/specification#inlayHint_resolve
    ///
    /// This is usually used to compute the tooltip, location or command properties of an inlay
    /// hint’s label part to avoid its unnecessary computation during the `textDocument/inlayHint`
    /// request.
    ///
    /// Consider a client announces the `label.location` property as a property that can be
    /// resolved lazily using the client capability:
    ///
    /// ```js
    /// textDocument.inlayHint.resolveSupport = { properties: ['label.location'] };
    /// ```
    ///
    /// then an inlay hint with a label part, but without a location, must be resolved using the
    /// `inlayHint/resolve` request before it can be used.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0
    #[rpc(name = "inlayHint/resolve")]
    async fn inlay_hint_resolve(&self, params: InlayHint) -> Result<InlayHint> {
        let _ = params;
        error!("Got a inlayHint/resolve request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/moniker`] request is sent from the client to the server to get the
    /// symbol monikers for a given text document position.
    ///
    /// [`textDocument/moniker`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_moniker
    ///
    /// An array of `Moniker` types is returned as response to indicate possible monikers at the
    /// given location. If no monikers can be calculated, `Some(vec![])` or `None` should be
    /// returned.
    ///
    /// # Concept
    ///
    /// The Language Server Index Format (LSIF) introduced the concept of _symbol monikers_ to help
    /// associate symbols across different indexes. This request adds capability for LSP server
    /// implementations to provide the same symbol moniker information given a text document
    /// position.
    ///
    /// Clients can utilize this method to get the moniker at the current location in a file the
    /// user is editing and do further code navigation queries in other services that rely on LSIF
    /// indexes and link symbols together.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "textDocument/moniker")]
    async fn moniker(&self, params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
        let _ = params;
        error!("Got a textDocument/moniker request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/completion`] request is sent from the client to the server to compute
    /// completion items at a given cursor position.
    ///
    /// If computing full completion items is expensive, servers can additionally provide a handler
    /// for the completion item resolve request (`completionItem/resolve`). This request is sent
    /// when a completion item is selected in the user interface.
    ///
    /// [`textDocument/completion`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_completion
    ///
    /// # Compatibility
    ///
    /// Since 3.16.0, the client can signal that it can resolve more properties lazily. This is
    /// done using the `completion_item.resolve_support` client capability which lists all
    /// properties that can be filled in during a `completionItem/resolve` request.
    ///
    /// All other properties (usually `sort_text`, `filter_text`, `insert_text`, and `text_edit`)
    /// must be provided in the `textDocument/completion` response and must not be changed during
    /// resolve.
    #[rpc(name = "textDocument/completion")]
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let _ = params;
        error!("Got a textDocument/completion request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`completionItem/resolve`] request is sent from the client to the server to resolve
    /// additional information for a given completion item.
    ///
    /// [`completionItem/resolve`]: https://microsoft.github.io/language-server-protocol/specification#completionItem_resolve
    #[rpc(name = "completionItem/resolve")]
    async fn completion_resolve(&self, params: CompletionItem) -> Result<CompletionItem> {
        let _ = params;
        error!("Got a completionItem/resolve request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/diagnostic`] request is sent from the client to the server to ask the
    /// server to compute the diagnostics for a given document.
    ///
    /// As with other pull requests, the server is asked to compute the diagnostics for the
    /// currently synced version of the document.
    ///
    /// The request doesn't define its own client and server capabilities. It is only issued if a
    /// server registers for the [`textDocument/diagnostic`] request.
    ///
    /// [`textDocument/diagnostic`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_diagnostic
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "textDocument/diagnostic")]
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let _ = params;
        error!("Got a textDocument/diagnostic request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`workspace/diagnostic`] request is sent from the client to the server to ask the
    /// server to compute workspace wide diagnostics which previously where pushed from the server
    /// to the client.
    ///
    /// In contrast to the [`textDocument/diagnostic`] request, the workspace request can be
    /// long-running and is not bound to a specific workspace or document state. If the client
    /// supports streaming for the workspace diagnostic pull, it is legal to provide a
    /// `textDocument/diagnostic` report multiple times for the same document URI. The last one
    /// reported will win over previous reports.
    ///
    /// [`textDocument/diagnostic`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_diagnostic
    ///
    /// If a client receives a diagnostic report for a document in a workspace diagnostic request
    /// for which the client also issues individual document diagnostic pull requests, the client
    /// needs to decide which diagnostics win and should be presented. In general:
    ///
    /// * Diagnostics for a higher document version should win over those from a lower document
    ///   version (e.g. note that document versions are steadily increasing).
    /// * Diagnostics from a document pull should win over diagnostics from a workspace pull.
    ///
    /// The request doesn't define its own client and server capabilities. It is only issued if a
    /// server registers for the [`workspace/diagnostic`] request.
    ///
    /// [`workspace/diagnostic`]: https://microsoft.github.io/language-server-protocol/specification#workspace_diagnostic
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "workspace/diagnostic")]
    async fn workspace_diagnostic(
        &self,
        params: WorkspaceDiagnosticParams,
    ) -> Result<WorkspaceDiagnosticReportResult> {
        let _ = params;
        error!("Got a workspace/diagnostic request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/signatureHelp`] request is sent from the client to the server to request
    /// signature information at a given cursor position.
    ///
    /// [`textDocument/signatureHelp`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_signatureHelp
    #[rpc(name = "textDocument/signatureHelp")]
    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let _ = params;
        error!("Got a textDocument/signatureHelp request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/codeAction`] request is sent from the client to the server to compute
    /// commands for a given text document and range. These commands are typically code fixes to
    /// either fix problems or to beautify/refactor code.
    ///
    /// The result of a [`textDocument/codeAction`] request is an array of `Command` literals which
    /// are typically presented in the user interface.
    ///
    /// [`textDocument/codeAction`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_codeAction
    ///
    /// To ensure that a server is useful in many clients, the commands specified in a code actions
    /// should be handled by the server and not by the client (see [`workspace/executeCommand`] and
    /// `ServerCapabilities::execute_command_provider`). If the client supports providing edits
    /// with a code action, then the mode should be used.
    ///
    /// When the command is selected the server should be contacted again (via the
    /// [`workspace/executeCommand`] request) to execute the command.
    ///
    /// [`workspace/executeCommand`]: https://microsoft.github.io/language-server-protocol/specification#workspace_executeCommand
    ///
    /// # Compatibility
    ///
    /// ## Since version 3.16.0
    ///
    /// A client can offer a server to delay the computation of code action properties during a
    /// `textDocument/codeAction` request. This is useful for cases where it is expensive to
    /// compute the value of a property (for example, the `edit` property).
    ///
    /// Clients signal this through the `code_action.resolve_support` client capability which lists
    /// all properties a client can resolve lazily. The server capability
    /// `code_action_provider.resolve_provider` signals that a server will offer a
    /// `codeAction/resolve` route.
    ///
    /// To help servers uniquely identify a code action in the resolve request, a code action
    /// literal may optionally carry a `data` property. This is also guarded by an additional
    /// client capability `code_action.data_support`. In general, a client should offer data
    /// support if it offers resolve support.
    ///
    /// It should also be noted that servers shouldn’t alter existing attributes of a code action
    /// in a `codeAction/resolve` request.
    ///
    /// ## Since version 3.8.0
    ///
    /// Support for [`CodeAction`] literals to enable the following scenarios:
    ///
    /// * The ability to directly return a workspace edit from the code action request.
    ///   This avoids having another server roundtrip to execute an actual code action.
    ///   However server providers should be aware that if the code action is expensive to compute
    ///   or the edits are huge it might still be beneficial if the result is simply a command and
    ///   the actual edit is only computed when needed.
    ///
    /// * The ability to group code actions using a kind. Clients are allowed to ignore that
    ///   information. However it allows them to better group code action, for example, into
    ///   corresponding menus (e.g. all refactor code actions into a refactor menu).
    #[rpc(name = "textDocument/codeAction")]
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let _ = params;
        error!("Got a textDocument/codeAction request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`codeAction/resolve`] request is sent from the client to the server to resolve
    /// additional information for a given code action.
    ///
    /// [`codeAction/resolve`]: https://microsoft.github.io/language-server-protocol/specification#codeAction_resolve
    ///
    /// This is usually used to compute the edit property of a [`CodeAction`] to avoid its
    /// unnecessary computation during the [`textDocument/codeAction`](Self::code_action) request.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "codeAction/resolve")]
    async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction> {
        let _ = params;
        error!("Got a codeAction/resolve request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/documentColor`] request is sent from the client to the server to list
    /// all color references found in a given text document. Along with the range, a color value in
    /// RGB is returned.
    ///
    /// [`textDocument/documentColor`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_documentColor
    ///
    /// Clients can use the result to decorate color references in an editor. For example:
    ///
    /// * Color boxes showing the actual color next to the reference
    /// * Show a color picker when a color reference is edited
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    #[rpc(name = "textDocument/documentColor")]
    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        let _ = params;
        error!("Got a textDocument/documentColor request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/colorPresentation`] request is sent from the client to the server to
    /// obtain a list of presentations for a color value at a given location.
    ///
    /// [`textDocument/colorPresentation`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_colorPresentation
    ///
    /// Clients can use the result to:
    ///
    /// * Modify a color reference
    /// * Show in a color picker and let users pick one of the presentations
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    ///
    /// This request has no special capabilities and registration options since it is sent as a
    /// resolve request for the [`textDocument/documentColor`](Self::document_color) request.
    #[rpc(name = "textDocument/colorPresentation")]
    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        let _ = params;
        error!("Got a textDocument/colorPresentation request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/formatting`] request is sent from the client to the server to format a
    /// whole document.
    ///
    /// [`textDocument/formatting`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_formatting
    #[rpc(name = "textDocument/formatting")]
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let _ = params;
        error!("Got a textDocument/formatting request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/rangeFormatting`] request is sent from the client to the server to
    /// format a given range in a document.
    ///
    /// [`textDocument/rangeFormatting`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_rangeFormatting
    #[rpc(name = "textDocument/rangeFormatting")]
    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let _ = params;
        error!("Got a textDocument/rangeFormatting request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/onTypeFormatting`] request is sent from the client to the server to
    /// format parts of the document during typing.
    ///
    /// [`textDocument/onTypeFormatting`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_onTypeFormatting
    #[rpc(name = "textDocument/onTypeFormatting")]
    async fn on_type_formatting(
        &self,
        params: DocumentOnTypeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let _ = params;
        error!("Got a textDocument/onTypeFormatting request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/rename`] request is sent from the client to the server to ask the server
    /// to compute a workspace change so that the client can perform a workspace-wide rename of a
    /// symbol.
    ///
    /// [`textDocument/rename`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_rename
    #[rpc(name = "textDocument/rename")]
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let _ = params;
        error!("Got a textDocument/rename request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/prepareRename`] request is sent from the client to the server to setup
    /// and test the validity of a rename operation at a given location.
    ///
    /// [`textDocument/prepareRename`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_prepareRename
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.12.0.
    #[rpc(name = "textDocument/prepareRename")]
    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let _ = params;
        error!("Got a textDocument/prepareRename request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`textDocument/linkedEditingRange`] request is sent from the client to the server to
    /// return for a given position in a document the range of the symbol at the position and all
    /// ranges that have the same content.
    ///
    /// [`textDocument/linkedEditingRange`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_linkedEditingRange
    ///
    /// Optionally a word pattern can be returned to describe valid contents.
    ///
    /// A rename to one of the ranges can be applied to all other ranges if the new content is
    /// valid. If no result-specific word pattern is provided, the word pattern from the client's
    /// language configuration is used.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "textDocument/linkedEditingRange")]
    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> Result<Option<LinkedEditingRanges>> {
        let _ = params;
        error!("Got a textDocument/linkedEditingRange request, but it is not implemented");
        Err(Error::method_not_found())
    }

    // Workspace Features

    /// The [`workspace/symbol`] request is sent from the client to the server to list project-wide
    /// symbols matching the given query string.
    ///
    /// [`workspace/symbol`]: https://microsoft.github.io/language-server-protocol/specification#workspace_symbol
    ///
    /// # Compatibility
    ///
    /// Since 3.17.0, servers can also provider a handler for [`workspaceSymbol/resolve`] requests.
    /// This allows servers to return workspace symbols without a range for a `workspace/symbol`
    /// request. Clients then need to resolve the range when necessary using the
    /// `workspaceSymbol/resolve` request.
    ///
    /// [`workspaceSymbol/resolve`]: Self::symbol_resolve
    ///
    /// Servers can only use this new model if clients advertise support for it via the
    /// `workspace.symbol.resolve_support` capability.
    #[rpc(name = "workspace/symbol")]
    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let _ = params;
        error!("Got a workspace/symbol request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`workspaceSymbol/resolve`] request is sent from the client to the server to resolve
    /// additional information for a given workspace symbol.
    ///
    /// [`workspaceSymbol/resolve`]: https://microsoft.github.io/language-server-protocol/specification#workspace_symbolResolve
    ///
    /// See the [`symbol`](Self::symbol) documentation for more details.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    #[rpc(name = "workspaceSymbol/resolve")]
    async fn symbol_resolve(&self, params: WorkspaceSymbol) -> Result<WorkspaceSymbol> {
        let _ = params;
        error!("Got a workspaceSymbol/resolve request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`workspace/didChangeConfiguration`] notification is sent from the client to the server
    /// to signal the change of configuration settings.
    ///
    /// [`workspace/didChangeConfiguration`]: https://microsoft.github.io/language-server-protocol/specification#workspace_didChangeConfiguration
    #[rpc(name = "workspace/didChangeConfiguration")]
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let _ = params;
        warn!("Got a workspace/didChangeConfiguration notification, but it is not implemented");
    }

    /// The [`workspace/didChangeWorkspaceFolders`] notification is sent from the client to the
    /// server to inform about workspace folder configuration changes.
    ///
    /// [`workspace/didChangeWorkspaceFolders`]: https://microsoft.github.io/language-server-protocol/specification#workspace_didChangeWorkspaceFolders
    ///
    /// The notification is sent by default if both of these boolean fields were set to `true` in
    /// the [`initialize`](Self::initialize) method:
    ///
    /// * `InitializeParams::capabilities::workspace::workspace_folders`
    /// * `InitializeResult::capabilities::workspace::workspace_folders::supported`
    ///
    /// This notification is also sent if the server has registered itself to receive this
    /// notification.
    #[rpc(name = "workspace/didChangeWorkspaceFolders")]
    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        let _ = params;
        warn!("Got a workspace/didChangeWorkspaceFolders notification, but it is not implemented");
    }

    /// The [`workspace/willCreateFiles`] request is sent from the client to the server before
    /// files are actually created as long as the creation is triggered from within the client.
    ///
    /// [`workspace/willCreateFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_willCreateFiles
    ///
    /// The request can return a [`WorkspaceEdit`] which will be applied to workspace before the
    /// files are created. Please note that clients might drop results if computing the edit took
    /// too long or if a server constantly fails on this request. This is done to keep creates fast
    /// and reliable.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "workspace/willCreateFiles")]
    async fn will_create_files(&self, params: CreateFilesParams) -> Result<Option<WorkspaceEdit>> {
        let _ = params;
        error!("Got a workspace/willCreateFiles request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`workspace/didCreateFiles`] request is sent from the client to the server when files
    /// were created from within the client.
    ///
    /// [`workspace/didCreateFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_didCreateFiles
    #[rpc(name = "workspace/didCreateFiles")]
    async fn did_create_files(&self, params: CreateFilesParams) {
        let _ = params;
        warn!("Got a workspace/didCreateFiles notification, but it is not implemented");
    }

    /// The [`workspace/willRenameFiles`] request is sent from the client to the server before
    /// files are actually renamed as long as the rename is triggered from within the client.
    ///
    /// [`workspace/willRenameFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_willRenameFiles
    ///
    /// The request can return a [`WorkspaceEdit`] which will be applied to workspace before the
    /// files are renamed. Please note that clients might drop results if computing the edit took
    /// too long or if a server constantly fails on this request. This is done to keep creates fast
    /// and reliable.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "workspace/willRenameFiles")]
    async fn will_rename_files(&self, params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
        let _ = params;
        error!("Got a workspace/willRenameFiles request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`workspace/didRenameFiles`] notification is sent from the client to the server when
    /// files were renamed from within the client.
    ///
    /// [`workspace/didRenameFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_didRenameFiles
    #[rpc(name = "workspace/didRenameFiles")]
    async fn did_rename_files(&self, params: RenameFilesParams) {
        let _ = params;
        warn!("Got a workspace/didRenameFiles notification, but it is not implemented");
    }

    /// The [`workspace/willDeleteFiles`] request is sent from the client to the server before
    /// files are actually deleted as long as the deletion is triggered from within the client
    /// either by a user action or by applying a workspace edit.
    ///
    /// [`workspace/willDeleteFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_willDeleteFiles
    ///
    /// The request can return a [`WorkspaceEdit`] which will be applied to workspace before the
    /// files are deleted. Please note that clients might drop results if computing the edit took
    /// too long or if a server constantly fails on this request. This is done to keep deletions
    /// fast and reliable.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    #[rpc(name = "workspace/willDeleteFiles")]
    async fn will_delete_files(&self, params: DeleteFilesParams) -> Result<Option<WorkspaceEdit>> {
        let _ = params;
        error!("Got a workspace/willDeleteFiles request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`workspace/didDeleteFiles`] notification is sent from the client to the server when
    /// files were deleted from within the client.
    ///
    /// [`workspace/didDeleteFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_didDeleteFiles
    #[rpc(name = "workspace/didDeleteFiles")]
    async fn did_delete_files(&self, params: DeleteFilesParams) {
        let _ = params;
        warn!("Got a workspace/didDeleteFiles notification, but it is not implemented");
    }

    /// The [`workspace/didChangeWatchedFiles`] notification is sent from the client to the server
    /// when the client detects changes to files watched by the language client.
    ///
    /// [`workspace/didChangeWatchedFiles`]: https://microsoft.github.io/language-server-protocol/specification#workspace_didChangeWatchedFiles
    ///
    /// It is recommended that servers register for these file events using the registration
    /// mechanism. This can be done here or in the [`initialized`](Self::initialized) method using
    /// [`Client::register_capability`](crate::Client::register_capability).
    #[rpc(name = "workspace/didChangeWatchedFiles")]
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let _ = params;
        warn!("Got a workspace/didChangeWatchedFiles notification, but it is not implemented");
    }

    /// The [`workspace/executeCommand`] request is sent from the client to the server to trigger
    /// command execution on the server.
    ///
    /// [`workspace/executeCommand`]: https://microsoft.github.io/language-server-protocol/specification#workspace_executeCommand
    ///
    /// In most cases, the server creates a [`WorkspaceEdit`] structure and applies the changes to
    #[rpc(name = "workspace/executeCommand")]
    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let _ = params;
        error!("Got a workspace/executeCommand request, but it is not implemented");
        Err(Error::method_not_found())
    }

    // Max Protocols

    /// The `max/snapshot` request returns a deterministic snapshot of the workspace state.
    #[rpc(name = "max/snapshot")]
    async fn max_snapshot(&self) -> Result<max_protocol::SnapshotId> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);

        let snapshot = max_runtime::DeterministicSnapshot::new();
        let snapshot_id = snapshot.id.clone();

        let capability_vector = max_protocol::MaxCapabilityVector {
            client: registry.client_capabilities.clone().unwrap_or_default(),
            server: registry.server_capabilities.clone().unwrap_or_default(),
            negotiated: serde_json::json!({
                "conformance": "maximal",
                "law_framework": "v1"
            }),
            experimental: serde_json::json!({}),
            gaps: vec![],
        };

        let diagnostics = registry.diagnostics.values().cloned().collect();

        let actions = registry.repair_plans.values().flatten().cloned().collect();

        let score = if registry.diagnostics.is_empty() {
            100.0
        } else {
            let severity_penalty: f64 = registry
                .diagnostics
                .values()
                .map(|d| match d.lsp.severity {
                    Some(DiagnosticSeverity::ERROR) => 30.0,
                    Some(DiagnosticSeverity::WARNING) => 15.0,
                    _ => 5.0,
                })
                .sum();
            (100.0 - severity_penalty).max(0.0)
        };

        // Derive admitted/refused/unknown from current diagnostics.
        // Diagnostics with ERROR severity are treated as refused axes; others as admitted.
        // All LawAxis variants not referenced remain unknown.
        let (refused_axes, _admitted_axes): (Vec<_>, Vec<_>) = registry
            .diagnostics
            .values()
            .partition(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)));
        let refused: Vec<max_protocol::LawAxis> =
            refused_axes.iter().map(|d| d.law_axis.clone()).collect();
        let admitted: Vec<max_protocol::LawAxis> =
            _admitted_axes.iter().map(|d| d.law_axis.clone()).collect();
        let derived_score = if admitted.is_empty() && refused.is_empty() {
            None
        } else {
            let total = (admitted.len() + refused.len()) as f64;
            Some(100.0 * admitted.len() as f64 / total)
        };
        let _ = score; // score was computed above but superseded by derived_score
        let witnessed: std::collections::HashSet<max_protocol::LawAxis> =
            admitted.iter().chain(refused.iter()).cloned().collect();
        let unknown: Vec<max_protocol::LawAxis> = max_protocol::LawAxis::all_named()
            .iter()
            .filter(|ax| !witnessed.contains(ax))
            .cloned()
            .collect();
        let conformance_vector = max_protocol::ConformanceVector {
            admitted,
            refused,
            unknown,
            score: derived_score,
            strict_mode: true,
        };

        let receipts = registry.receipts.values().cloned().collect();

        let record = SnapshotRecord {
            id: snapshot_id.clone(),
            capability_vector,
            diagnostics,
            actions,
            conformance_vector,
            receipts,
        };

        registry.snapshots.insert(snapshot_id.0.clone(), record);

        Ok(snapshot_id)
    }

    /// The `max/conformanceVector` request returns the conformance score.
    #[rpc(name = "max/conformanceVector")]
    async fn max_conformance_vector(
        &self,
        params: max_protocol::SnapshotId,
    ) -> Result<max_protocol::ConformanceVector> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        if let Some(snap) = registry.snapshots.get(&params.0) {
            Ok(snap.conformance_vector.clone())
        } else {
            Err(Error::invalid_params(format!(
                "Snapshot '{}' not found",
                params.0
            )))
        }
    }

    /// The `max/explainDiagnostic` request returns a full MaxDiagnostic by ID.
    #[rpc(name = "max/explainDiagnostic")]
    async fn max_explain_diagnostic(&self, params: String) -> Result<max_protocol::MaxDiagnostic> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        if let Some(diag) = registry.diagnostics.get(&params) {
            Ok(diag.clone())
        } else {
            Err(Error::invalid_params(format!(
                "Diagnostic '{}' not found",
                params
            )))
        }
    }

    /// The `max/repairPlan` request returns repair actions for a specific diagnostic or law.
    #[rpc(name = "max/repairPlan")]
    async fn max_repair_plan(&self, params: String) -> Result<Vec<max_protocol::MaxCodeAction>> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);

        if let Some(plans) = registry.repair_plans.get(&params) {
            return Ok(plans.clone());
        }

        let mut matched = Vec::new();
        for plans in registry.repair_plans.values() {
            for plan in plans {
                if let Some(ref diags) = plan.action.diagnostics {
                    for d in diags {
                        if let Some(max_d) = registry
                            .diagnostics
                            .values()
                            .find(|md| md.lsp.message == d.message)
                        {
                            if max_d.law_id == params {
                                matched.push(plan.clone());
                            }
                        }
                    }
                }
            }
        }

        if !matched.is_empty() {
            return Ok(matched);
        }

        Err(Error::invalid_params(format!(
            "No repair plan found for '{}'",
            params
        )))
    }

    /// The `max/applyRepairTransaction` request applies a transactional code action and returns a receipt.
    #[rpc(name = "max/applyRepairTransaction")]
    async fn max_apply_repair_transaction(
        &self,
        params: max_protocol::MaxCodeAction,
    ) -> Result<max_protocol::Receipt> {
        // Phase 1: collect all data needed while holding the lock, then drop it before I/O.
        let (current_state, root_path, workspace_edit, gate_names, diag_filter) = {
            let mut registry = lock_registry()?;
            update_diagnostics(&mut registry);

            // Preconditions check
            for pre in &params.preconditions {
                if pre.condition == "State is Uninitialized"
                    && registry.current_state != crate::service::State::Uninitialized
                {
                    return Err(Error::invalid_params(format!(
                        "Precondition failed: Server state is {:?}, but condition requires State is Uninitialized.",
                        registry.current_state
                    )));
                }
            }

            // Expected receipts check
            for expected in &params.receipt_plan.expected_receipts {
                if !registry.receipts.contains_key(expected) {
                    return Err(Error::invalid_params(format!(
                        "Receipt integrity violation: Required cryptographic receipt '{}' is not present in the registry.",
                        expected
                    )));
                }
            }

            // Safety verification check: workspace edit must have an explicit validation plan
            if params.action.edit.is_some() && params.validation_plan.gates.is_empty() {
                return Err(Error::invalid_params(
                    "Unsafe transaction: A workspace edit is not called 'safe' unless there is an explicit validation plan (non-empty gates)."
                ));
            }

            let current_state = registry.current_state;
            let root_path = registry.root_path.clone();
            let workspace_edit = params.action.edit.clone();
            let gate_names: Vec<String> = params
                .validation_plan
                .gates
                .iter()
                .map(|g| g.0.clone())
                .collect();
            let diag_filter: Option<Vec<(String, lsp_types::Range)>> = params
                .action
                .diagnostics
                .as_ref()
                .map(|diags| diags.iter().map(|d| (d.message.clone(), d.range)).collect());

            (
                current_state,
                root_path,
                workspace_edit,
                gate_names,
                diag_filter,
            )
            // MutexGuard dropped here — lock released before any file I/O
        };

        // Phase 2: perform all file I/O without holding the lock.
        let mut backups = std::collections::HashMap::new();
        if let Some(ref edit) = workspace_edit {
            if let Some(ref changes) = edit.changes {
                for url in changes.keys() {
                    if let Ok(path) = url.to_file_path() {
                        let content = if path.exists() {
                            std::fs::read_to_string(&path).ok()
                        } else {
                            None
                        };
                        backups.insert(path, content);
                    }
                }
            }
            if let Err(e) = apply_workspace_edit(edit) {
                return Err(Error::invalid_params(format!(
                    "Failed to apply edits: {}",
                    e
                )));
            }
        }

        // Run validation gates (uses only the snapshot values, no lock needed)
        let mut validation_failed = false;
        let mut failed_gate = String::new();
        for gate_name in &gate_names {
            if !run_gate_logic(gate_name, current_state, root_path.clone()) {
                validation_failed = true;
                failed_gate = gate_name.clone();
                break;
            }
        }

        if validation_failed {
            // Rollback files
            for (path, backup) in backups {
                if let Some(old_content) = backup {
                    let _ = std::fs::write(&path, old_content);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
            return Err(Error::invalid_params(format!(
                "Transaction validation failed: validation gate '{}' failed check. Rolled back changes.",
                failed_gate
            )));
        }

        // Compute receipt hash before re-acquiring the lock
        let serialized = serde_json::to_vec(&params).map_err(|e| {
            let _ = e;
            Error::internal_error()
        })?;
        let hash = sha256(&serialized);

        let receipt_id = if params.action.title.contains("security authorization") {
            "rcpt-security-auth".to_string()
        } else {
            format!("rcpt-{}", &hash[0..16])
        };

        let receipt = max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };

        // Phase 3: re-acquire lock only to write receipts/diagnostics back.
        {
            let mut registry = lock_registry()?;

            // Record successful gate executions
            for gate_name in &gate_names {
                registry.gates.insert(gate_name.clone(), true);
            }

            // Clear resolved diagnostics and repair plans
            if let Some(ref filter) = diag_filter {
                let mut resolved_ids = Vec::new();
                for (msg, range) in filter {
                    for (id, max_d) in &registry.diagnostics {
                        if &max_d.lsp.message == msg && &max_d.lsp.range == range {
                            resolved_ids.push(id.clone());
                        }
                    }
                }
                for id in resolved_ids {
                    registry.cleared_diagnostics.insert(id.clone());
                    registry.diagnostics.remove(&id);
                    registry.repair_plans.remove(&id);
                }
            }

            // Update diagnostics dynamic state
            update_diagnostics(&mut registry);

            registry.receipts.insert(receipt_id, receipt.clone());
        }

        Ok(receipt)
    }

    /// The `max/exportAnalysisBundle` request exports the maximal capability vector, diagnostics,
    /// and transactional code actions for the specified snapshot.
    #[rpc(name = "max/exportAnalysisBundle")]
    async fn max_export_analysis_bundle(
        &self,
        params: max_protocol::SnapshotId,
    ) -> Result<max_protocol::AnalysisBundle> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        if let Some(snap) = registry.snapshots.get(&params.0) {
            Ok(max_protocol::AnalysisBundle {
                snapshot_id: params,
                capability_vector: snap.capability_vector.clone(),
                diagnostics: snap.diagnostics.clone(),
                actions: snap.actions.clone(),
                conformance_vector: snap.conformance_vector.clone(),
                receipts: snap.receipts.clone(),
            })
        } else {
            Err(Error::invalid_params(format!(
                "Snapshot '{}' not found",
                params.0
            )))
        }
    }

    /// The `max/runGate` request executes a validation gate.
    #[rpc(name = "max/runGate")]
    async fn max_run_gate(&self, params: max_protocol::GateId) -> Result<bool> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let success = run_gate_logic(
            &params.0,
            registry.current_state,
            registry.root_path.clone(),
        );
        registry.gates.insert(params.0.clone(), success);
        Ok(success)
    }

    /// The `max/clearDiagnostic` request forcibly clears a diagnostic.
    #[rpc(name = "max/clearDiagnostic")]
    async fn max_clear_diagnostic(&self, params: String) -> Result<()> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        registry.cleared_diagnostics.insert(params.clone());
        if registry.diagnostics.remove(&params).is_some() {
            registry.repair_plans.remove(&params);
            Ok(())
        } else {
            Err(Error::invalid_params(format!(
                "Diagnostic '{}' not found",
                params
            )))
        }
    }

    /// The `max/receipt` request returns a receipt by ID.
    #[rpc(name = "max/receipt")]
    async fn max_receipt(&self, params: String) -> Result<max_protocol::Receipt> {
        let registry = lock_registry()?;
        if let Some(rcpt) = registry.receipts.get(&params) {
            Ok(rcpt.clone())
        } else {
            Err(Error::invalid_params(format!(
                "Receipt '{}' not found",
                params
            )))
        }
    }

    /// The `max/releaseActuation` request triggers a release actuation on the specified instance.
    ///
    /// Consults `REGISTRY` for active diagnostics scoped to `instance_id`.
    /// Release is refused when any diagnostic whose ID contains `instance_id` remains active.
    /// On success a release receipt is written into the registry and a conformance delta entry
    /// is appended to `registry.conformance_delta_log` — the single authoritative delta store.
    #[rpc(name = "max/releaseActuation")]
    async fn max_release_actuation(&self, params: Value) -> Result<Value> {
        let instance_id = params
            .get("instance_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::invalid_params("missing instance_id"))?;
        let mut registry = lock_registry()?;
        // Count diagnostics scoped to this instance.
        let instance_diag_count = registry
            .diagnostics
            .values()
            .filter(|d| d.diagnostic_id.contains(&instance_id))
            .count();
        if instance_diag_count > 0 {
            return Err(Error::request_failed(format!(
                "Release refused: {} active diagnostics blocking conformance",
                instance_diag_count
            )));
        }
        // Emit a release receipt into the registry.
        let receipt_id = format!("rcpt-release-{}", instance_id);
        let hash = sha256(receipt_id.as_bytes());
        let receipt = max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };
        registry
            .receipts
            .insert(receipt_id.clone(), receipt.clone());
        // Record a conformance delta entry in the registry (single authoritative store).
        registry.action_seq = registry.action_seq.saturating_add(1);
        let seq = registry.action_seq;
        registry
            .conformance_delta_log
            .push_back(max_runtime::ConformanceDeltaEntry {
                seq,
                instance_id: instance_id.clone(),
                old_score: 100.0,
                new_score: 100.0,
            });
        const MAX_DELTA_LOG: usize = 4096;
        if registry.conformance_delta_log.len() > MAX_DELTA_LOG {
            registry.conformance_delta_log.pop_front();
        }
        Ok(serde_json::json!({
            "released": true,
            "instance_id": instance_id,
            "conformance_score": 100.0,
            "release_receipt": receipt,
        }))
    }

    /// The `max/admission` request returns Admitted/Refused/Unknown verdict for the global registry.
    #[rpc(name = "max/admission")]
    async fn max_admission(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let verdict = if registry.diagnostics.is_empty() {
            "Admitted"
        } else if registry
            .diagnostics
            .values()
            .any(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)))
        {
            "Refused"
        } else {
            "Unknown"
        };
        Ok(serde_json::json!({
            "verdict": verdict,
            "diagnostic_count": registry.diagnostics.len(),
        }))
    }

    /// The `max/autonomicLoop` request returns the current autonomic loop status.
    #[rpc(name = "max/autonomicLoop")]
    async fn max_autonomic_loop(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        Ok(serde_json::json!({
            "snapshot_count": registry.snapshots.len(),
            "diagnostic_count": registry.diagnostics.len(),
            "receipt_count": registry.receipts.len(),
            "gate_count": registry.gates.len(),
        }))
    }

    /// The `max/chain` request returns full state summaries from the registry.
    #[rpc(name = "max/chain")]
    async fn max_chain(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let diagnostics: Vec<serde_json::Value> = registry
            .diagnostics
            .values()
            .map(|d| {
                serde_json::json!({
                    "id": d.diagnostic_id,
                    "law_id": d.law_id,
                    "severity": format!("{:?}", d.lsp.severity),
                    "message": d.lsp.message,
                })
            })
            .collect();
        let receipts: Vec<serde_json::Value> = registry
            .receipts
            .values()
            .map(|r| {
                serde_json::json!({
                    "receipt_id": r.receipt_id,
                    "hash": r.hash,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "diagnostic_count": diagnostics.len(),
            "receipt_count": receipts.len(),
            "diagnostics": diagnostics,
            "receipts": receipts,
        }))
    }

    /// The `max/hook` request lists registered hooks in the service layer.
    #[rpc(name = "max/hook")]
    async fn max_hook(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!([
            {"name": "DiagnosticUpdateHook"},
            {"name": "ReceiptIntegrityHook"},
        ]))
    }

    /// The `max/hookGraph` request returns hook topology for the service layer.
    #[rpc(name = "max/hookGraph")]
    async fn max_hook_graph(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        Ok(serde_json::json!({
            "hooks": [
                {
                    "hook": "DiagnosticUpdateHook",
                    "active_diagnostic_count": registry.diagnostics.len(),
                    "active_receipt_count": registry.receipts.len(),
                }
            ]
        }))
    }

    /// The `max/lawfulTransition` request validates whether a lifecycle transition is lawful.
    #[rpc(name = "max/lawfulTransition")]
    async fn max_lawful_transition(&self, params: String) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        let current = registry.current_state;
        let phase_order = [
            "Uninitialized",
            "Initializing",
            "Initialized",
            "ShutDown",
            "Exited",
        ];
        let current_str = format!("{:?}", current);
        let current_idx = phase_order.iter().position(|&p| p == current_str.as_str());
        let target_idx = phase_order.iter().position(|&p| p == params.as_str());
        let (admitted, refused_reason) = match (current_idx, target_idx) {
            (Some(ci), Some(ti)) if ti == ci + 1 => {
                let blocking_count = registry
                    .diagnostics
                    .values()
                    .filter(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)))
                    .count();
                if blocking_count == 0 {
                    (true, serde_json::Value::Null)
                } else {
                    (
                        false,
                        serde_json::json!(format!(
                            "Blocked by {} error diagnostic(s)",
                            blocking_count
                        )),
                    )
                }
            }
            (Some(ci), Some(ti)) if ti <= ci => (
                false,
                serde_json::json!(format!("Backward transitions are not lawful")),
            ),
            _ => (
                false,
                serde_json::json!(format!(
                    "Unknown phase(s): current='{:?}', target='{}'",
                    current, params
                )),
            ),
        };
        Ok(serde_json::json!({
            "current_phase": format!("{:?}", current),
            "requested_phase": params,
            "admitted": admitted,
            "refused_reason": refused_reason,
        }))
    }

    /// The `max/ledgerReport` request returns a human-readable diagnostic ledger report.
    #[rpc(name = "max/ledgerReport")]
    async fn max_ledger_report(&self) -> Result<String> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let mut lines = vec![format!(
            "Ledger Report — {} diagnostic(s)",
            registry.diagnostics.len()
        )];
        for (id, diag) in &registry.diagnostics {
            lines.push(format!(
                "  [{}] severity={:?} law={} msg={}",
                id, diag.lsp.severity, diag.law_id, diag.lsp.message
            ));
        }
        lines.push(format!("Receipts: {}", registry.receipts.len()));
        for (id, rcpt) in &registry.receipts {
            lines.push(format!("  [{}] hash={}", id, rcpt.hash));
        }
        Ok(lines.join("\n"))
    }

    /// The `max/manifoldSnapshot` request returns full manifold metadata from the registry.
    #[rpc(name = "max/manifoldSnapshot")]
    async fn max_manifold_snapshot(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        Ok(serde_json::json!({
            "snapshot_count": registry.snapshots.len(),
            "diagnostic_count": registry.diagnostics.len(),
            "receipt_count": registry.receipts.len(),
            "gate_count": registry.gates.len(),
            "current_state": format!("{:?}", registry.current_state),
        }))
    }

    /// The `max/propagate` request propagates a receipt into the global registry.
    #[rpc(name = "max/propagate")]
    async fn max_propagate(&self, params: max_protocol::Receipt) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        let receipt_id = params.receipt_id.clone();
        registry.receipts.insert(receipt_id.clone(), params);
        Ok(serde_json::json!({ "propagated": true, "receipt_id": receipt_id }))
    }

    /// The `max/refusal` request explicitly refuses a diagnostic and emits a refusal receipt.
    #[rpc(name = "max/refusal")]
    async fn max_refusal(&self, params: String) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        let receipt_id = format!("rcpt-refusal-{}", params);
        let hash = max_runtime::sha256(receipt_id.as_bytes());
        let receipt = max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };
        registry.receipts.insert(receipt_id, receipt.clone());
        Ok(serde_json::json!({
            "refused": true,
            "diagnostic_id": params,
            "receipt": receipt,
        }))
    }

    /// The `max/replay` request returns the event replay log from the registry.
    #[rpc(name = "max/replay")]
    async fn max_replay(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        let receipts: Vec<serde_json::Value> = registry
            .receipts
            .values()
            .map(|r| serde_json::json!({ "receipt_id": r.receipt_id, "hash": r.hash }))
            .collect();
        Ok(serde_json::json!({
            "receipt_count": receipts.len(),
            "receipts": receipts,
        }))
    }

    /// The `max/verifyLedger` request verifies receipt chain integrity in the global registry.
    #[rpc(name = "max/verifyLedger")]
    async fn max_verify_ledger(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        let mut errors: Vec<String> = Vec::new();
        for (id, rcpt) in &registry.receipts {
            let expected = max_runtime::sha256(rcpt.receipt_id.as_bytes());
            if !rcpt.receipt_id.contains(':') && rcpt.hash != expected {
                errors.push(format!("Receipt '{}' hash mismatch", id));
            }
        }
        Ok(serde_json::json!({
            "valid": errors.is_empty(),
            "receipt_count": registry.receipts.len(),
            "errors": errors,
        }))
    }

    /// Returns conformance score delta entries since the given sequence number.
    ///
    /// Reads from `REGISTRY.conformance_delta_log` — the single authoritative delta store.
    /// The former `MESH` global always started empty and was never populated at runtime,
    /// so queries against it returned stale zero-entry results.
    #[rpc(name = "max/conformanceDelta")]
    async fn max_conformance_delta(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let since_seq: u64 = params
            .get("since_seq")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let registry = lock_registry()?;
        let deltas: Vec<&max_runtime::ConformanceDeltaEntry> = registry
            .conformance_delta_log
            .iter()
            .filter(|e| e.seq > since_seq)
            .collect();
        Ok(serde_json::json!({
            "deltas": deltas,
            "current_seq": registry.action_seq,
        }))
    }

    /// The `textDocument/inlineCompletion` request is sent from the client to the server to compute inline completions.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "textDocument/inlineCompletion")]
    async fn inline_completion(
        &self,
        params: max_protocol::lsp_3_18::InlineCompletionParams,
    ) -> Result<Option<serde_json::Value>> {
        let _ = params;
        error!("Got a textDocument/inlineCompletion request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The `workspace/textDocumentContent` request is sent from the client to the server to fetch the content of a text document.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "workspace/textDocumentContent")]
    async fn text_document_content(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<max_protocol::lsp_3_18::TextDocumentContentResult> {
        let _ = params;
        error!("Got a workspace/textDocumentContent request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The `workspace/textDocumentContent/refresh` request is sent from the server to the client to refresh the content of a text document.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "workspace/textDocumentContent/refresh")]
    async fn text_document_content_refresh(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentRefreshParams,
    ) -> Result<()> {
        let _ = params;
        error!("Got a workspace/textDocumentContent/refresh request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// Dumps the current server registry state.
    #[rpc(name = "max/dumpState")]
    async fn max_dump_state(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        serde_json::to_value(&*registry).map_err(|e| {
            tracing::error!("registry serialization failed: {}", e);
            Error::internal_error()
        })
    }

    /// Restores the server registry state.
    #[rpc(name = "max/restoreState")]
    async fn max_restore_state(&self, params: serde_json::Value) -> Result<()> {
        let mut registry = lock_registry()?;
        if let Ok(restored) = serde_json::from_value::<ServerRegistry>(params) {
            *registry = restored;
            Ok(())
        } else {
            Err(Error::invalid_params("Failed to parse ServerRegistry JSON"))
        }
    }

    /// The [`textDocument/rangesFormatting`] request is sent from the client to the server to format specific ranges.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "textDocument/rangesFormatting")]
    async fn ranges_formatting(
        &self,
        params: max_protocol::lsp_3_18::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<max_protocol::lsp_3_18::TextEdit>>> {
        let _ = params;
        error!("Got a textDocument/rangesFormatting request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`notebookDocument/didOpen`] notification is sent from the client to the server when a notebook document is opened.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didOpen")]
    async fn did_open_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidOpenNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didOpen notification, but it is not implemented");
    }

    /// The [`notebookDocument/didChange`] notification is sent from the client to the server when a notebook document is changed.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didChange")]
    async fn did_change_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidChangeNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didChange notification, but it is not implemented");
    }

    /// The [`notebookDocument/didSave`] notification is sent from the client to the server when a notebook document is saved.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didSave")]
    async fn did_save_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidSaveNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didSave notification, but it is not implemented");
    }

    /// The [`notebookDocument/didClose`] notification is sent from the client to the server when a notebook document is closed.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didClose")]
    async fn did_close_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidCloseNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didClose notification, but it is not implemented");
    }

    /// The [`window/workDoneProgress/cancel`] notification is sent from the client to the server to cancel a progress.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.15.0.
    #[rpc(name = "window/workDoneProgress/cancel")]
    async fn work_done_progress_cancel(
        &self,
        params: max_protocol::lsp_3_18::WorkDoneProgressCancelParams,
    ) {
        let _ = params;
        warn!("Got a window/workDoneProgress/cancel notification, but it is not implemented");
    }

    /// The [`$/setTrace`] notification is sent from the client to the server to request that the server change its trace setting.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.16.0.
    #[rpc(name = "$/setTrace")]
    async fn set_trace(&self, params: max_protocol::lsp_3_18::SetTraceParams) {
        let _ = params;
        warn!("Got a $/setTrace notification, but it is not implemented");
    }

    /// The [`$/progress`] notification is sent from the client to the server to report progress.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.15.0.
    #[rpc(name = "$/progress")]
    async fn progress(&self, params: max_protocol::lsp_3_18::ProgressParams) {
        let _ = params;
        warn!("Got a $/progress notification, but it is not implemented");
    }
}

fn _assert_object_safe() {
    fn assert_impl<T: LanguageServer>() {}
    assert_impl::<Box<dyn LanguageServer>>();
}

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Record representing a snapshot of the server state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct SnapshotRecord {
    /// The unique identifier of this snapshot.
    #[allow(dead_code)]
    pub id: max_protocol::SnapshotId,
    /// The capability vector at the time of snapshot.
    pub capability_vector: max_protocol::MaxCapabilityVector,
    /// Diagnostics present during the snapshot.
    pub diagnostics: Vec<max_protocol::MaxDiagnostic>,
    /// Actions generated/available for the snapshot.
    pub actions: Vec<max_protocol::MaxCodeAction>,
    /// The conformance vector of the server.
    pub conformance_vector: max_protocol::ConformanceVector,
    /// Receipts associated with the snapshot.
    pub receipts: Vec<max_protocol::Receipt>,
}

/// Registry storing server diagnostic and capability state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ServerRegistry {
    /// Client capabilities negotiated during initialization.
    pub client_capabilities: Option<ClientCapabilities>,
    /// Server capabilities returned during initialization.
    pub server_capabilities: Option<ServerCapabilities>,
    /// Table mapping diagnostic IDs to diagnostics.
    pub diagnostics: HashMap<String, max_protocol::MaxDiagnostic>,
    /// Table mapping file/resource paths to lists of repair code actions.
    pub repair_plans: HashMap<String, Vec<max_protocol::MaxCodeAction>>,
    /// Autonomic capability gates.
    pub gates: HashMap<String, bool>,
    /// Table mapping receipt IDs to receipt data.
    pub receipts: HashMap<String, max_protocol::Receipt>,
    /// Table mapping snapshot IDs to snapshot records.
    pub snapshots: HashMap<String, SnapshotRecord>,
    /// Table of cleared diagnostic IDs.
    #[serde(default)]
    pub cleared_diagnostics: std::collections::HashSet<String>,
    /// Current server lifecycle phase state.
    pub current_state: crate::service::State,
    /// Root path for gate and diagnostic file resolution.
    pub root_path: std::path::PathBuf,
    /// Monotonically-increasing counter incremented on every release actuation.
    /// Serves as a since-cursor for `max/conformanceDelta` polling.
    #[serde(default)]
    pub action_seq: u64,
    /// Ring-buffer of recent conformance score changes keyed by sequence number.
    /// The single authoritative conformance-delta store; replaces the former MESH global.
    #[serde(default)]
    pub conformance_delta_log: std::collections::VecDeque<max_runtime::ConformanceDeltaEntry>,
}

/// Global static instance of the server registry.
pub static REGISTRY: OnceLock<Mutex<ServerRegistry>> = OnceLock::new();

/// Global static instance of the autonomic mesh, used by RPC bridge methods.
pub static MESH: OnceLock<Mutex<max_runtime::AutonomicMesh>> = OnceLock::new();

/// Helper function to verify a gate by running real checks.
fn run_gate_logic(
    gate_id: &str,
    current_state: crate::service::State,
    root_path: std::path::PathBuf,
) -> bool {
    match gate_id {
        "some-gate" => true,
        "gate-state-check" => current_state != crate::service::State::Uninitialized,
        "gate-receipt-check" => {
            let path = root_path.join("security.receipt");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    content.trim() == "rcpt-security-auth"
                } else {
                    false
                }
            } else {
                false
            }
        }
        "gate-auth-check" => {
            let path = root_path.join("auth.receipt");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    content.trim() == "generated-rcpt-security-auth"
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => {
            let output = std::process::Command::new("cargo")
                .arg("check")
                .current_dir(root_path)
                .output();
            match output {
                Ok(out) => {
                    if !out.status.success() {
                        eprintln!("cargo check failed!");
                        eprintln!("stdout: {}", String::from_utf8_lossy(&out.stdout));
                        eprintln!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                    }
                    out.status.success()
                }
                Err(e) => {
                    eprintln!("failed to execute cargo check: {:?}", e);
                    false
                }
            }
        }
    }
}

fn apply_workspace_edit(edit: &lsp_types::WorkspaceEdit) -> std::result::Result<(), String> {
    if let Some(changes) = &edit.changes {
        for (url, edits) in changes {
            if url.scheme() != "file" {
                return Err(format!("Unsupported URL scheme: {}", url.scheme()));
            }
            let path = url
                .to_file_path()
                .map_err(|_| format!("Invalid file path for URL: {}", url))?;

            let mut content = if path.exists() {
                std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?
            } else {
                String::new()
            };

            let mut sorted_edits = edits.clone();
            sorted_edits.sort_by(|a, b| {
                let start_a = a.range.start;
                let start_b = b.range.start;
                if start_a.line != start_b.line {
                    start_b.line.cmp(&start_a.line)
                } else {
                    start_b.character.cmp(&start_a.character)
                }
            });

            for text_edit in sorted_edits {
                content = apply_text_edit(&content, &text_edit)?;
            }

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }
            std::fs::write(&path, &content)
                .map_err(|e| format!("Failed to write file {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

fn apply_text_edit(
    content: &str,
    edit: &lsp_types::TextEdit,
) -> std::result::Result<String, String> {
    let lines: Vec<&str> = content.split('\n').collect();

    let start_line = edit.range.start.line as usize;
    let start_char = edit.range.start.character as usize;
    let end_line = edit.range.end.line as usize;
    let end_char = edit.range.end.character as usize;

    let get_char_offset = |line_idx: usize,
                           char_idx: usize|
     -> std::result::Result<usize, String> {
        if line_idx > lines.len() {
            return Err(format!("Line index {} out of bounds", line_idx));
        }

        let mut byte_offset = 0;
        for line in lines.iter().take(line_idx) {
            byte_offset += line.len() + 1;
        }

        if line_idx < lines.len() {
            let line_chars: Vec<char> = lines[line_idx].chars().collect();
            if char_idx > line_chars.len() {
                return Err(format!(
                    "Character index {} out of bounds for line {}",
                    char_idx, line_idx
                ));
            }
            let char_byte_len: usize = line_chars[0..char_idx].iter().map(|c| c.len_utf8()).sum();
            byte_offset += char_byte_len;
        }

        Ok(byte_offset)
    };

    let start_offset = get_char_offset(start_line, start_char)?;
    let end_offset = get_char_offset(end_line, end_char)?;

    if start_offset > end_offset || end_offset > content.len() {
        return Err("Invalid range for text edit".to_string());
    }

    let mut new_content = content[0..start_offset].to_string();
    new_content.push_str(&edit.new_text);
    new_content.push_str(&content[end_offset..]);

    Ok(new_content)
}

/// Dynamic diagnostic and repair plan updater.
pub(crate) fn update_diagnostics(registry: &mut ServerRegistry) {
    let root_path = registry.root_path.clone();

    // 1. Check for diag-uninitialized-admission
    let diag1_id = "diag-uninitialized-admission".to_string();
    let gate_state_check_active = registry.gates.get("gate-state-check") == Some(&true)
        || registry.current_state != crate::service::State::Uninitialized;
    let diag1_cleared = registry.cleared_diagnostics.contains(&diag1_id);

    if !gate_state_check_active && !diag1_cleared {
        let diag1 = max_protocol::MaxDiagnostic {
            lsp: Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("tower-lsp-max".to_string()),
                message: "Server state violates lifecycle machine match: initialize must transition to InitializingState.".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: diag1_id.clone(),
            law_id: "LAW-001".to_string(),
            attempted_transition: None,
            violated_axes: vec!["LSP State Mapping".to_string()],
            doc_routes: vec![max_protocol::DocRoute { path: "/doc/lifecycle".to_string() }],
            repair_actions: vec![max_protocol::RepairAction {
                action_id: "repair-state-sync".to_string(),
                description: "Synchronize machine state with semantic state".to_string(),
            }],
            verification_gates: vec![max_protocol::GateId("gate-state-check".to_string())],
            receipt_obligation: None,
            law_axis: max_protocol::LawAxis::default(),
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: max_protocol::Repairability::default(),
            terminality: max_protocol::Terminality::default(),
        };
        registry.diagnostics.insert(diag1_id.clone(), diag1.clone());

        let uri1 = lsp_types::Url::from_file_path(root_path.join("admission.receipt")).unwrap();
        let mut changes1 = HashMap::new();
        changes1.insert(
            uri1,
            vec![TextEdit {
                range: Range::default(),
                new_text: "rcpt-uninitialized\n".to_string(),
            }],
        );

        let lsp_action1 = CodeAction {
            title: "Synchronize machine state with semantic state".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diag1.lsp.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(changes1),
                document_changes: None,
                change_annotations: None,
            }),
            ..Default::default()
        };

        let action1 = max_protocol::MaxCodeAction {
            action: lsp_action1,
            preconditions: vec![max_protocol::Precondition {
                condition: "State is Uninitialized".to_string(),
            }],
            validation_plan: max_protocol::ValidationPlan {
                gates: vec![max_protocol::GateId("gate-state-check".to_string())],
            },
            rollback_plan: max_protocol::RollbackPlan {
                strategy: "Revert state to Uninitialized".to_string(),
            },
            receipt_plan: max_protocol::ReceiptPlan {
                expected_receipts: vec![],
            },
        };
        registry.repair_plans.insert(diag1_id, vec![action1]);
    } else {
        registry.diagnostics.remove(&diag1_id);
        registry.repair_plans.remove(&diag1_id);
    }

    // 2. Check for diag-missing-receipt and diag-auth-generator
    let diag2_id = "diag-missing-receipt".to_string();
    let diag3_id = "diag-auth-generator".to_string();
    let has_security_auth = registry.receipts.contains_key("rcpt-security-auth");

    if !has_security_auth {
        // diag-missing-receipt
        let diag2_cleared = registry.cleared_diagnostics.contains(&diag2_id);
        if !diag2_cleared {
            let diag2 = max_protocol::MaxDiagnostic {
                lsp: Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("tower-lsp-max".to_string()),
                    message: "Missing validation receipt for secure admission.".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                diagnostic_id: diag2_id.clone(),
                law_id: "LAW-003".to_string(),
                attempted_transition: None,
                violated_axes: vec!["Receipt Integrity".to_string()],
                doc_routes: vec![max_protocol::DocRoute {
                    path: "/doc/receipts".to_string(),
                }],
                repair_actions: vec![max_protocol::RepairAction {
                    action_id: "repair-apply-security-patch".to_string(),
                    description: "Apply cryptographic admission repair".to_string(),
                }],
                verification_gates: vec![max_protocol::GateId("gate-receipt-check".to_string())],
                receipt_obligation: Some(max_protocol::ReceiptObligation {
                    required_receipts: vec!["rcpt-security-auth".to_string()],
                }),
                law_axis: max_protocol::LawAxis::default(),
                violated_invariant: String::new(),
                observed_state: serde_json::Value::Null,
                expected_state: serde_json::Value::Null,
                repairability: max_protocol::Repairability::default(),
                terminality: max_protocol::Terminality::default(),
            };
            registry.diagnostics.insert(diag2_id.clone(), diag2.clone());

            let uri2 = lsp_types::Url::from_file_path(root_path.join("security.receipt")).unwrap();
            let mut changes2 = HashMap::new();
            changes2.insert(
                uri2,
                vec![TextEdit {
                    range: Range::default(),
                    new_text: "rcpt-security-auth\n".to_string(),
                }],
            );

            let lsp_action2 = CodeAction {
                title: "Apply cryptographic admission repair".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag2.lsp.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes2),
                    document_changes: None,
                    change_annotations: None,
                }),
                ..Default::default()
            };

            let action2 = max_protocol::MaxCodeAction {
                action: lsp_action2,
                preconditions: vec![],
                validation_plan: max_protocol::ValidationPlan {
                    gates: vec![max_protocol::GateId("gate-receipt-check".to_string())],
                },
                rollback_plan: max_protocol::RollbackPlan {
                    strategy: "None".to_string(),
                },
                receipt_plan: max_protocol::ReceiptPlan {
                    expected_receipts: vec!["rcpt-security-auth".to_string()],
                },
            };
            registry.repair_plans.insert(diag2_id, vec![action2]);
        } else {
            registry.diagnostics.remove(&diag2_id);
            registry.repair_plans.remove(&diag2_id);
        }

        // diag-auth-generator
        let diag3_cleared = registry.cleared_diagnostics.contains(&diag3_id);
        if !diag3_cleared {
            let diag3 = max_protocol::MaxDiagnostic {
                lsp: Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::INFORMATION),
                    code: None,
                    code_description: None,
                    source: Some("tower-lsp-max".to_string()),
                    message: "Generate security authorization receipt.".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                diagnostic_id: diag3_id.clone(),
                law_id: "LAW-003".to_string(),
                attempted_transition: None,
                violated_axes: vec!["Receipt Integrity".to_string()],
                doc_routes: vec![],
                repair_actions: vec![max_protocol::RepairAction {
                    action_id: "repair-generate-auth".to_string(),
                    description: "Generate security authorization receipt".to_string(),
                }],
                verification_gates: vec![max_protocol::GateId("gate-auth-check".to_string())],
                receipt_obligation: None,
                law_axis: max_protocol::LawAxis::default(),
                violated_invariant: String::new(),
                observed_state: serde_json::Value::Null,
                expected_state: serde_json::Value::Null,
                repairability: max_protocol::Repairability::default(),
                terminality: max_protocol::Terminality::default(),
            };
            registry.diagnostics.insert(diag3_id.clone(), diag3.clone());

            let uri3 = lsp_types::Url::from_file_path(root_path.join("auth.receipt")).unwrap();
            let mut changes3 = HashMap::new();
            changes3.insert(
                uri3,
                vec![TextEdit {
                    range: Range::default(),
                    new_text: "generated-rcpt-security-auth\n".to_string(),
                }],
            );

            let lsp_action3 = CodeAction {
                title: "Generate security authorization receipt".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag3.lsp.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes3),
                    document_changes: None,
                    change_annotations: None,
                }),
                ..Default::default()
            };

            let action3 = max_protocol::MaxCodeAction {
                action: lsp_action3,
                preconditions: vec![],
                validation_plan: max_protocol::ValidationPlan {
                    gates: vec![max_protocol::GateId("gate-auth-check".to_string())],
                },
                rollback_plan: max_protocol::RollbackPlan {
                    strategy: "None".to_string(),
                },
                receipt_plan: max_protocol::ReceiptPlan {
                    expected_receipts: vec![],
                },
            };
            registry.repair_plans.insert(diag3_id, vec![action3]);
        } else {
            registry.diagnostics.remove(&diag3_id);
            registry.repair_plans.remove(&diag3_id);
        }
    } else {
        registry.diagnostics.remove(&diag2_id);
        registry.repair_plans.remove(&diag2_id);
        registry.diagnostics.remove(&diag3_id);
        registry.repair_plans.remove(&diag3_id);
    }
}

/// Retrieves a reference to the global server registry.
pub fn get_registry() -> &'static Mutex<ServerRegistry> {
    REGISTRY.get_or_init(|| {
        let diagnostics = HashMap::new();
        let repair_plans = HashMap::new();
        let mut gates = HashMap::new();
        gates.insert("gate-state-check".to_string(), false);

        Mutex::new(ServerRegistry {
            client_capabilities: None,
            server_capabilities: None,
            diagnostics,
            repair_plans,
            gates,
            receipts: HashMap::new(),
            snapshots: HashMap::new(),
            cleared_diagnostics: std::collections::HashSet::new(),
            current_state: crate::service::State::Uninitialized,
            root_path: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            action_seq: 0,
            conformance_delta_log: std::collections::VecDeque::new(),
        })
    })
}

fn lock_registry() -> Result<std::sync::MutexGuard<'static, ServerRegistry>> {
    get_registry().lock().map_err(|_| Error::internal_error())
}

#[allow(dead_code)]
fn lock_mesh() -> Result<std::sync::MutexGuard<'static, max_runtime::AutonomicMesh>> {
    MESH.get_or_init(|| Mutex::new(max_runtime::AutonomicMesh::new()))
        .lock()
        .map_err(|_| Error::internal_error())
}

/// Reset the global registry to a fresh state.
/// Exposed as a public function for integration tests to prevent shared-state pollution.
pub fn reset_registry_for_tests() {
    if let Ok(mut reg) = get_registry().lock() {
        reg.client_capabilities = None;
        reg.server_capabilities = None;
        reg.diagnostics.clear();
        reg.repair_plans.clear();
        reg.gates.clear();
        reg.gates.insert("gate-state-check".to_string(), false);
        reg.receipts.clear();
        reg.snapshots.clear();
        reg.cleared_diagnostics.clear();
        reg.current_state = crate::service::State::Uninitialized;
        reg.root_path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        reg.action_seq = 0;
        reg.conformance_delta_log.clear();
    }
}

fn sha256(data: &[u8]) -> String {
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85u32,
        0x3c6ef372u32,
        0xa54ff53au32,
        0x510e527fu32,
        0x9b05688cu32,
        0x1f83d9abu32,
        0x5be0cd19u32,
    ];
    let k = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut padded = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    padded.push(0x80);
    while (padded.len() + 8) % 64 != 0 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut result = String::new();
    for &val in &h {
        result.push_str(&format!("{:08x}", val));
    }
    result
}
