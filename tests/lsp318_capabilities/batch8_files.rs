use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types as lsp;
use lsp_max::max_protocol::lsp_3_18 as lsp318;
use lsp_max::{Client, LspService, Server};
/// Batch 8 — workspace/did*Files, workspace/willRenameFiles,
/// workspace/willDeleteFiles, workspace/textDocumentContent,
/// notebookDocument/didOpen, notebookDocument/didChange.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use super::common::{assert_well_formed, encode_message, read_message, roundtrip};

#[derive(Clone, Default)]
struct Batch8Events {
    did_create_files: Arc<Mutex<Option<lsp::CreateFilesParams>>>,
    did_rename_files: Arc<Mutex<Option<lsp::RenameFilesParams>>>,
    did_delete_files: Arc<Mutex<Option<lsp::DeleteFilesParams>>>,
    did_open_notebook: Arc<Mutex<Option<lsp::DidOpenNotebookDocumentParams>>>,
    did_change_notebook: Arc<Mutex<Option<lsp::DidChangeNotebookDocumentParams>>>,
}

struct Batch8Backend {
    #[allow(dead_code)]
    client: Client,
    events: Batch8Events,
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for Batch8Backend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn initialized(&self, _: lsp::InitializedParams) {}
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn text_document_content(
        &self,
        _params: lsp318::TextDocumentContentParams,
    ) -> Result<lsp318::TextDocumentContentResult> {
        Ok(lsp318::TextDocumentContentResult {
            text: "batch8-stub".to_string(),
        })
    }

    async fn did_create_files(&self, params: lsp::CreateFilesParams) {
        *self.events.did_create_files.lock().unwrap() = Some(params);
    }
    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        *self.events.did_rename_files.lock().unwrap() = Some(params);
    }
    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        *self.events.did_delete_files.lock().unwrap() = Some(params);
    }
    async fn did_open_notebook_document(&self, params: lsp::DidOpenNotebookDocumentParams) {
        *self.events.did_open_notebook.lock().unwrap() = Some(params);
    }
    async fn did_change_notebook_document(&self, params: lsp::DidChangeNotebookDocumentParams) {
        *self.events.did_change_notebook.lock().unwrap() = Some(params);
    }
}

async fn batch8_start(
    events: Batch8Events,
) -> (
    tokio::io::DuplexStream,
    tokio::io::BufReader<tokio::io::DuplexStream>,
) {
    let ev = events.clone();
    let (service, socket) = LspService::new(|client| Batch8Backend { client, events: ev });
    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    tokio::spawn(server.serve(service));
    let mut tx = client_tx;
    let mut reader = tokio::io::BufReader::new(client_rx);

    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
    });
    tx.write_all(&encode_message(&init)).await.unwrap();
    let _r = tokio::time::timeout(Duration::from_millis(500), read_message(&mut reader))
        .await
        .unwrap()
        .unwrap();
    let ntf = serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}});
    tx.write_all(&encode_message(&ntf)).await.unwrap();
    (tx, reader)
}

async fn batch8_request(
    tx: &mut tokio::io::DuplexStream,
    reader: &mut tokio::io::BufReader<tokio::io::DuplexStream>,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let req = serde_json::json!({"jsonrpc":"2.0","id":2,"method":method,"params":params});
    tx.write_all(&encode_message(&req)).await.unwrap();
    loop {
        let msg = tokio::time::timeout(Duration::from_millis(500), read_message(reader))
            .await
            .unwrap()
            .unwrap();
        if msg.get("id").and_then(|i| i.as_i64()) == Some(2) {
            return msg;
        }
    }
}

async fn wait_flag_b8<T: Clone>(flag: &Arc<Mutex<Option<T>>>, label: &str) -> T {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > Duration::from_millis(300) {
            panic!("Timeout waiting for {}", label);
        }
        if let Some(v) = flag.lock().unwrap().clone() {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_will_rename_files_dispatch() {
    let params = serde_json::json!({
        "files": [{ "oldUri": "file:///a.rs", "newUri": "file:///b.rs" }]
    });
    let resp = roundtrip("workspace/willRenameFiles", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_will_delete_files_dispatch() {
    let params = serde_json::json!({ "files": [{ "uri": "file:///dead.rs" }] });
    let resp = roundtrip("workspace/willDeleteFiles", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_text_document_content_dispatch() {
    let events = Batch8Events::default();
    let (mut tx, mut reader) = batch8_start(events).await;
    let params = serde_json::json!({ "textDocument": { "uri": "file:///batch8.rs" } });
    let resp = batch8_request(
        &mut tx,
        &mut reader,
        "workspace/textDocumentContent",
        params,
    )
    .await;
    assert_eq!(resp.get("id").and_then(|i| i.as_i64()), Some(2));
    assert!(resp.get("error").is_none(), "Unexpected error: {:?}", resp);
    let result = resp.get("result").expect("Expected result field");
    assert_eq!(
        result.get("text").and_then(|t| t.as_str()),
        Some("batch8-stub")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_did_create_files_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_create_files.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didCreateFiles",
        "params": { "files": [{ "uri": "file:///created.rs" }] }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "workspace/didCreateFiles").await;
    assert_eq!(got.files.len(), 1);
    assert_eq!(got.files[0].uri, "file:///created.rs");
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_did_rename_files_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_rename_files.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didRenameFiles",
        "params": { "files": [{ "oldUri": "file:///x.rs", "newUri": "file:///y.rs" }] }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "workspace/didRenameFiles").await;
    assert_eq!(got.files.len(), 1);
    assert_eq!(got.files[0].old_uri, "file:///x.rs");
    assert_eq!(got.files[0].new_uri, "file:///y.rs");
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_did_delete_files_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_delete_files.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didDeleteFiles",
        "params": { "files": [{ "uri": "file:///gone.rs" }] }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "workspace/didDeleteFiles").await;
    assert_eq!(got.files.len(), 1);
    assert_eq!(got.files[0].uri, "file:///gone.rs");
}

#[tokio::test(flavor = "current_thread")]
async fn test_notebook_document_did_open_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_open_notebook.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notebookDocument/didOpen",
        "params": {
            "notebookDocument": {
                "uri": "file:///batch8.ipynb",
                "notebookType": "jupyter",
                "version": 1,
                "metadata": {},
                "cells": []
            },
            "cellTextDocuments": []
        }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "notebookDocument/didOpen").await;
    assert_eq!(got.notebook_document.uri.as_str(), "file:///batch8.ipynb");
    assert_eq!(got.notebook_document.notebook_type, "jupyter");
}

#[tokio::test(flavor = "current_thread")]
async fn test_notebook_document_did_change_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_change_notebook.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notebookDocument/didChange",
        "params": {
            "notebookDocument": {
                "version": 2,
                "uri": "file:///batch8.ipynb"
            },
            "change": {}
        }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "notebookDocument/didChange").await;
    assert_eq!(got.notebook_document.uri.as_str(), "file:///batch8.ipynb");
    assert_eq!(got.notebook_document.version, 2);
}
