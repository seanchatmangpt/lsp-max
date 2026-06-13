// lsp_echo_server — minimal LSP-speaking test helper binary.
//
// Reads LSP-framed JSON-RPC from stdin, responds to the four lifecycle
// messages, and exits cleanly. Used by e2e tests in place of `cat`.
//
// Protocol:
//   initialize  → {"jsonrpc":"2.0","id":<id>,"result":{"capabilities":{}}}
//   initialized → (notification; no response)
//   shutdown    → {"jsonrpc":"2.0","id":<id>,"result":null}
//   exit        → process::exit(0)
//   <anything else> → ignored

use std::io::{self, BufRead, Read, Write};

fn lsp_frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut reader = io::BufReader::new(stdin.lock());

    loop {
        // Read headers until blank line.
        let mut content_length: usize = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return; // EOF
            }
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            if line.is_empty() {
                break;
            }
            if let Some(rest) = line.strip_prefix("Content-Length: ") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        if content_length == 0 {
            continue;
        }

        // Read body.
        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let msg: serde_json::Value = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();
        let is_notification = id.is_none();

        match method {
            "initialize" => {
                if let Some(id) = id {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "capabilities": {} }
                    });
                    let s = resp.to_string();
                    let frame = lsp_frame(&s);
                    let _ = out.write_all(frame.as_bytes());
                    let _ = out.flush();
                }
            }
            "initialized" => {
                // Notification — no response.
            }
            "shutdown" => {
                if let Some(id) = id {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    let s = resp.to_string();
                    let frame = lsp_frame(&s);
                    let _ = out.write_all(frame.as_bytes());
                    let _ = out.flush();
                }
            }
            "exit" => {
                std::process::exit(0);
            }
            _ => {
                // Unknown notification or request — ignore notifications,
                // respond with method-not-found for requests.
                if !is_notification {
                    if let Some(id) = id {
                        let resp = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": "method not found" }
                        });
                        let s = resp.to_string();
                        let frame = lsp_frame(&s);
                        let _ = out.write_all(frame.as_bytes());
                        let _ = out.flush();
                    }
                }
            }
        }
    }
}
