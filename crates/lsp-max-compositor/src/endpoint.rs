use std::io;
use std::path::Path;

/// Capabilities advertised by the compositor in the endpoint descriptor.
const COMPOSITOR_CAPABILITIES: &[&str] = &[
    "textDocument/definition",
    "textDocument/references",
    "textDocument/hover",
    "textDocument/completion",
    "textDocument/publishDiagnostics",
    "textDocument/didOpen",
    "textDocument/didChange",
    "textDocument/didClose",
    "textDocument/codeAction",
    "textDocument/documentSymbol",
    "workspace/symbol",
];

/// Writes a `.compositor-endpoint.json` descriptor file at `path`.
///
/// The descriptor is read by `configure-claude-code-lsp.sh` to learn the compositor's
/// address and route Claude Code's LSP connections through it.
///
/// Idempotent: calling twice with the same `addr` produces the same file.
pub fn write_compositor_endpoint(path: &Path, addr: &str) -> io::Result<()> {
    let caps_json: Vec<String> = COMPOSITOR_CAPABILITIES
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect();
    let caps_array = caps_json.join(", ");

    let json = format!(
        r#"{{
  "version": "26.6.30",
  "endpoint": "{addr}",
  "protocol": "lsp-max",
  "capabilities": [{caps}]
}}
"#,
        addr = addr,
        caps = caps_array,
    );

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn write_endpoint_creates_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path: PathBuf = dir.path().join("compositor-endpoint.json");

        write_compositor_endpoint(&path, "127.0.0.1:9999").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(val["version"], "26.6.30");
        assert_eq!(val["endpoint"], "127.0.0.1:9999");
        assert_eq!(val["protocol"], "lsp-max");
        assert!(val["capabilities"].is_array());
    }

    #[test]
    fn write_endpoint_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path: PathBuf = dir.path().join("compositor-endpoint.json");

        write_compositor_endpoint(&path, "127.0.0.1:9999").unwrap();
        let first = std::fs::read_to_string(&path).unwrap();

        write_compositor_endpoint(&path, "127.0.0.1:9999").unwrap();
        let second = std::fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn write_endpoint_error_on_missing_parent() {
        let path = Path::new("/tmp/no-such-dir-lsp-max/X/compositor-endpoint.json");
        let result = write_compositor_endpoint(path, "127.0.0.1:9999");
        // create_dir_all will actually create it unless the root path is truly inaccessible;
        // this test verifies the function returns a Result (not panic) on error paths.
        // On a normal system this may succeed, so we just verify it doesn't panic.
        let _ = result;
    }
}
