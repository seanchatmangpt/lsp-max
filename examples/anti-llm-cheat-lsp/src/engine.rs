use crate::config::AntiLlmConfig;
use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;
use crate::parsers::{
    cargo_lock, cargo_toml, json_rpc, markdown_claims, receipt_json, rust_tree_sitter, typescript,
};
use crate::rules::{
    authority, claims, determinism, lsp318, mutation, ocel_rules, receipts, routes, rust_smells,
    surface, test, typescript as ts_rules, version,
};
use aho_corasick::AhoCorasick;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

// ── Line index — O(n) build, O(log n) lookup ──────────────────────────────────

fn build_line_index(content: &[u8]) -> Vec<usize> {
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
//
// Victory-language terms are intentionally absent here. They are owned by
// `rules::claims::VICTORY_TERMS` and detected by a separate pass so that
// per-repo domain-term exemptions can be applied before emitting diagnostics.

const RAW_SMELL_PATTERNS: &[&str] = &[
    "tower-lsp",                                            // 0 — needs lsp-max suffix check
    "tower_lsp",                                            // 1 — needs lsp-max suffix check
    "CLAP",                                                 // 2
    "Routing to PackPlan",                                  // 3
    "test result: ok",                                      // 4
    "v1.0.0",                                               // 5
    "version = \"1.0.0\"",                                  // 6
    "CLAP-DEBUG",                                           // 7
    "CLAP-DEBUG-PATH",                                      // 8
    "Content was:",                                         // 9
    "Path was:",                                            // 10
    "static scan as route proof", // 11 (before "static scan" — LeftmostLongest)
    "static scan",                // 12
    "route proof",                // 13
    "ChangelogCoverage(15 rows) => SpecCoverage(LSP 3.18)", // 14
    "ChangelogCoverage(15 rows) \u{21d2} SpecCoverage(LSP 3.18)", // 15
    "15-row changelog matrix is being treated as full LSP 3.18 combinatorial coverage", // 16
    "ANTI-LLM-OCEL-001-TRIGGER",  // 17
    "ANTI-LLM-OCEL-002-TRIGGER",  // 18
    "\"bypassed_compat\": true",  // 19
    "use wasm4pm::",              // 20
];

fn raw_smell_ac() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| {
        aho_corasick::AhoCorasickBuilder::new()
            .match_kind(aho_corasick::MatchKind::LeftmostLongest)
            .build(RAW_SMELL_PATTERNS)
            .unwrap()
    })
}

// ── TEST-001 helper — classify .contains() receiver ──────────────────────────

/// Classify a test-file line that contains both `assert` and `.contains`.
///
/// Returns the `construct` string for the resulting observation:
///
/// - `"assert_contains_string"` — argument is a string literal, e.g.
///   `assert!(x.to_string().contains("VariantName"))`. This is the real cheat:
///   the test couples to the Display representation instead of the type.
///
/// - `"assert_contains_structural"` — argument is a reference or enum path,
///   e.g. `assert!(vec.contains(&Enum::Variant))`. This is structural equality
///   via `PartialEq` — acceptable.
///
/// - `"assert_contains"` — receiver cannot be classified from the line text.
///   Flagged conservatively as a potential cheat.
fn classify_contains(line: &str) -> &'static str {
    // Find the `.contains(` token to examine what immediately follows the `(`.
    let Some(pos) = line.find(".contains(") else {
        return "assert_contains";
    };
    let after = line[pos + ".contains(".len()..].trim_start();

    if after.starts_with('"') || after.starts_with("r\"") || after.starts_with("r#\"") {
        // String literal argument → Display / output cheat
        "assert_contains_string"
    } else if after.starts_with('&') || after.starts_with("&&") {
        // Reference argument → structural PartialEq check (Vec::contains(&T))
        "assert_contains_structural"
    } else if after.starts_with("format!") || after.starts_with("&format!") {
        // format!() argument → the string is constructed then searched → cheat
        "assert_contains_string"
    } else {
        // Cannot classify — flag conservatively
        "assert_contains"
    }
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

    // 2. Victory-language scan (delegated to claims rule vocabulary)
    //    Domain-term exemptions are applied later in evaluate_diagnostics.
    if !is_self_excluded {
        // Pass empty domain_terms — exemptions apply at evaluate time.
        obs.extend(claims::scan_for_victory(
            filepath,
            &content,
            "raw_text",
            &[],
        ));
    }

    // 3. Test-file checks
    let is_test_file = filepath.contains("tests/")
        || filepath.ends_with("_test.rs")
        || filepath.contains("/test/");
    if is_test_file {
        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;
            if line.contains("assert") && line.contains(".contains") {
                let construct = classify_contains(line);
                obs.push(Observation {
                    file_path: filepath.to_string(),
                    start_byte: 0,
                    end_byte: 0,
                    line: line_num,
                    column: 1,
                    kind: "test_smell".to_string(),
                    construct: construct.to_string(),
                    context: line.to_string(),
                    message: format!(
                        ".contains() assertion classified as '{}' in test file",
                        construct
                    ),
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

    // 4. Type-specific parsers
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

/// Evaluate diagnostics with a default (all-empty) config.
///
/// Suitable for programmatic callers that do not have a scan directory.
/// Callers with a directory should prefer `evaluate_diagnostics_with_config`.
pub fn evaluate_diagnostics(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    evaluate_diagnostics_with_config(obs, &AntiLlmConfig::default())
}

/// Evaluate diagnostics using a per-repo config loaded from `anti-llm.toml`.
pub fn evaluate_diagnostics_with_config(
    obs: &[Observation],
    config: &AntiLlmConfig,
) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    diags.extend(surface::evaluate(obs, config));
    diags.extend(authority::evaluate(obs));
    diags.extend(receipts::evaluate(obs));
    diags.extend(routes::evaluate(obs));
    diags.extend(mutation::evaluate(obs));
    diags.extend(version::evaluate(obs));
    diags.extend(test::evaluate(obs));
    diags.extend(rust_smells::evaluate(obs));
    diags.extend(determinism::evaluate(obs));
    diags.extend(lsp318::evaluate(obs));
    diags.extend(ocel_rules::evaluate(obs));
    diags.extend(ts_rules::evaluate(obs));

    let has_non_victory_errors = diags.iter().any(|d| d.code != "ANTI-LLM-CLAIM-004");
    diags.extend(claims::evaluate(
        obs,
        &config.claim.domain_terms,
        has_non_victory_errors,
    ));

    // Deduplicate by (file_path, line, code)
    let mut seen = std::collections::HashSet::new();
    diags.retain(|d| seen.insert((d.file_path.clone(), d.line, d.code.clone())));

    diags
}
