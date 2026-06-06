use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use glob::Pattern;
use clap_noun_verb::Result;

use crate::diagnostics::{Finding, Receipt};
use crate::rules::{Rule, RulePack};

pub fn load_rules() -> Result<Vec<Rule>> {
    let mut rules = Vec::new();
    let rules_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("rules");
    
    if rules_dir.exists() {
        for entry in fs::read_dir(rules_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("toml") {
                let content = fs::read_to_string(entry.path())?;
                let pack: RulePack = toml::from_str(&content).map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
                rules.extend(pack.rules);
            }
        }
    }
    
    Ok(rules)
}

pub fn scan_workspace() -> Result<usize> {
    let rules = load_rules()?;
    let mut workspace_root = std::env::current_dir()?;
    // Walk up until we find a Cargo.toml with [workspace] or hit root
    while !workspace_root.join("Cargo.toml").exists() || !fs::read_to_string(workspace_root.join("Cargo.toml")).unwrap_or_default().contains("[workspace]") {
        if let Some(parent) = workspace_root.parent() {
            workspace_root = parent.to_path_buf();
        } else {
            workspace_root = std::env::current_dir()?;
            break;
        }
    }
    
    let scan_scope = vec![
        "src".to_string(), "tests".to_string(), "examples".to_string(), 
        "benches".to_string(), "playground".to_string(), "crates".to_string()
    ];
    
    let mut count = 0;
    
    for scope in &scan_scope {
        let path = workspace_root.join(scope);
        if !path.exists() { continue; }
        
        let walker = glob::glob(&format!("{}/**/*.rs", path.display()))
            .map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
            
        for entry in walker {
            let entry = entry.map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
            let rel_path = entry.strip_prefix(&workspace_root).unwrap_or(&entry);
            let path_str = rel_path.to_string_lossy().to_string();
            
            let content = match fs::read_to_string(&entry) {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            for rule in &rules {
                let mut matches_glob = false;
                for g in &rule.path_globs {
                    if Pattern::new(g).map(|p| p.matches(&path_str)).unwrap_or(false) {
                        matches_glob = true;
                        break;
                    }
                }
                if !matches_glob && !rule.path_globs.is_empty() { continue; }
                
                let mut excluded = false;
                for g in &rule.exclude_globs {
                    if Pattern::new(g).map(|p| p.matches(&path_str)).unwrap_or(false) {
                        excluded = true;
                        break;
                    }
                }
                if excluded { continue; }
                
                let re = Regex::new(&rule.pattern).map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
                
                for (line_idx, line) in content.lines().enumerate() {
                    if let Some(mat) = re.find(line) {
                        let finding = Finding {
                            source: "pattern-lsp".to_string(),
                            rule_id: rule.id.clone(),
                            path: path_str.clone(),
                            line: line_idx + 1,
                            column: mat.start() + 1,
                            severity: rule.severity.clone(),
                            matched_text: mat.as_str().to_string(),
                            workspace_root: workspace_root.to_string_lossy().to_string(),
                            scan_scope: scan_scope.clone(),
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

pub fn scan_document(uri: &str, content: &str) -> Result<Vec<Finding>> {
    let rules = load_rules()?;
    let mut findings = Vec::new();
    
    // We mock the path if needed, since URI could be anything.
    let path_str = uri.strip_prefix("file://").unwrap_or(uri).to_string();
    
    for rule in &rules {
        let re = Regex::new(&rule.pattern).map_err(|e| clap_noun_verb::NounVerbError::execution_error(e.to_string()))?;
        
        for (line_idx, line) in content.lines().enumerate() {
            if let Some(mat) = re.find(line) {
                findings.push(Finding {
                    source: "pattern-lsp".to_string(),
                    rule_id: rule.id.clone(),
                    path: path_str.clone(),
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
