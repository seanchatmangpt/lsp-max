use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_gen::{GeneratorContext, GeneratorEngine, TestMatrixGenerator};
use serde::Serialize;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct TestMatrixGenerateResult {
    pub output_file: String,
    pub row_count_hint: String,
    pub status: String,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestMatrixStatusResult {
    pub matrix_file_exists: bool,
    pub matrix_file: String,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct TestMatrixService;

impl TestMatrixService {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, dir: &str) -> Result<TestMatrixGenerateResult> {
        let ctx = GeneratorContext::new("testmatrix", std::path::PathBuf::from(dir));
        let engine = GeneratorEngine::new(vec![Box::new(TestMatrixGenerator)]);
        let written = engine
            .run("testmatrix", &ctx)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

        let output_file = written
            .first()
            .map(|w| w.path.display().to_string())
            .unwrap_or_else(|| "tests/test_matrix_generated.rs".to_string());

        Ok(TestMatrixGenerateResult {
            output_file,
            row_count_hint: "see generated file for matrix dimensions".into(),
            status: "CANDIDATE".into(),
            next_step: "Add generated file to tests/ and run `cargo test test_matrix`".into(),
        })
    }

    pub fn status(&self, dir: &str) -> TestMatrixStatusResult {
        let matrix_file = format!("{dir}/tests/test_matrix_generated.rs");
        TestMatrixStatusResult {
            matrix_file_exists: std::path::Path::new(&matrix_file).exists(),
            matrix_file,
            status: "CANDIDATE".into(),
        }
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("generate")]
pub fn generate(dir: Option<String>) -> Result<TestMatrixGenerateResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    TestMatrixService::new().generate(&dir)
}

#[verb("status")]
pub fn status(dir: Option<String>) -> Result<TestMatrixStatusResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    Ok(TestMatrixService::new().status(&dir))
}
