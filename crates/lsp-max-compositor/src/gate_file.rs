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
    ///
    /// Fail-closed behaviour: if the gate file is absent but a heartbeat file exists
    /// and is stale (> 30 s), the compositor was previously running and has crashed.
    /// In that case return `Some(true)` (BLOCKED) rather than `None` (CLEAR), so that
    /// the Λ_CD constraint is upheld even when the compositor process is gone.
    pub fn read(&self) -> Option<bool> {
        match std::fs::read(&self.path).ok() {
            Some(bytes) => match bytes.first() {
                Some(b'1') => Some(true),
                Some(b'0') => Some(false),
                _ => None,
            },
            None => {
                // Gate file absent. Check heartbeat to distinguish:
                //   - compositor never started → CLEAR (None)
                //   - compositor crashed        → BLOCKED (Some(true))
                if self.heartbeat_is_stale() {
                    Some(true)
                } else {
                    None
                }
            }
        }
    }

    /// Write a heartbeat timestamp to `<gate>.heartbeat`. Called periodically by the
    /// compositor to prove liveness. The gate check uses this to detect a compositor
    /// crash (heartbeat present but stale) and fail-closed.
    ///
    /// Uses write-then-rename atomicity identical to `write()`.
    pub fn write_heartbeat(&self) {
        let hb_path = self.path.with_extension("heartbeat");
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let tmp = hb_path.with_extension("heartbeat.tmp");
        if let Err(e) = std::fs::write(&tmp, ts.to_string()) {
            tracing::warn!(path = %tmp.display(), err = %e, "gate-file: heartbeat tmp write failed");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &hb_path) {
            tracing::warn!(error = %e, "gate-file: heartbeat rename failed");
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// Returns `true` if the heartbeat file exists and its timestamp is more than 30 s old.
    /// Returns `false` if the heartbeat file is absent (compositor never started) or is fresh.
    pub fn heartbeat_is_stale(&self) -> bool {
        let hb_path = self.path.with_extension("heartbeat");
        match std::fs::read_to_string(&hb_path) {
            Err(_) => false, // no heartbeat file — compositor never started
            Ok(s) => {
                let written_ts: u64 = s.trim().parse().unwrap_or(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now.saturating_sub(written_ts) > 30
            }
        }
    }

    /// Remove the gate file on shutdown so stale state does not persist across restarts.
    pub fn remove(&self) {
        let _ = std::fs::remove_file(&self.path);
        // Also remove the heartbeat file so a clean shutdown does not appear as a crash
        // to the next gate check before a new compositor starts.
        let hb_path = self.path.with_extension("heartbeat");
        let _ = std::fs::remove_file(&hb_path);
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
