//! ANDON invariant: `STALE_LSIF_INDEX = STOP`
//!
//! A stale LSIF index is worse than a missing one:
//! - Missing LSIF says "I do not know" → STOP (safe)
//! - Stale LSIF says "I know" while lying → STOP (dangerous)
//!
//! This invariant evaluates whether the LSIF output file on disk
//! still matches the digest recorded in its receipt.  If either the
//! LSIF file or the receipt has been mutated since the last admitted
//! index run, the state is `StaleLsifDigest` — which maps to ANDON
//! severity STOP, blocking admission.

use std::path::Path;

// ---------------------------------------------------------------------------
// State enum
// ---------------------------------------------------------------------------

/// All possible states of the LSIF index relative to its receipt.
#[derive(Debug, PartialEq)]
pub enum LsifIndexState {
    /// Receipt exists, LSIF file exists, and the LSIF digest matches the
    /// receipt's `lsif_digest` field.  Index is current.
    Admitted,
    /// LSIF file or receipt is absent (or unreadable / corrupt JSON).
    /// "I do not know" — STOP, not stale.
    Missing,
    /// LSIF file exists but its BLAKE3 digest does not match
    /// `receipt.lsif_digest`.  "I know" while lying — STOP.
    StaleLsifDigest { expected: String, actual: String },
    /// Source tree BLAKE3 digest does not match `receipt.source_digest`.
    /// The source changed since the last index run — index is outdated.
    StaleSourceDigest { expected: String, actual: String },
}

impl LsifIndexState {
    /// Returns the severity of the given state. Stale or Missing LSIF is STOP.
    pub fn severity(&self) -> crate::andon::core::Severity {
        match self {
            LsifIndexState::Admitted => crate::andon::core::Severity::Info,
            LsifIndexState::Missing => crate::andon::core::Severity::Stop,
            LsifIndexState::StaleLsifDigest { .. } => crate::andon::core::Severity::Stop,
            LsifIndexState::StaleSourceDigest { .. } => crate::andon::core::Severity::Stop,
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant struct
// ---------------------------------------------------------------------------

/// Evaluates the `STALE_LSIF_INDEX = STOP` invariant.
///
/// Probes:
/// - `TRUE`           → `Admitted`  (receipt + file both present, digest matches)
/// - `FALSE`          → `Missing`   (no receipt or no file)
/// - `COUNTERFACTUAL` → `StaleLsifDigest` (file mutated after receipt written)
pub struct StaleLsifIndexInvariant {
    /// Path to the JSON receipt file (e.g. `receipts/v26.6.28-lsif.receipt.json`).
    pub receipt_path: std::path::PathBuf,
    /// Path to the LSIF output file (e.g. `receipts/v26.6.28-lsif.lsif`).
    pub lsif_path: std::path::PathBuf,
    /// Root of the source tree that was indexed (e.g. `src/`).
    pub source_root: std::path::PathBuf,
}

impl StaleLsifIndexInvariant {
    /// Evaluate the invariant and return the current `LsifIndexState`.
    pub fn evaluate(&self) -> LsifIndexState {
        // 1. Both files must exist.
        if !self.receipt_path.exists() || !self.lsif_path.exists() {
            return LsifIndexState::Missing;
        }

        // 2. Receipt must be readable and valid JSON.
        let receipt_str = match std::fs::read_to_string(&self.receipt_path) {
            Ok(s) => s,
            Err(_) => return LsifIndexState::Missing,
        };
        let receipt: serde_json::Value = match serde_json::from_str(&receipt_str) {
            Ok(v) => v,
            Err(_) => return LsifIndexState::Missing,
        };

        // 3. Verify LSIF file digest against receipt.lsif_digest.
        let expected_lsif = receipt["lsif_digest"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let actual_lsif = blake3_file(&self.lsif_path);
        if expected_lsif != actual_lsif {
            return LsifIndexState::StaleLsifDigest {
                expected: expected_lsif,
                actual: actual_lsif,
            };
        }

        // 4. Optionally verify source digest if present in receipt.
        if let Some(expected_src) = receipt["source_digest"].as_str() {
            if !expected_src.is_empty() && !self.source_root.as_os_str().is_empty() {
                let actual_src = blake3_source_root(&self.source_root);
                if !actual_src.is_empty() && expected_src != actual_src {
                    return LsifIndexState::StaleSourceDigest {
                        expected: expected_src.to_string(),
                        actual: actual_src,
                    };
                }
            }
        }

        LsifIndexState::Admitted
    }
}

// ---------------------------------------------------------------------------
// Digest helpers
// ---------------------------------------------------------------------------

/// Compute the BLAKE3 digest of a file's raw bytes.
fn blake3_file(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => blake3::hash(&bytes).to_hex().to_string(),
        Err(_) => String::new(),
    }
}

/// Compute the BLAKE3 digest of sorted `.rs` files under `root`,
/// concatenated in lexicographic order (mirrors the receipt generation method).
fn blake3_source_root(root: &Path) -> String {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    collect_rs_files(root, &mut paths);
    paths.sort();

    let mut hasher = blake3::Hasher::new();
    for path in paths {
        if let Ok(bytes) = std::fs::read(&path) {
            hasher.update(&bytes);
        }
    }
    hasher.finalize().to_hex().to_string()
}

/// Recursively collect all `.rs` files under `dir` into `out`.
fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !matches!(name, "target" | ".git" | "node_modules") {
                collect_rs_files(&path, out);
            }
        } else if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("rs")
        {
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Build a receipt JSON string with the given lsif_digest field.
    fn make_receipt(lsif_digest: &str) -> String {
        serde_json::json!({
            "command": "cargo run -p lsp-max-lsif -- --root src/ --out receipts/test.lsif",
            "exit_code": 0,
            "source_boundary": "src/",
            "source_digest": "",
            "lsif_output_path": "receipts/test.lsif",
            "lsif_digest": lsif_digest,
            "vertex_count": 1,
            "document_count": 1,
            "moniker_count": 0,
            "reference_count": 0,
            "timestamp": "2026-06-27T00:00:00Z",
            "status": "ADMITTED"
        })
        .to_string()
    }

    fn write_file(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    // ------------------------------------------------------------------
    // TRUE: receipt matches LSIF file → Admitted
    // ------------------------------------------------------------------
    #[test]
    fn lsif_invariant_admitted_when_receipt_matches_file() {
        let dir = TempDir::new().unwrap();

        let lsif_content = b"{ \"type\": \"vertex\", \"label\": \"metaData\" }\n";
        let lsif_path = write_file(&dir, "test.lsif", lsif_content);

        let digest = blake3::hash(lsif_content).to_hex().to_string();
        let receipt_path = write_file(&dir, "test.receipt.json", make_receipt(&digest).as_bytes());

        let invariant = StaleLsifIndexInvariant {
            receipt_path,
            lsif_path,
            source_root: std::path::PathBuf::from("src/"),
        };

        assert_eq!(
            invariant.evaluate(),
            LsifIndexState::Admitted,
            "matching digest must yield Admitted"
        );
    }

    // ------------------------------------------------------------------
    // FALSE: no receipt file → Missing
    // ------------------------------------------------------------------
    #[test]
    fn lsif_invariant_missing_when_no_receipt() {
        let dir = TempDir::new().unwrap();

        let invariant = StaleLsifIndexInvariant {
            receipt_path: dir.path().join("nonexistent.receipt.json"),
            lsif_path: dir.path().join("nonexistent.lsif"),
            source_root: std::path::PathBuf::from("src/"),
        };

        assert_eq!(
            invariant.evaluate(),
            LsifIndexState::Missing,
            "absent files must yield Missing"
        );
    }

    // ------------------------------------------------------------------
    // COUNTERFACTUAL (falsification): mutate lsif file after writing receipt
    //   Valid state: digest matches → Admitted
    //   After mutation: digest differs → StaleLsifDigest   ← this MUST fail
    // ------------------------------------------------------------------
    #[test]
    fn lsif_invariant_stale_when_lsif_file_mutated() {
        let dir = TempDir::new().unwrap();

        let original_content = b"{ \"type\": \"vertex\", \"label\": \"metaData\" }\n";
        let lsif_path = write_file(&dir, "test.lsif", original_content);

        // Write receipt with the ORIGINAL digest.
        let original_digest = blake3::hash(original_content).to_hex().to_string();
        let receipt_path =
            write_file(&dir, "test.receipt.json", make_receipt(&original_digest).as_bytes());

        // Confirm it's Admitted before mutation.
        let invariant = StaleLsifIndexInvariant {
            receipt_path: receipt_path.clone(),
            lsif_path: lsif_path.clone(),
            source_root: std::path::PathBuf::from("src/"),
        };
        assert_eq!(invariant.evaluate(), LsifIndexState::Admitted);

        // Mutate the LSIF file (simulate out-of-band modification).
        std::fs::write(&lsif_path, b"CORRUPTED CONTENT\n").unwrap();

        // Now the invariant must detect staleness.
        let state = invariant.evaluate();
        match state {
            LsifIndexState::StaleLsifDigest { expected, actual } => {
                assert_eq!(expected, original_digest, "expected must be original digest");
                assert_ne!(
                    actual, original_digest,
                    "actual must differ after mutation"
                );
            }
            other => panic!(
                "COUNTERFACTUAL FAILED: expected StaleLsifDigest after mutation, got {other:?}"
            ),
        }
    }

    // ------------------------------------------------------------------
    // FALSE: receipt has wrong digest field → StaleLsifDigest
    // ------------------------------------------------------------------
    #[test]
    fn lsif_invariant_stale_when_receipt_corrupt() {
        let dir = TempDir::new().unwrap();

        let lsif_content = b"{ \"type\": \"vertex\", \"label\": \"metaData\" }\n";
        let lsif_path = write_file(&dir, "test.lsif", lsif_content);

        // Write receipt with a WRONG digest.
        let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
        let receipt_path =
            write_file(&dir, "test.receipt.json", make_receipt(wrong_digest).as_bytes());

        let invariant = StaleLsifIndexInvariant {
            receipt_path,
            lsif_path,
            source_root: std::path::PathBuf::from("src/"),
        };

        match invariant.evaluate() {
            LsifIndexState::StaleLsifDigest { expected, .. } => {
                assert_eq!(expected, wrong_digest, "expected must be the wrong digest from receipt");
            }
            other => panic!("expected StaleLsifDigest for corrupt receipt, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // ANDON TRIGGER: STALE_LSIF_INDEX = STOP
    // ------------------------------------------------------------------
    #[test]
    fn lsif_invariant_stale_index_is_stop() {
        assert_eq!(
            LsifIndexState::Missing.severity(),
            crate::andon::core::Severity::Stop,
            "Missing must map to Stop"
        );
        assert_eq!(
            LsifIndexState::StaleLsifDigest { expected: "".into(), actual: "".into() }.severity(),
            crate::andon::core::Severity::Stop,
            "StaleLsifDigest must map to Stop"
        );
        assert_eq!(
            LsifIndexState::StaleSourceDigest { expected: "".into(), actual: "".into() }.severity(),
            crate::andon::core::Severity::Stop,
            "StaleSourceDigest must map to Stop"
        );
        assert_eq!(
            LsifIndexState::Admitted.severity(),
            crate::andon::core::Severity::Info,
            "Admitted must not be Stop"
        );
    }
}
