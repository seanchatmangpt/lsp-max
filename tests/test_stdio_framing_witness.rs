//! PERMANENT behavioral stdio-framing witness.
//!
//! This witness catches LSP Content-Length frame corruption from ANY source:
//! a stray `print!`, a dependency writing to stdout, or a panic backtrace
//! leaking into the transport. The `#![deny(clippy::print_stdout)]` lint only
//! catches literal `print!`/`println!`; it cannot see runtime stdout writes.
//!
//! Every in-process integration test uses `tokio::io::duplex()`, which bypasses
//! real OS stdout and is structurally blind to this defect class. This test
//! spawns the `examples/stdio` server as a REAL subprocess whose transport is
//! real OS stdout, writes framed JSON-RPC to its stdin, and reads RAW stdout
//! bytes back.
//!
//! The LSP surface is read-only; this test only READS the child's stdout.
//!
//! Assertion outcomes use bounded statuses (ADMITTED / BLOCKED), never victory
//! language.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Frame an object as `Content-Length: N\r\n\r\n` + body.
fn frame(obj: &serde_json::Value) -> Vec<u8> {
    let body = serde_json::to_vec(obj).expect("serialize body");
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(&body);
    out
}

/// Locate the pre-built example binary, building it on demand if absent.
fn stdio_example_bin() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bin = manifest.join("target/debug/examples/stdio");
    if !bin.exists() {
        let status = Command::new("cargo")
            .args(["build", "-q", "--example", "stdio"])
            .current_dir(&manifest)
            .status()
            .expect("spawn cargo build --example stdio");
        assert!(status.success(), "BLOCKED: failed to build example stdio");
    }
    assert!(
        bin.exists(),
        "BLOCKED: example binary not found at {}",
        bin.display()
    );
    bin
}

/// A single parsed frame: declared length plus decoded JSON body.
struct ParsedFrame {
    json: serde_json::Value,
}

/// Parse a raw byte stream as a sequence of Content-Length frames.
/// Returns the parsed frames, or `Err(defect)` describing the first violation.
fn parse_frames(out: &[u8]) -> Result<Vec<ParsedFrame>, String> {
    let mut frames = Vec::new();
    let mut cursor = 0usize;
    while cursor < out.len() {
        // Find the header/body separator.
        let sep = find_subslice(&out[cursor..], b"\r\n\r\n").map(|i| cursor + i);
        let sep = match sep {
            Some(s) => s,
            None => {
                let trailing = &out[cursor..];
                let non_ws: Vec<u8> = trailing
                    .iter()
                    .copied()
                    .filter(|b| !b.is_ascii_whitespace())
                    .collect();
                if non_ws.is_empty() {
                    break;
                }
                return Err(format!(
                    "trailing non-frame bytes at {cursor}: {:?}",
                    String::from_utf8_lossy(&trailing[..trailing.len().min(120)])
                ));
            }
        };
        let header = String::from_utf8_lossy(&out[cursor..sep]).into_owned();
        let mut content_length: Option<usize> = None;
        for line in header.split("\r\n") {
            if line.is_empty() {
                continue;
            }
            let (k, v) = match line.split_once(':') {
                Some(kv) => kv,
                None => {
                    return Err(format!(
                        "malformed header line (not a Content-* field): {line:?}"
                    ))
                }
            };
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = Some(
                    v.trim()
                        .parse()
                        .map_err(|e| format!("invalid Content-Length: {e}"))?,
                );
            }
        }
        let clen = content_length
            .ok_or_else(|| format!("frame without Content-Length header: {header:?}"))?;
        let body_start = sep + 4;
        let body_end = body_start + clen;
        if body_end > out.len() {
            return Err(format!(
                "declared Content-Length {clen} exceeds remaining bytes {}",
                out.len() - body_start
            ));
        }
        let body = &out[body_start..body_end];
        let json: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| format!("frame body not valid JSON of declared length: {e}"))?;
        frames.push(ParsedFrame { json });
        cursor = body_end;
    }
    Ok(frames)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[test]
fn stdio_framing_witness() {
    let bin = stdio_example_bin();

    let messages: Vec<u8> = [
        frame(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "processId": std::process::id(), "capabilities": {} }
        })),
        frame(&serde_json::json!({
            "jsonrpc": "2.0", "method": "initialized", "params": {}
        })),
        frame(&serde_json::json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": { "textDocument": {
                "uri": "file:///tmp/lsp_max_witness/sample.txt",
                "languageId": "plaintext", "version": 1, "text": "hello\n"
            }}
        })),
        frame(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null
        })),
        frame(&serde_json::json!({
            "jsonrpc": "2.0", "method": "exit", "params": null
        })),
    ]
    .concat();

    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("BLOCKED: spawn stdio example subprocess");

    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(&messages)
        .expect("write framed messages to child stdin");

    // Read RAW stdout bytes on a thread so a misbehaving child cannot wedge us.
    let mut stdout = child.stdout.take().expect("child stdout");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });

    // Bounded wait for exit (the `exit` notification should terminate it).
    let mut status = None;
    for _ in 0..300 {
        match child.try_wait().expect("try_wait") {
            Some(s) => {
                status = Some(s);
                break;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let status = match status {
        Some(s) => s,
        None => {
            let _ = child.kill();
            panic!("BLOCKED: child did not exit within 30s after `exit` notification");
        }
    };

    let out = reader.join().expect("join stdout reader thread");

    // (a) The stream MUST begin with a Content-Length frame header.
    assert!(
        out.len() >= 15 && out[..15].eq_ignore_ascii_case(b"content-length:"),
        "stdout did not begin with Content-Length frame header; first 60 bytes: {:?}",
        String::from_utf8_lossy(&out[..out.len().min(60)])
    );

    // (b)+(c) Every frame header parses, every body is valid JSON of the
    // declared length, and there is NO log-line interleaving (any non-header
    // bytes between/around frames surface as a parse defect).
    let frames = match parse_frames(&out) {
        Ok(f) => f,
        Err(defect) => panic!("framing defect (stdout corruption): {defect}"),
    };

    // (d) At least the initialize response (id 1) with a result is present.
    let init_response = frames.iter().any(|f| {
        f.json.get("id").and_then(|i| i.as_i64()) == Some(1) && f.json.get("result").is_some()
    });
    assert!(
        init_response,
        "initialize response (id 1) absent from {} parsed frames",
        frames.len()
    );

    // (e) Regression witness for Track C: a compliant shutdown+exit must yield
    // process exit code 0. This assertion may be BLOCKED until Track C lands.
    assert_eq!(
        status.code(),
        Some(0),
        "child exit code was {:?}, expected 0 (compliant shutdown+exit). \
         Framing assertions ADMITTED; exit-code witness awaits Track C.",
        status.code()
    );
}
