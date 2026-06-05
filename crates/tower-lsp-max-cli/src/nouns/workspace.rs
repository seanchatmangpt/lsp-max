use clap_noun_verb::{NounVerbError, Result};
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct WorkspaceResult {
    pub initialized: bool,
    pub path: String,
    pub files_created: Vec<String>,
}

#[verb("init")]
pub fn cmd_init(path: String) -> Result<WorkspaceResult> {
    init_workspace(path)
}

#[derive(Serialize)]
pub struct AnalysisFinding {
    pub file: String,
    pub line: usize,
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct AnalyzeResult {
    pub analyzed: bool,
    pub issues: usize,
    pub files_scanned: usize,
    pub findings: Vec<AnalysisFinding>,
}

#[verb("analyze")]
pub fn cmd_analyze(path: String) -> Result<AnalyzeResult> {
    analyze_workspace(path)
}

fn init_workspace(path: String) -> Result<WorkspaceResult> {
    let root_path = Path::new(&path);
    if !root_path.exists() {
        fs::create_dir_all(root_path).map_err(|e| {
            NounVerbError::execution_error(format!("Failed to create workspace directory: {}", e))
        })?;
    }

    let ws_name = root_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("tower-lsp-max-workspace");

    let mut files_created = Vec::new();

    // 1. tower-lsp-max.json
    let config_path = root_path.join("tower-lsp-max.json");
    let config_content = format!(
        r#"{{
  "workspace_name": "{}",
  "version": "0.1.0",
  "strict_mode": true,
  "conformance_target": 1.0,
  "capabilities": {{
    "textDocument": {{
      "completion": {{
        "dynamicRegistration": true,
        "completionItemKind": {{
          "valueSet": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
        }}
      }},
      "hover": {{
        "contentFormat": ["markdown", "plaintext"]
      }}
    }}
  }}
}}
"#,
        ws_name
    );
    fs::write(&config_path, config_content).map_err(|e| {
        NounVerbError::execution_error(format!("Failed to write tower-lsp-max.json: {}", e))
    })?;
    files_created.push("tower-lsp-max.json".to_string());

    // 2. Cargo.toml
    let cargo_path = root_path.join("Cargo.toml");
    let cargo_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
tower-lsp-max = {{ path = "/Users/sac/tower-lsp-max" }}
tokio = {{ version = "1.17", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#,
        ws_name
    );
    fs::write(&cargo_path, cargo_content).map_err(|e| {
        NounVerbError::execution_error(format!("Failed to write Cargo.toml: {}", e))
    })?;
    files_created.push("Cargo.toml".to_string());

    // 3. .gitignore
    let gitignore_path = root_path.join(".gitignore");
    let gitignore_content = "/target\nCargo.lock\n";
    fs::write(&gitignore_path, gitignore_content).map_err(|e| {
        NounVerbError::execution_error(format!("Failed to write .gitignore: {}", e))
    })?;
    files_created.push(".gitignore".to_string());

    // 4. src/main.rs
    let src_dir = root_path.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| {
        NounVerbError::execution_error(format!("Failed to create src directory: {}", e))
    })?;

    let main_path = src_dir.join("main.rs");
    let main_content = r#"use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types::*;
use tower_lsp_max::{LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend;

#[tower_lsp_max::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "tower-lsp-max-scaffold".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|_| Backend);
    Server::new(stdin, stdout, socket).serve(service).await;
}
"#;
    fs::write(&main_path, main_content).map_err(|e| {
        NounVerbError::execution_error(format!("Failed to write src/main.rs: {}", e))
    })?;
    files_created.push("src/main.rs".to_string());

    Ok(WorkspaceResult {
        initialized: true,
        path,
        files_created,
    })
}

fn analyze_workspace(path: String) -> Result<AnalyzeResult> {
    let root_path = Path::new(&path);
    if !root_path.exists() {
        return Err(NounVerbError::execution_error(format!(
            "Path '{}' does not exist",
            path
        )));
    }

    let mut findings = Vec::new();
    let mut files_scanned = 0;

    // Check if configuration exists
    let config_path = root_path.join("tower-lsp-max.json");
    if !config_path.exists() {
        findings.push(AnalysisFinding {
            file: "tower-lsp-max.json".to_string(),
            line: 0,
            severity: "Warning".to_string(),
            code: "MISSING_CONFIG".to_string(),
            message: "Missing tower-lsp-max.json workspace configuration file".to_string(),
        });
    }

    // Recursively scan Rust files
    let mut files = Vec::new();
    if let Err(e) = visit_dirs(root_path, &mut files) {
        return Err(NounVerbError::execution_error(format!(
            "Failed to traverse directories: {}",
            e
        )));
    }

    for file_path in files {
        files_scanned += 1;
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                findings.push(AnalysisFinding {
                    file: file_path.to_string_lossy().into_owned(),
                    line: 0,
                    severity: "Error".to_string(),
                    code: "UNREADABLE_FILE".to_string(),
                    message: format!("Failed to read file: {}", e),
                });
                continue;
            }
        };

        let file_name = file_path.to_string_lossy().into_owned();
        let mut has_impl_language_server = false;
        let mut has_shutdown_method = false;

        for (idx, line) in content.lines().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            // Skip lines that are comments
            if trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*") {
                continue;
            }

            // Check 1: unwrap() calls
            if trimmed.contains(".unwrap()") {
                findings.push(AnalysisFinding {
                    file: file_name.clone(),
                    line: line_num,
                    severity: "Warning".to_string(),
                    code: "UNSAFE_UNWRAP".to_string(),
                    message: "Direct .unwrap() call found inside server code. Use robust error handling instead.".to_string(),
                });
            }

            // Check 2: expect() calls
            if trimmed.contains(".expect(") {
                findings.push(AnalysisFinding {
                    file: file_name.clone(),
                    line: line_num,
                    severity: "Warning".to_string(),
                    code: "UNSAFE_EXPECT".to_string(),
                    message: "Direct .expect() call found inside server code. Use robust error handling instead.".to_string(),
                });
            }

            // Check 3: Raw tower_lsp import
            if trimmed.contains("use tower_lsp::") {
                findings.push(AnalysisFinding {
                    file: file_name.clone(),
                    line: line_num,
                    severity: "Warning".to_string(),
                    code: "BASE_TOWER_LSP_USE".to_string(),
                    message: "Importing from base tower_lsp instead of tower_lsp_max. This prevents maximal LSP capability projection.".to_string(),
                });
            }

            // Check 4: Track LanguageServer trait implementation
            if trimmed.contains("impl LanguageServer for") {
                has_impl_language_server = true;
            }

            if has_impl_language_server
                && (trimmed.contains("fn shutdown") || trimmed.contains("async fn shutdown"))
            {
                has_shutdown_method = true;
            }
        }

        if has_impl_language_server && !has_shutdown_method {
            findings.push(AnalysisFinding {
                file: file_name,
                line: 1,
                severity: "Warning".to_string(),
                code: "MISSING_SHUTDOWN".to_string(),
                message: "LanguageServer implementation is missing a 'shutdown' method, which is required for standard lifecycle conformance.".to_string(),
            });
        }
    }

    let issues = findings.len();
    Ok(AnalyzeResult {
        analyzed: true,
        issues,
        files_scanned,
        findings,
    })
}

fn visit_dirs(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "target" || name == ".git" || name == ".cargo" {
                    continue;
                }
                visit_dirs(&path, files)?;
            } else if path.is_file() {
                if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
    }
    Ok(())
}
