use std::path::Path;

use serde::Deserialize;

use crate::GeneratorError;

/// Top-level structure of a `gen.toml` manifest file.
///
/// Example:
/// ```toml
/// [project]
/// name = "my-lsp"
/// language_id = "mylang"
///
/// [[generate]]
/// kind = "server"
/// name = "MyLsp"
/// output_dir = "."
///
/// [[generate]]
/// kind = "handler"
/// name = "hover"
/// output_dir = "src/handlers"
/// ```
#[derive(Debug, Deserialize)]
pub struct GenManifest {
    pub project: ProjectMeta,
    #[serde(default)]
    pub generate: Vec<GenerateEntry>,
}

/// Project-level metadata block from `[project]`.
#[derive(Debug, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub language_id: Option<String>,
}

/// A single `[[generate]]` entry.
#[derive(Debug, Deserialize)]
pub struct GenerateEntry {
    /// Which generator to invoke (e.g. `"server"`, `"handler"`).
    pub kind: String,
    /// Target name passed to `GeneratorContext`.
    pub name: String,
    /// Directory where generated files are rooted.
    pub output_dir: String,
}

impl GenManifest {
    /// Parse a `gen.toml` file from `p`.
    pub fn from_path(p: &Path) -> Result<Self, GeneratorError> {
        let raw = std::fs::read_to_string(p).map_err(|source| GeneratorError::Io {
            path: p.to_owned(),
            source,
        })?;
        let manifest: Self = toml::from_str(&raw)?;
        Ok(manifest)
    }
}
