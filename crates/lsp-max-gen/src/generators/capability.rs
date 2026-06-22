use std::path::PathBuf;

use tera::Tera;

use crate::{GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

// Template for the per-capability module. Wires a server capability field into
// `ServerCapabilities` via a `register` function the caller must invoke from
// their `initialize` handler.
const CAPABILITY_TEMPLATE: &str = r#"use lsp_types_max::ServerCapabilities;

/// CANDIDATE capability: {{ name }}
/// Admission requires: transcript + negative-control + receipt
pub fn register(caps: &mut ServerCapabilities) {
    // TODO: set the {{ name }} capability field
    // Example: caps.hover_provider = Some(HoverProviderCapability::Simple(true));
    let _ = caps;
}
"#;

// Emitted as `src/capabilities/mod.rs` with WriteMode::Skip so it is never
// overwritten once the caller has added their own `pub mod` declarations.
// The comment instructs them to add the new module manually.
const MOD_TEMPLATE: &str = r#"// CANDIDATE: add capability modules here manually to preserve existing declarations.
// Example: pub mod {{ snake_name }};
"#;

fn build_tera_ctx(gen_ctx: &GeneratorContext) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("name", &gen_ctx.name);
    ctx.insert("snake_name", &gen_ctx.snake_name);
    ctx.insert("kebab_name", &gen_ctx.kebab_name);
    ctx
}

/// Generates a capability declaration module and a `mod.rs` hint.
///
/// `ctx.name` is the capability name (e.g. `"hover"`, `"SemanticTokens"`).
///
/// Emits two files:
/// - `src/capabilities/<snake_name>.rs` — the `register` function skeleton
/// - `src/capabilities/mod.rs` — a comment-only hint (WriteMode::Skip; caller edits)
pub struct CapabilityGenerator;

impl Generator for CapabilityGenerator {
    fn name(&self) -> &str {
        "capability"
    }

    fn description(&self) -> &str {
        "Generate LSP capability declaration boilerplate"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let tera_ctx = build_tera_ctx(ctx);

        let capability_content = Tera::one_off(CAPABILITY_TEMPLATE, &tera_ctx, false)?;
        let mod_content = Tera::one_off(MOD_TEMPLATE, &tera_ctx, false)?;

        Ok(vec![
            GeneratedFile {
                path: PathBuf::from("src/capabilities")
                    .join(format!("{}.rs", ctx.snake_name)),
                content: capability_content,
                // Skip preserves any prior edits to the capability module.
                mode: WriteMode::Skip,
            },
            GeneratedFile {
                path: PathBuf::from("src/capabilities/mod.rs"),
                content: mod_content,
                // Skip so the caller's existing pub mod declarations are not clobbered.
                // Add `pub mod <snake_name>;` to this file by hand.
                mode: WriteMode::Skip,
            },
        ])
    }
}
