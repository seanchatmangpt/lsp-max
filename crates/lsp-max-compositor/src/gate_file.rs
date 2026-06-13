// GateFile — single-byte materialized view of last_andon_block.
// Written on every flush so PreToolUse hooks can read it with one syscall.
//
// Path: $XDG_RUNTIME_DIR/lsp-max-gate-{workspace_hash} or /tmp/lsp-max-gate-{workspace_hash}
// Content: b"1" when ANDON is set, b"0" when clear.
//
// The hook check reduces to:
//   [ "$(cat /tmp/lsp-max-gate-...)" = "0" ] || exit 1

use std::path::PathBuf;

pub struct GateFile {
    path: PathBuf,
}

impl GateFile {
    /// Derive a workspace-specific gate file path from the working directory.
    pub fn for_workspace() -> Self {
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let hash = fnv1a(workspace.to_string_lossy().as_bytes());
        let dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        Self {
            path: dir.join(format!("lsp-max-gate-{hash:016x}")),
        }
    }

    /// Construct from an explicit path. The write path uses write-then-rename atomicity:
    /// a sibling `.tmp` file is written first, then `rename(2)` atomically replaces the gate
    /// file. This eliminates the truncate window that would otherwise produce a false CLEAR.
    pub fn from_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Write the current ANDON state. `true` = ANDON set (gate CLOSED), `false` = clear.
    /// Best-effort: errors are logged but do not propagate — gate file loss is advisory,
    /// not fatal. The enforcement path falls back to the LSP query.
    ///
    /// Uses write-then-rename atomicity: the byte is written to a sibling `.tmp` file first,
    /// then `rename(2)` atomically replaces the gate file. A reader racing the write window
    /// always observes either the previous complete state or the new complete state — never
    /// an empty file that would produce a false CLEAR.
    pub fn write(&self, andon: bool) {
        let byte: &[u8] = if andon { b"1" } else { b"0" };
        // Write to sibling tmp file then rename — rename(2) is atomic, eliminates torn-read window.
        let tmp = self.path.with_extension("tmp");
        if let Err(e) = std::fs::write(&tmp, byte) {
            tracing::warn!(path = %tmp.display(), err = %e, "gate-file: tmp write failed, gate unchanged");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &self.path) {
            tracing::warn!(error = %e, "gate-file: rename failed, stale state possible");
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// Read the gate file. Returns `None` if the file does not exist or cannot be read
    /// (compositor not running). Returns `Some(true)` if ANDON is set.
    pub fn read(&self) -> Option<bool> {
        let bytes = std::fs::read(&self.path).ok()?;
        match bytes.first() {
            Some(b'1') => Some(true),
            Some(b'0') => Some(false),
            _ => None,
        }
    }

    /// Remove the gate file on shutdown so stale state does not persist across restarts.
    pub fn remove(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for GateFile {
    fn drop(&mut self) {
        self.remove();
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
