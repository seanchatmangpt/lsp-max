//! Implementation for document synchronization methods.

use lsp_types_max::*;

pub async fn initialized(_params: InitializedParams) {}

pub async fn did_open(_params: DidOpenTextDocumentParams) {}

pub async fn did_change(_params: DidChangeTextDocumentParams) {}

pub async fn will_save(_params: WillSaveTextDocumentParams) {}

pub async fn did_save(_params: DidSaveTextDocumentParams) {}

pub async fn did_close(_params: DidCloseTextDocumentParams) {}

pub async fn did_change_configuration(_params: DidChangeConfigurationParams) {}

pub async fn did_change_workspace_folders(_params: DidChangeWorkspaceFoldersParams) {}

pub async fn did_change_watched_files(_params: DidChangeWatchedFilesParams) {}
