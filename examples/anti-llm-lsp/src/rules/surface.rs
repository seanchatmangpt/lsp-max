use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;

pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    // Check for plain tower-lsp
    for o in obs {
        if o.construct.contains("tower-lsp") || o.construct.contains("tower_lsp") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-SURFACE-001".to_string(),
                category: "surface".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Plain tower-lsp found in codebase. All tower LSP hosts must migrate to tower-lsp-max.".to_string(),
                forbidden_implication: "Pass(plain LSP) => Pass(LSP 3.18)".to_string(),
                blocking: true,
                required_correction: "Replace plain 'tower-lsp' dependency and use 'tower-lsp-max'.".to_string(),
                required_next_proof: "Run cargo check / cargo test to verify tower-lsp-max integration.".to_string(),
            });
        }

        // Check for observer as authority
        if o.construct == "PackObserver" || o.message.contains("observer dependency") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-SURFACE-003".to_string(),
                category: "surface".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Pack observer dependency treated as runtime authority.".to_string(),
                forbidden_implication: "Pack observes surface => runtime uses surface".to_string(),
                blocking: true,
                required_correction:
                    "Do not use PackObserver/static analyzer results as runtime authority."
                        .to_string(),
                required_next_proof: "Verify with active capability checks.".to_string(),
            });
        }

        // Check for 3.18 claim without initialize transcript
        if o.construct == "initialize without 3.18 caps" {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-SURFACE-005".to_string(),
                category: "surface".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "LSP 3.18 claimed but initialize capability transcript lacks 3.18 support.".to_string(),
                forbidden_implication: "Basic LSP transcript => LSP 3.18".to_string(),
                blocking: true,
                required_correction: "Negotiate and log 3.18 inlineCompletion/foldingRange capability in initialize transcript.".to_string(),
                required_next_proof: "Provide a client-to-server initialize handshake transcript.".to_string(),
            });
        }
    }

    diags
}
