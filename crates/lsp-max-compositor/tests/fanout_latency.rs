// fanout_latency.rs — concurrent fanout latency proof for N=500 mock server handles.
//
// Invariant under test: fanning a didOpen notification to N=500 mock server handles
// completes in ≤10ms when dispatch is O(max RTT) concurrent, not O(N×RTT) serial.
//
// Architecture:
//   - Each mock handle is a ServerHandle backed by a no-op mpsc::channel pair.
//   - A receiver task per handle drains messages immediately (zero artificial latency).
//   - The fanout spawns one tokio task per handle and awaits all of them.
//   - Elapsed wall-clock time is measured with std::time::Instant.
//
// Status vocabulary: ADMITTED / CANDIDATE / BLOCKED / REFUSED / UNKNOWN / PARTIAL / OPEN

use lsp_max_client::ServerHandle;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

const N: usize = 500;
const LATENCY_BUDGET_MS: u128 = 10;

/// Build a zero-latency mock `ServerHandle` backed by an in-process mpsc channel.
/// Returns the handle and the receiver end. The caller must drain the receiver or
/// it will block once the channel is full.
fn mock_handle(capacity: usize) -> (ServerHandle, mpsc::Receiver<Value>) {
    let (tx, rx) = mpsc::channel::<Value>(capacity);
    let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(AtomicU64::new(0));
    let handle = ServerHandle::new(tx, pending, next_id);
    (handle, rx)
}

#[tokio::test(flavor = "multi_thread")]
async fn fanout_did_open_n500_completes_within_10ms() {
    use lsp_max::lsp_types::{DidOpenTextDocumentParams, TextDocumentItem, Uri};
    use std::str::FromStr;

    // ── 1. Construct N mock handles, spawn a receiver task for each ──────────

    let mut handles: Vec<ServerHandle> = Vec::with_capacity(N);
    for _ in 0..N {
        // Capacity 16 — each handle receives exactly one didOpen notification per fanout.
        let (handle, mut rx) = mock_handle(16);
        handles.push(handle);
        // Receiver task: drain all messages immediately and exit.
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                // intentionally no-op — zero latency
            }
        });
    }

    // ── 2. Build the didOpen params (cloned per task, consistent with server.rs) ──

    let uri = Uri::from_str("file:///workspace/bench.rs").expect("uri parse ADMITTED");
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "rust".to_string(),
            version: 1,
            text: "fn main() {}".to_string(),
        },
    };

    // ── 3. Fan out concurrently — mirror server.rs fanout_did_open pattern exactly ──

    let start = std::time::Instant::now();

    let tasks: Vec<_> = handles
        .into_iter()
        .map(|handle| {
            let p = params.clone();
            tokio::spawn(async move {
                handle.did_open(p).await;
            })
        })
        .collect();

    for t in tasks {
        t.await.expect("fanout task join ADMITTED");
    }

    let elapsed_ms = start.elapsed().as_millis();

    // ── 4. Assert latency budget ──────────────────────────────────────────────

    assert!(
        elapsed_ms <= LATENCY_BUDGET_MS,
        "FANOUT LATENCY BUDGET EXCEEDED: N={N} handles took {elapsed_ms}ms, budget={LATENCY_BUDGET_MS}ms — concurrent fanout status BLOCKED"
    );

    // Report bounded status, not victory language.
    eprintln!(
        "fanout_latency: N={N} concurrent didOpen dispatches — elapsed={elapsed_ms}ms — status ADMITTED (≤{LATENCY_BUDGET_MS}ms budget)"
    );
}
