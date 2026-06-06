//! Extensions for Progress and Cancellation.

use lsp_types_max::notification::{Cancel, SetTrace, WorkDoneProgressCancel};
use lsp_types_max::{
    CancelParams, NumberOrString, ProgressToken, SetTraceParams, TraceValue,
    WorkDoneProgressCancelParams,
};

use super::Client;

impl Client {
    /// Sends a `$/cancelRequest` notification to the client.
    ///
    /// This notification is sent from the server to the client to cancel a specific request
    /// previously sent by the server.
    ///
    /// # Initialization
    ///
    /// This notification will only be sent if the server is initialized.
    pub async fn cancel_request(&self, id: NumberOrString) {
        self.send_notification::<Cancel>(CancelParams { id }).await;
    }

    /// Sends a `window/workDoneProgress/cancel` notification to the client.
    ///
    /// This notification is sent from the server to the client to inform it that the server
    /// has cancelled a specific work done progress.
    ///
    /// # Initialization
    ///
    /// This notification will only be sent if the server is initialized.
    pub async fn work_done_progress_cancel(&self, token: ProgressToken) {
        self.send_notification::<WorkDoneProgressCancel>(WorkDoneProgressCancelParams { token })
            .await;
    }

    /// Sends a `$/setTrace` notification to the client.
    ///
    /// This notification is used to modify the trace setting of the client.
    ///
    /// # Initialization
    ///
    /// This notification will only be sent if the server is initialized.
    pub async fn set_trace(&self, value: TraceValue) {
        self.send_notification::<SetTrace>(SetTraceParams { value })
            .await;
    }
}
