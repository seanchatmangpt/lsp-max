use crate::control_plane::admission::{
    AdmittedData, CandidateData, GraphAdmissionLaw, QuarantinedData, RawData, RefusedData,
    ReplayedData, SupersededData, ADMITTED, CANDIDATE, QUARANTINED, RAW, REFUSED, REPLAYED,
    SUPERSEDED,
};
use crate::control_plane::invariants::VerificationReport;
use crate::control_plane::receipts::{to_hex, CryptographicReceipt};
use crate::Machine;
use wasm4pm_compat::engine_bridge::{GraduateToWasm4pm, GraduationCandidate, GraduationReason};

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, RAW, RawData> {
    fn candidate(&self) -> GraduationCandidate {
        let mut byte_accumulator = Vec::new();
        for element in &self.data.elements {
            if let Ok(serialized) = serde_json::to_vec(element) {
                byte_accumulator.extend_from_slice(&serialized);
            }
        }
        let graph_hash = crate::sha256(&byte_accumulator);
        GraduationCandidate::new(
            GraduationReason::NeedsDiscovery,
            format!(
                "RAW graph admission state with {} elements",
                self.data.elements.len()
            ),
            format!("sha256:{}", graph_hash),
        )
    }
}

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, CANDIDATE, CandidateData> {
    fn candidate(&self) -> GraduationCandidate {
        GraduationCandidate::new(
            GraduationReason::NeedsConformanceExecution,
            format!(
                "CANDIDATE graph state (elements: {}, quads: {})",
                self.data.elements.len(),
                self.data.quads.len()
            ),
            format!("sha256:{}", self.data.graph_hash),
        )
    }
}

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, ADMITTED, AdmittedData> {
    fn candidate(&self) -> GraduationCandidate {
        let payload_hash = self.data.receipt.compute_payload_hash();
        GraduationCandidate::new(
            GraduationReason::NeedsReceipts,
            format!(
                "ADMITTED graph state (quads: {}, graph: {:?}, receipt sequence: {})",
                self.data.quad_count, self.data.graph_name, self.data.receipt.sequence
            ),
            format!("blake3:{}", to_hex(&payload_hash.0)),
        )
    }
}

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, REFUSED, RefusedData> {
    fn candidate(&self) -> GraduationCandidate {
        let mut hasher = blake3::Hasher::new();
        if let Ok(serialized) = serde_json::to_vec(&self.data.report) {
            hasher.update(&serialized);
        } else {
            hasher.update(b"unserializable report");
        }
        GraduationCandidate::new(
            GraduationReason::NeedsConformanceExecution,
            format!(
                "REFUSED graph state (diagnostics: {}, execution_time: {}ms)",
                self.data.report.diagnostics.len(),
                self.data.report.execution_time_ms
            ),
            format!("blake3:{}", hasher.finalize()),
        )
    }
}

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, QUARANTINED, QuarantinedData> {
    fn candidate(&self) -> GraduationCandidate {
        let mut hasher = blake3::Hasher::new();
        for dep in &self.data.missing_dependencies {
            hasher.update(dep.as_bytes());
        }
        GraduationCandidate::new(
            GraduationReason::NeedsReplay,
            format!(
                "QUARANTINED graph state (elements: {}, missing dependencies: {})",
                self.data.elements.len(),
                self.data.missing_dependencies.len()
            ),
            format!("blake3:{}", hasher.finalize()),
        )
    }
}

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, SUPERSEDED, SupersededData> {
    fn candidate(&self) -> GraduationCandidate {
        let mut hasher = blake3::Hasher::new();
        hasher.update(format!("{:?}", self.data.graph_name).as_bytes());
        hasher.update(format!("{:?}", self.data.superseded_by).as_bytes());
        GraduationCandidate::new(
            GraduationReason::RebuildingProcessMiningLocally,
            format!(
                "SUPERSEDED graph state (graph: {:?}, superseded by: {:?})",
                self.data.graph_name, self.data.superseded_by
            ),
            format!("blake3:{}", hasher.finalize()),
        )
    }
}

impl GraduateToWasm4pm for Machine<GraphAdmissionLaw, REPLAYED, ReplayedData> {
    fn candidate(&self) -> GraduationCandidate {
        let payload_hash = self.data.receipt.compute_payload_hash();
        GraduationCandidate::new(
            GraduationReason::NeedsReplay,
            format!(
                "REPLAYED graph state (graph: {:?}, receipt sequence: {})",
                self.data.graph_name, self.data.receipt.sequence
            ),
            format!("blake3:{}", to_hex(&payload_hash.0)),
        )
    }
}

impl GraduateToWasm4pm for CryptographicReceipt {
    fn candidate(&self) -> GraduationCandidate {
        let payload_hash = self.compute_payload_hash();
        GraduationCandidate::new(
            GraduationReason::NeedsReceipts,
            format!("CryptographicReceipt (sequence: {})", self.sequence),
            format!("blake3:{}", to_hex(&payload_hash.0)),
        )
    }
}

impl GraduateToWasm4pm for VerificationReport {
    fn candidate(&self) -> GraduationCandidate {
        let mut hasher = blake3::Hasher::new();
        if let Ok(serialized) = serde_json::to_vec(self) {
            hasher.update(&serialized);
        } else {
            hasher.update(b"unserializable verification report");
        }
        GraduationCandidate::new(
            GraduationReason::NeedsConformanceExecution,
            format!(
                "VerificationReport (is_success: {}, diagnostics: {})",
                self.is_success,
                self.diagnostics.len()
            ),
            format!("blake3:{}", hasher.finalize()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::receipts::Blake3Hash;
    use lsp_max_lsif::lsif::{Element, PositionEncoding, Vertex, VertexType};

    fn make_meta_element() -> Element {
        Element::Vertex(Vertex::MetaData {
            id: lsp_types_max::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            version: "0.6.0".to_string(),
            position_encoding: PositionEncoding::Utf16,
            project_root: "file:///".to_string(),
            tool_info: None,
        })
    }

    #[test]
    fn test_raw_graduation() {
        let machine = Machine::new(
            RAW,
            RawData {
                elements: vec![make_meta_element()],
            },
        );
        let candidate = machine.candidate();
        assert_eq!(candidate.reason, GraduationReason::NeedsDiscovery);
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("RAW graph"));
    }

    #[test]
    fn test_candidate_graduation() {
        let machine = Machine::new(
            CANDIDATE,
            CandidateData {
                elements: vec![],
                quads: vec![],
                graph_hash: "abcd".to_string(),
            },
        );
        let candidate = machine.candidate();
        assert_eq!(
            candidate.reason,
            GraduationReason::NeedsConformanceExecution
        );
        assert!(candidate.is_grounded());
        assert_eq!(candidate.evidence_ref, "sha256:abcd");
    }

    #[test]
    fn test_admitted_graduation() {
        let receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash: Blake3Hash([1u8; 32]),
            sequence: 42,
            signature: [0u8; 64],
        };
        let machine = Machine::new(
            ADMITTED,
            AdmittedData {
                graph_name: oxigraph::model::GraphName::DefaultGraph,
                quad_count: 10,
                receipt,
            },
        );
        let candidate = machine.candidate();
        assert_eq!(candidate.reason, GraduationReason::NeedsReceipts);
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("ADMITTED graph"));
    }

    #[test]
    fn test_refused_graduation() {
        let report = VerificationReport {
            is_success: false,
            diagnostics: vec![],
            execution_time_ms: 120,
        };
        let machine = Machine::new(REFUSED, RefusedData { report });
        let candidate = machine.candidate();
        assert_eq!(
            candidate.reason,
            GraduationReason::NeedsConformanceExecution
        );
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("REFUSED graph"));
    }

    #[test]
    fn test_quarantined_graduation() {
        let machine = Machine::new(
            QUARANTINED,
            QuarantinedData {
                elements: vec![],
                quads: vec![],
                missing_dependencies: vec!["urn:dep:1".to_string()],
            },
        );
        let candidate = machine.candidate();
        assert_eq!(candidate.reason, GraduationReason::NeedsReplay);
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("QUARANTINED graph"));
    }

    #[test]
    fn test_superseded_graduation() {
        let machine = Machine::new(
            SUPERSEDED,
            SupersededData {
                graph_name: oxigraph::model::GraphName::DefaultGraph,
                superseded_by: oxigraph::model::GraphName::DefaultGraph,
            },
        );
        let candidate = machine.candidate();
        assert_eq!(
            candidate.reason,
            GraduationReason::RebuildingProcessMiningLocally
        );
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("SUPERSEDED graph"));
    }

    #[test]
    fn test_replayed_graduation() {
        let receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash: Blake3Hash([2u8; 32]),
            sequence: 12,
            signature: [0u8; 64],
        };
        let machine = Machine::new(
            REPLAYED,
            ReplayedData {
                graph_name: oxigraph::model::GraphName::DefaultGraph,
                receipt,
            },
        );
        let candidate = machine.candidate();
        assert_eq!(candidate.reason, GraduationReason::NeedsReplay);
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("REPLAYED graph"));
    }

    #[test]
    fn test_receipt_graduation() {
        let receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash: Blake3Hash([3u8; 32]),
            sequence: 99,
            signature: [0u8; 64],
        };
        let candidate = receipt.candidate();
        assert_eq!(candidate.reason, GraduationReason::NeedsReceipts);
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("CryptographicReceipt"));
    }

    #[test]
    fn test_verification_report_graduation() {
        let report = VerificationReport {
            is_success: true,
            diagnostics: vec![],
            execution_time_ms: 10,
        };
        let candidate = report.candidate();
        assert_eq!(
            candidate.reason,
            GraduationReason::NeedsConformanceExecution
        );
        assert!(candidate.is_grounded());
        assert!(candidate.subject.contains("VerificationReport"));
    }
}
