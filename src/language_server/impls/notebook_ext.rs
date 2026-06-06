//! Handlers for notebookDocument/* LSP 3.17 request methods.

use crate::jsonrpc::Result;
use lsp_types_max::*;

/// Handler for the `notebookDocument/didOpen` notification.
///
/// This notification is sent from the client to the server when a notebook document is opened.
pub async fn did_open_notebook(params: DidOpenNotebookDocumentParams) {
    let _ = params.notebook_document;
    let _ = params.cell_text_documents;
}

/// Handler for the `notebookDocument/didChange` notification.
///
/// This notification is sent from the client to the server when a notebook document changes.
/// The change describes single state change to the notebook document.
pub async fn did_change_notebook(params: DidChangeNotebookDocumentParams) {
    let _ = params.notebook_document;
    let _ = params.change;
}

/// Handler for the `notebookDocument/didSave` notification.
///
/// This notification is sent from the client to the server when a notebook document is saved.
pub async fn did_save_notebook(params: DidSaveNotebookDocumentParams) {
    let _ = params.notebook_document;
}

/// Handler for the `notebookDocument/didClose` notification.
///
/// This notification is sent from the client to the server when a notebook document is closed.
pub async fn did_close_notebook(params: DidCloseNotebookDocumentParams) {
    let _ = params.notebook_document;
    let _ = params.cell_text_documents;
}

/// A manager for Notebook Documents can be implemented by the backend
/// to track the state of opened notebooks and their constituent cells.
pub struct NotebookManager {
    // Implementation would typically use a DashMap or similar for thread-safe state tracking
}

impl NotebookManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_did_open(&self, params: DidOpenNotebookDocumentParams) {
        // Track the notebook and its initial cells
    }

    pub fn handle_did_change(&self, params: DidChangeNotebookDocumentParams) {
        // Apply changes to the tracked notebook state
        // This includes structure changes (adding/removing cells),
        // metadata changes, and cell text content changes.
    }
}
