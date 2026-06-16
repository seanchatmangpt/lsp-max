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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_empty_bytes_returns_offset_basis() {
        assert_eq!(fnv1a(b""), 0xcbf29ce484222325u64);
    }

    #[test]
    fn fnv1a_deterministic_same_input_same_output() {
        assert_eq!(fnv1a(b"lsp-max"), fnv1a(b"lsp-max"));
    }

    #[test]
    fn fnv1a_distinct_inputs_differ() {
        assert_ne!(fnv1a(b"foo"), fnv1a(b"bar"));
    }

    #[test]
    fn gate_file_path_contains_lsp_max_gate_prefix() {
        let path = gate_file_path();
        assert!(path.to_string_lossy().contains("lsp-max-gate-"));
    }

    #[test]
    fn gate_file_path_has_sixteen_hex_char_suffix() {
        let path = gate_file_path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let suffix = name.strip_prefix("lsp-max-gate-").expect("missing lsp-max-gate- prefix");
        assert_eq!(suffix.len(), 16, "hash suffix must be 16 hex chars");
        assert!(
            suffix.chars().all(|c| c.is_ascii_hexdigit()),
            "suffix must be lowercase hex, got: {suffix}"
        );
    }

    #[test]
    fn gate_file_path_is_deterministic() {
        assert_eq!(gate_file_path(), gate_file_path());
    }
}
