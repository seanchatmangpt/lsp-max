use async_trait::async_trait;
use lsp_types::*;
use serde_json::Value;

/// Error type for client operations.
#[derive(Debug)]
pub struct ClientError(pub String);

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientError: {}", self.0)
    }
}
impl std::error::Error for ClientError {}

/// Trait implemented by the Language Client to handle server-to-client messages.
/// This encompasses all standard LSP 3.18 reverse-requests and notifications.
#[async_trait]
pub trait LanguageClient: Send + Sync + 'static {
    // --- Window ---

    /// `window/showMessage` notification
    async fn show_message(&self, params: ShowMessageParams) {
        let _ = params;
    }

    /// `window/showMessageRequest` request
    async fn show_message_request(&self, params: ShowMessageRequestParams) -> Result<Option<MessageActionItem>, ClientError> {
        let _ = params;
        Ok(None)
    }

    /// `window/logMessage` notification
    async fn log_message(&self, params: LogMessageParams) {
        let _ = params;
    }

    /// `window/workDoneProgress/create` request
    async fn work_done_progress_create(&self, params: WorkDoneProgressCreateParams) -> Result<(), ClientError> {
        let _ = params;
        Ok(())
    }

    /// `window/showDocument` request
    async fn show_document(&self, params: ShowDocumentParams) -> Result<ShowDocumentResult, ClientError> {
        let _ = params;
        Ok(ShowDocumentResult { success: false })
    }

    // --- Telemetry ---

    /// `telemetry/event` notification
    async fn telemetry_event(&self, params: Value) {
        let _ = params;
    }

    // --- Client / Workspace ---

    /// `client/registerCapability` request
    async fn register_capability(&self, params: RegistrationParams) -> Result<(), ClientError> {
        let _ = params;
        Ok(())
    }

    /// `client/unregisterCapability` request
    async fn unregister_capability(&self, params: UnregistrationParams) -> Result<(), ClientError> {
        let _ = params;
        Ok(())
    }

    /// `workspace/workspaceFolders` request
    async fn workspace_folders(&self) -> Result<Option<Vec<WorkspaceFolder>>, ClientError> {
        Ok(None)
    }

    /// `workspace/configuration` request
    async fn configuration(&self, params: ConfigurationParams) -> Result<Vec<Value>, ClientError> {
        let _ = params;
        Ok(Vec::new())
    }

    /// `workspace/applyEdit` request
    async fn apply_edit(&self, params: ApplyWorkspaceEditParams) -> Result<ApplyWorkspaceEditResponse, ClientError> {
        let _ = params;
        Ok(ApplyWorkspaceEditResponse { applied: false, failure_reason: None, failed_change: None })
    }

    // --- Refresh Requests ---

    async fn code_lens_refresh(&self) -> Result<(), ClientError> { Ok(()) }
    async fn semantic_tokens_refresh(&self) -> Result<(), ClientError> { Ok(()) }
    async fn inlay_hint_refresh(&self) -> Result<(), ClientError> { Ok(()) }
    async fn inline_value_refresh(&self) -> Result<(), ClientError> { Ok(()) }
    async fn workspace_diagnostic_refresh(&self) -> Result<(), ClientError> { Ok(()) }

    // --- Diagnostics ---

    /// `textDocument/publishDiagnostics` notification
    async fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
        let _ = params;
    }
}
