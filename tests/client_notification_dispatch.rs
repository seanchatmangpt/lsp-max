//! Integration tests for client notification dispatch.
//!
//! Chicago-style: real duplex streams drive the client read loop with
//! LSP-framed notifications; assertions run against a real `LanguageClient`
//! that records received diagnostics. No mocks.

use lsp_max::client::{ClientBuilder, LanguageClient};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Real client that records every publishDiagnostics params it receives.
/// The log is shared via Arc<Mutex<_>> so the test can poll it after the
/// client value is moved into the read loop.
#[derive(Clone, Default)]
struct RecordingClient {
    diagnostics: Arc<Mutex<Vec<Value>>>,
}

impl RecordingClient {
    fn log(&self) -> Arc<Mutex<Vec<Value>>> {
        Arc::clone(&self.diagnostics)
    }
}

#[async_trait::async_trait]
impl LanguageClient for RecordingClient {
    async fn publish_diagnostics(&self, params: lsp_max::lsp_types::PublishDiagnosticsParams) {
        self.diagnostics
            .lock()
            .expect("lock")
            .push(serde_json::to_value(&params).expect("serialize params"));
    }
}

/// Frame a JSON-RPC notification with LSP Content-Length framing.
fn frame(msg: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(msg).expect("serialize frame");
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(&body);
    out
}

/// Drive the real read loop over a duplex stream. Returns the feed writer so
/// the stream stays open while the test polls the client's recorded state.
async fn drive_read_loop(
    client: RecordingClient,
    msgs: &[Value],
) -> tokio::io::DuplexStream {
    let (mut feed_writer, feed_reader) = tokio::io::duplex(4096);
    let (_, discard_output) = tokio::io::duplex(64);
    let _handle = ClientBuilder::new().build(client, feed_reader, discard_output);
    for m in msgs {
        feed_writer
            .write_all(&frame(m))
            .await
            .expect("write frame");
    }
    feed_writer.flush().await.expect("flush");
    feed_writer
}

/// Poll the recorded diagnostics until non-empty or the deadline passes.
async fn wait_for_diagnostics(log: &Mutex<Vec<Value>>) -> Vec<Value> {
    for _ in 0..200 {
        {
            let q = log.lock().expect("lock").clone();
            if !q.is_empty() {
                return q;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    log.lock().expect("lock").clone()
}

/// A compositor-multiplexed publishDiagnostics with andon-prefixed message must
/// reach the client with its params unchanged.
#[tokio::test]
async fn publish_diagnostics_reaches_client_with_intact_params() {
    let client = RecordingClient::default();
    let params = json!({
        "uri": "file:///w/CCC-JSON-001.json",
        "diagnostics": [
            {
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 12}
                },
                "severity": 1,
                "source": "lsp-max-compositor[anti-llm-cheat-lsp]",
                "message": "[andon:anti-llm-cheat-lsp] FORBIDDEN: plain tower-lsp reference"
            }
        ]
    });
    let msg = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": params
    });
    let log = client.log();
    let _feed = drive_read_loop(client, &[msg]).await;
    let got = wait_for_diagnostics(&log).await;
    assert!(
        !got.is_empty(),
        "publishDiagnostics never reached the client"
    );
    assert_eq!(
        got[0], params,
        "params must pass through unchanged (andon prefix intact)"
    );
}

/// Unknown notifications are dropped with a log; the session stays alive and
/// later valid notifications still land.
#[tokio::test]
async fn unknown_notification_dropped_session_alive() {
    let client = RecordingClient::default();
    let msgs = vec![
        json!({
            "jsonrpc": "2.0",
            "method": "$/anUnknownMethod",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///a.json",
                "diagnostics": []
            }
        }),
    ];
    let log = client.log();
    let _feed = drive_read_loop(client, &msgs).await;
    let got = wait_for_diagnostics(&log).await;
    assert!(
        !got.is_empty(),
        "session must survive unknown notifications"
    );
    assert_eq!(got[0]["uri"], "file:///a.json");
}

/// A framed body that is not valid JSON is skipped loudly; the read loop
/// continues and a following valid notification is dispatched.
#[tokio::test]
async fn malformed_frame_skipped_session_alive() {
    let client = RecordingClient::default();
    let log = client.log();
    let (mut feed_writer, feed_reader) = tokio::io::duplex(4096);
    let (_, discard_output) = tokio::io::duplex(64);
    let _handle = ClientBuilder::new().build(client, feed_reader, discard_output);

    feed_writer
        .write_all(b"Content-Length: 15\r\n\r\n{not json here}")
        .await
        .expect("write bad frame");
    feed_writer
        .write_all(&frame(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": "file:///b.json", "diagnostics": []}
        })))
        .await
        .expect("write good frame");
    feed_writer.flush().await.expect("flush");

    let got = wait_for_diagnostics(&log).await;
    assert!(
        !got.is_empty(),
        "read loop must survive a malformed frame"
    );
    assert_eq!(got[0]["uri"], "file:///b.json");
}

/// Malformed params on a known method are warned and dropped; the session
/// stays alive.
#[tokio::test]
async fn malformed_params_dropped_session_alive() {
    let client = RecordingClient::default();
    let msgs = vec![
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"no_uri_field": true}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": "file:///c.json", "diagnostics": []}
        }),
    ];
    let log = client.log();
    let _feed = drive_read_loop(client, &msgs).await;
    let got = wait_for_diagnostics(&log).await;
    assert_eq!(got.len(), 1, "malformed params must be dropped, valid ones dispatched");
    assert_eq!(got[0]["uri"], "file:///c.json");
}
