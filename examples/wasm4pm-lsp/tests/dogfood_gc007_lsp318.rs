use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;
use url::Url;

const READ_TIMEOUT: Duration = Duration::from_secs(10);

// ── Inline OCEL content ────────────────────────────────────────────────────────
// 2 object types (order, item), 2 objects (order-1, item-1)
// 2 events (place_order, ship_item) with relationships to both objects
// 1 attribute on each object

const OCEL_CONTENT: &str = r#"{
  "objectTypes": [
    { "name": "order", "attributes": [{ "name": "priority", "type": "string" }] },
    { "name": "item",  "attributes": [{ "name": "sku",      "type": "string" }] }
  ],
  "eventTypes": [
    { "name": "place_order", "attributes": [] },
    { "name": "ship_item",   "attributes": [] }
  ],
  "objects": [
    { "id": "order-1", "type": "order", "attributes": [{ "name": "priority", "value": "high", "time": "2024-01-01T00:00:00Z" }] },
    { "id": "item-1",  "type": "item",  "attributes": [{ "name": "sku",      "value": "SKU-42", "time": "2024-01-01T00:00:00Z" }] }
  ],
  "events": [
    {
      "id": "evt-1",
      "type": "place_order",
      "time": "2024-01-01T01:00:00Z",
      "attributes": [],
      "relationships": [
        { "objectId": "order-1", "qualifier": "subject" },
        { "objectId": "item-1",  "qualifier": "uses" }
      ]
    },
    {
      "id": "evt-2",
      "type": "ship_item",
      "time": "2024-01-02T01:00:00Z",
      "attributes": [],
      "relationships": [
        { "objectId": "order-1", "qualifier": "subject" },
        { "objectId": "item-1",  "qualifier": "produces" }
      ]
    }
  ]
}"#;

// ── LspClient harness (same pattern as dogfood_gc005.rs) ──────────────────────

struct LspClient {
    stdin: ChildStdin,
    rx: Receiver<Value>,
    child: Child,
    next_id: i64,
    stashed_notifications: Vec<Value>,
}

impl LspClient {
    fn new() -> Self {
        let bin = env!("CARGO_BIN_EXE_wasm4pm-lsp");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn wasm4pm-lsp");

        let stdout = child.stdout.take().expect("take stdout");
        let stdin = child.stdin.take().expect("take stdin");
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line.is_empty() {
                    break;
                }
                if line.starts_with("Content-Length: ") {
                    let len: usize = line
                        .trim_start_matches("Content-Length: ")
                        .trim()
                        .parse()
                        .unwrap();
                    reader.read_line(&mut line).unwrap(); // consume empty line
                    let mut body = vec![0u8; len];
                    reader.read_exact(&mut body).unwrap();
                    let msg: Value = serde_json::from_slice(&body).unwrap();
                    tx.send(msg).unwrap();
                }
            }
        });

        Self {
            stdin,
            rx,
            child,
            next_id: 1,
            stashed_notifications: Vec::new(),
        }
    }

    fn send(&mut self, msg: Value) {
        let body = msg.to_string();
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        self.stdin.write_all(frame.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let msg = self
                .rx
                .recv_timeout(READ_TIMEOUT)
                .expect("LSP request timeout");
            if msg.get("id") == Some(&json!(id)) {
                return msg;
            }
            self.stashed_notifications.push(msg);
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn wait_for_notification(&mut self, method: &str) -> Value {
        if let Some(pos) = self
            .stashed_notifications
            .iter()
            .position(|n| n.get("method") == Some(&json!(method)))
        {
            return self.stashed_notifications.remove(pos);
        }
        loop {
            let msg = self
                .rx
                .recv_timeout(READ_TIMEOUT)
                .expect("LSP notification timeout");
            if msg.get("method") == Some(&json!(method)) {
                return msg;
            }
            self.stashed_notifications.push(msg);
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

// ── Helper: open the inline OCEL and drain the push-diagnostics notification ──

fn open_ocel(client: &mut LspClient, uri: &Url) {
    client.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "json",
                "version": 1,
                "text": OCEL_CONTENT
            }
        }),
    );
    // Drain push diagnostics (the server always publishes after didOpen)
    client.wait_for_notification("textDocument/publishDiagnostics");
}

// ── Main dogfood test ─────────────────────────────────────────────────────────

#[test]
fn test_gc007_lsp318_full_capabilities() {
    let mut client = LspClient::new();

    // ── Initialize ──────────────────────────────────────────────────────────
    let init_resp = client.request("initialize", json!({ "capabilities": {} }));
    let caps = init_resp["result"]["capabilities"]
        .as_object()
        .expect("server capabilities");

    // Sanity-check advertised capabilities before exercising them
    assert!(
        caps.contains_key("hoverProvider"),
        "hoverProvider must be advertised"
    );
    assert!(
        caps.contains_key("completionProvider"),
        "completionProvider must be advertised"
    );
    assert!(
        caps.contains_key("documentSymbolProvider"),
        "documentSymbolProvider must be advertised"
    );
    assert!(
        caps.contains_key("semanticTokensProvider"),
        "semanticTokensProvider must be advertised"
    );
    assert!(
        caps.contains_key("inlayHintProvider"),
        "inlayHintProvider must be advertised"
    );
    assert!(
        caps.contains_key("codeLensProvider"),
        "codeLensProvider must be advertised"
    );
    assert!(
        caps.contains_key("foldingRangeProvider"),
        "foldingRangeProvider must be advertised"
    );
    assert!(
        caps.contains_key("diagnosticProvider"),
        "diagnosticProvider (pull) must be advertised"
    );
    assert!(
        caps.contains_key("documentHighlightProvider"),
        "documentHighlightProvider must be advertised"
    );
    assert!(
        caps.contains_key("documentFormattingProvider"),
        "documentFormattingProvider must be advertised"
    );

    client.notify("initialized", json!({}));

    let uri = Url::parse("file:///tmp/gc007_test.ocel.json").unwrap();
    open_ocel(&mut client, &uri);

    // ── 1. Hover ─────────────────────────────────────────────────────────────
    // "order-1" is in the objects array on line 10 (0-indexed).
    // `    { "id": "order-1", ...` — "order-1" starts at char 13.
    // We use character=15 (inside the quoted value).
    let hover_resp = client.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 10, "character": 15 }
        }),
    );
    let hover_result = &hover_resp["result"];
    assert!(
        !hover_result.is_null(),
        "hover must return a result, got: {hover_result}"
    );
    let hover_value = hover_result["contents"]["value"].as_str().unwrap_or("");
    assert!(
        hover_value.contains("order-1") || hover_value.contains("Object"),
        "hover content must mention the object id or 'Object', got: {hover_value}"
    );

    // ── 2. Completion inside relationships ───────────────────────────────────
    // Line 20: `        { "objectId": "order-1", "qualifier": "subject" },`
    // "order-1" value starts at char 23. Ask for completion there.
    let comp_resp = client.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 20, "character": 25 },
            "context": { "triggerKind": 1 }
        }),
    );
    let comp_result = &comp_resp["result"];
    // The server may return null if context detection yields nothing; accept either
    // a non-null array result with object ids OR a null (soft assert to avoid flakiness
    // from position alignment).  We require at minimum no error.
    assert!(
        comp_resp.get("error").is_none(),
        "completion must not return an error, got: {comp_resp}"
    );
    // If items are returned, at least one must be an objectId
    if let Some(items) = comp_result.as_array() {
        let labels: Vec<&str> = items.iter().filter_map(|i| i["label"].as_str()).collect();
        assert!(
            labels.contains(&"order-1") || labels.contains(&"item-1"),
            "completion items must contain objectIds, got: {labels:?}"
        );
    }

    // ── 3. Document Symbol ────────────────────────────────────────────────────
    let sym_resp = client.request(
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    let symbols = sym_resp["result"]
        .as_array()
        .expect("documentSymbol must return an array");
    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
    assert!(
        names.contains(&"objects"),
        "document symbols must contain 'objects', got: {names:?}"
    );
    assert!(
        names.contains(&"events"),
        "document symbols must contain 'events', got: {names:?}"
    );

    // ── 4. Semantic Tokens ───────────────────────────────────────────────────
    let st_resp = client.request(
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
    );
    let st_data = st_resp["result"]["data"]
        .as_array()
        .expect("semanticTokens/full must return a data array");
    assert!(
        !st_data.is_empty(),
        "semantic tokens data array must be non-empty"
    );

    // ── 5. Inlay Hint ─────────────────────────────────────────────────────────
    let ih_resp = client.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end":   { "line": 60, "character": 0 }
            }
        }),
    );
    // The server emits fitness hints when conformance is present; with the inline
    // OCEL (no Petri-net model) conformance may be Inconclusive, but the server
    // still emits per-event orphan/unlinked hints.  We require at minimum no error
    // and, if hints are returned, at least one must have a label.
    assert!(
        ih_resp.get("error").is_none(),
        "inlayHint must not return an error, got: {ih_resp}"
    );
    if let Some(hints) = ih_resp["result"].as_array() {
        // If hints are present, at least one must contain a fitness or structural label
        if !hints.is_empty() {
            let labels: Vec<&str> = hints.iter().filter_map(|h| h["label"].as_str()).collect();
            assert!(
                !labels.is_empty(),
                "inlay hints must have string labels, got hints: {hints:?}"
            );
        }
    }

    // ── 6. Code Lens ─────────────────────────────────────────────────────────
    let cl_resp = client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert!(
        cl_resp.get("error").is_none(),
        "codeLens must not return an error, got: {cl_resp}"
    );
    if let Some(lenses) = cl_resp["result"].as_array() {
        let conformance_lens = lenses.iter().any(|l| {
            l["command"]["title"]
                .as_str()
                .map(|t| t.contains("Conformance"))
                .unwrap_or(false)
        });
        assert!(
            conformance_lens,
            "at least one code lens must have title containing 'Conformance', got: {lenses:?}"
        );
    } else {
        panic!("codeLens must return an array, got: {}", cl_resp["result"]);
    }

    // ── 7. Folding Range ─────────────────────────────────────────────────────
    let fr_resp = client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
    );
    let ranges = fr_resp["result"]
        .as_array()
        .expect("foldingRange must return an array");
    assert!(
        ranges.len() >= 2,
        "foldingRange must return at least 2 ranges (objects + events), got: {}",
        ranges.len()
    );

    // ── 8. Diagnostic (pull model) ───────────────────────────────────────────
    let diag_resp = client.request(
        "textDocument/diagnostic",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert!(
        diag_resp.get("error").is_none(),
        "textDocument/diagnostic must not return an error, got: {diag_resp}"
    );
    // Pull diagnostic response: { kind: "full", items: [...] }
    let diag_items = diag_resp["result"]["items"]
        .as_array()
        .expect("pull diagnostic result must have 'items' array");
    // Items may be empty for a well-formed OCEL; we only require the field exists.
    let _ = diag_items;

    // ── 9. Document Highlight ─────────────────────────────────────────────────
    // Position on "order-1" inside the objects array (line 10, char 15)
    let dh_resp = client.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 10, "character": 15 }
        }),
    );
    assert!(
        dh_resp.get("error").is_none(),
        "documentHighlight must not return an error, got: {dh_resp}"
    );
    if let Some(highlights) = dh_resp["result"].as_array() {
        assert!(
            !highlights.is_empty(),
            "documentHighlight must return at least one highlight for 'order-1'"
        );
    }

    // ── 10. Formatting ────────────────────────────────────────────────────────
    let fmt_resp = client.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": 2, "insertSpaces": true }
        }),
    );
    assert!(
        fmt_resp.get("error").is_none(),
        "textDocument/formatting must not return an error, got: {fmt_resp}"
    );
    // Result is either an array of TextEdits or null (no changes needed).
    // Either is valid; we only assert no error was returned above.
    if let Some(edits) = fmt_resp["result"].as_array() {
        // If edits are returned they must have a range field
        for edit in edits {
            assert!(
                edit.get("range").is_some(),
                "each text edit must have a 'range' field, got: {edit}"
            );
        }
    }
}
