//! Probe v3: run the REAL `run_stdio` compositor loop in a subprocess (this
//! test binary re-executes itself in compositor role), drive it over stdio,
//! and assert child diagnostics relay to the editor side.

use lsp_max_compositor::{CompositorConfig, ExtensionRouter, MergeContext};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Resolve the external claude-code-config-lsp binary used as the compositor
/// child server. Order: explicit env override, then a fallback search over the
/// known workspace locations. `None` ⇒ caller must SKIP loudly — never fake a
/// pass against a hardcoded path.
fn resolve_child_bin() -> Option<String> {
    if let Ok(override_bin) = std::env::var("CCC_PROBE_CHILD_BIN") {
        if !override_bin.trim().is_empty() && std::path::Path::new(&override_bin).exists() {
            return Some(override_bin);
        }
    }
    let candidates = [
        "/Users/sac/claude-code-config-lsp/target/release/claude-code-config-lsp",
        "/Users/sac/claude-code-config-lsp/target/debug/claude-code-config-lsp",
        "/Users/sac/claude-code-config-lsp/dist/claude-code-config-lsp-26.6.24",
    ];
    candidates
        .iter()
        .find(|c| std::path::Path::new(c).exists())
        .map(|c| c.to_string())
}

#[tokio::test]
async fn run_stdio_relay_works_end_to_end() {
    if std::env::var("CCC_PROBE_ROLE").is_ok() {
        return; // child-role processes never re-enter this test
    }
    let Some(child_bin) = resolve_child_bin() else {
        eprintln!(
            "[SKIP] diagnostic_relay_probe::run_stdio_relay_works_end_to_end: \
             external claude-code-config-lsp binary not found — set \
             CCC_PROBE_CHILD_BIN or build /Users/sac/claude-code-config-lsp \
             (cargo build --release). SKIP is typed and loud, not a pass."
        );
        return;
    };
    let self_exe = std::env::current_exe().unwrap();
    let fixture = std::env::temp_dir().join("ccc-probe3-ws");
    let _ = std::fs::create_dir_all(fixture.join(".claude"));
    let toml = format!(
        "[[server]]\nid = \"ccc-child\"\nprimary_extensions = [\".json\", \".md\"]\nsecondary_extensions = []\npriority = \"diagnostics-only\"\nandon_code_prefixes = [\"CCC-\"]\ncommand = \"{child_bin}\"\nargs = [\"serve\", \"--stdio\"]\n"
    );
    std::fs::write(fixture.join("lsp-max.toml"), toml).unwrap();

    let mut child = Command::new(&self_exe)
        .env("CCC_PROBE_ROLE", "1")
        .env("CCC_PROBE_WS", &fixture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn self as compositor");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).is_err() || line.is_empty() {
                break;
            }
            if let Some(rest) = line.strip_prefix("Content-Length: ") {
                let len: usize = rest.trim().parse().unwrap();
                let mut term = [0u8; 2];
                let _ = r.read_exact(&mut term);
                let mut body = vec![0u8; len];
                if r.read_exact(&mut body).is_err() {
                    break;
                }
                if tx.send(serde_json::from_slice(&body).unwrap()).is_err() {
                    break;
                }
            }
        }
    });

    macro_rules! send {
        ($v:expr) => {{
            let body = serde_json::to_string(&$v).unwrap();
            stdin
                .write_all(format!("Content-Length: {}\r\n\r\n{}", body.len(), body).as_bytes())
                .unwrap();
            stdin.flush().unwrap();
        }};
    }
    macro_rules! recv_until {
        ($pred:expr, $secs:expr, $label:expr) => {{
            let deadline = Instant::now() + Duration::from_secs($secs);
            let mut got = None;
            while Instant::now() < deadline {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(m) => {
                        if $pred(&m) {
                            got = Some(m);
                            break;
                        }
                    }
                    Err(_) => {}
                }
            }
            got.unwrap_or_else(|| panic!("{}: nothing matching arrived", $label))
        }};
    }

    let t_init = std::time::Instant::now();
    send!(
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{},"rootUri": fixture}})
    );
    let init = recv_until!(
        |m: &serde_json::Value| m.get("id") == Some(&serde_json::json!(1)),
        20,
        "init response"
    );
    eprintln!(
        "PROBE3: init response after {:?}: {}",
        t_init.elapsed(),
        init
    );
    send!(serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    send!(
        serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri": format!("file://{}/settings.json", fixture.display()), "languageId":"json","version":1,"text":"{\"model\": \"gpt-4\"}"}}})
    );
    let t0 = std::time::Instant::now();
    let got = recv_until!(
        |m: &serde_json::Value| {
            m.get("method") == Some(&serde_json::json!("textDocument/publishDiagnostics"))
                && m.to_string().contains("CCC-JSON-004")
        },
        20,
        "publishDiagnostics"
    );
    eprintln!("PROBE3: init+didOpen round trips took {:?}", t0.elapsed());
    assert!(
        got.to_string().contains("CCC-JSON-004"),
        "relay body: {got}"
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test(flavor = "multi_thread")]
async fn compositor_role() {
    if std::env::var("CCC_PROBE_ROLE").is_err() {
        return; // skip in normal test mode
    }
    let ws = std::env::var("CCC_PROBE_ROLE_WS")
        .unwrap_or_else(|_| std::env::var("CCC_PROBE_WS").unwrap());
    std::env::set_current_dir(&ws).unwrap();
    let config = CompositorConfig::load().expect("walk-up config load");
    let router = ExtensionRouter::from_config(&config);
    let merge_ctx = MergeContext::from_config(&config);
    lsp_max_compositor::server::run_stdio(router, merge_ctx, std::sync::Arc::new(config)).await;
}
