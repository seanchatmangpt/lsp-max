use tera::{Context, Tera};

use crate::gen::{GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

// ── Templates ─────────────────────────────────────────────────────────────────

const CARGO_TOML_TMPL: &str = r#"[package]
name = "{{ kebab_name }}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{{ kebab_name }}"
path = "src/main.rs"

[dependencies]
lsp-max = "26.6.9"
lsp-types-max = "26.6.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["io-std", "io-util", "macros", "rt-multi-thread"] }
async-trait = "0.1"
tracing = "0.1"
tracing-subscriber = "0.3"
"#;

const MAIN_RS_TMPL: &str = r#"use {{ snake_name }}::Backend;
use lsp_max::Server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let (service, socket) = lsp_max::LspService::new(|client| Backend::new(client));
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
"#;

const LIB_RS_TMPL: &str = r#"pub mod backend;

pub use backend::{{ name }}Backend as Backend;
"#;

const BACKEND_RS_TMPL: &str = r#"use lsp_max::Client;
use lsp_types_max::*;

pub struct {{ name }}Backend {
    client: Client,
}

impl {{ name }}Backend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for {{ name }}Backend {
    async fn initialize(
        &self,
        _params: InitializeParams,
    ) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "{{ name }} server CANDIDATE")
            .await;
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }
}
"#;

// ── Generator impl ─────────────────────────────────────────────────────────────

/// Scaffold a minimal lsp-max server crate with `Cargo.toml`, `src/main.rs`,
/// `src/lib.rs`, and `src/backend.rs`.
///
/// All emitted files are [`WriteMode::Skip`] so re-running does not overwrite
/// manual edits.
pub struct ServerGenerator;

impl Generator for ServerGenerator {
    fn name(&self) -> &str {
        "server"
    }

    fn description(&self) -> &str {
        "Scaffold a minimal lsp-max server crate (CANDIDATE)"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let tera_ctx = build_tera_context(ctx);

        let files = [
            ("Cargo.toml", CARGO_TOML_TMPL),
            ("src/main.rs", MAIN_RS_TMPL),
            ("src/lib.rs", LIB_RS_TMPL),
            ("src/backend.rs", BACKEND_RS_TMPL),
        ];

        files
            .iter()
            .map(|(path, tmpl)| render_file(path, tmpl, &tera_ctx))
            .collect()
    }
}

// ── Shared helpers ─────────────────────────────────────────────────────────────

/// Build a [`tera::Context`] from a [`GeneratorContext`], exposing the three
/// naming forms plus any `extra` fields forwarded from the CLI caller.
pub(crate) fn build_tera_context(ctx: &GeneratorContext) -> Context {
    let mut c = Context::new();
    c.insert("name", &ctx.name);
    c.insert("snake_name", &ctx.snake_name);
    c.insert("kebab_name", &ctx.kebab_name);
    if let Some(obj) = ctx.extra.as_object() {
        for (k, v) in obj {
            c.insert(k.as_str(), v);
        }
    }
    c
}

/// Render a single Tera template string and wrap it into a [`GeneratedFile`].
pub(crate) fn render_file(
    rel_path: &str,
    template: &str,
    ctx: &Context,
) -> Result<GeneratedFile, GeneratorError> {
    let content = Tera::one_off(template, ctx, false)?;
    Ok(GeneratedFile {
        path: rel_path.into(),
        content,
        mode: WriteMode::Skip,
    })
}
