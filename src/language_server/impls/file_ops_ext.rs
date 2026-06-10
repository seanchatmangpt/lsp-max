//! Implementation for Workspace File Operations (WillCreateFiles, DidRenameFiles, etc.).

use crate::jsonrpc::Result;
use lsp_types_max::*;

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn will_create_files(params: CreateFilesParams) -> Result<Option<WorkspaceEdit>> {
    let _ = params.files;
    Ok(None)
}

/// Handler for the `workspace/didCreateFiles` endpoint.
pub async fn did_create_files(params: CreateFilesParams) {
    let _ = params.files;
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn will_rename_files(params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
    let _ = params.files;
    Ok(None)
}

/// Handler for the `workspace/didRenameFiles` endpoint.
pub async fn did_rename_files(params: RenameFilesParams) {
    let _ = params.files;
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn will_delete_files(params: DeleteFilesParams) -> Result<Option<WorkspaceEdit>> {
    let _ = params.files;
    Ok(None)
}

/// Handler for the `workspace/didDeleteFiles` endpoint.
pub async fn did_delete_files(params: DeleteFilesParams) {
    let _ = params.files;
}
