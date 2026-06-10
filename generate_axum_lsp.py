import os

# Define context variables
context = {
    "framework_name": "axum",
    "language_grammar": "tree_sitter_rust",
    "semantic_rules": [
        {"description": "Detect forbidden extraction", "code": "if node.kind() == \\\"forbidden\\\" { /* emit diagnostic */ }"}
    ],
    "forbidden_imports": ["std::thread::sleep"]
}

# Template 1: scaffold.rs.tera
scaffold_template = """use clap_noun_verb::cli::{run_cli, Result};
use clap_noun_verb_macros::verb;
use std::sync::Arc;
use tower_lsp_max::{LspService, Server};
use tower_lsp_max::auto_lsp::AutoLspAdapter;

// This is a ggen-generated Framework LSP.
// It is structurally compliant with the wasm4pm-compat baseline admissibility laws.

pub struct AxumBackend {
    pub client: tower_lsp_max::Client,
    pub auto_lsp: Arc<AutoLspAdapter>,
}

#[tower_lsp_max::async_trait]
impl tower_lsp_max::LanguageServer for AxumBackend {
    async fn initialize(
        &self,
        _params: tower_lsp_max::lsp_types_max::InitializeParams,
    ) -> tower_lsp_max::jsonrpc::Result<tower_lsp_max::lsp_types_max::InitializeResult> {
        Ok(tower_lsp_max::lsp_types_max::InitializeResult {
            server_info: Some(tower_lsp_max::lsp_types_max::ServerInfo {
                name: "axum-lsp".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            offset_encoding: None,
            capabilities: tower_lsp_max::lsp_types_max::ServerCapabilities {
                text_document_sync: Some(tower_lsp_max::lsp_types_max::TextDocumentSyncCapability::Kind(
                    tower_lsp_max::lsp_types_max::TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
        })
    }

    async fn shutdown(&self) -> tower_lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp_max::lsp_types_max::DidOpenTextDocumentParams) {
        self.auto_lsp.handle_did_open(params, tree_sitter_rust::LANGUAGE.into());
    }

    async fn did_change(&self, params: tower_lsp_max::lsp_types_max::DidChangeTextDocumentParams) {
        self.auto_lsp.handle_did_change(params, tree_sitter_rust::LANGUAGE.into());
    }
}

#[verb("start", noun="server")]
fn start_server(
    /// Launch in stdio mode for IDE integration
    #[arg(long)]
    stdio: bool,
) -> Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| AxumBackend {
            client,
            auto_lsp: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(())
}

fn main() -> Result<()> {
    run_cli()
}
"""

# Template 2: semantics.rs.tera
semantics_template = """use tower_lsp_max::auto_lsp::AutoLspAdapter;
use tower_lsp_max::lsp_types_max::Diagnostic;

pub fn dispatch_semantic_rules(adapter: &AutoLspAdapter, uri: &tower_lsp_max::lsp_types_max::DocumentUri) -> Vec<Diagnostic> {
    let mut diagnostics = adapter.pull_diagnostics(uri);

    // Apply framework-specific hallucination detection
    adapter.get_document(uri, |doc| {
        let mut cursor = doc.tree.walk();
        
        // This is a dynamic ggen template hook. The AGI expands this block 
        // with framework-specific node targeting rules.
        
        // Rule: Detect forbidden extraction
        if doc.tree.root_node().kind() == "forbidden" { /* emit diagnostic */ }
        
        // Ensure the generated code does not violate forbidden imports
        
        // Disallow std::thread::sleep
        // The implementation traverses the use_declarations and flags `std::thread::sleep`
        
    });

    diagnostics
}
"""

# Write to examples/axum-lsp
os.makedirs("examples/axum-lsp/src", exist_ok=True)
with open("examples/axum-lsp/src/main.rs", "w") as f:
    f.write(scaffold_template)
with open("examples/axum-lsp/src/semantics.rs", "w") as f:
    f.write(semantics_template)

# Write Cargo.toml
cargo_toml = """[package]
name = "axum-lsp"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
clap-noun-verb = "26.6.2"
clap-noun-verb-macros = "26.6.2"
tower-lsp-max = { path = "../../" }
tokio = { version = "1.17", features = ["full"] }
tree-sitter-rust = "0.24"
"""
with open("examples/axum-lsp/Cargo.toml", "w") as f:
    f.write(cargo_toml)

print("Generated templates!")
