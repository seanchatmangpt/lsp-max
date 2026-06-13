use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;
use crate::parsers::{
    cargo_lock, cargo_toml, json_rpc, markdown_claims, receipt_json, rust_tree_sitter, typescript,
};
use crate::rules::{
    authority, claims, lsp318, mutation, ocel_rules, receipts, routes, rust_smells, surface, test,
    typescript as ts_rules, version,
};
use aho_corasick::AhoCorasick;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

// ── Line index — O(n) build, O(log n) lookup ──────────────────────────────────

fn build_line_index(content: &[u8]) -> Vec<usize> {
    // Collect byte offsets of every '\n'. Index i holds the byte offset of the
    // newline that ends line (i+1). A binary search on this gives line number
    // for any byte offset in O(log n) instead of O(n).
    let mut offsets = Vec::with_capacity(content.len() / 40 + 1);
    offsets.push(0); // line 1 starts at byte 0
    for pos in memchr::memchr_iter(b'\n', content) {
        offsets.push(pos + 1); // line N+1 starts after the newline
    }
    offsets
}

fn byte_to_line(line_index: &[usize], byte_offset: usize) -> usize {
    match line_index.partition_point(|&start| start <= byte_offset) {
        0 => 1,
        n => n,
    }
}

// ── Raw-smell automaton (compiled once) ───────────────────────────────────────

const RAW_SMELL_PATTERNS: &[&str] = &[
    "tower-lsp",
    "tower_lsp",
    "CLAP",
    "Victory confirmed",
    "fully admitted",
    "all gaps resolved",
    "successfully proven",
    "Routing to PackPlan",
    "test result: ok",
    "v1.0.0",
    "version = \"1.0.0\"",
    "CLAP-DEBUG",
    "CLAP-DEBUG-PATH",
    "Content was:",
    "Path was:",
    "static scan as route proof",
    "static scan",
    "route proof",
    "ChangelogCoverage(15 rows) => SpecCoverage(LSP 3.18)",
    "ChangelogCoverage(15 rows) \u{21d2} SpecCoverage(LSP 3.18)",
    "15-row changelog matrix is being treated as full LSP 3.18 combinatorial coverage",
    "ANTI-LLM-OCEL-001-TRIGGER",
    "ANTI-LLM-OCEL-002-TRIGGER",
    "\"bypassed_compat\": true",
    "use wasm4pm::",
];

fn raw_smell_ac() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| {
        // LeftmostLongest: when patterns overlap at the same start position (e.g. "CLAP" vs
        // "CLAP-DEBUG", or "static scan" vs "static scan as route proof"), the longest match
        // wins. This preserves the most-specific diagnostic construct name.
        aho_corasick::AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::LeftmostLongest)
            .build(RAW_SMELL_PATTERNS)
            .unwrap()
    })
}

// ── File scanner ──────────────────────────────────────────────────────────────

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

    // Skip self-references (engine.rs / lsp318.rs define some of these strings as data)
    let is_self_excluded = filepath.ends_with("src/rules/lsp318.rs")
        || filepath.ends_with("src/engine.rs")
        || filepath.ends_with("rules/lsp318.rs")
        || filepath.ends_with("engine.rs");

    // 1. Raw text scan — single AhoCorasick pass over entire file
    if !is_self_excluded {
        // Build line index once — O(n) using SIMD memchr, then O(log n) per match.
        let line_index = build_line_index(content.as_bytes());

        for mat in raw_smell_ac().find_iter(&content) {
            let pattern_idx = mat.pattern().as_usize();
            let smell = RAW_SMELL_PATTERNS[pattern_idx];
            let idx = mat.start();

            // tower-lsp / tower_lsp: skip lsp-max suffixed variants
            if pattern_idx == 0 || pattern_idx == 1 {
                let suffix = &content[idx + smell.len()..];
                if suffix.starts_with("-max")
                    || suffix.starts_with("_max")
                    || suffix.starts_with("::max")
                {
                    continue;
                }
            }

            let line_count = byte_to_line(&line_index, idx);
            obs.push(Observation {
                file_path: filepath.to_string(),
                start_byte: idx,
                end_byte: idx + smell.len(),
                line: line_count,
                column: 1,
                kind: "raw_text".to_string(),
                construct: smell.to_string(),
                context: smell.to_string(),
                message: format!("Raw text pattern '{}' detected", smell),
            });
        }
    }

    // 2. Test-file checks
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

    // 3. Type-specific parsers
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

// ── Directory scanner — ignore::Walk respects .gitignore ─────────────────────

pub fn scan_directory(dirpath: &str) -> Vec<Observation> {
    let mut obs = Vec::new();
    let path = Path::new(dirpath);
    if !path.is_dir() {
        return obs;
    }

    // ignore::WalkBuilder skips target/, .git/, node_modules/ via .gitignore automatically.
    // We still explicitly add fixtures/ as an override since it may not be in .gitignore.
    let walker = ignore::WalkBuilder::new(path)
        .hidden(false)
        .add_custom_ignore_filename(".anti-llm-ignore")
        .filter_entry(|e| e.file_name().to_string_lossy() != "fixtures")
        .build();

    for entry in walker.flatten() {
        if entry.path().is_file() {
            obs.extend(scan_file(&entry.path().to_string_lossy()));
        }
    }

    obs
}

pub fn evaluate_diagnostics(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    diags.extend(surface::evaluate(obs));
    diags.extend(authority::evaluate(obs));
    diags.extend(receipts::evaluate(obs));
    diags.extend(routes::evaluate(obs));
    diags.extend(mutation::evaluate(obs));
    diags.extend(version::evaluate(obs));
    diags.extend(test::evaluate(obs));
    diags.extend(rust_smells::evaluate(obs));
    diags.extend(lsp318::evaluate(obs));
    diags.extend(ocel_rules::evaluate(obs));
    diags.extend(ts_rules::evaluate(obs));

    let has_non_victory_errors = diags.iter().any(|d| d.code != "ANTI-LLM-CLAIM-004");
    diags.extend(claims::evaluate(obs, has_non_victory_errors));

    diags
}
