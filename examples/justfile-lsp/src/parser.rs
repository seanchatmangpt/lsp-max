//! Justfile parser — identifies recipe declarations and extracts diagnostics.
//!
//! Parsing is line-based and intentionally minimal: just-the-interpreter's grammar is
//! complex (imports, aliases, settings, attributes), but for LSP diagnostics we only
//! need recipe names, their attributes, and body shell calls.

use regex::Regex;
use std::sync::OnceLock;

// ── Compiled-once regex statics ───────────────────────────────────────────────

fn recipe_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([a-zA-Z0-9_-]+)\s*(\([^)]*\))?\s*:").unwrap())
}

fn shell_inject_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Common injection patterns: unquoted $VAR in shell strings passed to -c
    RE.get_or_init(|| Regex::new(r#"\$\{?[A-Z_][A-Z0-9_]*\}?[^"']"#).unwrap())
}

fn silent_prefix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s+[^@#\s]").unwrap())
}

fn victory_lang_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(done|complete|fully admitted|all clean|solved|victory|guaranteed)\b")
            .unwrap()
    })
}

// ── Diagnostic types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Severity {
    #[allow(dead_code)]
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct JustDiagnostic {
    pub line: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

// ── Parser ────────────────────────────────────────────────────────────────────

pub fn parse(content: &str) -> Vec<JustDiagnostic> {
    let mut diags = Vec::new();
    let mut in_recipe = false;
    let mut recipe_name = String::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_u = line_idx as u32;

        // Skip comments
        if line.trim_start().starts_with('#') {
            continue;
        }

        // Recipe declaration
        if let Some(cap) = recipe_re().captures(line) {
            in_recipe = true;
            recipe_name = cap[1].to_string();

            // JUST-001: recipe names should be kebab-case
            if recipe_name.contains('_') && !recipe_name.starts_with('_') {
                diags.push(JustDiagnostic {
                    line: line_u,
                    column_start: 0,
                    column_end: recipe_name.len() as u32,
                    severity: Severity::Warning,
                    code: "JUST-001",
                    message: format!(
                        "Recipe '{recipe_name}' uses underscore — prefer kebab-case for just recipes"
                    ),
                });
            }

            // JUST-002: private recipes (prefixed _) should not be in the default list implicitly
            if recipe_name.starts_with('_') {
                diags.push(JustDiagnostic {
                    line: line_u,
                    column_start: 0,
                    column_end: recipe_name.len() as u32,
                    severity: Severity::Info,
                    code: "JUST-002",
                    message: format!(
                        "Recipe '{recipe_name}' is prefixed with '_' — it will be hidden from `just --list`"
                    ),
                });
            }

            continue;
        }

        // Reset recipe context on blank line or top-level declaration
        if line.trim().is_empty() || (!line.starts_with(' ') && !line.starts_with('\t')) {
            in_recipe = false;
            recipe_name.clear();
        }

        if in_recipe {
            // JUST-003: recipe body lines without @ prefix echo the command — warn in diagnostic recipes
            if let Some(mat) = silent_prefix_re().find(line) {
                if recipe_name.contains("dx-") || recipe_name.contains("bench-") {
                    let _ = mat; // used below for column
                                 // Only warn if the line is a non-trivial shell command (contains a space after indent)
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("cargo ")
                        || trimmed.starts_with("echo ")
                        || trimmed.starts_with("bash ")
                    {
                        diags.push(JustDiagnostic {
                            line: line_u,
                            column_start: (line.len() - trimmed.len()) as u32,
                            column_end: line.len() as u32,
                            severity: Severity::Info,
                            code: "JUST-003",
                            message: format!(
                                "Recipe '{recipe_name}' body line is not prefixed with '@' — command will be echoed to stdout"
                            ),
                        });
                    }
                }
            }

            // JUST-004: unquoted variable expansion in shell strings (potential injection)
            if let Some(mat) = shell_inject_re().find(line) {
                diags.push(JustDiagnostic {
                    line: line_u,
                    column_start: mat.start() as u32,
                    column_end: mat.end() as u32,
                    severity: Severity::Warning,
                    code: "JUST-004",
                    message: "Unquoted variable expansion — wrap in double-quotes to prevent word-splitting".to_string(),
                });
            }
        }

        // JUST-005: victory language in comments or echo strings
        if let Some(mat) = victory_lang_re().find(line) {
            // Only flag in comments or echo calls, not in variable names
            let trimmed = line.trim_start();
            if trimmed.starts_with('#')
                || trimmed.starts_with("echo ")
                || trimmed.starts_with("@echo ")
            {
                diags.push(JustDiagnostic {
                    line: line_u,
                    column_start: mat.start() as u32,
                    column_end: mat.end() as u32,
                    severity: Severity::Warning,
                    code: "JUST-005",
                    message: format!(
                        "Victory language '{}' in Justfile comment or echo — use bounded status words (ADMITTED, CANDIDATE, BLOCKED)",
                        &line[mat.start()..mat.end()]
                    ),
                });
            }
        }
    }

    diags
}
