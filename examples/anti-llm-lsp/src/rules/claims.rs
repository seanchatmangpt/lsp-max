use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;

pub fn evaluate(obs: &[Observation], _failset_nonempty: bool) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    for o in obs {
        // Enforce no victory language under AGENTS.md Rule 8
        let is_victory_lang = o.construct == "fully admitted"
            || o.construct == "Victory confirmed"
            || o.construct == "victory confirmed"
            || o.construct == "victory"
            || o.construct == "done"
            || o.construct == "all clean"
            || o.construct == "no issues"
            || o.construct == "everything passes"
            || o.construct == "solved"
            || o.construct == "guaranteed"
            || o.construct == "impossible to fake"
            || o.construct == "all gaps resolved"
            || o.construct == "successfully proven"
            || o.context.to_lowercase().contains("victory confirmed")
            || o.context.to_lowercase().contains("victory audit");

        if is_victory_lang {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-CLAIM-004".to_string(),
                category: "claim".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Claimed victory, done, or fully admitted in violation of the AGENTS.md bounded status rules.".to_string(),
                forbidden_implication: "StatusWord(ADMITTED) => Admitted".to_string(),
                blocking: true,
                required_correction: "Replace victory/done language with bounded status vocabulary (e.g. REPORTED_ADMITTED_BY_DOGFOOD, CANDIDATE).".to_string(),
                required_next_proof: "Run admissibility scan and ensure no victory claims exist.".to_string(),
            });
        }
    }

    diags
}
