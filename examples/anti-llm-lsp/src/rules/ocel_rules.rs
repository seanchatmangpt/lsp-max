use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;

pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    for o in obs {
        if o.construct == "ocel_no_event" || o.context.contains("ANTI-LLM-OCEL-001-TRIGGER") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-001".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Diagnostic emitted without corresponding OCEL process event.".to_string(),
                forbidden_implication: "DiagnosticEmitted => ProcessEvidenceRecorded".to_string(),
                blocking: true,
                required_correction: "Emit an OCEL event whenever a diagnostic is raised.".to_string(),
                required_next_proof: "Verify that OCEL contains DiagnosticEmitted linked to the diagnostic.".to_string(),
            });
        }

        if o.construct == "ocel_no_binding" || o.context.contains("ANTI-LLM-OCEL-002-TRIGGER") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-002".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Receipt claim exists without OCEL object/event binding.".to_string(),
                forbidden_implication: "ReceiptExists => ReceiptBoundToProcess".to_string(),
                blocking: true,
                required_correction: "Ensure that all receipts are bound to a corresponding Receipt object and ReceiptValidated event.".to_string(),
                required_next_proof: "Check for corresponding event/object link in exported OCEL.".to_string(),
            });
        }

        if o.construct == "ocel_no_compat" || o.context.contains("\"bypassed_compat\": true") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-003".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "OCEL export produced without wasm4pm-compat typed boundary.".to_string(),
                forbidden_implication: "JSONShape(OCEL) => CompatAdmittedOCEL".to_string(),
                blocking: true,
                required_correction: "Construct the exported OCEL log through typed wasm4pm-compat APIs.".to_string(),
                required_next_proof: "Verify code does not serialize raw JSON bypasses.".to_string(),
            });
        }

        if o.construct == "ocel_full_wasm4pm" || o.context.contains("use wasm4pm::") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-004".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Full wasm4pm authority used where wasm4pm-compat boundary was required.".to_string(),
                forbidden_implication: "CompatEvidenceBoundary => FullMiningAuthority".to_string(),
                blocking: true,
                required_correction: "Use only wasm4pm-compat typed boundaries in this checkpoint.".to_string(),
                required_next_proof: "Check dependencies to ensure full wasm4pm is excluded.".to_string(),
            });
        }
    }

    diags
}
