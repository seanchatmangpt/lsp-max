use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;

pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    for o in obs {
        if o.construct == "assert_contains" {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-TEST-001".to_string(),
                category: "test".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "String assertion in test file. Tests must verify structural properties instead of substring search on outputs.".to_string(),
                forbidden_implication: "TestStdout => Receipt".to_string(),
                blocking: true,
                required_correction: "Use structural/field matching or response code validation instead of contains() on raw test strings.".to_string(),
                required_next_proof: "Refactor tests to parse response bodies into JSON/structs.".to_string(),
            });
        }

        if o.construct == "negative_control_reference" {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-TEST-003".to_string(),
                category: "test".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Standard test references or verifies negative controls directly. This is prohibited except in authorized dogfooding tests.".to_string(),
                forbidden_implication: "Positive case passes => law holds".to_string(),
                blocking: true,
                required_correction: "Remove negative control directory or fixture references from standard tests.".to_string(),
                required_next_proof: "Run test suite to verify tests use mocked or isolated test fixtures.".to_string(),
            });
        }
    }

    diags
}
