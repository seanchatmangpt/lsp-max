use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct TemplateEntry {
    pub name: String,
    pub pack: String,
    pub path: String,
    pub size_bytes: u64,
    pub variables_hint: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateListResult {
    pub templates: Vec<TemplateEntry>,
    pub total: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateInfoResult {
    pub name: String,
    pub pack: String,
    pub path: String,
    pub content_preview: String, // first 20 lines
    pub variables_hint: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateRenderResult {
    pub template: String,
    pub rendered: String,
    pub variables_used: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateValidateResult {
    pub template: String,
    pub valid: bool,
    pub issues: Vec<String>,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct TemplateService;

impl TemplateService {
    pub fn new() -> Self {
        Self
    }

    fn templates_root(base: &str) -> PathBuf {
        PathBuf::from(base).join("templates")
    }

    fn extract_variables(content: &str) -> Vec<String> {
        // Extract {{ variable }} patterns from Tera template syntax.
        // Control structures (for/if/endfor) are intentionally skipped.
        let mut vars = vec![];
        let mut remaining = content;
        while let Some(start) = remaining.find("{{") {
            remaining = &remaining[start + 2..];
            if let Some(end) = remaining.find("}}") {
                let expr = remaining[..end].trim().to_string();
                if !expr.starts_with('%') && !expr.contains("for ") && !expr.contains("if ") {
                    let var = expr.split('|').next().unwrap_or("").trim().to_string();
                    if !var.is_empty() && !vars.contains(&var) {
                        vars.push(var);
                    }
                }
                remaining = &remaining[end + 2..];
            } else {
                break;
            }
        }
        vars
    }

    pub fn list(&self, base: &str, pack: Option<&str>) -> TemplateListResult {
        let root = Self::templates_root(base);
        let mut templates = vec![];

        let mut scan_dir = |dir: &PathBuf, pack_name: &str| {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("tera") {
                        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        let variables_hint = Self::extract_variables(&content);
                        templates.push(TemplateEntry {
                            name: path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            pack: pack_name.to_string(),
                            path: path.display().to_string(),
                            size_bytes: size,
                            variables_hint,
                        });
                    }
                }
            }
        };

        if let Some(p) = pack {
            scan_dir(&root.join(p), p);
        } else if let Ok(packs) = fs::read_dir(&root) {
            for pack_entry in packs.flatten() {
                let pack_path = pack_entry.path();
                if pack_path.is_dir() {
                    let pname = pack_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    scan_dir(&pack_path, &pname);
                }
            }
        }

        templates.sort_by(|a, b| a.name.cmp(&b.name));
        let total = templates.len();
        TemplateListResult {
            templates,
            total,
            status: "CANDIDATE".into(),
        }
    }

    pub fn info(&self, name: &str, pack: &str, base: &str) -> Result<TemplateInfoResult> {
        let path = Self::templates_root(base).join(pack).join(name);
        let content = fs::read_to_string(&path).map_err(|e| {
            clap_noun_verb::error::NounVerbError::execution_error(format!(
                "template not found: {}: {e}",
                path.display()
            ))
        })?;
        let preview = content.lines().take(20).collect::<Vec<_>>().join("\n");
        let variables_hint = Self::extract_variables(&content);
        Ok(TemplateInfoResult {
            name: name.to_string(),
            pack: pack.to_string(),
            path: path.display().to_string(),
            content_preview: preview,
            variables_hint,
            status: "CANDIDATE".into(),
        })
    }

    pub fn render(
        &self,
        name: &str,
        pack: &str,
        base: &str,
        vars: serde_json::Value,
    ) -> Result<TemplateRenderResult> {
        let path = Self::templates_root(base).join(pack).join(name);
        let content = fs::read_to_string(&path)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

        let variables_used = Self::extract_variables(&content);
        let mut ctx = tera::Context::new();
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj {
                ctx.insert(k.as_str(), v);
            }
        }
        let rendered = tera::Tera::one_off(&content, &ctx, false)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

        Ok(TemplateRenderResult {
            template: name.to_string(),
            rendered,
            variables_used,
            status: "CANDIDATE".into(),
        })
    }

    pub fn validate(&self, name: &str, pack: &str, base: &str) -> TemplateValidateResult {
        let path = Self::templates_root(base).join(pack).join(name);
        let mut issues = vec![];

        match fs::read_to_string(&path) {
            Err(e) => issues.push(format!("OPEN: cannot read template: {e}")),
            Ok(content) => {
                // Law enforcement: templates must not reference forbidden identifiers.
                if content.contains("tower-lsp") || content.contains("tower_lsp") {
                    issues.push(
                        "LawViolation: template contains forbidden tower-lsp reference".into(),
                    );
                }
                // Dry-parse via Tera; variable-undefined errors are expected without a context.
                let result = tera::Tera::one_off(&content, &tera::Context::new(), false);
                if let Err(e) = result {
                    let msg = e.to_string();
                    if !msg.contains("Variable") && !msg.contains("undefined") {
                        issues.push(format!("BLOCKED: Tera parse error: {msg}"));
                    }
                }
            }
        }

        TemplateValidateResult {
            template: name.to_string(),
            valid: issues.is_empty(),
            issues,
            status: "CANDIDATE".into(),
        }
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("list")]
pub fn list(pack: Option<String>, dir: Option<String>) -> Result<TemplateListResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(TemplateService::new().list(&dir, pack.as_deref()))
}

#[verb("info")]
pub fn info(name: String, pack: Option<String>, dir: Option<String>) -> Result<TemplateInfoResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let pack = pack.unwrap_or_else(|| "lsp-max".to_string());
    TemplateService::new().info(&name, &pack, &dir)
}

#[verb("render")]
pub fn render(
    name: String,
    pack: Option<String>,
    dir: Option<String>,
    vars: Option<String>,
) -> Result<TemplateRenderResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let pack = pack.unwrap_or_else(|| "lsp-max".to_string());
    let vars: serde_json::Value = vars
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));
    TemplateService::new().render(&name, &pack, &dir, vars)
}

#[verb("validate")]
pub fn validate(
    name: String,
    pack: Option<String>,
    dir: Option<String>,
) -> Result<TemplateValidateResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let pack = pack.unwrap_or_else(|| "lsp-max".to_string());
    Ok(TemplateService::new().validate(&name, &pack, &dir))
}
