// gate_path — sole authoritative derivation of the Λ_CD gate file path.
//
// Both the compositor (gate writer/reader) and external consumers such as
// ggen-lsp route through this one function so the path formula cannot diverge
// by construction. Status of divergence: UNCONSTRUCTABLE — there is exactly
// one site that computes it.
//
// Path: $XDG_RUNTIME_DIR/lsp-max-gate-{fnv1a(cwd):016x}, or /tmp when
// $XDG_RUNTIME_DIR is unset. Computing or writing this runtime-state path is
// permitted; it is not a source mutation.

use std::path::PathBuf;

/// FNV-1a 64-bit over raw bytes. Internal to the gate-path derivation.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Derive the workspace-specific Λ_CD gate file path from the current working
/// directory. This is the single authoritative formula; the compositor's
/// `GateFile::for_workspace()` and ggen-lsp both call it.
pub fn gate_file_path() -> PathBuf {
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let hash = fnv1a(workspace.to_string_lossy().as_bytes());
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    dir.join(format!("lsp-max-gate-{hash:016x}"))
}
