use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;

pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    for o in obs {
        // v1.0.0 or version = "1.0.0" found
        if o.construct == "version = \"1.0.0\""
            || o.context.contains("v1.0.0")
            || o.context.contains("1.0.0")
        {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-VERSION-001".to_string(),
                category: "version".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Default template version '1.0.0' or 'v1.0.0' found in project configuration.".to_string(),
                forbidden_implication: "Template default => release law".to_string(),
                blocking: true,
                required_correction: "Specify CalVer version (e.g. v26.6.5) instead of standard v1.0.0 template version.".to_string(),
                required_next_proof: "Check project Cargo.toml metadata.".to_string(),
            });
        }
    }

    diags
}
