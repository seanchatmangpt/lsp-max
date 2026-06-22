use std::path::PathBuf;

use tera::{Context as TeraContext, Tera};

use crate::{GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

/// Generate `max/*` protocol method stubs and type scaffolds.
///
/// Produces two files per invocation:
/// - `src/max_<snake_name>.rs`  — Rust module with Params/Result types and METHOD const (Skip)
/// - `src/max_<snake_name>_registration.md` — registration guide (Skip)
///
/// Status: CANDIDATE — admission requires transcript, negative-control, receipt.
pub struct ProtocolGenerator;

/// Rust source template for a `max/*` protocol extension module.
///
/// `name` is expected to be PascalCase (from `ctx.name`).
/// `snake_name` is the snake_case form (from `ctx.snake_name`).
const RUST_TEMPLATE: &str = r#"use lsp_max_protocol::MaxDiagnostic;
use serde::{Deserialize, Serialize};

// CANDIDATE: max/{{ name }} protocol extension
// Admission requires: transcript, negative-control, receipt
// Method: max/{{ name }}

#[derive(Debug, Serialize, Deserialize)]
pub struct {{ name }}Params {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct {{ name }}Result {
    pub status: String, // CANDIDATE | ADMITTED | REFUSED | UNKNOWN
    pub diagnostics: Vec<MaxDiagnostic>,
}

pub const METHOD: &str = "max/{{ snake_name }}";
"#;

/// Markdown registration guide template.
const MD_TEMPLATE: &str = r#"# max/{{ snake_name }} Registration — CANDIDATE

To register this extension in your `LanguageServer` impl:

1. Add to `InitializeResult::capabilities` via `ServerCapabilities` extension fields
2. Handle `max/{{ snake_name }}` in a custom `request` handler
3. Return `{{ name }}Result` with `status: "CANDIDATE"` until receipt chain is admitted

Status: CANDIDATE — transcript required for ADMITTED
"#;

impl Generator for ProtocolGenerator {
    fn name(&self) -> &str {
        "protocol"
    }

    fn description(&self) -> &str {
        "Generate max/* protocol method stubs and type scaffolds"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut tera_ctx = TeraContext::new();
        // ctx.name is already PascalCase per GeneratorContext contract.
        tera_ctx.insert("name", &ctx.name);
        tera_ctx.insert("snake_name", &ctx.snake_name);

        let rust_content = Tera::one_off(RUST_TEMPLATE, &tera_ctx, false)?;
        let md_content = Tera::one_off(MD_TEMPLATE, &tera_ctx, false)?;

        Ok(vec![
            GeneratedFile {
                path: PathBuf::from(format!("src/max_{}.rs", ctx.snake_name)),
                content: rust_content,
                mode: WriteMode::Skip,
            },
            GeneratedFile {
                path: PathBuf::from(format!("src/max_{}_registration.md", ctx.snake_name)),
                content: md_content,
                mode: WriteMode::Skip,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::GeneratorContext;

    fn ctx(name: &str) -> GeneratorContext {
        GeneratorContext::new(name, PathBuf::from("/tmp/test"))
    }

    #[test]
    fn generates_two_files_for_snapshot() {
        let gen = ProtocolGenerator;
        let files = gen.generate(&ctx("Snapshot")).unwrap();
        assert_eq!(files.len(), 2);
        let paths: Vec<_> = files.iter().map(|f| f.path.to_str().unwrap()).collect();
        assert!(paths.iter().any(|p| p.contains("max_snapshot.rs")));
        assert!(paths.iter().any(|p| p.contains("max_snapshot_registration.md")));
    }

    #[test]
    fn rust_file_contains_method_const_and_types() {
        let gen = ProtocolGenerator;
        let files = gen.generate(&ctx("Snapshot")).unwrap();
        let rust = files.iter().find(|f| f.path.to_str().unwrap().ends_with(".rs")).unwrap();
        assert!(rust.content.contains("SnapshotParams"));
        assert!(rust.content.contains("SnapshotResult"));
        assert!(rust.content.contains("max/snapshot"));
        assert!(rust.content.contains("CANDIDATE"));
    }

    #[test]
    fn no_forbidden_tower_lsp_reference() {
        let gen = ProtocolGenerator;
        let files = gen.generate(&ctx("MyExtension")).unwrap();
        for file in &files {
            assert!(!file.content.contains("tower-lsp"));
            assert!(!file.content.contains("tower_lsp"));
        }
    }

    #[test]
    fn write_mode_is_skip_for_both_files() {
        let gen = ProtocolGenerator;
        let files = gen.generate(&ctx("Snapshot")).unwrap();
        for file in &files {
            assert!(matches!(file.mode, WriteMode::Skip));
        }
    }
}
