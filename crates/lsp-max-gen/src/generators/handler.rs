use std::path::PathBuf;

use tera::Tera;

use crate::{GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

// Each variant uses Tera template syntax. The `name` and `snake_name` variables
// are injected by `build_tera_ctx`. Templates must never reference `tower-lsp`
// or `tower_lsp`; the engine law-checks all output before writing.

const HOVER_TEMPLATE: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{Hover, HoverContents, HoverParams, MarkedString};

pub async fn hover(params: HoverParams) -> Result<Option<Hover>> {
    // CANDIDATE: hover handler for {{ name }} — transcript required for ADMITTED
    let _ = params;
    Ok(None)
}
"#;

const COMPLETION_TEMPLATE: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{CompletionParams, CompletionResponse};

pub async fn completion(params: CompletionParams) -> Result<Option<CompletionResponse>> {
    // CANDIDATE: completion handler — transcript required for ADMITTED
    let _ = params;
    Ok(None)
}
"#;

const GOTO_DEFINITION_TEMPLATE: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{GotoDefinitionParams, GotoDefinitionResponse};

pub async fn goto_definition(params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
    // CANDIDATE: goto_definition handler — transcript required for ADMITTED
    let _ = params;
    Ok(None)
}
"#;

const DIAGNOSTICS_TEMPLATE: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{
    DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, FullDocumentDiagnosticReport,
};

pub async fn diagnostic(params: DocumentDiagnosticParams) -> Result<DocumentDiagnosticReportResult> {
    // CANDIDATE: pull diagnostics — transcript required for ADMITTED
    let _ = params;
    Ok(DocumentDiagnosticReportResult::Report(
        DocumentDiagnosticReport::Full(FullDocumentDiagnosticReport {
            result_id: None,
            items: vec![],
        }),
    ))
}
"#;

const CODE_ACTION_TEMPLATE: &str = r#"use lsp_max::jsonrpc::Result;
use lsp_types_max::{CodeActionParams, CodeActionResponse};

pub async fn code_action(params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
    // CANDIDATE: code_action handler — transcript required for ADMITTED
    let _ = params;
    Ok(None)
}
"#;

// Generic fallback for method names not yet given a typed template.
const GENERIC_TEMPLATE: &str = r#"// CANDIDATE: {{ snake_name }} handler — transcript required for ADMITTED
pub async fn {{ snake_name }}() {
    // TODO: replace with typed params and return value once the method signature is known
}
"#;

fn build_tera_ctx(gen_ctx: &GeneratorContext) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("name", &gen_ctx.name);
    ctx.insert("snake_name", &gen_ctx.snake_name);
    ctx.insert("kebab_name", &gen_ctx.kebab_name);
    ctx
}

/// Maps the `ctx.name` (LSP method name, snake_case) to the appropriate
/// embedded template string.
fn select_template(name: &str) -> &'static str {
    match name {
        "hover" => HOVER_TEMPLATE,
        "completion" => COMPLETION_TEMPLATE,
        "goto_definition" => GOTO_DEFINITION_TEMPLATE,
        "diagnostics" => DIAGNOSTICS_TEMPLATE,
        "code_action" => CODE_ACTION_TEMPLATE,
        _ => GENERIC_TEMPLATE,
    }
}

/// Generates a single LSP request/notification handler module.
///
/// `ctx.name` is the LSP method name (e.g. `"hover"`, `"completion"`).
/// The output path is `src/handlers/<snake_name>.rs` relative to `ctx.output_dir`.
/// `WriteMode::Skip` prevents overwriting handlers the caller has already edited.
pub struct HandlerGenerator;

impl Generator for HandlerGenerator {
    fn name(&self) -> &str {
        "handler"
    }

    fn description(&self) -> &str {
        "Generate an LSP request/notification handler module"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let template = select_template(&ctx.snake_name);
        let tera_ctx = build_tera_ctx(ctx);
        let content = Tera::one_off(template, &tera_ctx, false)?;

        let path = PathBuf::from("src/handlers").join(format!("{}.rs", ctx.snake_name));

        Ok(vec![GeneratedFile {
            path,
            content,
            mode: WriteMode::Skip,
        }])
    }
}
