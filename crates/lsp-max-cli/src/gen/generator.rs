use std::path::PathBuf;

use crate::gen::{GeneratorContext, GeneratorError};

/// Implemented by every scaffold generator.
pub trait Generator: Send + Sync {
    /// Short machine-readable identifier (e.g. `"server"`, `"handler"`).
    fn name(&self) -> &str;
    /// Human-readable description surfaced by `lsp-max-gen list`.
    fn description(&self) -> &str;
    /// Produce the set of files this generator would emit for `ctx`.
    /// The engine is responsible for writing them; generators only return content.
    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError>;
}

/// A single file to be written by the engine.
pub struct GeneratedFile {
    /// Path relative to `ctx.output_dir`.
    pub path: PathBuf,
    /// UTF-8 content for the file.
    pub content: String,
    /// Whether the engine should overwrite an existing file or leave it in place.
    pub mode: WriteMode,
}

/// Controls how the engine handles a pre-existing file at the target path.
pub enum WriteMode {
    /// Replace any existing file with the generated content.
    Overwrite,
    /// Leave an existing file untouched; emit `SKIPPED` in the result set.
    Skip,
}
