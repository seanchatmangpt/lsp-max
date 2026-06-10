use super::super::invariants::{verify_invariants, VerificationReport};
use super::super::receipts::{Blake3Hash, CryptographicReceipt};
use super::types::{RawData, RAW};
use crate::Machine;
use lsp_max_base::abstractions::RelationAdmitter;
use lsp_max_lsif::lsif::Element;
use lsp_max_protocol::MaxDiagnostic;

pub struct AdmittedGraph {
    pub store: oxigraph::store::Store,
    pub active_graph: oxigraph::model::GraphName,
}

pub struct AdmittedRelationGraph {
    pub store: oxigraph::store::Store,
    pub graph_name: oxigraph::model::GraphName,
    pub receipt: CryptographicReceipt,
}

impl RelationAdmitter for AdmittedGraph {
    type Parsed = Vec<Element>;
    type RelationGraph = AdmittedRelationGraph;
    type Refusal = VerificationReport;

    fn admit(&self, parsed: Self::Parsed) -> Result<Self::RelationGraph, Self::Refusal> {
        let raw_machine = Machine::new(RAW, RawData { elements: parsed });

        let snapshot_id = match &self.active_graph {
            oxigraph::model::GraphName::NamedNode(node) => {
                let s = node.as_str();
                if let Some(pos) = s.rfind(':') {
                    &s[pos + 1..]
                } else {
                    "default-snapshot"
                }
            }
            _ => "default-snapshot",
        };

        let candidate_machine = match raw_machine.admit_candidate(snapshot_id) {
            Ok(m) => m,
            Err(e) => {
                let diagnostics = vec![MaxDiagnostic {
                    diagnostic_id: "diag-ingest-error".to_string(),
                    lsp: lsp_types_max::Diagnostic {
                        message: e.to_string(),
                        severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
                        ..Default::default()
                    },
                    ..Default::default()
                }];
                return Err(VerificationReport {
                    is_success: false,
                    diagnostics,
                    execution_time_ms: 0,
                });
            }
        };

        let validation_store = oxigraph::store::Store::new().unwrap();

        for quad in self.store.iter().flatten() {
            validation_store.insert(&quad).unwrap();
        }

        for quad in &candidate_machine.data.quads {
            validation_store.insert(quad).unwrap();
        }

        if let Err(e) = super::validation::validate_shacl_shapes(&candidate_machine.data.quads) {
            let diagnostics = vec![MaxDiagnostic {
                diagnostic_id: "diag-shacl-error".to_string(),
                lsp: lsp_types_max::Diagnostic {
                    message: format!("SHACL validation failed: {}", e),
                    severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
                    ..Default::default()
                },
                violated_invariant: "INVARIANT_1".to_string(),
                ..Default::default()
            }];
            return Err(VerificationReport {
                is_success: false,
                diagnostics,
                execution_time_ms: 0,
            });
        }

        let report = verify_invariants(&validation_store);

        if !report.is_success {
            return Err(report);
        }

        for quad in &candidate_machine.data.quads {
            self.store.insert(quad).unwrap();
        }

        use rand_core::OsRng;
        let mut csprng = OsRng;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);

        let consequence_hash = Blake3Hash([0u8; 32]);
        let mut receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::new_v4(),
            law_id: uuid::Uuid::new_v4(),
            consequence_hash,
            sequence: 1,
            signature: [0u8; 64],
        };
        let payload_hash = receipt.compute_payload_hash();
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(&payload_hash.0);
        receipt.signature = signature.to_bytes();

        let _admitted_machine = candidate_machine
            .admit_admitted(&self.store, self.active_graph.clone(), receipt.clone())
            .unwrap();

        let views = crate::control_plane::views::get_views();
        views
            .committed_epoch
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let store_clone = self.store.clone();
        let views_clone = views;
        std::thread::spawn(move || {
            crate::control_plane::views::update_views(&store_clone, views_clone);
        });

        Ok(AdmittedRelationGraph {
            store: self.store.clone(),
            graph_name: self.active_graph.clone(),
            receipt,
        })
    }
}
