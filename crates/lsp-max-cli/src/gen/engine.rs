use std::fs;
use std::path::PathBuf;

use crate::gen::{
    generator::{Generator, WriteMode},
    GeneratorContext, GeneratorError,
};

/// Identifiers whose presence in generated content constitutes a law violation.
const FORBIDDEN: &[&str] = &["tower-lsp", "tower_lsp"];

/// Outcome for a single file after the engine has processed it.
pub struct WrittenFile {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// `"WRITTEN"` or `"SKIPPED"`.
    pub status: &'static str,
}

/// Drives a set of [`Generator`] implementations.
///
/// The engine is the single point responsible for:
/// - dispatching to the correct generator by name,
/// - law-checking generated content before it touches the filesystem,
/// - creating parent directories, and
/// - honouring [`WriteMode`].
pub struct GeneratorEngine {
    generators: Vec<Box<dyn Generator>>,
}

impl GeneratorEngine {
    pub fn new(generators: Vec<Box<dyn Generator>>) -> Self {
        Self { generators }
    }

    /// Run the named generator for `ctx` and write the resulting files to disk.
    ///
    /// Returns `GeneratorError::NotFound` when no generator matches `generator_name`.
    /// Returns `GeneratorError::LawViolation` if any generated content contains a
    /// forbidden identifier before any file is written.
    pub fn run(
        &self,
        generator_name: &str,
        ctx: &GeneratorContext,
    ) -> Result<Vec<WrittenFile>, GeneratorError> {
        let gen = self
            .generators
            .iter()
            .find(|g| g.name() == generator_name)
            .ok_or_else(|| GeneratorError::NotFound(generator_name.to_owned()))?;

        let files = gen.generate(ctx)?;

        // Law-check all content before touching the filesystem.
        for file in &files {
            for &forbidden in FORBIDDEN {
                if file.content.contains(forbidden) {
                    return Err(GeneratorError::LawViolation {
                        reason: format!(
                            "file {:?} contains forbidden identifier `{}`",
                            file.path, forbidden
                        ),
                    });
                }
            }
        }

        let mut written = Vec::with_capacity(files.len());

        for file in files {
            let abs = ctx.output_dir.join(&file.path);

            if matches!(file.mode, WriteMode::Skip) && abs.exists() {
                written.push(WrittenFile {
                    path: abs,
                    status: "SKIPPED",
                });
                continue;
            }

            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent).map_err(|source| GeneratorError::Io {
                    path: parent.to_owned(),
                    source,
                })?;
            }

            fs::write(&abs, &file.content).map_err(|source| GeneratorError::Io {
                path: abs.clone(),
                source,
            })?;

            written.push(WrittenFile {
                path: abs,
                status: "WRITTEN",
            });
        }

        Ok(written)
    }
}
