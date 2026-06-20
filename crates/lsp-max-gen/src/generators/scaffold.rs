use crate::{GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

use super::server::{build_tera_context, render_file, ServerGenerator};

// ── Handler stub templates ────────────────────────────────────────────────────

const HANDLERS_MOD_TMPL: &str = r#"pub mod completion;
pub mod hover;
"#;

/// `textDocument/hover` stub — CANDIDATE, returns `Ok(None)`.
const HOVER_RS_TMPL: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{Hover, HoverParams};

/// Handle `textDocument/hover`.
///
/// Status: CANDIDATE — no transcript or receipt yet; returns `Ok(None)`.
pub async fn handle(
    _params: HoverParams,
) -> Result<Option<Hover>> {
    Ok(None)
}
"#;

/// `textDocument/completion` stub — CANDIDATE, returns `Ok(None)`.
const COMPLETION_RS_TMPL: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{CompletionParams, CompletionResponse};

/// Handle `textDocument/completion`.
///
/// Status: CANDIDATE — no transcript or receipt yet; returns `Ok(None)`.
pub async fn handle(
    _params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    Ok(None)
}
"#;

// ── Generator impl ─────────────────────────────────────────────────────────────

/// Full scaffold: server crate layout produced by [`ServerGenerator`] plus
/// `src/handlers/` stubs for `textDocument/hover` and
/// `textDocument/completion`.
///
/// Handler stubs return `Ok(None)` and are marked `CANDIDATE`; they carry no
/// receipt and are not admitted.
pub struct ScaffoldGenerator;

impl Generator for ScaffoldGenerator {
    fn name(&self) -> &str {
        "scaffold"
    }

    fn description(&self) -> &str {
        "Full scaffold: server crate + hover and completion handler stubs (CANDIDATE)"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        // Base server crate files.
        let mut files = ServerGenerator.generate(ctx)?;

        let tera_ctx = build_tera_context(ctx);

        // Handler stubs — no Tera substitution needed, but go through the
        // shared helper so future name-based interpolation requires no rework.
        let handler_files = [
            ("src/handlers/mod.rs", HANDLERS_MOD_TMPL),
            ("src/handlers/hover.rs", HOVER_RS_TMPL),
            ("src/handlers/completion.rs", COMPLETION_RS_TMPL),
        ];

        for (path, tmpl) in &handler_files {
            files.push(render_file(path, tmpl, &tera_ctx)?);
        }

        // Patch src/lib.rs to re-export the handlers module; the base
        // ServerGenerator emits it as WriteMode::Skip, so if a lib.rs already
        // exists on disk the engine will leave it alone.  Here we upgrade the
        // in-memory version produced for this run so the initial write includes
        // the handlers declaration.
        for f in &mut files {
            if f.path.as_os_str() == "src/lib.rs" {
                f.content.push_str("\npub mod handlers;\n");
                f.mode = WriteMode::Skip;
            }
        }

        Ok(files)
    }
}
