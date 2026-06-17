use crate::diagnostics::AntiLlmDiagnostic;
use crate::observations::Observation;

// Oracle class witness KEY strings from wasm4pm-compat::witnesses_anti_cheat.
// Using string constants so this compiles without the sibling repo available.
const ORACLE_A8: &str = "anti-cheat/oracle-a8-audit-log-tampering";
const ORACLE_A9: &str = "anti-cheat/oracle-a9-temporal-anomaly";
const ORACLE_A10: &str = "anti-cheat/oracle-a10-causal-violation";
const ORACLE_A11: &str = "anti-cheat/oracle-a11-unknown-collapse";
const ORACLE_A12: &str = "anti-cheat/oracle-a12-cyclic-dependency";

pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();

    for o in obs {
        // OCEL-001: diagnostic emitted without OCEL process event (A10 — causal violation)
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
                oracle_class: Some(ORACLE_A10.to_string()),
                confidence: Some(0.90),
            });
        }

        // OCEL-002: receipt claim without OCEL binding (A8 — audit log tampering)
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
                oracle_class: Some(ORACLE_A8.to_string()),
                confidence: Some(0.95),
            });
        }

        // OCEL-003: OCEL export bypassed wasm4pm-compat typed boundary (A8 — audit log tampering)
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
                oracle_class: Some(ORACLE_A8.to_string()),
                confidence: Some(0.99),
            });
        }

        // ADMIT-001: fitness=1.0 without measurement provenance (A10 — causal violation / A11 — unknown collapse)
        if o.construct == "fitness_bare_constant" {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-ADMIT-001".to_string(),
                category: "admission".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Fitness report asserts fitness=1.0 and admitted=true without a provenance block — premature admission. The report was asserted, not measured.".to_string(),
                forbidden_implication: "FitnessReport => MeasuredFitness".to_string(),
                blocking: true,
                required_correction: "Add a provenance block with run_id, measured_by, and measured_on fields derived from an actual conformance run.".to_string(),
                required_next_proof: "Fitness report includes provenance.run_id pointing to a logged conformance execution.".to_string(),
                oracle_class: Some(ORACLE_A11.to_string()),
                confidence: Some(0.92),
            });
        }

        // ADMIT-002: PARTIAL_ALIVE without corresponding OCEL report (A10 — causal violation)
        if o.construct == "partial_alive_no_ocel" {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-ADMIT-002".to_string(),
                category: "admission".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: format!("Registry marks '{}' as PARTIAL_ALIVE but no OCEL fitness report file exists — premature status flip.", o.context),
                forbidden_implication: "RegistryStatus(PARTIAL_ALIVE) => MeasuredFitnessReport".to_string(),
                blocking: true,
                required_correction: "Produce an OCEL fitness report with measured provenance before flipping status to PARTIAL_ALIVE.".to_string(),
                required_next_proof: "Corresponding fitness report file exists with admitted=true and provenance.run_id.".to_string(),
                oracle_class: Some(ORACLE_A10.to_string()),
                confidence: Some(0.88),
            });
        }

        // ADMIT-003: admitted=true without run_id (A8 — audit log tampering)
        if o.construct == "admitted_no_run_id" {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-ADMIT-003".to_string(),
                category: "admission".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Fitness report sets admitted=true without run_id in provenance — admission cannot be traced to a measured run.".to_string(),
                forbidden_implication: "AdmittedTrue => MeasuredRunId".to_string(),
                blocking: true,
                required_correction: "Add run_id (or provenance.run_id) to the fitness report from the actual conformance execution that earned admission.".to_string(),
                required_next_proof: "run_id resolves to a log entry in the OCEL audit trail.".to_string(),
                oracle_class: Some(ORACLE_A8.to_string()),
                confidence: Some(0.93),
            });
        }

        // OCEL-004: full wasm4pm used where wasm4pm-compat was required (A8 — boundary violation)
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
                oracle_class: Some(ORACLE_A8.to_string()),
                confidence: Some(0.85),
            });
        }

        // OCEL-005: temporal ordering violation (A9 — temporal anomaly)
        if o.construct == "temporal_violation" || o.context.contains("timestamp_before_ancestor") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-005".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "OCEL event timestamp precedes its causal ancestor — impossible ordering.".to_string(),
                forbidden_implication: "EventTimestamp => MonotonicCausalOrder".to_string(),
                blocking: true,
                required_correction: "Fix the session clock or the event emission order so timestamps respect causality.".to_string(),
                required_next_proof: "All ancestor events carry strictly earlier timestamps than their descendants.".to_string(),
                oracle_class: Some(ORACLE_A9.to_string()),
                confidence: Some(0.97),
            });
        }

        // OCEL-006: unknown state collapsed to admitted/refused without resolution (A11)
        if o.construct == "unknown_state_collapsed"
            || o.context.contains("UNKNOWN_TO_ADMITTED_NO_RESOLUTION")
        {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-006".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "ConformanceVector UNKNOWN axis collapsed to ADMITTED or REFUSED without a resolution event in the OCEL log.".to_string(),
                forbidden_implication: "UnknownAxis => ResolutionEventBeforeAdmission".to_string(),
                blocking: true,
                required_correction: "Emit a ResolutionEvent (test result, receipt validation, or explicit refusal) before transitioning the axis out of UNKNOWN.".to_string(),
                required_next_proof: "OCEL log contains ResolutionEvent before ConformanceVectorUpdate for the axis.".to_string(),
                oracle_class: Some(ORACLE_A11.to_string()),
                confidence: Some(0.94),
            });
        }

        // OCEL-007: cyclic dependency in event causality graph (A12)
        if o.construct == "causal_cycle" || o.context.contains("CAUSAL_CYCLE_DETECTED") {
            diags.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-OCEL-007".to_string(),
                category: "process_evidence".to_string(),
                file_path: o.file_path.clone(),
                line: o.line,
                column: o.column,
                message: "Cycle detected in OCEL event causality graph — the DAG invariant is violated.".to_string(),
                forbidden_implication: "OcelCausalityGraph => AcyclicDAG".to_string(),
                blocking: true,
                required_correction: "Remove the circular dependency from the event or object reference graph.".to_string(),
                required_next_proof: "OCEL graph passes cycle detection with zero cycles found.".to_string(),
                oracle_class: Some(ORACLE_A12.to_string()),
                confidence: Some(0.99),
            });
        }
    }

    diags
}
