//! wasm4pm evidence integration — coordinator module.
//!
//! Types and converters live in [`super::evidence_types`].
//! Oxigraph store extractors live in [`super::evidence_extractors`].

pub use super::evidence_extractors::*;
pub use super::evidence_types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use wasm4pm_compat::witness::Ocel20;

    #[test]
    fn test_workspace_evidence_raw_and_admitted() {
        let ws = WorkspaceEvidencePayload {
            id: "urn:test-workspace".to_string(),
            kind: "Project".to_string(),
            document_uris: vec!["file:///src/lib.rs".to_string()],
        };

        let raw_ev = to_raw_evidence::<_, Ocel20>(ws.clone());
        assert_eq!(raw_ev.value, ws);

        let admitted_ev = workspace_to_admitted_evidence(ws.clone());
        assert_eq!(admitted_ev.into_inner(), ws);
    }

    #[test]
    fn test_range_evidence_raw_and_admitted() {
        let range = RangeEvidencePayload {
            start_line: 10,
            start_character: 5,
            end_line: 10,
            end_character: 25,
        };

        let raw_ev = to_raw_evidence::<_, Ocel20>(range.clone());
        assert_eq!(raw_ev.value, range);

        let admitted_ev = range_to_admitted_evidence(range.clone());
        assert_eq!(admitted_ev.into_inner(), range);
    }

    #[test]
    fn test_diagnostic_evidence_raw_and_admitted() {
        let diag = DiagnosticEvidencePayload {
            message: "Mismatched type".to_string(),
            severity: Some("Error".to_string()),
            code: Some("E0308".to_string()),
            source: Some("rustc".to_string()),
            range: Some(RangeEvidencePayload {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 10,
            }),
        };

        let raw_ev = to_raw_evidence::<_, Ocel20>(diag.clone());
        assert_eq!(raw_ev.value, diag);

        let admitted_ev = diagnostic_to_admitted_evidence(diag.clone());
        assert_eq!(admitted_ev.into_inner(), diag);
    }
}
