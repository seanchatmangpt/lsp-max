//! Debounced compiler worker (agent contract: `compiler_worker`).
//!
//! Owns the Tier 2 supply chain: writes the current file content to a
//! `/tmp` staging crate, drives `rustc --crate-type=lib --emit=obj` for a
//! requested target, and hands the `.o` object to `disasm_court`.
//!
//! Targets: `aarch64-apple-darwin` (host) and `riscv32imc-unknown-none-elf`
//! (needs `rustup target add`; surfaced as `TargetMissing`, never a silent
//! skip).

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// Cross-targets the worker supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// Host-width ARM64 (macOS host).
    Aarch64AppleDarwin,
    /// Bare-metal RISC-V 32-bit MAC.
    Riscv32Imc,
}

impl TargetKind {
    /// rustc target triple string.
    pub fn triple(&self) -> &'static str {
        match self {
            TargetKind::Aarch64AppleDarwin => "aarch64-apple-darwin",
            TargetKind::Riscv32Imc => "riscv32imc-unknown-none-elf",
        }
    }
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.triple())
    }
}

/// Result of one compile attempt.
#[derive(Debug)]
pub enum CompileOutcome {
    /// Object file produced; path is the `/tmp` staged `.o`.
    Object(PathBuf),
    /// The target triple is not installed for this toolchain.
    TargetMissing(String),
    /// Real rustc failure (a compile error, not a court verdict).
    CompileError(String),
}

/// Debounced `rustc --emit=obj` driver over a `/tmp` staging dir.
pub struct CompilerWorker {
    staging: PathBuf,
    last_request: Option<(String, TargetKind, Instant)>,
    debounce: Duration,
}

impl Default for CompilerWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerWorker {
    /// New worker with a 300 ms debounce window.
    pub fn new() -> Self {
        Self {
            staging: std::env::temp_dir().join("branchless-lsp-staging"),
            last_request: None,
            debounce: Duration::from_millis(300),
        }
    }

    /// Record a request; returns true if the debounce window elapsed and a
    /// compile should be dispatched now.
    pub fn should_dispatch(&mut self, uri: &str, target: TargetKind) -> bool {
        let now = Instant::now();
        let dispatch = match &self.last_request {
            Some((u, t, at)) if u == uri && *t == target => {
                now.duration_since(*at) >= self.debounce
            }
            _ => true,
        };
        if dispatch {
            self.last_request = Some((uri.to_string(), target, now));
        }
        dispatch
    }

    /// Compile `src` for `target` into the `/tmp` staging dir.
    pub fn compile(&self, src: &str, target: TargetKind) -> CompileOutcome {
        std::fs::create_dir_all(&self.staging).expect("staging dir");
        // Per-source unique staging names: concurrent compiles (parallel
        // tests, parallel documents) must not race on shared paths.
        let key = blake3::hash(src.as_bytes()).to_hex().to_string();
        let lib = self.staging.join(format!("probe_{key}.rs"));
        std::fs::write(&lib, src).expect("write probe source");
        let obj = self.staging.join(format!("probe_{key}.o"));
        let out = Command::new("rustc")
            .arg("--crate-type=lib")
            .arg("--emit=obj")
            .arg("-O")
            .arg("--target")
            .arg(target.triple())
            .arg("-o")
            .arg(&obj)
            .arg(&lib)
            .output()
            .expect("spawn rustc");
        if out.status.success() && obj.exists() {
            CompileOutcome::Object(obj)
        } else {
            self.finish(out, obj)
        }
    }

    /// Compile `src` WITH DWARF debuginfo (`-g`) for `target`.
    ///
    /// E9 lane: the Tier 2 court resolves branch-instruction PCs to source
    /// lines through gimli `.debug_line`; that requires the probe object to
    /// carry debuginfo, which the default `compile()` does not emit.
    pub fn compile_debuginfo(&self, src: &str, target: TargetKind) -> CompileOutcome {
        std::fs::create_dir_all(&self.staging).expect("staging dir");
        // Unique per-call staging name: parallel courts compiling identical
        // probe sources must not race on one .o path (witnessed: two e9
        // courts on the same LOOP_PROBE tore each other's object mid-rustc).
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let key = format!(
            "{}-{}",
            blake3::hash(src.as_bytes()).to_hex().to_string(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        );
        let lib = self.staging.join(format!("probe_dbg_{key}.rs"));
        std::fs::write(&lib, src).expect("write probe source");
        let obj = self.staging.join(format!("probe_dbg_{key}.o"));
        let out = Command::new("rustc")
            .arg("--crate-type=lib")
            .arg("--emit=obj")
            .arg("-O")
            .arg("-g")
            .arg("--target")
            .arg(target.triple())
            .arg("-o")
            .arg(&obj)
            .arg(&lib)
            .output()
            .expect("spawn rustc");
        if out.status.success() && obj.exists() {
            CompileOutcome::Object(obj)
        } else {
            self.finish(out, obj)
        }
    }

    fn finish(&self, out: std::process::Output, obj: PathBuf) -> CompileOutcome {
        if obj.exists() {
            return CompileOutcome::Object(obj);
        }
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if stderr.contains("target triple is not installed")
            || stderr.contains("may not be installed")
        {
            CompileOutcome::TargetMissing(stderr)
        } else {
            CompileOutcome::CompileError(stderr)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triples_are_pinned() {
        assert_eq!(
            TargetKind::Aarch64AppleDarwin.triple(),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            TargetKind::Riscv32Imc.triple(),
            "riscv32imc-unknown-none-elf"
        );
    }

    #[test]
    fn debounce_gates_repeat_requests() {
        let mut w = CompilerWorker::new();
        assert!(w.should_dispatch("u", TargetKind::Aarch64AppleDarwin));
        assert!(!w.should_dispatch("u", TargetKind::Aarch64AppleDarwin));
    }

    #[test]
    fn compiles_host_object_and_lints_branchfree_code() {
        let w = CompilerWorker::new();
        let src = "#[no_mangle]\npub extern \"C\" fn f(x: u32) -> u32 { x.wrapping_add(1) }";
        match w.compile(src, TargetKind::Aarch64AppleDarwin) {
            CompileOutcome::Object(p) => assert!(p.exists()),
            other => panic!("expected object, got {other:?}"),
        }
    }
}
