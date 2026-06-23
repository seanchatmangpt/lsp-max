use thiserror::Error;

/// Errors produced by the generator engine.
#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("generator not found: {0}")]
    NotFound(String),

    #[error("I/O error writing {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("template render error: {0}")]
    Template(#[from] tera::Error),

    #[error("manifest parse error: {0}")]
    ManifestParse(#[from] toml::de::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Emitted when generated content would reintroduce a forbidden identifier.
    #[error("law violation in generated content: {reason}")]
    LawViolation { reason: String },
}
