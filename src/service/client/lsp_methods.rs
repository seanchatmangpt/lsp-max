use lsp_types_max::*;
use serde::Serialize;
use serde_json::Value;
use std::fmt::Display;

use super::Client;
use crate::jsonrpc;

impl Client {
    // Lifecycle Messages

    /// Registers a new capability with the client.
    ///
    /// This corresponds to the [`client/registerCapability`] request.
    ///
    /// [`client/registerCapability`]: https://microsoft.github.io/language-server-protocol/specification#client_registerCapability
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    pub async fn register_capability(
        &self,
        registrations: Vec<Registration>,
    ) -> jsonrpc::Result<()> {
        use lsp_types_max::request::RegisterCapability;
        self.send_request::<RegisterCapability>(RegistrationParams { registrations })
            .await
    }

    /// Unregisters a capability with the client.
    ///
    /// This corresponds to the [`client/unregisterCapability`] request.
    ///
    /// [`client/unregisterCapability`]: https://microsoft.github.io/language-server-protocol/specification#client_unregisterCapability
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    pub async fn unregister_capability(
        &self,
        unregisterations: Vec<Unregistration>,
    ) -> jsonrpc::Result<()> {
        use lsp_types_max::request::UnregisterCapability;
        self.send_request::<UnregisterCapability>(UnregistrationParams { unregisterations })
            .await
    }

    // Window Features

    /// Notifies the client to display a particular message in the user interface.
    ///
    /// This corresponds to the [`window/showMessage`] notification.
    ///
    /// [`window/showMessage`]: https://microsoft.github.io/language-server-protocol/specification#window_showMessage
    pub async fn show_message<M: Display>(&self, typ: MessageType, message: M) {
        use lsp_types_max::notification::ShowMessage;
        self.send_notification_unchecked::<ShowMessage>(ShowMessageParams {
            typ,
            message: message.to_string(),
        })
        .await;
    }

    /// Requests the client to display a particular message in the user interface.
    ///
    /// Unlike the `show_message` notification, this request can also pass a list of actions and
    /// wait for an answer from the client.
    ///
    /// This corresponds to the [`window/showMessageRequest`] request.
    ///
    /// [`window/showMessageRequest`]: https://microsoft.github.io/language-server-protocol/specification#window_showMessageRequest
    pub async fn show_message_request<M: Display>(
        &self,
        typ: MessageType,
        message: M,
        actions: Option<Vec<MessageActionItem>>,
    ) -> jsonrpc::Result<Option<MessageActionItem>> {
        use lsp_types_max::request::ShowMessageRequest;
        self.send_request_unchecked::<ShowMessageRequest>(ShowMessageRequestParams {
            typ,
            message: message.to_string(),
            actions,
        })
        .await
    }

    /// Notifies the client to log a particular message.
    ///
    /// This corresponds to the [`window/logMessage`] notification.
    ///
    /// [`window/logMessage`]: https://microsoft.github.io/language-server-protocol/specification#window_logMessage
    pub async fn log_message<M: Display>(&self, typ: MessageType, message: M) {
        use lsp_types_max::notification::LogMessage;
        self.send_notification_unchecked::<LogMessage>(LogMessageParams {
            typ,
            message: message.to_string(),
        })
        .await;
    }

    /// Notifies the client to log a trace.
    ///
    /// This corresponds to the [`$/logTrace`] notification.
    ///
    /// [`$/logTrace`]: https://microsoft.github.io/language-server-protocol/specification#logTrace
    pub async fn log_trace(&self, params: LogTraceParams) {
        use lsp_types_max::notification::LogTrace;
        self.send_notification_unchecked::<LogTrace>(params).await;
    }

    /// Asks the client to display a particular resource referenced by a URI in the user interface.
    ///
    /// Returns `Ok(true)` if the document was successfully shown, or `Ok(false)` otherwise.
    ///
    /// This corresponds to the [`window/showDocument`] request.
    ///
    /// [`window/showDocument`]: https://microsoft.github.io/language-server-protocol/specification#window_showDocument
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    pub async fn show_document(&self, params: ShowDocumentParams) -> jsonrpc::Result<bool> {
        use lsp_types_max::request::ShowDocument;
        let response = self.send_request::<ShowDocument>(params).await?;
        Ok(response.success)
    }

    /// Asks the client to create a work done progress token.
    ///
    /// This corresponds to the [`window/workDoneProgress/create`] request.
    ///
    /// [`window/workDoneProgress/create`]: https://microsoft.github.io/language-server-protocol/specification#window_workDoneProgress_create
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.15.0.
    pub async fn work_done_progress_create(
        &self,
        params: WorkDoneProgressCreateParams,
    ) -> jsonrpc::Result<()> {
        use lsp_types_max::request::WorkDoneProgressCreate;
        self.send_request::<WorkDoneProgressCreate>(params).await
    }

    /// Notifies the client to log a telemetry event.
    ///
    /// This corresponds to the [`telemetry/event`] notification.
    ///
    /// [`telemetry/event`]: https://microsoft.github.io/language-server-protocol/specification#telemetry_event
    pub async fn telemetry_event<S: Serialize>(&self, data: S) {
        use lsp_types_max::notification::TelemetryEvent;
        match serde_json::to_value(data) {
            Err(e) => tracing::error!("invalid JSON in `telemetry/event` notification: {}", e),
            Ok(mut value) => {
                if !value.is_null() && !value.is_array() && !value.is_object() {
                    value = Value::Array(vec![value]);
                }
                let params = match value {
                    Value::Object(map) => OneOf::Left(map),
                    Value::Array(vec) => OneOf::Right(vec),
                    _ => OneOf::Right(Vec::new()),
                };
                self.send_notification_unchecked::<TelemetryEvent>(params)
                    .await;
            }
        }
    }

    /// Asks the client to refresh the code lenses currently shown in editors. As a result, the
    /// client should ask the server to recompute the code lenses for these editors.
    ///
    /// This is useful if a server detects a configuration change which requires a re-calculation
    /// of all code lenses.
    ///
    /// Note that the client still has the freedom to delay the re-calculation of the code lenses
    /// if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/codeLens/refresh`] request.
    ///
    /// [`workspace/codeLens/refresh`]: https://microsoft.github.io/language-server-protocol/specification#codeLens_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    pub async fn code_lens_refresh(&self) -> jsonrpc::Result<()> {
        use lsp_types_max::request::CodeLensRefresh;
        self.send_request::<CodeLensRefresh>(()).await
    }

    /// Asks the client to refresh the editors for which this server provides semantic tokens. As a
    /// result, the client should ask the server to recompute the semantic tokens for these
    /// editors.
    ///
    /// This is useful if a server detects a project-wide configuration change which requires a
    /// re-calculation of all semantic tokens. Note that the client still has the freedom to delay
    /// the re-calculation of the semantic tokens if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/semanticTokens/refresh`] request.
    ///
    /// [`workspace/semanticTokens/refresh`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_semanticTokens
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    pub async fn semantic_tokens_refresh(&self) -> jsonrpc::Result<()> {
        use lsp_types_max::request::SemanticTokensRefresh;
        self.send_request::<SemanticTokensRefresh>(()).await
    }

    /// Asks the client to refresh the inline values currently shown in editors. As a result, the
    /// client should ask the server to recompute the inline values for these editors.
    ///
    /// This is useful if a server detects a configuration change which requires a re-calculation
    /// of all inline values. Note that the client still has the freedom to delay the
    /// re-calculation of the inline values if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/inlineValue/refresh`] request.
    ///
    /// [`workspace/inlineValue/refresh`]: https://microsoft.github.io/language-server-protocol/specification#workspace_inlineValue_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    pub async fn inline_value_refresh(&self) -> jsonrpc::Result<()> {
        use lsp_types_max::request::InlineValueRefreshRequest;
        self.send_request::<InlineValueRefreshRequest>(()).await
    }

    /// Asks the client to refresh the inlay hints currently shown in editors. As a result, the
    /// client should ask the server to recompute the inlay hints for these editors.
    ///
    /// This is useful if a server detects a configuration change which requires a re-calculation
    /// of all inlay hints. Note that the client still has the freedom to delay the re-calculation
    /// of the inlay hints if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/inlayHint/refresh`] request.
    ///
    /// [`workspace/inlayHint/refresh`]: https://microsoft.github.io/language-server-protocol/specification#workspace_inlayHint_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    pub async fn inlay_hint_refresh(&self) -> jsonrpc::Result<()> {
        use lsp_types_max::request::InlayHintRefreshRequest;
        self.send_request::<InlayHintRefreshRequest>(()).await
    }

    /// Asks the client to refresh all folding ranges.
    ///
    /// This corresponds to the [`workspace/foldingRange/refresh`] request.
    ///
    /// [`workspace/foldingRange/refresh`]: https://microsoft.github.io/language-server-protocol/specification#workspace_foldingRange_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    pub async fn folding_range_refresh(&self) -> jsonrpc::Result<()> {
        self.send_request::<super::FoldingRangeRefresh>(()).await
    }

    /// Asks the client to refresh all needed document and workspace diagnostics.
    ///
    /// This is useful if a server detects a project wide configuration change which requires a
    /// re-calculation of all diagnostics.
    ///
    /// This corresponds to the [`workspace/diagnostic/refresh`] request.
    ///
    /// [`workspace/diagnostic/refresh`]: https://microsoft.github.io/language-server-protocol/specification#diagnostic_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    pub async fn workspace_diagnostic_refresh(&self) -> jsonrpc::Result<()> {
        use lsp_types_max::request::WorkspaceDiagnosticRefresh;
        self.send_request::<WorkspaceDiagnosticRefresh>(()).await
    }

    /// Asks the client to refresh the content of a text document.
    ///
    /// This corresponds to the [`workspace/textDocumentContent/refresh`] request.
    pub async fn text_document_content_refresh(
        &self,
        params: tower_lsp_max_protocol::lsp_3_18::TextDocumentContentRefreshParams,
    ) -> jsonrpc::Result<()> {
        self.send_request::<tower_lsp_max_protocol::lsp_3_18::TextDocumentContentRefreshRequest>(
            params,
        )
        .await
    }

    /// Submits validation diagnostics for an open file with the given URI.
    ///
    /// This corresponds to the [`textDocument/publishDiagnostics`] notification.
    ///
    /// [`textDocument/publishDiagnostics`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_publishDiagnostics
    ///
    /// # Initialization
    ///
    /// This notification will only be sent if the server is initialized.
    pub async fn publish_diagnostics(
        &self,
        uri: Uri,
        diags: Vec<Diagnostic>,
        version: Option<i32>,
    ) {
        use lsp_types_max::notification::PublishDiagnostics;
        self.send_notification_unchecked::<PublishDiagnostics>(PublishDiagnosticsParams::new(
            uri, diags, version,
        ))
        .await;
    }

    // Workspace Features

    /// Fetches configuration settings from the client.
    ///
    /// The request can fetch several configuration settings in one roundtrip. The order of the
    /// returned configuration settings correspond to the order of the passed
    /// [`ConfigurationItem`]s (e.g. the first item in the response is the result for the first
    /// configuration item in the params).
    ///
    /// This corresponds to the [`workspace/configuration`] request.
    ///
    /// [`workspace/configuration`]: https://microsoft.github.io/language-server-protocol/specification#workspace_configuration
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    pub async fn configuration(
        &self,
        items: Vec<ConfigurationItem>,
    ) -> jsonrpc::Result<Vec<Value>> {
        use lsp_types_max::request::WorkspaceConfiguration;
        self.send_request::<WorkspaceConfiguration>(ConfigurationParams { items })
            .await
    }

    /// Fetches the current open list of workspace folders.
    ///
    /// Returns `None` if only a single file is open in the tool. Returns an empty `Vec` if a
    /// workspace is open but no folders are configured.
    ///
    /// This corresponds to the [`workspace/workspaceFolders`] request.
    ///
    /// [`workspace/workspaceFolders`]: https://microsoft.github.io/language-server-protocol/specification#workspace_workspaceFolders
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    pub async fn workspace_folders(&self) -> jsonrpc::Result<Option<Vec<WorkspaceFolder>>> {
        use lsp_types_max::request::WorkspaceFoldersRequest;
        self.send_request::<WorkspaceFoldersRequest>(()).await
    }

    /// Requests a workspace resource be edited on the client side and returns whether the edit was
    /// applied.
    ///
    /// This corresponds to the [`workspace/applyEdit`] request.
    ///
    /// [`workspace/applyEdit`]: https://microsoft.github.io/language-server-protocol/specification#workspace_applyEdit
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    pub async fn apply_edit(
        &self,
        edit: WorkspaceEdit,
    ) -> jsonrpc::Result<ApplyWorkspaceEditResponse> {
        use lsp_types_max::request::ApplyWorkspaceEdit;
        self.send_request::<ApplyWorkspaceEdit>(ApplyWorkspaceEditParams { edit, label: None })
            .await
    }

    /// Starts a stream of `$/progress` notifications for a client-provided [`ProgressToken`].
    ///
    /// This method also takes a `title` argument briefly describing the kind of operation being
    /// performed, e.g. "Indexing" or "Linking Dependencies".
    ///
    /// [`ProgressToken`]: https://docs.rs/lsp-types/latest/lsp_types/type.ProgressToken.html
    ///
    /// # Initialization
    ///
    /// These notifications will only be sent if the server is initialized.
    pub fn progress<T>(&self, token: ProgressToken, title: T) -> super::Progress
    where
        T: Into<String>,
    {
        super::Progress::new(self.clone(), token, title.into())
    }
}
