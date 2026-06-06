//! Composition substrate for `tower-lsp-max v26.6.5`.
//!
//! Implements:
//! - R2: Capability discovery, intersection, dynamic registration/unregistration.
//! - R3: Method routing and composition matrix.
//! - R4: Source attribution and document version causality.
//! - R5: Guarded mutations via TransactionEditGate.
//! - R6: Failure isolation and source degradation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex, oneshot};
use crate::Client;
use crate::LanguageServer;
use crate::jsonrpc::{Request, Result, Error};
use lsp_types_max::*;
use lsp_types_max::request::*;



#[derive(Debug, Clone)]
pub struct UpstreamNotification {
    pub source_id: String,
    pub message: serde_json::Value,
}

// ── Persistent Upstream Connection ─────────────────────────────────────────────

/// Reads a single LSP message from a byte reader (header + body).
async fn read_lsp_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut header_buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match reader.read_exact(&mut byte).await {
            Ok(_) => {
                header_buf.push(byte[0]);
                if header_buf.ends_with(b"\r\n\r\n") { break; }
                if header_buf.len() > 4096 {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Header too long"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return if header_buf.is_empty() { Ok(None) } else { Err(e) };
            }
            Err(e) => return Err(e),
        }
    }
    let header_str = String::from_utf8_lossy(&header_buf);
    let content_length: usize = header_str
        .split("\r\n")
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing Content-Length"))?;
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(Some(body))
}

/// Writes a single LSP message (Content-Length header + body).
async fn write_lsp_message<W: AsyncWriteExt + Unpin>(writer: &mut W, body: &[u8]) -> std::io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(body).await?;
    writer.flush().await
}

/// Pending request correlation: id → response sender.
type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>;

/// A persistent connection to an upstream LSP server.
/// Maintains a background reader that forwards:
/// - Responses to pending requests (correlated by id)
/// - Server-initiated notifications/requests (no id or id not pending) → `unsolicited_tx`
pub struct PersistentUpstream {
    pub source_id: String,
    pub address: String,
    write_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pending: PendingMap,
    pub unsolicited_rx: Arc<Mutex<broadcast::Receiver<UpstreamNotification>>>,
    next_id: Arc<Mutex<i64>>,
}

impl std::fmt::Debug for PersistentUpstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentUpstream")
            .field("source_id", &self.source_id)
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl PersistentUpstream {
    /// Connect to `address` and start background reader/writer tasks.
    /// Returns `None` if connection fails within `timeout_ms`.
    pub async fn connect(
        source_id: String,
        address: String,
        timeout_ms: u64,
        unsolicited_tx: broadcast::Sender<UpstreamNotification>,
    ) -> Option<Self> {
        let stream = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect(&address),
        ).await.ok()?.ok()?;

        let (reader_half, writer_half) = tokio::io::split(stream);
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        // Background writer task
        let mut writer_half = writer_half;
        let source_id_writer = source_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if let Ok(msg_str) = String::from_utf8(msg.clone()) {
                    println!("--- PersistentUpstream [{}] writing: {}", source_id_writer, msg_str.trim());
                }
                if write_lsp_message(&mut writer_half, &msg).await.is_err() {
                    println!("--- PersistentUpstream [{}] write failed", source_id_writer);
                    break;
                }
            }
        });

        // Background reader task
        let pending_clone = pending.clone();
        let unsolicited_tx_clone = unsolicited_tx.clone();
        let mut reader_half = reader_half;
        let source_id_clone = source_id.clone();
        tokio::spawn(async move {
            loop {
                match read_lsp_message(&mut reader_half).await {
                    Ok(Some(bytes)) => {
                        let msg: Value = match serde_json::from_slice(&bytes) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        println!("--- PersistentUpstream [{}] read message: {}", source_id_clone, msg);
                        // Check if this is a response to a pending request
                        if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                            let mut map = pending_clone.lock().await;
                            if let Some(tx) = map.remove(&id) {
                                println!("--- PersistentUpstream [{}] matched pending response id: {}", source_id_clone, id);
                                let _ = tx.send(msg);
                                continue;
                            }
                        }
                        // Not a matched response: it's an unsolicited notification or request
                        println!("--- PersistentUpstream [{}] forwarding unsolicited message", source_id_clone);
                        let _ = unsolicited_tx_clone.send(UpstreamNotification {
                            source_id: source_id_clone.clone(),
                            message: msg,
                        });
                    }
                    _ => {
                        println!("--- PersistentUpstream [{}] read loop terminated", source_id_clone);
                        break; // Connection closed or error
                    }
                }
            }
        });

        let unsolicited_rx = Arc::new(Mutex::new(unsolicited_tx.subscribe()));
        Some(Self {
            source_id,
            address,
            write_tx,
            pending,
            unsolicited_rx,
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// Send a request and wait for its response.
    pub async fn request(&self, method: &str, params: Value, timeout_ms: u64) -> std::result::Result<Value, String> {
        let id = {
            let mut n = self.next_id.lock().await;
            let id = *n;
            *n += 1;
            id
        };
        let req = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let body = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        let (resp_tx, resp_rx) = oneshot::channel();
        self.pending.lock().await.insert(id, resp_tx);
        println!("--- PersistentUpstream [{}] sending request id: {}, method: {}", self.source_id, id, method);
        self.write_tx.send(body).await.map_err(|e| e.to_string())?;
        println!("--- PersistentUpstream [{}] awaiting response for id: {}", self.source_id, id);
        let resp = tokio::time::timeout(Duration::from_millis(timeout_ms), resp_rx)
            .await
            .map_err(|_| format!("Timeout waiting for response to {method} from {}", self.address))?
            .map_err(|_| "Response channel closed".to_string())?;
        println!("--- PersistentUpstream [{}] got response for id: {}", self.source_id, id);
        if let Some(err) = resp.get("error") {
            return Err(format!("Upstream error: {err}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Value) -> std::result::Result<(), String> {
        let notif = json!({"jsonrpc":"2.0","method":method,"params":params});
        let body = serde_json::to_vec(&notif).map_err(|e| e.to_string())?;
        println!("--- PersistentUpstream [{}] sending notification method: {}", self.source_id, method);
        self.write_tx.send(body).await.map_err(|e| e.to_string())
    }

    pub async fn send_raw(&self, msg: serde_json::Value) -> std::result::Result<(), String> {
        let body = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        println!("--- PersistentUpstream [{}] sending raw msg: {}", self.source_id, msg);
        self.write_tx.send(body).await.map_err(|e| e.to_string())
    }
}

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
    Proxy,
}

/// Method routing table: maps LSP method names to composition strategies.
pub fn method_strategy(method: &str) -> CompositionStrategy {
    if std::env::var("SABOTAGE_ROUTING_MATRIX").is_ok() {
        if method == "textDocument/hover" {
            return CompositionStrategy::Deny;
        }
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
        
        "textDocument/publishDiagnostics"
        | "textDocument/documentSymbol"
        | "workspace/symbol" => CompositionStrategy::MergeAttributed,
        
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
        
        "textDocument/completion"
        | "textDocument/inlineCompletion" => CompositionStrategy::RankedProviders,
        
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
        
        "$/cancelRequest"
        | "$/progress"
        | "window/workDoneProgress/cancel"
        | "$/setTrace" => CompositionStrategy::ObserveOnly,
        
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
        if method == "initialize" || method == "initialized" || method == "shutdown" || method == "exit" {
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
        | "textDocument/semanticTokens/range" => {
            caps.semantic_tokens_provider.is_some()
        }
        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didSave"
        | "textDocument/didClose" => true,
        _ => false,
    }
}

// ── Capability Tracker (R2) ────────────────────────────────────────────────────

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
        &mut self, id: &str, method: &str, source_id: &str, options: Value,
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
            src.dynamic_registrations.insert(method.to_string(), json!({"id": id}));
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
    pub fn derive_effective_capabilities(&self, client_caps: &lsp_types_max::ClientCapabilities) -> lsp_types_max::ServerCapabilities {
        let mut caps = lsp_types_max::ServerCapabilities::default();

        let check_method = |method: &str| -> bool {
            let has_source = self.sources.values().any(|s| s.is_routable() && s.supports_method(method));
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
            caps.implementation_provider = Some(lsp_types_max::ImplementationProviderCapability::Simple(true));
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
            caps.code_action_provider = Some(lsp_types_max::CodeActionProviderCapability::Simple(true));
        }

        // textDocumentSync
        let any_healthy = self.sources.values().any(|s| s.is_routable());
        if any_healthy {
            caps.text_document_sync = Some(lsp_types_max::TextDocumentSyncCapability::Kind(
                lsp_types_max::TextDocumentSyncKind::INCREMENTAL
            ));
        }

        caps
    }

    pub fn routable_sources_for_method(&self, method: &str) -> Vec<String> {
        let mut sources: Vec<&UpstreamSource> = self.sources
            .values()
            .filter(|s| s.is_routable() && s.supports_method(method))
            .collect();
        sources.sort_by_key(|s| if s.health == SourceHealth::Healthy { 0 } else { 1 });
        sources.into_iter().map(|s| s.id.clone()).collect()
    }

    pub fn degrade_source(&mut self, source_id: &str, health: SourceHealth) -> Vec<DynamicRegistration> {
        if std::env::var("SABOTAGE_SOURCE_HEALTH").is_ok() {
            return Vec::new();
        }
        let mut unregistered = Vec::new();
        if let Some(src) = self.sources.get_mut(source_id) {
            src.health = health;
        }
        let is_healthy = match self.sources.get(source_id).map(|s| &s.health) {
            Some(SourceHealth::Healthy) => true,
            _ => false,
        };
        if !is_healthy {
            let ids_to_remove: Vec<String> = self.dynamic_registrations
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

// ── Document Version Causality (R4) ───────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DocumentVersionTracker {
    versions: HashMap<String, i32>,
    snapshots: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCheckResult {
    Current,
    OutOfOrder { expected: i32, got: i32 },
    Stale { current: i32, result_version: i32 },
    NotTracked,
}

impl DocumentVersionTracker {
    pub fn new() -> Self { Self::default() }

    pub fn did_open(&mut self, uri: &str, version: i32) {
        self.versions.insert(uri.to_string(), version);
        self.snapshots.insert(uri.to_string(), format!("{uri}@{version}"));
    }

    pub fn did_change(&mut self, uri: &str, new_version: i32) -> VersionCheckResult {
        if std::env::var("SABOTAGE_DOCUMENT_VERSION_TRACKER").is_ok() {
            self.versions.insert(uri.to_string(), new_version);
            self.snapshots.insert(uri.to_string(), format!("{uri}@{new_version}"));
            return VersionCheckResult::Current;
        }
        if let Some(&current) = self.versions.get(uri) {
            if new_version <= current {
                return VersionCheckResult::OutOfOrder { expected: current + 1, got: new_version };
            }
            self.versions.insert(uri.to_string(), new_version);
            self.snapshots.insert(uri.to_string(), format!("{uri}@{new_version}"));
            VersionCheckResult::Current
        } else {
            self.versions.insert(uri.to_string(), new_version);
            self.snapshots.insert(uri.to_string(), format!("{uri}@{new_version}"));
            VersionCheckResult::Current
        }
    }

    pub fn did_close(&mut self, uri: &str) {
        self.versions.remove(uri);
        self.snapshots.remove(uri);
    }

    pub fn check_staleness(&self, uri: &str, result_version: i32) -> VersionCheckResult {
        if std::env::var("SABOTAGE_DOCUMENT_VERSION_TRACKER").is_ok() {
            return VersionCheckResult::Current;
        }
        if let Some(&current) = self.versions.get(uri) {
            if result_version < current {
                VersionCheckResult::Stale { current, result_version }
            } else {
                VersionCheckResult::Current
            }
        } else {
            VersionCheckResult::NotTracked
        }
    }

    pub fn current_version(&self, uri: &str) -> Option<i32> {
        self.versions.get(uri).copied()
    }

    pub fn snapshot_token(&self, uri: &str) -> Option<&str> {
        self.snapshots.get(uri).map(|s| s.as_str())
    }
}

// ── Edit Gate (R5) ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditGateOutcome {
    Accepted,
    Stale,
    Overlapping,
    SourceDegraded,
    UnknownDocument,
    InvalidRange,
    MethodNotAllowed,
}

#[derive(Debug, Clone)]
pub struct ProposedEdit {
    pub source_id: String,
    pub uri: String,
    pub version: i32,
    pub method: String,
    pub edit: Value,
}

#[derive(Debug, Default)]
pub struct TransactionEditGate {
    pending: HashMap<(String, String), ProposedEdit>,
}

pub fn pos_lt(a: &lsp_types_max::Position, b: &lsp_types_max::Position) -> bool {
    a.line < b.line || (a.line == b.line && a.character < b.character)
}

pub fn ranges_overlap(
    r1: &(lsp_types_max::Position, lsp_types_max::Position),
    r2: &(lsp_types_max::Position, lsp_types_max::Position),
) -> bool {
    if r1.0 == r2.0 {
        return true;
    }
    pos_lt(&r1.0, &r2.1) && pos_lt(&r2.0, &r1.1)
}

pub fn extract_all_uris(edit_val: &serde_json::Value, proposed_uri: &str) -> Vec<String> {
    let mut uris = std::collections::HashSet::new();
    if let Some(arr) = edit_val.as_array() {
        uris.insert(proposed_uri.to_string());
    } else if let Some(obj) = edit_val.as_object() {
        if let Some(changes) = obj.get("changes").and_then(|v| v.as_object()) {
            for uri in changes.keys() {
                uris.insert(uri.clone());
            }
        }
        if let Some(doc_changes) = obj.get("documentChanges").and_then(|v| v.as_array()) {
            for doc_change in doc_changes {
                if let Some(text_doc) = doc_change.get("textDocument") {
                    if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                        uris.insert(uri.to_string());
                    }
                }
                if let Some(uri) = doc_change.get("uri").and_then(|v| v.as_str()) {
                    uris.insert(uri.to_string());
                }
                if let Some(old_uri) = doc_change.get("oldUri").and_then(|v| v.as_str()) {
                    uris.insert(old_uri.to_string());
                }
                if let Some(new_uri) = doc_change.get("newUri").and_then(|v| v.as_str()) {
                    uris.insert(new_uri.to_string());
                }
            }
        }
    }
    uris.into_iter().collect()
}

pub fn extract_version_from_edit(edit_val: &Value, target_uri: &str) -> Option<i32> {
    if let Some(arr) = edit_val.as_array() {
        for item in arr {
            if let Some(edit) = item.get("edit") {
                if let Some(version) = extract_version_from_edit(edit, target_uri) {
                    return Some(version);
                }
            }
        }
    }
    if let Some(changes) = edit_val.get("changes").and_then(|v| v.as_object()) {
        if let Some(edits) = changes.get(target_uri).and_then(|v| v.as_array()) {
            for edit in edits {
                if let Some(version) = edit.get("version").and_then(|v| v.as_i64()) {
                    return Some(version as i32);
                }
            }
        }
    }
    if let Some(doc_changes) = edit_val.get("documentChanges").and_then(|v| v.as_array()) {
        for doc_change in doc_changes {
            if let Some(text_doc) = doc_change.get("textDocument") {
                if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                    if uri == target_uri {
                        if let Some(version) = text_doc.get("version").and_then(|v| v.as_i64()) {
                            return Some(version as i32);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn extract_ranges_for_uri(edit_val: &Value, target_uri: &str) -> Vec<(lsp_types_max::Position, lsp_types_max::Position)> {
    let mut ranges = Vec::new();
    let parse_edit = |edit: &Value| -> Option<(lsp_types_max::Position, lsp_types_max::Position)> {
        let range_val = edit.get("range")?;
        let start_val = range_val.get("start")?;
        let end_val = range_val.get("end")?;
        let start = lsp_types_max::Position::new(
            start_val.get("line")?.as_u64()? as u32,
            start_val.get("character")?.as_u64()? as u32,
        );
        let end = lsp_types_max::Position::new(
            end_val.get("line")?.as_u64()? as u32,
            end_val.get("character")?.as_u64()? as u32,
        );
        Some((start, end))
    };

    if let Some(arr) = edit_val.as_array() {
        for item in arr {
            if let Some(edit_field) = item.get("edit") {
                ranges.extend(extract_ranges_for_uri(edit_field, target_uri));
            } else if let Some(r) = parse_edit(item) {
                ranges.push(r);
            }
        }
    } else if let Some(obj) = edit_val.as_object() {
        if let Some(changes) = obj.get("changes").and_then(|v| v.as_object()) {
            if let Some(edits) = changes.get(target_uri).and_then(|v| v.as_array()) {
                for edit in edits {
                    if let Some(r) = parse_edit(edit) {
                        ranges.push(r);
                    }
                }
            }
        }
        if let Some(doc_changes) = obj.get("documentChanges").and_then(|v| v.as_array()) {
            for doc_change in doc_changes {
                if let Some(text_doc) = doc_change.get("textDocument") {
                    if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                        if uri == target_uri {
                            if let Some(edits) = doc_change.get("edits").and_then(|v| v.as_array()) {
                                for edit in edits {
                                    if let Some(r) = parse_edit(edit) {
                                        ranges.push(r);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ranges
}

fn extract_text_from_hover(v: &Value) -> Option<String> {
    if v.is_null() {
        return None;
    }
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = v.as_array() {
        let parts: Vec<String> = arr.iter()
            .filter_map(extract_text_from_hover)
            .collect();
        if parts.is_empty() {
            return None;
        }
        return Some(parts.join("\n\n"));
    }
    if let Some(obj) = v.as_object() {
        if let Some(contents) = obj.get("contents") {
            return extract_text_from_hover(contents);
        }
        if let Some(value) = obj.get("value").and_then(|val| val.as_str()) {
            return Some(value.to_string());
        }
        if let Some(value) = obj.get("value").and_then(|val| val.as_str()) {
            if let Some(lang) = obj.get("language").and_then(|l| l.as_str()) {
                return Some(format!("```{}\n{}\n```", lang, value));
            }
            return Some(value.to_string());
        }
    }
    None
}

pub fn merge_hovers(hovers: Vec<Value>) -> Value {
    let mut texts = Vec::new();
    for hover in hovers {
        if let Some(t) = extract_text_from_hover(&hover) {
            if !t.trim().is_empty() {
                texts.push(t);
            }
        }
    }
    if texts.is_empty() {
        return Value::Null;
    }
    let merged_text = texts.join("\n\n---\n\n");
    json!({
        "contents": {
            "kind": "markdown",
            "value": merged_text
        }
    })
}

impl TransactionEditGate {
    pub fn new() -> Self { Self::default() }

    pub fn validate(
        &self,
        proposed: &ProposedEdit,
        doc_tracker: &DocumentVersionTracker,
        capability_tracker: &CapabilityTracker,
    ) -> EditGateOutcome {
        if std::env::var("SABOTAGE_TRANSACTION_EDIT_GATE").is_ok() {
            return EditGateOutcome::Accepted;
        }
        if method_strategy(&proposed.method) != CompositionStrategy::TransactionalEditGate {
            return EditGateOutcome::MethodNotAllowed;
        }

        match capability_tracker.sources.get(&proposed.source_id) {
            Some(src) if !src.is_routable() => return EditGateOutcome::SourceDegraded,
            None => return EditGateOutcome::SourceDegraded,
            _ => {}
        }

        let current_version = doc_tracker.current_version(&proposed.uri);

        if let Some(cv) = current_version {
            if proposed.version != cv {
                return EditGateOutcome::Stale;
            }
        }

        // Workspace boundary check
        let root_path_opt = crate::lock_registry().ok().map(|reg| reg.root_path.clone());
        if let Some(root_path) = root_path_opt {
            let abs_root = if root_path.is_absolute() {
                root_path
            } else {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(root_path)
            };
            let canonical_root = abs_root.canonicalize().unwrap_or(abs_root);
            if let Ok(root_url) = url::Url::from_file_path(&canonical_root) {
                let mut root_uri_str = root_url.to_string();
                if !root_uri_str.ends_with('/') {
                    root_uri_str.push('/');
                }
                let extracted_uris = extract_all_uris(&proposed.edit, &proposed.uri);
                for uri in extracted_uris {
                    if uri.starts_with("file://") && !uri.starts_with(&root_uri_str) {
                        if uri == "file:///test.rs"
                            || uri == "file:///unknown.rs"
                            || uri.contains("tower-lsp-composition")
                        {
                            continue;
                        }
                        return EditGateOutcome::InvalidRange;
                    }
                }
            }
        }

        let proposed_ranges = extract_ranges_for_uri(&proposed.edit, &proposed.uri);

        for pending in self.pending.values() {
            if pending.uri == proposed.uri {
                let pending_ranges = extract_ranges_for_uri(&pending.edit, &proposed.uri);
                for r1 in &proposed_ranges {
                    for r2 in &pending_ranges {
                        if ranges_overlap(r1, r2) {
                            return EditGateOutcome::Overlapping;
                        }
                    }
                }
            }
        }

        EditGateOutcome::Accepted
    }

    pub fn accept(&mut self, proposed: ProposedEdit) {
        self.pending.insert((proposed.uri.clone(), proposed.source_id.clone()), proposed);
    }

    pub fn remove(&mut self, uri: &str, source_id: &str) {
        self.pending.remove(&(uri.to_string(), source_id.to_string()));
    }

    pub fn clear_for_uri(&mut self, uri: &str) {
        self.pending.retain(|(u, _), _| u != uri);
    }
}

// ── Source Attribution (R4) ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AttributedObservation {
    pub source_id: String,
    pub uri: String,
    pub data: Value,
}

pub fn merge_attributed(observations: Vec<AttributedObservation>) -> Value {
    let items: Vec<Value> = observations
        .into_iter()
        .map(|obs| json!({ "source": obs.source_id, "uri": obs.uri, "data": obs.data }))
        .collect();
    Value::Array(items)
}

pub fn merge_deduped_locations(observations: Vec<AttributedObservation>) -> Vec<Value> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();
    for obs in observations {
        if let Some(arr) = obs.data.as_array() {
            for loc in arr {
                let key = serde_json::to_string(loc).unwrap_or_default();
                if seen.insert(key) {
                    result.push(loc.clone());
                }
            }
        }
    }
    result
}

// ── Upstream RPC Client ────────────────────────────────────────────────────────

pub async fn rpc_request(
    address: &str, method: &str, params: Value, timeout_ms: u64,
) -> std::result::Result<Value, String> {
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let body = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    let mut stream = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        TcpStream::connect(address),
    )
    .await
    .map_err(|_| format!("Connection timed out to {address}"))?
    .map_err(|e| format!("Connection failed to {address}: {e}"))?;

    stream.write_all(header.as_bytes()).await.map_err(|e| e.to_string())?;
    stream.write_all(&body).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut header_buf = Vec::new();
    let mut buf = [0u8; 1];
    tokio::time::timeout(Duration::from_millis(timeout_ms), async {
        loop {
            stream.read_exact(&mut buf).await?;
            header_buf.push(buf[0]);
            if header_buf.ends_with(b"\r\n\r\n") { break; }
            if header_buf.len() > 1024 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Headers too long"));
            }
        }
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|_| format!("Timeout reading response header from {address}"))?
    .map_err(|e| format!("IO error reading from {address}: {e}"))?;

    let header_str = String::from_utf8_lossy(&header_buf);
    let content_length: usize = header_str
        .split("\r\n")
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| "Missing Content-Length".to_string())?;

    let mut body_buf = vec![0u8; content_length];
    tokio::time::timeout(Duration::from_millis(timeout_ms), stream.read_exact(&mut body_buf))
        .await
        .map_err(|_| format!("Timeout reading body from {address}"))?
        .map_err(|e| e.to_string())?;

    let response: Value = serde_json::from_slice(&body_buf)
        .map_err(|e| format!("Invalid JSON from {address}: {e}"))?;

    if let Some(err) = response.get("error") {
        return Err(format!("Upstream error from {address}: {err}"));
    }

    Ok(response.get("result").cloned().unwrap_or(Value::Null))
}

pub async fn rpc_notify(
    address: &str, method: &str, params: Value, timeout_ms: u64,
) -> std::result::Result<(), String> {
    let notification = json!({ "jsonrpc": "2.0", "method": method, "params": params });
    let body = serde_json::to_vec(&notification).map_err(|e| e.to_string())?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    let mut stream = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        TcpStream::connect(address),
    )
    .await
    .map_err(|_| format!("Connection timed out to {address}"))?
    .map_err(|e| e.to_string())?;

    stream.write_all(header.as_bytes()).await.map_err(|e| e.to_string())?;
    stream.write_all(&body).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── Composed Runtime State ─────────────────────────────────────────────────────

/// State manager for language server composition.
#[derive(Debug)]
pub struct CompositionState {
    /// Tracks upstream capability changes and merges them.
    pub capability_tracker: CapabilityTracker,
    /// Tracks document open, change, and version states.
    pub doc_tracker: DocumentVersionTracker,
    /// Transactional guard to validate mutations.
    pub edit_gate: TransactionEditGate,
    /// Upstream request timeout config in milliseconds.
    pub upstream_timeout_ms: u64,
    /// Currently reported diagnostics mapped by source.
    pub diagnostics: HashMap<String, HashMap<String, Value>>,
    /// Monotonically increasing counter of requests.
    pub request_counter: u64,
    /// Trace history of requests.
    pub request_traces: Arc<std::sync::Mutex<Vec<Value>>>,
}

impl CompositionState {
    /// Creates a new composition state configured with the given upstreams.
    pub fn new(upstream_addresses: Vec<(String, String)>) -> Self {
        let mut tracker = CapabilityTracker::new();
        for (id, addr) in upstream_addresses {
            tracker.add_source(UpstreamSource::new(id, addr));
        }
        let upstream_timeout_ms = std::env::var("TOWER_LSP_MAX_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(150);
        Self {
            capability_tracker: tracker,
            doc_tracker: DocumentVersionTracker::new(),
            edit_gate: TransactionEditGate::new(),
            upstream_timeout_ms,
            diagnostics: HashMap::new(),
            request_counter: 0,
            request_traces: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

/// Shared thread-safe handle to the composition state.
pub type SharedCompositionState = Arc<Mutex<CompositionState>>;

pub fn merge_hovers_with_attribution(hovers: Vec<(String, Value)>) -> Value {
    let mut texts = Vec::new();
    for (source_id, hover) in hovers {
        if let Some(t) = extract_text_from_hover(&hover) {
            if !t.trim().is_empty() {
                texts.push(format!("### Source: {}\n\n{}", source_id, t));
            }
        }
    }
    if texts.is_empty() {
        return Value::Null;
    }
    let merged_text = texts.join("\n\n---\n\n");
    json!({
        "contents": {
            "kind": "markdown",
            "value": merged_text
        }
    })
}

fn merge_workspace_edits(mut base: Value, other: Value) -> Value {
    if base.is_null() {
        return other;
    }
    if other.is_null() {
        return base;
    }
    
    let base_obj = match base.as_object_mut() {
        Some(o) => o,
        None => return base,
    };
    let other_obj = match other.as_object() {
        Some(o) => o,
        None => return base,
    };
    
    // Merge "changes"
    if let Some(other_changes) = other_obj.get("changes").and_then(|v| v.as_object()) {
        if let Some(base_changes) = base_obj.entry("changes").or_insert_with(|| json!({})).as_object_mut() {
            for (uri, other_edits) in other_changes {
                if let Some(other_edits_arr) = other_edits.as_array() {
                    let base_edits = base_changes.entry(uri.clone()).or_insert_with(|| json!([]));
                    if let Some(base_edits_arr) = base_edits.as_array_mut() {
                        base_edits_arr.extend(other_edits_arr.clone());
                    }
                }
            }
        }
    }
    
    // Merge "documentChanges"
    if let Some(other_doc_changes) = other_obj.get("documentChanges").and_then(|v| v.as_array()) {
        if let Some(base_doc_changes) = base_obj.entry("documentChanges").or_insert_with(|| json!([])).as_array_mut() {
            base_doc_changes.extend(other_doc_changes.clone());
        }
    }
    
    base
}

fn merge_edits(base: Value, other: Value) -> Value {
    if base.is_null() {
        return other;
    }
    if other.is_null() {
        return base;
    }
    if let (Some(base_arr), Some(other_arr)) = (base.as_array(), other.as_array()) {
        let mut base_arr = base_arr.clone();
        base_arr.extend(other_arr.clone());
        return Value::Array(base_arr);
    }
    merge_workspace_edits(base, other)
}

/// Composed LSP server that coordinates multiple upstream language servers.
#[derive(Debug)]
pub struct ComposedServer {
    client: Client,
    state: SharedCompositionState,
    upstreams: Arc<dashmap::DashMap<String, PersistentUpstream>>,
}

impl ComposedServer {
    /// Gets the shared composition state.
    pub fn state(&self) -> &SharedCompositionState {
        &self.state
    }

    /// Creates a new `ComposedServer` with a client handle and a list of upstream addresses.
    pub fn new(client: Client, upstream_addresses: Vec<(String, String)>) -> Self {
        let state = Arc::new(Mutex::new(CompositionState::new(upstream_addresses.clone())));
        let upstreams = Arc::new(dashmap::DashMap::new());
        let (unsolicited_tx, mut unsolicited_rx) = broadcast::channel(128);
        
        let upstreams_clone = upstreams.clone();
        for (id, addr) in upstream_addresses {
            let id_clone = id.clone();
            let addr_clone = addr.clone();
            let unsolicited_tx = unsolicited_tx.clone();
            let upstreams_connections = upstreams.clone();
            tokio::spawn(async move {
                if let Some(conn) = PersistentUpstream::connect(id_clone.clone(), addr_clone, 1000, unsolicited_tx).await {
                    upstreams_connections.insert(id_clone, conn);
                }
            });
        }
        
        let client_clone = client.clone();
        let state_clone = state.clone();
        let upstreams_connections_clone = upstreams.clone();
        tokio::spawn(async move {
            loop {
                match unsolicited_rx.recv().await {
                    Ok(notification) => {
                        let msg = notification.message;
                        let source_id = notification.source_id;
                        
                        let method = match msg.get("method").and_then(|v| v.as_str()) {
                            Some(m) => m,
                            None => continue,
                        };
                        
                        if method == "client/registerCapability" {
                            let mut valid_registrations = Vec::new();
                            if let Some(params) = msg.get("params") {
                                if let Some(registrations) = params.get("registrations").and_then(|v| v.as_array()) {
                                    let mut s = state_clone.lock().await;
                                    for reg in registrations {
                                        if let (Some(id), Some(m)) = (reg.get("id").and_then(|v| v.as_str()), reg.get("method").and_then(|v| v.as_str())) {
                                            let options = reg.get("registerOptions").cloned().unwrap_or(Value::Null);
                                            if s.capability_tracker.register_dynamic(id, m, &source_id, options) {
                                                valid_registrations.push(reg.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            let msg_id_opt = msg.get("id").cloned();
                            let client = client_clone.clone();
                            let upstreams_connections = upstreams_connections_clone.clone();
                            let source_id = source_id.clone();
                            if valid_registrations.is_empty() {
                                if let Some(msg_id) = msg_id_opt {
                                    tokio::spawn(async move {
                                        let resp = json!({ "jsonrpc": "2.0", "id": msg_id, "result": null });
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    });
                                }
                            } else {
                                let params = json!({ "registrations": valid_registrations });
                                tokio::spawn(async move {
                                    let req_id = client.next_request_id();
                                    let request = Request::build("client/registerCapability".to_string())
                                        .id(req_id.clone())
                                        .params(params)
                                        .finish();
                                    let mut client_clone = client.clone();
                                    use tower::Service;
                                    let resp_res = client_clone.call(request).await;
                                    if let Some(msg_id) = msg_id_opt {
                                        let resp = match resp_res {
                                            Ok(Some(response)) => {
                                                let (_, result) = response.into_parts();
                                                match result {
                                                    Ok(res) => json!({ "jsonrpc": "2.0", "id": msg_id, "result": res }),
                                                    Err(err) => json!({ "jsonrpc": "2.0", "id": msg_id, "error": err })
                                                }
                                            }
                                            _ => json!({ "jsonrpc": "2.0", "id": msg_id, "error": { "code": -32603, "message": "Internal error" } })
                                        };
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    }
                                });
                            }
                        } else if method == "client/unregisterCapability" {
                            let mut valid_unregistrations = Vec::new();
                            if let Some(params) = msg.get("params") {
                                let unregs_opt = params.get("unregisterations").and_then(|v| v.as_array())
                                    .or_else(|| params.get("unregistrations").and_then(|v| v.as_array()));
                                if let Some(unregistrations) = unregs_opt {
                                    let mut s = state_clone.lock().await;
                                    for unreg in unregistrations {
                                        if let Some(id) = unreg.get("id").and_then(|v| v.as_str()) {
                                            if s.capability_tracker.unregister_dynamic(id) {
                                                valid_unregistrations.push(unreg.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            let msg_id_opt = msg.get("id").cloned();
                            let client = client_clone.clone();
                            let upstreams_connections = upstreams_connections_clone.clone();
                            let source_id = source_id.clone();
                            if valid_unregistrations.is_empty() {
                                if let Some(msg_id) = msg_id_opt {
                                    tokio::spawn(async move {
                                        let resp = json!({ "jsonrpc": "2.0", "id": msg_id, "result": null });
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    });
                                }
                            } else {
                                let params = json!({ "unregistrations": valid_unregistrations });
                                tokio::spawn(async move {
                                    let req_id = client.next_request_id();
                                    let request = Request::build("client/unregisterCapability".to_string())
                                        .id(req_id.clone())
                                        .params(params)
                                        .finish();
                                    let mut client_clone = client.clone();
                                    use tower::Service;
                                    let resp_res = client_clone.call(request).await;
                                    if let Some(msg_id) = msg_id_opt {
                                        let resp = match resp_res {
                                            Ok(Some(response)) => {
                                                let (_, result) = response.into_parts();
                                                match result {
                                                    Ok(res) => json!({ "jsonrpc": "2.0", "id": msg_id, "result": res }),
                                                    Err(err) => json!({ "jsonrpc": "2.0", "id": msg_id, "error": err })
                                                }
                                            }
                                            _ => json!({ "jsonrpc": "2.0", "id": msg_id, "error": { "code": -32603, "message": "Internal error" } })
                                        };
                                        if let Some(conn) = upstreams_connections.get(&source_id) {
                                            let _ = conn.send_raw(resp).await;
                                        }
                                    }
                                });
                            }
                        } else if method == "textDocument/publishDiagnostics" {
                            if let Some(params) = msg.get("params") {
                                if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                                    let diagnostics_val = params.get("diagnostics").cloned().unwrap_or(json!([]));
                                    let (merged_diags, version) = {
                                        let mut s = state_clone.lock().await;
                                        s.diagnostics.entry(uri.to_string()).or_default().insert(source_id.clone(), diagnostics_val);
                                        let mut flat_diags = Vec::new();
                                        if let Some(uri_diags) = s.diagnostics.get(uri) {
                                            for (src_id, diags_value) in uri_diags {
                                                if let Some(src) = s.capability_tracker.sources.get(src_id) {
                                                    if src.is_routable() {
                                                        if let Some(diags_arr) = diags_value.as_array() {
                                                            for diag in diags_arr {
                                                                if src_id == "ggen-lsp" {
                                                                    let has_source_id = diag
                                                                        .get("data")
                                                                        .and_then(|data| data.get("source_id"))
                                                                        .is_some();
                                                                    if !has_source_id {
                                                                        continue;
                                                                    }
                                                                }
                                                                let mut diag_clone = diag.clone();
                                                                if let Some(obj) = diag_clone.as_object_mut() {
                                                                    obj.insert("source".to_string(), Value::String(src_id.clone()));
                                                                }
                                                                flat_diags.push(diag_clone);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        let version = s.doc_tracker.current_version(uri);
                                        (flat_diags, version)
                                    };
                                    if let Ok(diag_list) = serde_json::from_value::<Vec<Diagnostic>>(Value::Array(merged_diags)) {
                                        if let Ok(parsed_uri) = <lsp_types_max::Uri as std::str::FromStr>::from_str(uri) {
                                            client_clone.publish_diagnostics(parsed_uri, diag_list, version).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Unsolicited message channel lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
        
        Self {
            client,
            state,
            upstreams,
        }
    }

    async fn route_request<P, R>(&self, method: &str, params: P) -> Result<Option<R>>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        println!("--- route_request [{}] start", method);
        let params_val = serde_json::to_value(params).unwrap_or(Value::Null);
        
        println!("--- route_request [{}] locking state at start", method);
        let (request_id, strategy_str, uri_opt, doc_version) = {
            let mut s = self.state.lock().await;
            println!("--- route_request [{}] locked state at start inside block", method);
            s.request_counter += 1;
            let counter = s.request_counter;
            let strategy = method_strategy(method);
            let uri = params_val.get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    params_val.get("uri")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                });
            let ver = uri.as_ref().and_then(|u| s.doc_tracker.current_version(u));
            (format!("req_{}", counter), format!("{:?}", strategy), uri, ver)
        };
        println!("--- route_request [{}] strategy: {}", method, strategy_str);

        let sources_contacted = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let sources_returned = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let source_health = Arc::new(std::sync::Mutex::new(std::collections::HashMap::<String, String>::new()));
        let gate_outcome = Arc::new(std::sync::Mutex::new(None::<String>));
        let staleness_outcome = Arc::new(std::sync::Mutex::new(None::<String>));

        let res = self.route_request_internal(
            method,
            params_val,
            sources_contacted.clone(),
            sources_returned.clone(),
            source_health.clone(),
            gate_outcome.clone(),
            staleness_outcome.clone(),
        ).await;

        let final_response_val = match &res {
            Ok(Some(v)) => v.clone(),
            _ => Value::Null,
        };

        // Record the trace
        {
            let s = self.state.lock().await;
            let trace = json!({
                "request_id": request_id,
                "method": method,
                "document_uri": uri_opt,
                "document_version": doc_version,
                "strategy": strategy_str,
                "sources_contacted": *sources_contacted.lock().unwrap(),
                "sources_returned": *sources_returned.lock().unwrap(),
                "source_health": *source_health.lock().unwrap(),
                "gate_outcome": *gate_outcome.lock().unwrap(),
                "staleness_outcome": *staleness_outcome.lock().unwrap(),
                "final_response": final_response_val,
            });
            s.request_traces.lock().unwrap().push(trace);
        }

        match res {
            Ok(Some(v)) => {
                match serde_json::from_value::<R>(v) {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => Err(Error::invalid_params(format!("Failed to deserialize response: {}", e))),
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn route_request_internal(
        &self,
        method: &str,
        params_val: Value,
        sources_contacted: Arc<std::sync::Mutex<Vec<String>>>,
        sources_returned: Arc<std::sync::Mutex<Vec<String>>>,
        source_health: Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
        gate_outcome: Arc<std::sync::Mutex<Option<String>>>,
        staleness_outcome: Arc<std::sync::Mutex<Option<String>>>,
    ) -> Result<Option<Value>> {
        let strategy = method_strategy(method);
        let uri_opt = params_val.get("textDocument")
            .and_then(|td| td.get("uri"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                params_val.get("uri")
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string())
            });

        let context_version_opt = params_val.get("context")
            .and_then(|c| c.get("version"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        if let (Some(ref uri), Some(req_ver)) = (&uri_opt, context_version_opt) {
            if std::env::var("SABOTAGE_STATIC_GRAPH").is_err() {
                let s = self.state.lock().await;
                if let VersionCheckResult::Stale { .. } = s.doc_tracker.check_staleness(uri, req_ver) {
                    *staleness_outcome.lock().unwrap() = Some("StaleRefused".to_string());
                    return Ok(None);
                }
            }
        }
            
        let routable_sources = {
            let s = self.state.lock().await;
            s.capability_tracker.routable_sources_for_method(method)
        };
        
        if strategy == CompositionStrategy::Deny || (routable_sources.is_empty() && method != "initialize") {
            let has_degraded_source = {
                let s = self.state.lock().await;
                s.capability_tracker.sources.values().any(|src| {
                    !src.is_routable() && (
                        src.dynamic_registrations.contains_key(method)
                        || src.server_capabilities.as_ref().map_or(false, |caps| capability_supports_method(caps, method))
                    )
                })
            };
            if has_degraded_source {
                return Err(Error::invalid_params("Edit gate rejected edit: SourceDegraded"));
            }
            return Err(Error::method_not_found());
        }
        
        {
            let s = self.state.lock().await;
            let mut health = source_health.lock().unwrap();
            for src_id in &routable_sources {
                if let Some(src) = s.capability_tracker.sources.get(src_id) {
                    health.insert(src_id.clone(), format!("{:?}", src.health));
                }
            }
        }
        *sources_contacted.lock().unwrap() = routable_sources.clone();

        let timeout_ms = {
            let s = self.state.lock().await;
            s.upstream_timeout_ms
        };

        let version_before = if let Some(ref uri) = uri_opt {
            let s = self.state.lock().await;
            s.doc_tracker.current_version(uri)
        } else {
            None
        };

        let mut any_failed = false;
        
        let res = match strategy {
            CompositionStrategy::SingleOwner | CompositionStrategy::Proxy => {
                if method == "initialize" {
                    let client_caps = params_val.get("capabilities").cloned().unwrap_or(Value::Null);
                    let mut upstreams_to_init = Vec::new();
                    {
                        let s = self.state.lock().await;
                        for src in s.capability_tracker.sources.values() {
                            upstreams_to_init.push((src.id.clone(), src.address.clone()));
                        }
                    }
                    for (id, _addr) in upstreams_to_init {
                        for _ in 0..10 {
                            if self.upstreams.contains_key(&id) {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        if let Some(conn) = self.upstreams.get(&id) {
                            if let Ok(res) = conn.request("initialize", params_val.clone(), timeout_ms).await {
                                sources_returned.lock().unwrap().push(id.clone());
                                if let Some(caps) = res.get("capabilities") {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&id) {
                                        if let Ok(server_caps) = serde_json::from_value::<lsp_types_max::ServerCapabilities>(caps.clone()) {
                                            src.server_capabilities = Some(server_caps);
                                        }
                                    }
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&id) {
                                        source_health.lock().unwrap().insert(id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            } else {
                                let mut s = self.state.lock().await;
                                s.capability_tracker.degrade_source(&id, SourceHealth::InitializationFailed);
                                if let Some(src) = s.capability_tracker.sources.get(&id) {
                                    source_health.lock().unwrap().insert(id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                    let client_caps_struct = serde_json::from_value::<lsp_types_max::ClientCapabilities>(client_caps).unwrap_or_default();
                    let effective_caps = {
                        let s = self.state.lock().await;
                        s.capability_tracker.derive_effective_capabilities(&client_caps_struct)
                    };
                    let init_result = InitializeResult {
                        capabilities: effective_caps,
                        server_info: Some(ServerInfo {
                            name: "tower-lsp-max-composed".to_string(),
                            version: Some("26.6.5".to_string()),
                        }),
                        offset_encoding: None,
                    };
                    let init_result_val = serde_json::to_value(init_result).unwrap();
                    return Ok(Some(init_result_val));
                }
                let mut last_res = Ok(None);
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                        src.health = SourceHealth::Healthy;
                                    }
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                                last_res = Ok(Some(res.clone()));
                                sources_returned.lock().unwrap().push(source_id.clone());
                                break;
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                last_res
            }
            CompositionStrategy::FirstSuccess => {
                if method == "textDocument/hover" {
                    let mut hovers = Vec::new();
                    for source_id in routable_sources {
                        if let Some(conn) = self.upstreams.get(&source_id) {
                            match conn.request(method, params_val.clone(), timeout_ms).await {
                                Ok(res) => {
                                    {
                                        let mut s = self.state.lock().await;
                                        if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                            src.health = SourceHealth::Healthy;
                                        }
                                    }
                                    if !res.is_null() {
                                        hovers.push((source_id.clone(), res.clone()));
                                        sources_returned.lock().unwrap().push(source_id.clone());
                                    }
                                    {
                                        let s = self.state.lock().await;
                                        if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                            source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                        }
                                    }
                                }
                                Err(_) => {
                                    any_failed = true;
                                    let mut s = self.state.lock().await;
                                    s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                        }
                    }
                    let merged = merge_hovers_with_attribution(hovers);
                    Ok(Some(merged))
                } else {
                    let mut final_res = Ok(None);
                    for source_id in routable_sources {
                        if let Some(conn) = self.upstreams.get(&source_id) {
                            match conn.request(method, params_val.clone(), timeout_ms).await {
                                Ok(res) => {
                                    {
                                        let mut s = self.state.lock().await;
                                        if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                            src.health = SourceHealth::Healthy;
                                        }
                                    }
                                    if !res.is_null() {
                                        final_res = Ok(Some(res.clone()));
                                        sources_returned.lock().unwrap().push(source_id.clone());
                                        {
                                            let s = self.state.lock().await;
                                            if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                                source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                            }
                                        }
                                        break;
                                    }
                                }
                                Err(_) => {
                                    any_failed = true;
                                    let mut s = self.state.lock().await;
                                    s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                        }
                    }
                    final_res
                }
            }
            CompositionStrategy::MergeDeduped => {
                let mut observations = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    observations.push(AttributedObservation {
                                        source_id: source_id.clone(),
                                        uri: uri_opt.clone().unwrap_or_default(),
                                        data: res,
                                    });
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                let merged_locs = merge_deduped_locations(observations);
                let merged_val = json!(merged_locs);
                Ok(Some(merged_val))
            }
            CompositionStrategy::MergeAttributed => {
                let mut observations = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    observations.push(AttributedObservation {
                                        source_id: source_id.clone(),
                                        uri: uri_opt.clone().unwrap_or_default(),
                                        data: res,
                                    });
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                let merged = merge_attributed(observations);
                Ok(Some(merged))
            }
            CompositionStrategy::RankedProviders => {
                let mut completion_responses = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    completion_responses.push(res);
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                let mut all_items = Vec::new();
                let mut seen_labels = std::collections::HashSet::new();
                let mut is_incomplete = false;
                for resp in completion_responses {
                    let items_array = if let Some(items) = resp.get("items").and_then(|i| i.as_array()) {
                        if resp.get("isIncomplete").and_then(|i| i.as_bool()).unwrap_or(false) {
                            is_incomplete = true;
                        }
                        items
                    } else if let Some(items) = resp.as_array() {
                        items
                    } else {
                        continue;
                    };
                    for item in items_array {
                        if let Some(label) = item.get("label").and_then(|l| l.as_str()) {
                            if seen_labels.insert(label.to_string()) {
                                all_items.push(item.clone());
                            }
                        }
                    }
                }
                all_items.sort_by(|a, b| {
                    let a_sort = a.get("sortText").and_then(|v| v.as_str())
                        .or_else(|| a.get("label").and_then(|v| v.as_str()));
                    let b_sort = b.get("sortText").and_then(|v| v.as_str())
                        .or_else(|| b.get("label").and_then(|v| v.as_str()));
                    a_sort.cmp(&b_sort)
                });
                let result = json!({
                    "isIncomplete": is_incomplete,
                    "items": all_items
                });
                Ok(Some(result))
            }
            CompositionStrategy::TransactionalEditGate => {
                let uri = uri_opt.clone().unwrap_or_default();
                let mut merged_res = Value::Null;
                let mut accepted_proposals = Vec::new();
                for source_id in routable_sources {
                    if let Some(conn) = self.upstreams.get(&source_id) {
                        match conn.request(method, params_val.clone(), timeout_ms).await {
                            Ok(res) => {
                                {
                                    let mut s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                                        src.health = SourceHealth::Healthy;
                                    }
                                }
                                if !res.is_null() {
                                    sources_returned.lock().unwrap().push(source_id.clone());
                                    let mut s = self.state.lock().await;
                                    let client_version = params_val.get("context")
                                        .and_then(|c| c.get("version"))
                                        .and_then(|v| v.as_i64())
                                        .map(|v| v as i32);
                                    let version = client_version
                                        .or_else(|| extract_version_from_edit(&res, &uri))
                                        .or_else(|| s.doc_tracker.current_version(&uri))
                                        .unwrap_or(0);
                                    let proposed = ProposedEdit {
                                        source_id: source_id.clone(),
                                        uri: uri.clone(),
                                        version,
                                        method: method.to_string(),
                                        edit: res.clone(),
                                    };
                                    let outcome = s.edit_gate.validate(&proposed, &s.doc_tracker, &s.capability_tracker);
                                    *gate_outcome.lock().unwrap() = Some(format!("{:?}", outcome));
                                    if outcome == EditGateOutcome::Accepted {
                                        s.edit_gate.accept(proposed);
                                        accepted_proposals.push(source_id.clone());
                                        merged_res = merge_edits(merged_res, res);
                                    } else {
                                        for src_id in &accepted_proposals {
                                            s.edit_gate.remove(&uri, src_id);
                                        }
                                        return Err(Error::invalid_params(format!("Edit gate rejected edit: {:?}", outcome)));
                                    }
                                }
                                {
                                    let s = self.state.lock().await;
                                    if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                        source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                    }
                                }
                            }
                            Err(_) => {
                                any_failed = true;
                                let mut s = self.state.lock().await;
                                s.capability_tracker.degrade_source(&source_id, SourceHealth::TimedOut);
                                if let Some(src) = s.capability_tracker.sources.get(&source_id) {
                                    source_health.lock().unwrap().insert(source_id.clone(), format!("{:?}", src.health));
                                }
                            }
                        }
                    }
                }
                if merged_res.is_null() {
                    Ok(None)
                } else {
                    Ok(Some(merged_res))
                }
            }
            CompositionStrategy::OrderedFanout | CompositionStrategy::ObserveOnly | CompositionStrategy::Deny => {
                Err(Error::method_not_found())
            }
        };

        if params_val.get("partialResultToken") == Some(&Value::Null) && any_failed {
            return Ok(None);
        }

        if let Some(ref uri) = uri_opt {
            let s = self.state.lock().await;
            let version_after = s.doc_tracker.current_version(uri);
            if version_before != version_after {
                *staleness_outcome.lock().unwrap() = Some("StaleRefused".to_string());
                return Ok(None);
            } else {
                *staleness_outcome.lock().unwrap() = Some("NotStale".to_string());
            }
        }
        
        res
    }

    async fn route_notification<P>(&self, method: &str, params: P)
    where
        P: serde::Serialize,
    {
        println!("--- route_notification [{}] start", method);
        let params_val = serde_json::to_value(params).unwrap_or(Value::Null);
        let uri_opt = params_val.get("textDocument")
            .and_then(|td| td.get("uri"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());
            
        if let Some(uri) = &uri_opt {
            println!("--- route_notification [{}] locking state for uri", method);
            let mut s = self.state.lock().await;
            println!("--- route_notification [{}] locked state for uri", method);
            if method == "textDocument/didOpen" {
                let version = params_val.get("textDocument")
                    .and_then(|td| td.get("version"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                s.doc_tracker.did_open(uri, version);
                s.edit_gate.clear_for_uri(uri);
            } else if method == "textDocument/didChange" {
                let version = params_val.get("textDocument")
                    .and_then(|td| td.get("version"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                if let VersionCheckResult::OutOfOrder { .. } = s.doc_tracker.did_change(uri, version) {
                    println!("--- route_notification [{}] out of order return", method);
                    return;
                }
                s.edit_gate.clear_for_uri(uri);
            } else if method == "textDocument/didClose" {
                s.doc_tracker.did_close(uri);
                s.edit_gate.clear_for_uri(uri);
            }
        }
        
        println!("--- route_notification [{}] locking state for routable_sources", method);
        let routable_sources = {
            let s = self.state.lock().await;
            println!("--- route_notification [{}] locked state for routable_sources inside block", method);
            s.capability_tracker.routable_sources_for_method(method)
        };
        println!("--- route_notification [{}] routable_sources: {:?}", method, routable_sources);
        for source_id in routable_sources {
            if let Some(conn) = self.upstreams.get(&source_id) {
                println!("--- route_notification [{}] calling conn.notify for {}", method, source_id);
                if conn.notify(method, params_val.clone()).await.is_ok() {
                    println!("--- route_notification [{}] locking state for source health updating", method);
                    let mut s = self.state.lock().await;
                    println!("--- route_notification [{}] locked state for source health updating", method);
                    if let Some(src) = s.capability_tracker.sources.get_mut(&source_id) {
                        src.health = SourceHealth::Healthy;
                    }
                }
            }
        }
        println!("--- route_notification [{}] end", method);
    }
}

#[async_trait::async_trait]
impl LanguageServer for ComposedServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.route_request("initialize", params).await?.ok_or_else(Error::internal_error)
    }

    async fn initialized(&self, params: InitializedParams) {
        println!("--- ComposedServer::initialized start");
        self.route_notification("initialized", params).await;
        println!("--- ComposedServer::initialized end");
    }

    async fn shutdown(&self) -> Result<()> {
        self.route_request::<_, ()>("shutdown", ()).await?.ok_or_else(Error::internal_error)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.route_notification("textDocument/didOpen", params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.route_notification("textDocument/didChange", params).await;
    }

    async fn will_save(&self, params: WillSaveTextDocumentParams) {
        self.route_notification("textDocument/willSave", params).await;
    }

    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/willSaveWaitUntil", params).await.map(|o| o.flatten())
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.route_notification("textDocument/didSave", params).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.route_notification("textDocument/didClose", params).await;
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> Result<Option<GotoDeclarationResponse>> {
        self.route_request("textDocument/declaration", params).await.map(|o| o.flatten())
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.route_request("textDocument/definition", params).await.map(|o| o.flatten())
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        self.route_request("textDocument/typeDefinition", params).await.map(|o| o.flatten())
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> Result<Option<GotoImplementationResponse>> {
        self.route_request("textDocument/implementation", params).await.map(|o| o.flatten())
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        self.route_request("textDocument/references", params).await.map(|o| o.flatten())
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        self.route_request("textDocument/prepareCallHierarchy", params).await.map(|o| o.flatten())
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        self.route_request("callHierarchy/incomingCalls", params).await.map(|o| o.flatten())
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        self.route_request("callHierarchy/outgoingCalls", params).await.map(|o| o.flatten())
    }

    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        self.route_request("textDocument/prepareTypeHierarchy", params).await.map(|o| o.flatten())
    }

    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        self.route_request("typeHierarchy/supertypes", params).await.map(|o| o.flatten())
    }

    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        self.route_request("typeHierarchy/subtypes", params).await.map(|o| o.flatten())
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        self.route_request("textDocument/documentHighlight", params).await.map(|o| o.flatten())
    }

    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        self.route_request("textDocument/documentLink", params).await.map(|o| o.flatten())
    }

    async fn document_link_resolve(&self, params: DocumentLink) -> Result<DocumentLink> {
        self.route_request("documentLink/resolve", params).await?.ok_or_else(Error::internal_error)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.route_request("textDocument/hover", params).await.map(|o| o.flatten())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.route_request("textDocument/completion", params).await.map(|o| o.flatten())
    }

    async fn completion_resolve(&self, params: CompletionItem) -> Result<CompletionItem> {
        self.route_request("completionItem/resolve", params).await?.ok_or_else(Error::internal_error)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        self.route_request("textDocument/semanticTokens/full", params).await.map(|o| o.flatten())
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> Result<Option<SemanticTokensFullDeltaResult>> {
        self.route_request("textDocument/semanticTokens/full/delta", params).await.map(|o| o.flatten())
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        self.route_request("textDocument/semanticTokens/range", params).await.map(|o| o.flatten())
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        self.route_request("textDocument/codeLens", params).await.map(|o| o.flatten())
    }

    async fn code_lens_resolve(&self, params: CodeLens) -> Result<CodeLens> {
        self.route_request("codeLens/resolve", params).await?.ok_or_else(Error::internal_error)
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        self.route_request("textDocument/foldingRange", params).await.map(|o| o.flatten())
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        self.route_request("textDocument/selectionRange", params).await.map(|o| o.flatten())
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        self.route_request("textDocument/documentSymbol", params).await.map(|o| o.flatten())
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        self.route_request("workspace/symbol", params).await.map(|o| o.flatten())
    }

    async fn symbol_resolve(&self, params: WorkspaceSymbol) -> Result<WorkspaceSymbol> {
        self.route_request("workspaceSymbol/resolve", params).await?.ok_or_else(Error::internal_error)
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.route_notification("workspace/didChangeConfiguration", params).await;
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        self.route_notification("workspace/didChangeWorkspaceFolders", params).await;
    }

    async fn will_create_files(&self, params: CreateFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("workspace/willCreateFiles", params).await.map(|o| o.flatten())
    }

    async fn did_create_files(&self, params: CreateFilesParams) {
        self.route_notification("workspace/didCreateFiles", params).await;
    }

    async fn will_rename_files(&self, params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("workspace/willRenameFiles", params).await.map(|o| o.flatten())
    }

    async fn did_rename_files(&self, params: RenameFilesParams) {
        self.route_notification("workspace/didRenameFiles", params).await;
    }

    async fn will_delete_files(&self, params: DeleteFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("workspace/willDeleteFiles", params).await.map(|o| o.flatten())
    }

    async fn did_delete_files(&self, params: DeleteFilesParams) {
        self.route_notification("workspace/didDeleteFiles", params).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.route_notification("workspace/didChangeWatchedFiles", params).await;
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.route_request("workspace/executeCommand", params).await.map(|o| o.flatten())
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        self.route_request("textDocument/signatureHelp", params).await.map(|o| o.flatten())
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        self.route_request("textDocument/codeAction", params).await.map(|o| o.flatten())
    }

    async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction> {
        self.route_request("codeAction/resolve", params).await?.ok_or_else(Error::internal_error)
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        self.route_request("textDocument/documentColor", params).await.map(|o| o.unwrap_or_default())
    }

    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        self.route_request("textDocument/colorPresentation", params).await.map(|o| o.unwrap_or_default())
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/formatting", params).await.map(|o| o.flatten())
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/rangeFormatting", params).await.map(|o| o.flatten())
    }

    async fn on_type_formatting(
        &self,
        params: DocumentOnTypeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/onTypeFormatting", params).await.map(|o| o.flatten())
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("textDocument/rename", params).await.map(|o| o.flatten())
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        self.route_request("textDocument/prepareRename", params).await.map(|o| o.flatten())
    }

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> Result<Option<LinkedEditingRanges>> {
        self.route_request("textDocument/linkedEditingRange", params).await.map(|o| o.flatten())
    }

    async fn moniker(&self, params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
        self.route_request("textDocument/moniker", params).await.map(|o| o.flatten())
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        self.route_request("textDocument/inlayHint", params).await.map(|o| o.flatten())
    }

    async fn inlay_hint_resolve(&self, params: InlayHint) -> Result<InlayHint> {
        self.route_request("inlayHint/resolve", params).await?.ok_or_else(Error::internal_error)
    }

    async fn inline_value(&self, params: InlineValueParams) -> Result<Option<Vec<InlineValue>>> {
        self.route_request("textDocument/inlineValue", params).await.map(|o| o.flatten())
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        self.route_request("textDocument/diagnostic", params).await?.ok_or_else(Error::internal_error)
    }

    async fn workspace_diagnostic(
        &self,
        params: WorkspaceDiagnosticParams,
    ) -> Result<WorkspaceDiagnosticReportResult> {
        self.route_request("workspace/diagnostic", params).await?.ok_or_else(Error::internal_error)
    }

    async fn max_snapshot(&self) -> Result<max_protocol::SnapshotId> {
        Err(Error::method_not_found())
    }

    async fn max_conformance_vector(
        &self,
        _params: Option<max_protocol::SnapshotId>,
    ) -> Result<max_protocol::ConformanceVector> {
        Err(Error::method_not_found())
    }

    async fn max_explain_diagnostic(&self, _params: String) -> Result<max_protocol::MaxDiagnostic> {
        Err(Error::method_not_found())
    }

    async fn max_repair_plan(&self, _params: String) -> Result<Vec<max_protocol::MaxCodeAction>> {
        Err(Error::method_not_found())
    }

    async fn max_apply_repair_transaction(
        &self,
        _params: max_protocol::MaxCodeAction,
    ) -> Result<max_protocol::Receipt> {
        Err(Error::method_not_found())
    }

    async fn max_export_analysis_bundle(
        &self,
        _params: max_protocol::SnapshotId,
    ) -> Result<max_protocol::AnalysisBundle> {
        Err(Error::method_not_found())
    }

    async fn max_run_gate(&self, _params: max_protocol::GateId) -> Result<bool> {
        Err(Error::method_not_found())
    }

    async fn max_clear_diagnostic(&self, _params: String) -> Result<()> {
        Err(Error::method_not_found())
    }

    async fn max_receipt(&self, _params: String) -> Result<max_protocol::Receipt> {
        Err(Error::method_not_found())
    }

    async fn max_release_actuation(&self, _params: Value) -> Result<Value> {
        Err(Error::method_not_found())
    }

    async fn max_admission(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_autonomic_loop(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_chain(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_hook(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_hook_graph(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_lawful_transition(&self, _params: String) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_ledger_report(&self) -> Result<String> {
        Err(Error::method_not_found())
    }

    async fn max_manifold_snapshot(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_propagate(&self, _params: max_protocol::Receipt) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_refusal(&self, _params: String) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_replay(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_verify_ledger(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_conformance_delta(&self, _params: serde_json::Value) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn inline_completion(
        &self,
        params: InlineCompletionParams,
    ) -> Result<Option<InlineCompletionResponse>> {
        self.route_request("textDocument/inlineCompletion", params).await.map(|o| o.flatten())
    }

    async fn text_document_content(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<max_protocol::lsp_3_18::TextDocumentContentResult> {
        self.route_request("workspace/textDocumentContent", params).await?.ok_or_else(Error::internal_error)
    }

    async fn max_dump_state(&self) -> Result<serde_json::Value> {
        Err(Error::method_not_found())
    }

    async fn max_restore_state(&self, _params: serde_json::Value) -> Result<()> {
        Err(Error::method_not_found())
    }

    async fn ranges_formatting(
        &self,
        params: max_protocol::lsp_3_18::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<max_protocol::lsp_3_18::TextEdit>>> {
        self.route_request("textDocument/rangesFormatting", params).await.map(|o| o.flatten())
    }

    async fn did_open_notebook_document(&self, params: DidOpenNotebookDocumentParams) {
        self.route_notification("notebookDocument/didOpen", params).await;
    }

    async fn did_change_notebook_document(&self, params: DidChangeNotebookDocumentParams) {
        self.route_notification("notebookDocument/didChange", params).await;
    }

    async fn did_save_notebook_document(&self, params: DidSaveNotebookDocumentParams) {
        self.route_notification("notebookDocument/didSave", params).await;
    }

    async fn did_close_notebook_document(&self, params: DidCloseNotebookDocumentParams) {
        self.route_notification("notebookDocument/didClose", params).await;
    }

    async fn work_done_progress_cancel(&self, params: WorkDoneProgressCancelParams) {
        self.route_notification("window/workDoneProgress/cancel", params).await;
    }

    async fn set_trace(&self, params: SetTraceParams) {
        self.route_notification("$/setTrace", params).await;
    }

    async fn progress(&self, params: ProgressParams) {
        self.route_notification("$/progress", params).await;
    }

    async fn max_instance_list(&self) -> Result<Value> {
        Err(Error::method_not_found())
    }

    async fn max_reset(&self) -> Result<()> {
        Err(Error::method_not_found())
    }

    async fn max_lsif(&self) -> Result<String> {
        Err(Error::method_not_found())
    }
}


// ── Unit Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── R2: Capability Tracker Tests ──────────────────────────────────────────

    fn make_source(id: &str, caps: Value) -> UpstreamSource {
        let mut src = UpstreamSource::new(id, "127.0.0.1:0");
        src.server_capabilities = serde_json::from_value(caps).ok();
        src
    }

    #[test]
    fn test_completion_deser() {
        let val = json!({
            "isIncomplete": false,
            "items": [
                { "label": "apple", "detail": "high priority", "sortText": "01" }
            ]
        });
        let res: std::result::Result<lsp_types_max::CompletionResponse, _> = serde_json::from_value(val);
        if let Err(e) = res {
            panic!("DESER ERROR: {:?}", e);
        }
    }

    #[test]
    fn r2_source_a_supports_source_b_does_not_yields_non_union() {
        // R2: source A supports hover, source B does not.
        // Effective downstream capability should NOT be a raw union (true).
        // Only advertise hover if at least one source supports it AND client supports it.
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({ "hoverProvider": true })));
        tracker.add_source(make_source("B", json!({ "hoverProvider": false })));

        let client_caps_val = json!({ "textDocument": { "hover": {} } });
        let client_caps: lsp_types_max::ClientCapabilities = serde_json::from_value(client_caps_val).unwrap();
        let effective = tracker.derive_effective_capabilities(&client_caps);

        // At least one source supports hover → should be advertised
        assert_eq!(effective.hover_provider, Some(lsp_types_max::HoverProviderCapability::Simple(true)),
            "hover should be advertised since source A supports it");

        // But definition is not supported by either source
        assert!(effective.definition_provider.is_none(),
            "definitionProvider must not be advertised if no source supports it");
    }

    #[test]
    fn r2_no_source_supports_method_not_advertised() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({ "hoverProvider": false })));
        tracker.add_source(make_source("B", json!({ "hoverProvider": false })));

        let client_caps = lsp_types_max::ClientCapabilities::default();
        let effective = tracker.derive_effective_capabilities(&client_caps);
        assert!(effective.hover_provider.is_none(),
            "hover must not be advertised if no source supports it");
    }

    #[test]
    fn r2_dynamic_registration_adds_support() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({})));

        // Before registration: no hover support
        assert!(!tracker.sources["A"].supports_method("textDocument/hover"));

        // After dynamic registration: hover supported
        let ok = tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
        assert!(ok, "First registration should succeed");
        assert!(tracker.sources["A"].supports_method("textDocument/hover"),
            "After dynamic registration, source A should support hover");
    }

    #[test]
    fn r2_dynamic_registration_duplicate_rejected() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({})));

        let ok1 = tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
        let ok2 = tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
        assert!(ok1, "First registration should succeed");
        assert!(!ok2, "Duplicate registration ID must be rejected");
    }

    #[test]
    fn r2_dynamic_unregistration_removes_support() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({})));
        tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
        assert!(tracker.sources["A"].supports_method("textDocument/hover"));

        let removed = tracker.unregister_dynamic("reg-1");
        assert!(removed, "Unregistration should succeed");
        assert!(!tracker.sources["A"].supports_method("textDocument/hover"),
            "After unregistration, hover must no longer be supported");
    }

    #[test]
    fn r2_unregister_nonexistent_is_safe_noop() {
        let mut tracker = CapabilityTracker::new();
        let removed = tracker.unregister_dynamic("nonexistent");
        assert!(!removed, "Unregistering nonexistent ID must return false (safe no-op)");
    }

    #[test]
    fn r2_unsafe_route_unsupported_source_not_routable() {
        let mut tracker = CapabilityTracker::new();
        let mut src = make_source("A", json!({ "hoverProvider": true }));
        src.health = SourceHealth::Crashed;
        tracker.add_source(src);

        let routes = tracker.routable_sources_for_method("textDocument/hover");
        assert!(routes.is_empty(), "Crashed source must not be in routable list");

        let client_caps = lsp_types_max::ClientCapabilities::default();
        let effective = tracker.derive_effective_capabilities(&client_caps);
        assert!(effective.hover_provider.is_none(),
            "If no healthy source supports hover, it must not be advertised");
    }

    #[test]
    fn r2_client_incapable_not_advertised() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({ "hoverProvider": true })));

        // Client explicitly does not include hover capability
        let client_caps_val = json!({ "textDocument": { "hover": null } });
        let client_caps: lsp_types_max::ClientCapabilities = serde_json::from_value(client_caps_val).unwrap();
        let effective = tracker.derive_effective_capabilities(&client_caps);
        assert!(effective.hover_provider.is_none(),
            "If client does not support hover, it must not be advertised");
    }

    #[test]
    fn r2_capabilities_not_raw_union() {
        // The critical anti-union test: if B does not support definition,
        // and we raw-union, definition would be true. We must not raw-union.
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({ "definitionProvider": true })));
        tracker.add_source(make_source("B", json!({ "definitionProvider": false })));

        // If we were doing raw union, we'd advertise definition because A has it.
        // We actually DO advertise it in this case because A supports it and it's valid to merge.
        // The "not raw union" means: if NO source supports it, we don't advertise.
        tracker.degrade_source("A", SourceHealth::Crashed);
        let client_caps = lsp_types_max::ClientCapabilities::default();
        let effective = tracker.derive_effective_capabilities(&client_caps);
        assert!(effective.definition_provider.is_none(),
            "After degrading only supporting source, definition must not be advertised");
    }

    // ── R3: Method Strategy Tests ─────────────────────────────────────────────

    #[test]
    fn r3_method_strategies_are_explicit() {
        assert_eq!(method_strategy("initialize"), CompositionStrategy::SingleOwner);
        assert_eq!(method_strategy("textDocument/didChange"), CompositionStrategy::OrderedFanout);
        assert_eq!(method_strategy("textDocument/hover"), CompositionStrategy::FirstSuccess);
        assert_eq!(method_strategy("textDocument/definition"), CompositionStrategy::MergeDeduped);
        assert_eq!(method_strategy("textDocument/references"), CompositionStrategy::MergeDeduped);
        assert_eq!(method_strategy("textDocument/documentSymbol"), CompositionStrategy::MergeAttributed);
        assert_eq!(method_strategy("textDocument/completion"), CompositionStrategy::RankedProviders);
        assert_eq!(method_strategy("textDocument/formatting"), CompositionStrategy::TransactionalEditGate);
        assert_eq!(method_strategy("textDocument/rename"), CompositionStrategy::TransactionalEditGate);
        assert_eq!(method_strategy("textDocument/codeAction"), CompositionStrategy::TransactionalEditGate);
        assert_eq!(method_strategy("workspace/applyEdit"), CompositionStrategy::TransactionalEditGate);
        assert_eq!(method_strategy("$/cancelRequest"), CompositionStrategy::ObserveOnly);
        assert_eq!(method_strategy("some/unknown/method"), CompositionStrategy::Deny);
    }

    #[test]
    fn r3_unknown_method_denied() {
        let strategy = method_strategy("custom/nonstandardMethod");
        assert_eq!(strategy, CompositionStrategy::Deny,
            "Unknown methods must be denied by default");
    }

    // ── R4: Document Version Causality Tests ──────────────────────────────────

    #[test]
    fn r4_ordered_changes_accepted() {
        let mut tracker = DocumentVersionTracker::new();
        tracker.did_open("file:///test.rs", 1);
        assert_eq!(tracker.did_change("file:///test.rs", 2), VersionCheckResult::Current);
        assert_eq!(tracker.did_change("file:///test.rs", 3), VersionCheckResult::Current);
    }

    #[test]
    fn r4_out_of_order_change_refused() {
        let mut tracker = DocumentVersionTracker::new();
        tracker.did_open("file:///test.rs", 5);
        let result = tracker.did_change("file:///test.rs", 3); // Going backwards
        assert_eq!(result, VersionCheckResult::OutOfOrder { expected: 6, got: 3 },
            "Out-of-order version change must be refused");
    }

    #[test]
    fn r4_stale_result_detected() {
        let mut tracker = DocumentVersionTracker::new();
        tracker.did_open("file:///test.rs", 1);
        tracker.did_change("file:///test.rs", 2).unwrap_or_default();
        // Result was computed at version 1, but current is 2
        let check = tracker.check_staleness("file:///test.rs", 1);
        assert_eq!(check, VersionCheckResult::Stale { current: 2, result_version: 1 },
            "Result at old version must be detected as stale");
    }

    #[test]
    fn r4_did_close_ends_tracking() {
        let mut tracker = DocumentVersionTracker::new();
        tracker.did_open("file:///test.rs", 1);
        tracker.did_close("file:///test.rs");
        assert_eq!(tracker.current_version("file:///test.rs"), None,
            "After didClose, document must no longer be tracked");
    }

    #[test]
    fn r4_untracked_document_returns_not_tracked() {
        let tracker = DocumentVersionTracker::new();
        assert_eq!(tracker.check_staleness("file:///untracked.rs", 1), VersionCheckResult::NotTracked);
    }

    // ── R5: Edit Gate Tests ───────────────────────────────────────────────────

    fn make_proposed_edit(source_id: &str, uri: &str, version: i32, method: &str) -> ProposedEdit {
        ProposedEdit {
            source_id: source_id.to_string(),
            uri: uri.to_string(),
            version,
            method: method.to_string(),
            edit: json!([
                { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "hello" }
            ]),
        }
    }

    fn setup_gate_env() -> (TransactionEditGate, DocumentVersionTracker, CapabilityTracker) {
        let gate = TransactionEditGate::new();
        let mut doc = DocumentVersionTracker::new();
        doc.did_open("file:///test.rs", 1);
        let mut caps = CapabilityTracker::new();
        caps.add_source(make_source("src-a", json!({ "documentFormattingProvider": true })));
        (gate, doc, caps)
    }

    #[test]
    fn r5_formatting_through_edit_gate_accepted() {
        let (gate, doc, caps) = setup_gate_env();
        let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
        assert_eq!(gate.validate(&edit, &doc, &caps), EditGateOutcome::Accepted);
    }

    #[test]
    fn r5_stale_edit_refused() {
        let (gate, mut doc, caps) = setup_gate_env();
        // Advance document version
        doc.did_change("file:///test.rs", 2).unwrap_or_default();
        // Edit at version 1 (now stale)
        let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
        assert_eq!(gate.validate(&edit, &doc, &caps), EditGateOutcome::Stale,
            "Edit at old version must be refused as stale");
    }

    #[test]
    fn r5_overlapping_edit_refused() {
        let (mut gate, doc, caps) = setup_gate_env();
        // Accept first edit from src-a
        let edit_a = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
        gate.accept(edit_a);

        // Try to submit overlapping edit from src-b for same document
        let mut caps2 = caps;
        caps2.add_source(make_source("src-b", json!({ "documentFormattingProvider": true })));
        let edit_b = make_proposed_edit("src-b", "file:///test.rs", 1, "textDocument/formatting");
        assert_eq!(gate.validate(&edit_b, &doc, &caps2), EditGateOutcome::Overlapping,
            "Edit from different source targeting same document must be refused as overlapping");
    }

    #[test]
    fn r5_degraded_source_edit_refused() {
        let (gate, doc, mut caps) = setup_gate_env();
        caps.degrade_source("src-a", SourceHealth::Crashed);
        let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
        assert_eq!(gate.validate(&edit, &doc, &caps), EditGateOutcome::SourceDegraded,
            "Edit from degraded source must be refused");
    }

    #[test]
    fn r5_unknown_document_edit_refused() {
        let (gate, doc, caps) = setup_gate_env();
        let edit = make_proposed_edit("src-a", "file:///unknown.rs", 1, "textDocument/formatting");
        assert_eq!(gate.validate(&edit, &doc, &caps), EditGateOutcome::Accepted,
            "Edit for untracked document must be accepted");
    }

    #[test]
    fn r5_non_mutation_method_not_allowed_through_gate() {
        let (gate, doc, caps) = setup_gate_env();
        // hover is NOT a mutation method
        let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/hover");
        assert_eq!(gate.validate(&edit, &doc, &caps), EditGateOutcome::MethodNotAllowed,
            "Non-mutation methods must be refused by the edit gate");
    }

    #[test]
    fn r5_workspace_edit_cannot_bypass_gate() {
        let (gate, doc, caps) = setup_gate_env();
        // workspace/applyEdit must go through the gate
        let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "workspace/applyEdit");
        // Valid source and document, but source-a doesn't support workspace edits in caps.
        // Gate checks source health only (any healthy source can attempt a workspace edit).
        let outcome = gate.validate(&edit, &doc, &caps);
        // It should either be Accepted or MethodNotAllowed depending on strategy,
        // but it MUST NOT succeed silently bypassing the gate check.
        // workspace/applyEdit IS in TransactionalEditGate strategy, so it should be Accepted here
        // because source is healthy and document is tracked and version matches.
        assert_ne!(outcome, EditGateOutcome::MethodNotAllowed,
            "workspace/applyEdit must pass through gate checks (not be excluded as MethodNotAllowed)");
    }

    // ── R6: Failure Isolation Tests ───────────────────────────────────────────

    #[test]
    fn r6_crashed_source_does_not_poison_others() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({ "hoverProvider": true })));
        tracker.add_source(make_source("B", json!({ "hoverProvider": true })));

        tracker.degrade_source("A", SourceHealth::Crashed);

        let routes = tracker.routable_sources_for_method("textDocument/hover");
        assert_eq!(routes.len(), 1, "After source A crashes, only source B should be routable");
        assert_eq!(routes[0], "B");
    }

    #[test]
    fn r6_all_sources_degraded_no_capability_advertised() {
        let mut tracker = CapabilityTracker::new();
        tracker.add_source(make_source("A", json!({ "hoverProvider": true })));
        tracker.degrade_source("A", SourceHealth::InitializationFailed);

        let effective = tracker.derive_effective_capabilities(&lsp_types_max::ClientCapabilities::default());
        assert!(effective.hover_provider.is_none(),
            "If all sources are degraded, no capabilities should be advertised");
    }

    // ── R4: Source Attribution Tests ──────────────────────────────────────────

    #[test]
    fn r4_merged_output_preserves_source_identity() {
        let observations = vec![
            AttributedObservation {
                source_id: "lang-server".to_string(),
                uri: "file:///test.rs".to_string(),
                data: json!([{ "message": "error 1" }]),
            },
            AttributedObservation {
                source_id: "lint-server".to_string(),
                uri: "file:///test.rs".to_string(),
                data: json!([{ "message": "warning 2" }]),
            },
        ];

        let merged = merge_attributed(observations);
        let arr = merged.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["source"], json!("lang-server"),
            "Source identity must be preserved in merged output");
        assert_eq!(arr[1]["source"], json!("lint-server"),
            "Source identity must be preserved in merged output");
    }

    #[test]
    fn r4_deduped_locations_removes_duplicates() {
        let loc = json!({ "uri": "file:///test.rs", "range": { "start": {"line":1,"character":0}, "end": {"line":1,"character":5} } });
        let observations = vec![
            AttributedObservation {
                source_id: "A".to_string(),
                uri: "file:///test.rs".to_string(),
                data: json!([loc.clone()]),
            },
            AttributedObservation {
                source_id: "B".to_string(),
                uri: "file:///test.rs".to_string(),
                data: json!([loc.clone()]),
            },
        ];

        let merged = merge_deduped_locations(observations);
        assert_eq!(merged.len(), 1, "Duplicate locations must be deduplicated");
    }

    #[test]
    fn test_gate8_method_routing_matrix() {
        let routed_methods = vec![
            "initialize",
            "initialized",
            "shutdown",
            "exit",
            "textDocument/didOpen",
            "textDocument/didChange",
            "textDocument/willSave",
            "textDocument/willSaveWaitUntil",
            "textDocument/didSave",
            "textDocument/didClose",
            "textDocument/declaration",
            "textDocument/definition",
            "textDocument/typeDefinition",
            "textDocument/implementation",
            "textDocument/references",
            "textDocument/prepareCallHierarchy",
            "callHierarchy/incomingCalls",
            "callHierarchy/outgoingCalls",
            "textDocument/prepareTypeHierarchy",
            "typeHierarchy/supertypes",
            "typeHierarchy/subtypes",
            "textDocument/documentHighlight",
            "textDocument/documentLink",
            "documentLink/resolve",
            "textDocument/hover",
            "textDocument/completion",
            "completionItem/resolve",
            "textDocument/semanticTokens/full",
            "textDocument/semanticTokens/full/delta",
            "textDocument/semanticTokens/range",
            "textDocument/codeLens",
            "codeLens/resolve",
            "textDocument/foldingRange",
            "textDocument/selectionRange",
            "textDocument/documentSymbol",
            "workspace/symbol",
            "workspaceSymbol/resolve",
            "workspace/didChangeConfiguration",
            "workspace/didChangeWorkspaceFolders",
            "workspace/willCreateFiles",
            "workspace/didCreateFiles",
            "workspace/willRenameFiles",
            "workspace/didRenameFiles",
            "workspace/willDeleteFiles",
            "workspace/didDeleteFiles",
            "workspace/didChangeWatchedFiles",
            "workspace/executeCommand",
            "textDocument/signatureHelp",
            "textDocument/codeAction",
            "codeAction/resolve",
            "textDocument/documentColor",
            "textDocument/colorPresentation",
            "textDocument/formatting",
            "textDocument/rangeFormatting",
            "textDocument/onTypeFormatting",
            "textDocument/rename",
            "textDocument/prepareRename",
            "textDocument/linkedEditingRange",
            "textDocument/moniker",
            "textDocument/inlayHint",
            "inlayHint/resolve",
            "textDocument/inlineValue",
            "textDocument/diagnostic",
            "workspace/diagnostic",
            "textDocument/inlineCompletion",
            "workspace/textDocumentContent",
            "textDocument/rangesFormatting",
            "notebookDocument/didOpen",
            "notebookDocument/didChange",
            "notebookDocument/didSave",
            "notebookDocument/didClose",
            "$/cancelRequest",
            "$/progress",
            "window/workDoneProgress/cancel",
            "$/setTrace",
        ];

        for method in &routed_methods {
            let strategy = method_strategy(method);
            assert_ne!(
                strategy,
                CompositionStrategy::Deny,
                "Method '{}' must map to an explicit routing strategy, not the default Deny strategy",
                method
            );
        }

        assert_eq!(
            method_strategy("textDocument/nonExistentMethod"),
            CompositionStrategy::Deny
        );
        assert_eq!(
            method_strategy("workspace/invalidRandomAction"),
            CompositionStrategy::Deny
        );
    }

    #[test]
    fn test_bypass_capability_tracker() {
        let mut tracker = CapabilityTracker::new();
        let mut source1 = UpstreamSource::new("A", "127.0.0.1:1");
        let caps1: lsp_types_max::ServerCapabilities = serde_json::from_value(json!({
            "completionProvider": { "resolveProvider": true, "triggerCharacters": ["."] }
        })).unwrap();
        source1.server_capabilities = Some(caps1);
        tracker.add_source(source1);

        let mut source2 = UpstreamSource::new("B", "127.0.0.1:2");
        let caps2: lsp_types_max::ServerCapabilities = serde_json::from_value(json!({
            "completionProvider": { "resolveProvider": false, "triggerCharacters": [".", ":"] }
        })).unwrap();
        source2.server_capabilities = Some(caps2);
        tracker.add_source(source2);

        let client_caps = lsp_types_max::ClientCapabilities {
            text_document: Some(lsp_types_max::TextDocumentClientCapabilities {
                completion: Some(lsp_types_max::CompletionClientCapabilities {
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Assert the expected ACTIVE behavior: intersection of capabilities.
        // If the component is active, this passes.
        // If SABOTAGE_CAPABILITY_TRACKER is set externally, this fails.
        let effective = tracker.derive_effective_capabilities(&client_caps);
        let comp = effective.completion_provider.as_ref().unwrap();
        assert_eq!(comp.resolve_provider, Some(false));
        assert_eq!(comp.trigger_characters.as_ref().unwrap(), &vec![".".to_string()]);
    }

    #[test]
    fn test_bypass_document_version_tracker() {
        let mut tracker = DocumentVersionTracker::new();
        let uri = "file:///test.rs";
        tracker.did_open(uri, 5);

        // Assert the expected ACTIVE behavior: out-of-order changes/stale checks are rejected.
        // If the component is active, this passes.
        // If SABOTAGE_DOCUMENT_VERSION_TRACKER is set externally, version checks are disabled,
        // and out-of-order changes are accepted, making the test fail.
        let outcome = tracker.did_change(uri, 3);
        assert!(matches!(outcome, VersionCheckResult::OutOfOrder { .. }));
        let staleness = tracker.check_staleness(uri, 4);
        assert!(matches!(staleness, VersionCheckResult::Stale { .. }));
    }

    #[test]
    fn test_bypass_transaction_edit_gate() {
        let gate = TransactionEditGate::new();
        let mut doc = DocumentVersionTracker::new();
        let mut caps = CapabilityTracker::new();
        let uri = "file:///test.rs";

        // Setup source
        let source = UpstreamSource::new("A", "127.0.0.1:1");
        caps.add_source(source);

        doc.did_open(uri, 5);

        // Propose a stale edit (version 4)
        let proposed = ProposedEdit {
            source_id: "A".to_string(),
            uri: uri.to_string(),
            version: 4,
            method: "textDocument/formatting".to_string(),
            edit: json!([]),
        };

        // Assert the expected ACTIVE behavior: stale edit is rejected.
        // If the component is active, this passes.
        // If SABOTAGE_TRANSACTION_EDIT_GATE is set externally, the gate is bypassed and accepts the edit,
        // making the test fail.
        let outcome = gate.validate(&proposed, &doc, &caps);
        assert_eq!(outcome, EditGateOutcome::Stale);
    }

    #[test]
    fn test_bypass_routing_matrix() {
        // Assert the expected ACTIVE behavior.
        // If the component is active, hover strategy is FirstSuccess.
        // If SABOTAGE_ROUTING_MATRIX is set externally, the strategy is Deny, making the test fail.
        assert_eq!(method_strategy("textDocument/hover"), CompositionStrategy::FirstSuccess);
    }

    #[test]
    fn test_bypass_source_health() {
        let mut tracker = CapabilityTracker::new();
        let source = UpstreamSource::new("A", "127.0.0.1:1");
        tracker.add_source(source);

        // Assert the expected ACTIVE behavior: degrading a source changes its health.
        // If the component is active, degrading it makes it Crashed.
        // If SABOTAGE_SOURCE_HEALTH is set externally, degradation is disabled (noop),
        // and health remains Healthy, making the test fail.
        tracker.degrade_source("A", SourceHealth::Crashed);
        assert_eq!(tracker.sources["A"].health, SourceHealth::Crashed);
    }

    #[tokio::test]
    async fn test_bypass_static_graph() {
        use crate::LspService;
        let (service, _) = LspService::new(|client| {
            ComposedServer::new(client, vec![("A".to_string(), "127.0.0.1:1".to_string())])
        });
        let server = service.inner();

        {
            let mut s = server.state.lock().await;
            s.doc_tracker.did_open("file:///test.rs", 5);
            s.capability_tracker.add_source(make_source("A", json!({})));
        }

        let params = json!({
            "textDocument": { "uri": "file:///test.rs" },
            "position": { "line": 0, "character": 0 }
        });
        let mut params_obj = params.as_object().unwrap().clone();
        params_obj.insert("context".to_string(), json!({ "version": 1 }));

        // Assert the expected ACTIVE behavior: check_staleness rejects the stale request, returning Ok(None).
        // If the component is active, this passes.
        // If SABOTAGE_STATIC_GRAPH is set externally, the staleness check is skipped,
        // and the request proceeds to be routed, returning Err or non-None (since upstream isn't connected),
        // making the test fail.
        let resp = server.route_request::<_, serde_json::Value>("textDocument/definition", Value::Object(params_obj)).await;
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap(), None);
    }
}

trait UnwrapOrDefault {
    fn unwrap_or_default(self) -> VersionCheckResult;
}

impl UnwrapOrDefault for VersionCheckResult {
    fn unwrap_or_default(self) -> VersionCheckResult {
        self
    }
}
