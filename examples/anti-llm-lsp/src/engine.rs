use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;
use crate::parsers::{
    cargo_lock, cargo_toml, json_rpc, markdown_claims, receipt_json, rust_tree_sitter, typescript,
};
use crate::rules::{
    authority, claims, lsp318, mutation, ocel_rules, receipts, routes, rust_smells, surface, test,
    typescript as ts_rules, version,
};
use std::fs;
use std::path::Path;

pub fn scan_file(filepath: &str) -> Vec<Observation> {
    let mut obs = Vec::new();
    let path = Path::new(filepath);
    if !path.is_file() {
        return obs;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return obs,
    };

    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();

    // 1. Raw Text Scan
    let raw_smells = [
        ("tower-lsp", "tower-lsp"),
        ("tower_lsp", "tower_lsp"),
        ("CLAP", "CLAP"),
        ("Victory confirmed", "Victory confirmed"),
        ("fully admitted", "fully admitted"),
        ("all gaps resolved", "all gaps resolved"),
        ("successfully proven", "successfully proven"),
        ("Routing to PackPlan", "Routing to PackPlan"),
        ("test result: ok", "test result: ok"),
        ("v1.0.0", "v1.0.0"),
        ("version = \"1.0.0\"", "version = \"1.0.0\""),
        ("CLAP-DEBUG", "CLAP-DEBUG"),
        ("CLAP-DEBUG-PATH", "CLAP-DEBUG-PATH"),
        ("Content was:", "Content was:"),
        ("Path was:", "Path was:"),
        ("static scan as route proof", "static scan as route proof"),
        ("static scan", "static scan"),
        ("route proof", "route proof"),
        (
            "ChangelogCoverage(15 rows) => SpecCoverage(LSP 3.18)",
            "ChangelogCoverage(15 rows) => SpecCoverage(LSP 3.18)",
        ),
        (
            "ChangelogCoverage(15 rows) \u{21d2} SpecCoverage(LSP 3.18)",
            "ChangelogCoverage(15 rows) \u{21d2} SpecCoverage(LSP 3.18)",
        ),
        (
            "15-row changelog matrix is being treated as full LSP 3.18 combinatorial coverage",
            "15-row changelog matrix is being treated as full LSP 3.18 combinatorial coverage",
        ),
        ("ANTI-LLM-OCEL-001-TRIGGER", "ANTI-LLM-OCEL-001-TRIGGER"),
        ("ANTI-LLM-OCEL-002-TRIGGER", "ANTI-LLM-OCEL-002-TRIGGER"),
        ("\"bypassed_compat\": true", "\"bypassed_compat\": true"),
        ("use wasm4pm::", "use wasm4pm::"),
    ];

    for &(smell, orig) in &raw_smells {
        if content.contains(smell) {
            let occurrences: Vec<_> = content.match_indices(smell).collect();
            for (idx, _) in occurrences {
                // Check if followed by max/max-related suffixes (i.e. not plain tower-lsp)
                if smell == "tower-lsp" || smell == "tower_lsp" {
                    let suffix = &content[idx + smell.len()..];
                    if suffix.starts_with("-max")
                        || suffix.starts_with("_max")
                        || suffix.starts_with("::max")
                    {
                        continue;
                    }
                }

                if filepath.ends_with("src/rules/lsp318.rs")
                    || filepath.ends_with("src/engine.rs")
                    || filepath.ends_with("rules/lsp318.rs")
                    || filepath.ends_with("engine.rs")
                {
                    continue;
                }

                // Approximate line/col
                let line_count = content[..idx].lines().count() + 1;
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: idx,
                    end_byte: idx + smell.len(),
                    line: line_count,
                    column: 1,
                    kind: "raw_text".to_string(),
                    construct: orig.to_string(),
                    context: smell.to_string(),
                    message: format!("Raw text pattern '{}' detected", orig),
                });
            }
        }
    }

    // Additional V0 test checks (ANTI-LLM-TEST-001 and ANTI-LLM-TEST-003)
    let is_test_file = filepath.contains("tests/")
        || filepath.ends_with("_test.rs")
        || filepath.contains("/test/");
    if is_test_file {
        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;
            if line.contains("assert") && line.contains(".contains") {
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    line: line_num,
                    column: 1,
                    kind: "test_smell".to_string(),
                    construct: "assert_contains".to_string(),
                    context: line.to_string(),
                    message: "String assertion containing '.contains' detected in test file"
                        .to_string(),
                });
            }
            if !filepath.contains("dogfood.rs") && line.contains("negative_controls") {
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    line: line_num,
                    column: 1,
                    kind: "test_smell".to_string(),
                    construct: "negative_control_reference".to_string(),
                    context: line.to_string(),
                    message: "Standard test references negative controls directory".to_string(),
                });
            }
        }
    }

    // 2. Specific Parsers
    if filename == "Cargo.toml" {
        obs.extend(cargo_toml::parse_cargo_toml(filepath, &content));
    } else if filename == "Cargo.lock" {
        obs.extend(cargo_lock::parse_cargo_lock(filepath, &content));
    } else if filename.ends_with(".rs") {
        obs.extend(rust_tree_sitter::parse_rust_ast(filepath, &content));
    } else if filename.ends_with(".md") {
        obs.extend(markdown_claims::parse_markdown_claims(filepath, &content));
    } else if filename.ends_with(".json") || filename.ends_with(".jsonl") {
        if filepath.contains("transcripts") {
            obs.extend(json_rpc::parse_json_rpc_transcript(filepath, &content));
        } else if filepath.contains("receipts") {
            obs.extend(receipt_json::parse_receipt_json(filepath, &content));
        }
    } else if filename.ends_with(".ts")
        || filename.ends_with(".tsx")
        || filename.ends_with(".js")
        || filename.ends_with(".jsx")
        || filename.ends_with(".mts")
        || filename.ends_with(".mjs")
        || filename.ends_with(".cts")
        || filename.ends_with(".cjs")
    {
        obs.extend(typescript::parse_typescript(filepath, &content));
    }

    obs
}

pub fn scan_directory(dirpath: &str) -> Vec<Observation> {
    let mut obs = Vec::new();
    let path = Path::new(dirpath);
    if !path.is_dir() {
        return obs;
    }

    let mut queue = vec![path.to_path_buf()];
    while let Some(dir) = queue.pop() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                if p.is_dir() {
                    if name != "target"
                        && name != ".git"
                        && name != ".cargo"
                        && name != "fixtures"
                        && name != "node_modules"
                    {
                        queue.push(p);
                    }
                } else {
                    obs.extend(scan_file(&p.to_string_lossy()));
                }
            }
        }
    }
    obs
}

pub fn evaluate_diagnostics(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    // Surface rules
    diags.extend(surface::evaluate(obs));

    // Authority rules
    diags.extend(authority::evaluate(obs));

    // Receipt rules
    diags.extend(receipts::evaluate(obs));

    // Route rules
    diags.extend(routes::evaluate(obs));

    // Mutation rules
    diags.extend(mutation::evaluate(obs));

    // Version rules
    diags.extend(version::evaluate(obs));

    // Test rules
    diags.extend(test::evaluate(obs));

    // Rust smells / strange-code rules
    diags.extend(rust_smells::evaluate(obs));

    // LSP 3.18 rules
    diags.extend(lsp318::evaluate(obs));

    // OCEL and wasm4pm-compat rules
    diags.extend(ocel_rules::evaluate(obs));

    // TypeScript rules
    diags.extend(ts_rules::evaluate(obs));

    // Claims rules: check if victory language appears while other diagnostics are present
    let other_non_victory_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.code != "ANTI-LLM-CLAIM-004")
        .collect();
    let has_non_victory_errors = !other_non_victory_diags.is_empty();
    diags.extend(claims::evaluate(obs, has_non_victory_errors));

    diags
}
