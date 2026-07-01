use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pattern {
    ServiceBased,
    LSPHarness,
    Dogfood,
    PropertySweep,
    Merger,
    Other,
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::ServiceBased => write!(f, "Service-Based"),
            Pattern::LSPHarness => write!(f, "LSP Backend Harness"),
            Pattern::Dogfood => write!(f, "Diagnostic Dogfood"),
            Pattern::PropertySweep => write!(f, "Deterministic Property Sweep"),
            Pattern::Merger => write!(f, "Merger/Dedup"),
            Pattern::Other => write!(f, "Other"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssertionType {
    AssertTrue,
    AssertFalse,
    AssertEq,
    AssertNe,
    IsOk,
    IsErr,
    IsSome,
    IsNone,
    Panic,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFunction {
    pub name: String,
    pub framework: String,
    pub assertions: Vec<AssertionType>,
    pub patterns: Vec<Pattern>,
    pub is_hollow: bool,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFile {
    pub path: String,
    pub test_count: usize,
    pub hollow_count: usize,
    pub falsification_ratio: f64,
    pub tests: Vec<TestFunction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub summary: ReportSummary,
    pub by_file: Vec<TestFile>,
    pub hollow_tests: Vec<HollowTest>,
    pub patterns: PatternStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_tests: usize,
    pub substantive_tests: usize,
    pub hollow_tests: usize,
    pub hollow_ratio: f64,
    pub scanned_files: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HollowTest {
    pub file: String,
    pub test_name: String,
    pub assertion_count: usize,
    pub assertion_types: Vec<AssertionType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternStats {
    pub service_based: usize,
    pub lsp_harness: usize,
    pub dogfood: usize,
    pub property_sweep: usize,
    pub merger: usize,
    pub other: usize,
}

pub struct TestAnalyzer;

impl TestAnalyzer {
    pub fn detect_pattern(code: &str) -> Pattern {
        // Service-Based: make_temp_* + .new() + assert_eq!
        if code.contains("make_temp_") && code.contains(".new()") && code.contains("assert_eq!") {
            return Pattern::ServiceBased;
        }

        // LSP Backend Harness: #[tokio::test] + server.initialize + assert_eq!
        if code.contains("#[tokio::test]") && code.contains("initialize") && code.contains("assert_eq!")
        {
            return Pattern::LSPHarness;
        }

        // Diagnostic Dogfood: engine::evaluate + check_diag_code
        if code.contains("engine::evaluate") || (code.contains("evaluate") && code.contains("check_diag"))
        {
            return Pattern::Dogfood;
        }

        // Deterministic Property Sweep: GRID/case sweep loops + assert_eq!
        if (code.contains("GRID") || code.contains("case") || code.contains("for "))
            && code.contains("assert_eq!")
        {
            return Pattern::PropertySweep;
        }

        // Merger/Dedup: router.register + servers_for_uri + multiple asserts
        if (code.contains("router.register") || code.contains("register_server"))
            && (code.contains("servers_for_uri") || code.contains("servers"))
        {
            return Pattern::Merger;
        }

        Pattern::Other
    }

    pub fn detect_assertions(code: &str) -> Vec<AssertionType> {
        let mut assertions = Vec::new();

        // Use regex to find assertion macros
        if let Ok(re_eq) = Regex::new(r"assert_eq!\s*\(") {
            if re_eq.is_match(code) {
                assertions.push(AssertionType::AssertEq);
            }
        }

        if let Ok(re_ne) = Regex::new(r"assert_ne!\s*\(") {
            if re_ne.is_match(code) {
                assertions.push(AssertionType::AssertNe);
            }
        }

        if let Ok(re_true) = Regex::new(r"assert!\s*\([^!]*\)") {
            if re_true.is_match(code) {
                // Simple heuristic: if followed by !, it's likely assert!(..., ...) or similar
                if !code.contains("assert_ne!") {
                    assertions.push(AssertionType::AssertTrue);
                }
            }
        }

        if let Ok(re_false) = Regex::new(r"assert!\s*\(\s*!\s*") {
            if re_false.is_match(code) {
                assertions.push(AssertionType::AssertFalse);
            }
        }

        if let Ok(re_ok) = Regex::new(r"\.is_ok\(\)") {
            if re_ok.is_match(code) {
                assertions.push(AssertionType::IsOk);
            }
        }

        if let Ok(re_err) = Regex::new(r"\.is_err\(\)") {
            if re_err.is_match(code) {
                assertions.push(AssertionType::IsErr);
            }
        }

        if let Ok(re_some) = Regex::new(r"\.is_some\(\)") {
            if re_some.is_match(code) {
                assertions.push(AssertionType::IsSome);
            }
        }

        if let Ok(re_none) = Regex::new(r"\.is_none\(\)") {
            if re_none.is_match(code) {
                assertions.push(AssertionType::IsNone);
            }
        }

        if let Ok(re_panic) = Regex::new(r"panic!|unwrap\(\)|expect\(") {
            if re_panic.is_match(code) {
                assertions.push(AssertionType::Panic);
            }
        }

        assertions
    }

    pub fn classify_test_function(code: &str, function_name: &str) -> TestFunction {
        let assertions = Self::detect_assertions(code);
        let pattern = Self::detect_pattern(code);

        // Hollow test detection: only single is_ok()/is_err()/is_some() without follow-ups
        let is_hollow = (assertions.len() == 1)
            && matches!(
                assertions[0],
                AssertionType::IsOk
                    | AssertionType::IsErr
                    | AssertionType::IsSome
                    | AssertionType::IsNone
            );

        let framework = if code.contains("#[tokio::test]") {
            "tokio::test".to_string()
        } else if code.contains("#[test]") {
            "test".to_string()
        } else {
            "unknown".to_string()
        };

        let line_count = code.lines().count();
        let patterns = vec![pattern];

        TestFunction {
            name: function_name.to_string(),
            framework,
            assertions,
            patterns,
            is_hollow,
            line_count,
        }
    }

    pub fn score_falsification(test_fns: &[TestFunction]) -> (usize, usize, f64) {
        let total = test_fns.len();
        let hollow = test_fns.iter().filter(|t| t.is_hollow).count();
        let substantive = total - hollow;
        let ratio = if total == 0 {
            0.0
        } else {
            (hollow as f64) / (total as f64)
        };

        (substantive, hollow, ratio)
    }

    pub fn analyze_file(path: &Path) -> anyhow::Result<TestFile> {
        let content = std::fs::read_to_string(path)?;
        let path_str = path.display().to_string();

        // Extract test functions using regex to find #[test] or #[tokio::test]
        let mut tests = Vec::new();

        // Match test functions more carefully
        if let Ok(re_test) =
            Regex::new(r#"(#\[(tokio::)?test\].*?\n\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\([^)]*\)\s*\{)"#)
        {
            let mut last_pos = 0;
            for cap in re_test.captures_iter(&content) {
                if let Some(matched) = cap.get(0) {
                    let start = matched.start();
                    let test_name = cap.get(3).map(|m| m.as_str()).unwrap_or("unknown");

                    // Find the matching closing brace
                    let after_open = matched.end();
                    let mut brace_count = 1;
                    let mut end_pos = after_open;

                    for (i, c) in content[after_open..].chars().enumerate() {
                        if c == '{' {
                            brace_count += 1;
                        } else if c == '}' {
                            brace_count -= 1;
                            if brace_count == 0 {
                                end_pos = after_open + i + 1;
                                break;
                            }
                        }
                    }

                    let test_code = &content[start..end_pos];
                    let test_fn = Self::classify_test_function(test_code, test_name);
                    tests.push(test_fn);
                    last_pos = end_pos;
                }
            }
        }

        let (substantive, hollow, ratio) = Self::score_falsification(&tests);
        let test_count = tests.len();
        let hollow_count = hollow;

        Ok(TestFile {
            path: path_str,
            test_count,
            hollow_count,
            falsification_ratio: ratio,
            tests,
        })
    }

    pub fn analyze_directory(root: &Path) -> anyhow::Result<Vec<TestFile>> {
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only analyze Rust test files
            if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                match Self::analyze_file(path) {
                    Ok(test_file) if test_file.test_count > 0 => {
                        files.push(test_file);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to analyze {}: {}", path.display(), e);
                    }
                    _ => {}
                }
            }
        }

        Ok(files)
    }

    pub fn generate_report(test_files: Vec<TestFile>) -> AnalysisReport {
        let mut total_tests = 0;
        let mut total_hollow = 0;
        let mut pattern_counts = HashMap::new();

        for file in &test_files {
            total_tests += file.test_count;
            total_hollow += file.hollow_count;

            for test in &file.tests {
                for pattern in &test.patterns {
                    *pattern_counts.entry(*pattern).or_insert(0) += 1;
                }
            }
        }

        let substantive_tests = total_tests - total_hollow;
        let hollow_ratio = if total_tests == 0 {
            0.0
        } else {
            (total_hollow as f64) / (total_tests as f64)
        };

        let hollow_tests: Vec<HollowTest> = test_files
            .iter()
            .flat_map(|file| {
                file.tests
                    .iter()
                    .filter(|t| t.is_hollow)
                    .map(move |test| HollowTest {
                        file: file.path.clone(),
                        test_name: test.name.clone(),
                        assertion_count: test.assertions.len(),
                        assertion_types: test.assertions.clone(),
                    })
            })
            .collect();

        let patterns = PatternStats {
            service_based: pattern_counts.get(&Pattern::ServiceBased).copied().unwrap_or(0),
            lsp_harness: pattern_counts.get(&Pattern::LSPHarness).copied().unwrap_or(0),
            dogfood: pattern_counts.get(&Pattern::Dogfood).copied().unwrap_or(0),
            property_sweep: pattern_counts.get(&Pattern::PropertySweep).copied().unwrap_or(0),
            merger: pattern_counts.get(&Pattern::Merger).copied().unwrap_or(0),
            other: pattern_counts.get(&Pattern::Other).copied().unwrap_or(0),
        };

        AnalysisReport {
            summary: ReportSummary {
                total_tests,
                substantive_tests,
                hollow_tests: total_hollow,
                hollow_ratio,
                scanned_files: test_files.len(),
            },
            by_file: test_files,
            hollow_tests,
            patterns,
        }
    }
}
