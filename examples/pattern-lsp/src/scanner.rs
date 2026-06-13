use glob::Pattern;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use clap_noun_verb::Result;

use crate::diagnostics::{Finding, Receipt};
use crate::rules::{Rule, RulePack};

// ── Compiled rule (pre-compiled regex + glob patterns) ────────────────────────

struct CompiledRule {
    id: String,
    severity: String,
    re: Regex,
    path_globs: Vec<Pattern>,
    exclude_globs: Vec<Pattern>,
}

impl CompiledRule {
    fn from_rule(rule: Rule) -> Option<Self> {
        let re = Regex::new(&rule.pattern).ok()?;
        let path_globs = rule
            .path_globs
            .iter()
            .filter_map(|g| Pattern::new(g).ok())
            .collect();
        let exclude_globs = rule
            .exclude_globs
            .iter()
            .filter_map(|g| Pattern::new(g).ok())
            .collect();
        Some(Self {
            id: rule.id,
            severity: rule.severity,
            re,
            path_globs,
            exclude_globs,
        })
    }

    fn matches_path(&self, path_str: &str) -> bool {
        let included =
            self.path_globs.is_empty() || self.path_globs.iter().any(|p| p.matches(path_str));
        let excluded = self.exclude_globs.iter().any(|p| p.matches(path_str));
        included && !excluded
    }
}

// ── Rule cache — compiled once per process ────────────────────────────────────

fn compiled_rules() -> &'static Vec<CompiledRule> {
    static RULES: OnceLock<Vec<CompiledRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        load_raw_rules()
            .unwrap_or_default()
            .into_iter()
            .filter_map(CompiledRule::from_rule)
            .collect()
    })
}

fn load_raw_rules() -> Result<Vec<Rule>> {
    let mut rules = Vec::new();
    let rules_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("rules");

    if rules_dir.exists() {
        for entry in fs::read_dir(rules_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("toml") {
                let content = fs::read_to_string(entry.path())?;
                let pack: RulePack = toml::from_str(&content)
                    .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
                rules.extend(pack.rules);
            }
        }
    }

    Ok(rules)
}

// ── Workspace scan ────────────────────────────────────────────────────────────

pub fn scan_workspace() -> Result<usize> {
    let rules = compiled_rules();
    let mut workspace_root = std::env::current_dir()?;

    while !workspace_root.join("Cargo.toml").exists()
        || !fs::read_to_string(workspace_root.join("Cargo.toml"))
            .unwrap_or_default()
            .contains("[workspace]")
    {
        if let Some(parent) = workspace_root.parent() {
            workspace_root = parent.to_path_buf();
        } else {
            workspace_root = std::env::current_dir()?;
            break;
        }
    }

    let scan_scope = [
        "src",
        "tests",
        "examples",
        "benches",
        "playground",
        "crates",
    ];
    let mut count = 0;

    for scope in &scan_scope {
        let scope_path = workspace_root.join(scope);
        if !scope_path.exists() {
            continue;
        }

        // ignore::WalkBuilder respects .gitignore — target/ is skipped automatically.
        let walker = ignore::WalkBuilder::new(&scope_path)
            .hidden(false)
            .build()
            .filter_map(|e| e.ok())
            .filter(|e| {
                !e.path().is_dir() && e.path().extension().and_then(|x| x.to_str()) == Some("rs")
            });

        for entry in walker {
            let rel_path = entry
                .path()
                .strip_prefix(&workspace_root)
                .unwrap_or(entry.path());
            let path_str = rel_path.to_string_lossy();

            let content = match fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for rule in rules {
                if !rule.matches_path(&path_str) {
                    continue;
                }

                for (line_idx, line) in content.lines().enumerate() {
                    if let Some(mat) = rule.re.find(line) {
                        let finding = Finding {
                            source: "pattern-lsp".to_string(),
                            rule_id: rule.id.clone(),
                            path: path_str.to_string(),
                            line: line_idx + 1,
                            column: mat.start() + 1,
                            severity: rule.severity.clone(),
                            matched_text: mat.as_str().to_string(),
                            workspace_root: workspace_root.to_string_lossy().to_string(),
                            scan_scope: scan_scope.iter().map(|s| s.to_string()).collect(),
                        };
                        let receipt: Receipt = finding.into();
                        println!("{}", serde_json::to_string(&receipt).unwrap());
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

// ── Single-document scan ──────────────────────────────────────────────────────

pub fn scan_document(uri: &str, content: &str) -> Result<Vec<Finding>> {
    let rules = compiled_rules();
    let path_str = uri.strip_prefix("file://").unwrap_or(uri);
    let mut findings = Vec::new();

    for rule in rules {
        if !rule.matches_path(path_str) {
            continue;
        }
        for (line_idx, line) in content.lines().enumerate() {
            if let Some(mat) = rule.re.find(line) {
                findings.push(Finding {
                    source: "pattern-lsp".to_string(),
                    rule_id: rule.id.clone(),
                    path: path_str.to_string(),
                    line: line_idx + 1,
                    column: mat.start() + 1,
                    severity: rule.severity.clone(),
                    matched_text: mat.as_str().to_string(),
                    workspace_root: "".into(),
                    scan_scope: vec![],
                });
            }
        }
    }
    Ok(findings)
}
