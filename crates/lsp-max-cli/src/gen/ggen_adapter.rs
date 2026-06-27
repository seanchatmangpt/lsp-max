use std::path::{Path, PathBuf};
use std::process::Command;

use crate::gen::{GenManifest, GeneratorError};

/// Detects and delegates to the `ggen` binary when available.
/// Falls back silently — callers check `is_available()` first.
///
/// Status is `CANDIDATE` until a ggen receipt chain confirms `ADMITTED`.
pub struct GgenAdapter {
    binary: Option<PathBuf>,
}

/// Result of a `ggen sync` (or fallback) run.
pub struct SyncReport {
    /// Paths written by ggen (or empty on fallback).
    pub files_written: Vec<String>,
    /// Bounded status: "CANDIDATE" when ggen ran, "CANDIDATE-FALLBACK" when not.
    pub status: String,
}

/// Result of a `ggen validate` (or fallback) run.
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<String>,
    /// Bounded status reflecting the validation path taken.
    pub status: String,
}

// ── Detection helpers ─────────────────────────────────────────────────────────

const COMMON_INSTALL_PATHS: &[&str] = &["/usr/local/bin/ggen", "/opt/homebrew/bin/ggen"];

fn probe_binary() -> Option<PathBuf> {
    // 1. Explicit override via env var.
    if let Ok(val) = std::env::var("GGEN_BIN") {
        let p = PathBuf::from(val);
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. `which ggen` — resolves through $PATH.
    if let Ok(out) = Command::new("which").arg("ggen").output() {
        if out.status.success() {
            let raw = String::from_utf8_lossy(&out.stdout);
            let candidate = PathBuf::from(raw.trim());
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // 3. Well-known install locations.
    for path_str in COMMON_INSTALL_PATHS {
        let p = Path::new(path_str);
        if p.is_file() {
            return Some(p.to_owned());
        }
    }

    None
}

// ── GgenAdapter ───────────────────────────────────────────────────────────────

impl GgenAdapter {
    /// Probe for the `ggen` binary and construct the adapter.
    ///
    /// Does not fail if `ggen` is absent; callers use `is_available()` to
    /// choose between delegating and the embedded fallback.
    pub fn new() -> Self {
        Self {
            binary: probe_binary(),
        }
    }

    /// `true` when a `ggen` binary was located during construction.
    pub fn is_available(&self) -> bool {
        self.binary.is_some()
    }

    /// Run `ggen sync` in `dir`, or return a `CANDIDATE-FALLBACK` report when
    /// the binary is absent so callers can degrade gracefully.
    pub fn sync(&self, dir: &Path) -> Result<SyncReport, GeneratorError> {
        match &self.binary {
            Some(bin) => {
                let out = Command::new(bin)
                    .arg("sync")
                    .current_dir(dir)
                    .output()
                    .map_err(|source| GeneratorError::Io {
                        path: bin.clone(),
                        source,
                    })?;

                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                    return Err(GeneratorError::LawViolation {
                        reason: format!("ggen sync exited non-zero: {stderr}"),
                    });
                }

                let stdout = String::from_utf8_lossy(&out.stdout);
                let files_written = stdout
                    .lines()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty())
                    .collect();

                Ok(SyncReport {
                    files_written,
                    // Receipt chain must confirm ADMITTED; CANDIDATE until then.
                    status: "CANDIDATE".into(),
                })
            }
            None => Ok(SyncReport {
                files_written: vec![],
                // Embedded Tera engine is active; status stays CANDIDATE-FALLBACK.
                status: "CANDIDATE-FALLBACK".into(),
            }),
        }
    }

    /// Run `ggen init --name <name>` in `dir`, or write a minimal `gen.toml`
    /// stub when the binary is absent.
    pub fn init(&self, name: &str, dir: &Path) -> Result<(), GeneratorError> {
        match &self.binary {
            Some(bin) => {
                let out = Command::new(bin)
                    .args(["init", "--name", name])
                    .current_dir(dir)
                    .output()
                    .map_err(|source| GeneratorError::Io {
                        path: bin.clone(),
                        source,
                    })?;

                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                    return Err(GeneratorError::LawViolation {
                        reason: format!("ggen init exited non-zero: {stderr}"),
                    });
                }
                Ok(())
            }
            None => {
                // Emit a minimal stub so downstream tools have a parseable manifest.
                let stub = format!(
                    r#"[project]
name = "{name}"
language_id = ""

# ggen binary not detected — CANDIDATE: install ggen to enable full generation
# https://github.com/seanchatmangpt/ggen

[ontology]
source = "schema/domain.ttl"
packs = ["lsp-max"]

[generation]
output_dir = "src"
"#
                );
                let toml_path = dir.join("gen.toml");
                std::fs::write(&toml_path, stub).map_err(|source| GeneratorError::Io {
                    path: toml_path,
                    source,
                })?;
                Ok(())
            }
        }
    }

    /// Run `ggen validate` in `dir`, or check that `gen.toml` is parseable when
    /// the binary is absent.
    pub fn validate(&self, dir: &Path) -> Result<ValidationReport, GeneratorError> {
        match &self.binary {
            Some(bin) => {
                let out = Command::new(bin)
                    .arg("validate")
                    .current_dir(dir)
                    .output()
                    .map_err(|source| GeneratorError::Io {
                        path: bin.clone(),
                        source,
                    })?;

                let valid = out.status.success();
                let stderr = String::from_utf8_lossy(&out.stderr);
                let issues: Vec<String> = if valid {
                    vec![]
                } else {
                    stderr
                        .lines()
                        .map(|l| l.trim().to_owned())
                        .filter(|l| !l.is_empty())
                        .collect()
                };

                Ok(ValidationReport {
                    valid,
                    issues,
                    status: if valid {
                        "CANDIDATE".into()
                    } else {
                        "BLOCKED".into()
                    },
                })
            }
            None => {
                let toml_path = dir.join("gen.toml");
                match GenManifest::from_path(&toml_path) {
                    Ok(_) => Ok(ValidationReport {
                        valid: true,
                        issues: vec![],
                        // Manifest parsed; ggen not present so receipt chain is OPEN.
                        status: "CANDIDATE-FALLBACK".into(),
                    }),
                    Err(e) => Ok(ValidationReport {
                        valid: false,
                        issues: vec![e.to_string()],
                        status: "BLOCKED".into(),
                    }),
                }
            }
        }
    }

    /// Run `ggen diff` (or `ggen sync --dry-run`) in `dir`, or list
    /// `[[generate]]` entries from `gen.toml` as pending when the binary is absent.
    pub fn diff(&self, dir: &Path) -> Result<Vec<String>, GeneratorError> {
        match &self.binary {
            Some(bin) => {
                // Try `ggen diff`; fall back to `ggen sync --dry-run` on failure.
                let out = Command::new(bin)
                    .arg("diff")
                    .current_dir(dir)
                    .output()
                    .map_err(|source| GeneratorError::Io {
                        path: bin.clone(),
                        source,
                    })?;

                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    return Ok(stdout
                        .lines()
                        .map(|l| l.trim().to_owned())
                        .filter(|l| !l.is_empty())
                        .collect());
                }

                // `ggen diff` not available; try `ggen sync --dry-run`.
                let dry = Command::new(bin)
                    .args(["sync", "--dry-run"])
                    .current_dir(dir)
                    .output()
                    .map_err(|source| GeneratorError::Io {
                        path: bin.clone(),
                        source,
                    })?;

                let stdout = String::from_utf8_lossy(&dry.stdout);
                Ok(stdout
                    .lines()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty())
                    .collect())
            }
            None => {
                // ggen absent — list [[generate]] entries from gen.toml as pending.
                let toml_path = dir.join("gen.toml");
                let manifest = GenManifest::from_path(&toml_path)?;
                let pending = manifest
                    .generate
                    .iter()
                    .map(|e| {
                        format!(
                            "PENDING (CANDIDATE-FALLBACK): {} {:?} -> {}",
                            e.kind, e.name, e.output_dir
                        )
                    })
                    .collect();
                Ok(pending)
            }
        }
    }
}

impl Default for GgenAdapter {
    fn default() -> Self {
        Self::new()
    }
}
