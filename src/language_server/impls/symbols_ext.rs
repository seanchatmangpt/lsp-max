//! Implementation for Advanced Symbols (WorkspaceSymbolResolve, CodeActionCapabilityResolveSupport).

use crate::jsonrpc::Result;
use lsp_types_max::{CodeAction, WorkspaceSymbol};

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn symbol_resolve(params: WorkspaceSymbol) -> Result<WorkspaceSymbol> {
    Ok(params)
}

/// Handler for the `codeAction/resolve` endpoint.
///
/// This method resolves additional information for a code action,
/// such as its workspace edit, if it was lazily requested by the client.
pub async fn code_action_resolve(params: CodeAction) -> Result<CodeAction> {
    Ok(params)
}
