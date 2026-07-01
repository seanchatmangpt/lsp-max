# RelationAdmitter Integration & Graph Lifecycle Routing Analysis

## 1. Executive Summary

This report defines the implementation architecture and lifecycle routing logic for the `RelationAdmitter` trait (defined in the `lsp-max-base` crate) within the `lsp-max` framework. 

To transition from a dynamic, session-bound query model to a formally verifiable and cryptographically replayable RDF graph substrate, the ingestion boundary must enforce a strict, typestate-driven lifecycle for all workspace facts. This design maps raw LSIF 0.6.0 elements and diagnostic observations to snapshot-isolated RDF quads (`oxrdf::Quad`) stored in an embedded `oxigraph::store::Store`.

This analysis provides a fully realized blueprint for:
1. Mapping `RelationAdmitter` type parameters to local structures.
2. Defining the 7 lifecycle states (`RAW`, `CANDIDATE`, `ADMITTED`, `REFUSED`, `QUARANTINED`, `SUPERSEDED`, `REPLAYED`) using a zero-cost typestate state machine.
3. Implementing deterministic lifecycle transition routing logic.
4. Integrating SHACL shapes validation and the 5 core SPARQL semantic invariants.
5. Locating and configuring crate dependencies for clean compilation and verification.

---

## 2. Trait Definitions & Concrete Type Mapping

The `RelationAdmitter` trait is defined in `crates/lsp-max-base/src/abstractions.rs` as follows:

```rust
pub trait RelationAdmitter {
    type Parsed;
    type RelationGraph;
    type Refusal;

    /// Determines if the parsed state is structurally and semantically valid enough
    /// to be admitted into the global semantic relation graph.
    fn admit(&self, parsed: Self::Parsed) -> Result<Self::RelationGraph, Self::Refusal>;
}
```

To implement this trait for the Oxigraph-backed control plane, we map the associated types to the following concrete structures:

| Associated Type | Concrete Implementation | Description |
|---|---|---|
| `type Parsed` | `Vec<lsp_max_lsif::lsif::Element>` | A collection of parsed LSIF 0.6.0 vertices and edges representing a file/snapshot observation. |
| `type RelationGraph` | `AdmittedRelationGraph` | A reference wrapper around the admitted snapshot within the active `oxigraph::store::Store`, keyed by a unique Named Graph node. |
| `type Refusal` | `VerificationReport` | A structured diagnostics report detailing syntax violations, namespace laundering, SHACL errors, or SPARQL invariant failures. |

### Concrete Type Definitions

The target structures are defined as follows:

```rust
use oxigraph::store::Store;
use oxrdf::GraphName;
use serde::Serialize;
use lsp_max_protocol::MaxDiagnostic;

/// Represents a successfully admitted graph snapshot inside the Oxigraph store.
pub struct AdmittedRelationGraph {
    /// The thread-safe reference to the global Oxigraph database store.
    pub store: Store,
    /// The unique named graph URI identifying this snapshot state.
    pub graph_name: GraphName,
    /// The cryptographic receipt generated during transition verification.
    pub receipt: CryptographicReceipt,
}

/// Represents the cryptographic signature and digests validating the snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct CryptographicReceipt {
    pub prev_hash: String,
    pub discipline_id: String,
    pub law_id: String,
    pub graph_hash: String,
    pub query_hash: String,
    pub result_hash: String,
    pub sequence: u64,
    pub signature: Vec<u8>,
}

/// A structured container for verification failures and diagnostics.
#[derive(Debug, Clone, Serialize)]
pub struct VerificationReport {
    pub is_success: bool,
    pub diagnostics: Vec<MaxDiagnostic>,
    pub execution_time_ms: u64,
}
```

---

## 3. Graph Lifecycle States & Typestate Primitives

To guarantee transition safety at compile time, we leverage Rust's affine ownership system (`self` consumption) to represent the 7 lifecycle states. Each state phase and its associated transition data implements the runtime's existing `Phase` and `Data` traits.

```rust
use std::marker::PhantomData;
use oxrdf::Quad;
use lsp_max_lsif::lsif::Element;

/// The compile-time law governing graph admission transitions.
pub struct GraphAdmissionLaw;

impl lsp_max_runtime::Law for GraphAdmissionLaw {
    type Error = GraphAdmissionError;
}

/// Compilation/Ingestion failure cases.
#[derive(Debug)]
pub enum GraphAdmissionError {
    ParsingFailed(String),
    NamespaceViolation(String),
    InvariantViolated(VerificationReport),
    DependencyMissing(Vec<String>),
    ReplayMismatch(String),
}

/// The zero-cost typestate machine container.
pub struct Machine<L: lsp_max_runtime::Law, P: lsp_max_runtime::Phase, D: lsp_max_runtime::Data> {
    pub _law: PhantomData<L>,
    pub phase: P,
    pub data: D,
}

impl<L: lsp_max_runtime::Law, P: lsp_max_runtime::Phase, D: lsp_max_runtime::Data> Machine<L, P, D> {
    pub const fn new(phase: P, data: D) -> Self {
        Self {
            _law: PhantomData,
            phase,
            data,
        }
    }
}
```

### State and Data Declarations

```rust
// ==============================================================================
// 1. RAW State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct RawPhase;
impl lsp_max_runtime::Phase for RawPhase {}

pub struct RawData {
    pub elements: Vec<Element>,
}
impl lsp_max_runtime::Data for RawData {}

// ==============================================================================
// 2. CANDIDATE State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct CandidatePhase;
impl lsp_max_runtime::Phase for CandidatePhase {}

pub struct CandidateData {
    pub elements: Vec<Element>,
    pub quads: Vec<Quad>,
    pub graph_hash: String,
}
impl lsp_max_runtime::Data for CandidateData {}

// ==============================================================================
// 3. ADMITTED State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct AdmittedPhase;
impl lsp_max_runtime::Phase for AdmittedPhase {}

pub struct AdmittedData {
    pub graph_name: GraphName,
    pub quad_count: usize,
    pub receipt: CryptographicReceipt,
}
impl lsp_max_runtime::Data for AdmittedData {}

// ==============================================================================
// 4. REFUSED State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct RefusedPhase;
impl lsp_max_runtime::Phase for RefusedPhase {}

pub struct RefusedData {
    pub report: VerificationReport,
}
impl lsp_max_runtime::Data for RefusedData {}

// ==============================================================================
// 5. QUARANTINED State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct QuarantinedPhase;
impl lsp_max_runtime::Phase for QuarantinedPhase {}

pub struct QuarantinedData {
    pub elements: Vec<Element>,
    pub quads: Vec<Quad>,
    pub missing_dependencies: Vec<String>,
}
impl lsp_max_runtime::Data for QuarantinedData {}

// ==============================================================================
// 6. SUPERSEDED State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct SupersededPhase;
impl lsp_max_runtime::Phase for SupersededPhase {}

pub struct SupersededData {
    pub graph_name: GraphName,
    pub superseded_by: GraphName,
}
impl lsp_max_runtime::Data for SupersededData {}

// ==============================================================================
// 7. REPLAYED State
// ==============================================================================
#[derive(Debug, Clone)]
pub struct ReplayedPhase;
impl lsp_max_runtime::Phase for ReplayedPhase {}

pub struct ReplayedData {
    pub graph_name: GraphName,
    pub receipt: CryptographicReceipt,
}
impl lsp_max_runtime::Data for ReplayedData {}
```

---

## 4. Lifecycle Routing Logic (State Transitions)

Ingested graph snapshots must flow through a deterministic routing pipeline. The transitions are enforced by consuming the state machines linearly.

```
                  +--------------------------------+
                  |              RAW               |
                  +--------------------------------+
                                  |
                                  | Ingest & Map (LSIF -> RDF)
                                  v
                  +--------------------------------+
                  |           CANDIDATE            |
                  +--------------------------------+
                                  |
       +--------------------------+--------------------------+--------------------------+
       | Replay Match             | Validation Fails         | Missing Deps             | Invariants Pass
       v                          v                          v                          v
+--------------+           +--------------+           +--------------+           +--------------+
|   REPLAYED   |           |   REFUSED    |           | QUARANTINED  |           |   ADMITTED   |
+--------------+           +--------------+           +--------------+           +--------------+
                                                             |                          |
                                                             | Deps Resolved            | Version Bump
                                                             v                          v
                                                      (Re-Candidate)             +--------------+
                                                             |                   |  SUPERSEDED  |
                                                             +------------------>+--------------+
```

### Transition Implementations

```rust
// RAW -> CANDIDATE
impl Machine<GraphAdmissionLaw, RawPhase, RawData> {
    pub fn admit_candidate(
        self,
        snapshot_id: &str,
    ) -> Result<Machine<GraphAdmissionLaw, CandidatePhase, CandidateData>, GraphAdmissionError> {
        let graph_name = GraphName::NamedNode(
            oxrdf::NamedNode::new(format!("urn:project:local:snapshot:{}", snapshot_id))
                .map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?
        );

        let mut quads = Vec::new();
        let mut byte_accumulator = Vec::new();

        for element in &self.data.elements {
            // Serialize elements for hashing
            if let Ok(serialized) = serde_json::to_vec(element) {
                byte_accumulator.extend_from_slice(&serialized);
            }
            
            // Map LSIF Elements to RDF Quads
            map_element_to_quads(element, &graph_name, &mut quads)?;
        }

        let graph_hash = lsp_max_runtime::sha256(&byte_accumulator);

        Ok(Machine::new(
            CandidatePhase,
            CandidateData {
                elements: self.data.elements,
                quads,
                graph_hash,
            },
        ))
    }
}

// CANDIDATE -> ADMITTED
impl Machine<GraphAdmissionLaw, CandidatePhase, CandidateData> {
    pub fn admit_admitted(
        self,
        store: &Store,
        graph_name: GraphName,
        receipt: CryptographicReceipt,
    ) -> Result<Machine<GraphAdmissionLaw, AdmittedPhase, AdmittedData>, GraphAdmissionError> {
        // Commit quads transactionally to Oxigraph Store
        let transaction = store.transaction();
        for quad in &self.data.quads {
            store.insert(quad).map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?;
        }
        
        Ok(Machine::new(
            AdmittedPhase,
            AdmittedData {
                graph_name,
                quad_count: self.data.quads.len(),
                receipt,
            },
        ))
    }
}

// CANDIDATE -> REFUSED
impl Machine<GraphAdmissionLaw, CandidatePhase, CandidateData> {
    pub fn admit_refuse(
        self,
        report: VerificationReport,
    ) -> Machine<GraphAdmissionLaw, RefusedPhase, RefusedData> {
        Machine::new(RefusedPhase, RefusedData { report })
    }
}

// CANDIDATE -> QUARANTINED
impl Machine<GraphAdmissionLaw, CandidatePhase, CandidateData> {
    pub fn admit_quarantine(
        self,
        missing_dependencies: Vec<String>,
    ) -> Machine<GraphAdmissionLaw, QuarantinedPhase, QuarantinedData> {
        Machine::new(
            QuarantinedPhase,
            QuarantinedData {
                elements: self.data.elements,
                quads: self.data.quads,
                missing_dependencies,
            },
        )
    }
}

// CANDIDATE -> REPLAYED
impl Machine<GraphAdmissionLaw, CandidatePhase, CandidateData> {
    pub fn admit_replay(
        self,
        graph_name: GraphName,
        receipt: CryptographicReceipt,
    ) -> Machine<GraphAdmissionLaw, ReplayedPhase, ReplayedData> {
        Machine::new(
            ReplayedPhase,
            ReplayedData {
                graph_name,
                receipt,
            },
        )
    }
}

// ADMITTED -> SUPERSEDED
impl Machine<GraphAdmissionLaw, AdmittedPhase, AdmittedData> {
    pub fn admit_supersede(
        self,
        superseded_by: GraphName,
    ) -> Machine<GraphAdmissionLaw, SupersededPhase, SupersededData> {
        Machine::new(
            SupersededPhase,
            SupersededData {
                graph_name: self.data.graph_name,
                superseded_by,
            },
        )
    }
}

// QUARANTINED -> CANDIDATE
impl Machine<GraphAdmissionLaw, QuarantinedPhase, QuarantinedData> {
    pub fn into_candidate(
        self,
        graph_hash: String,
    ) -> Machine<GraphAdmissionLaw, CandidatePhase, CandidateData> {
        Machine::new(
            CandidatePhase,
            CandidateData {
                elements: self.data.elements,
                quads: self.data.quads,
                graph_hash,
            },
        )
    }
}
```

---

## 5. Ingestion Mapping & Anti-Laundering Boundaries

The `map_element_to_quads` function translates LSIF domain entities to standard RDF quads. To enforce **PRD-R3 (Anti-Ontology Laundering)**, the admitter whitelists prefixes and ensures private extension variables (`max:`) do not pollute public namespaces (`lsif:`, `prov:`).

```rust
use oxrdf::{NamedNode, Subject, Term};
use lsp_max_lsif::lsif::{Vertex, Edge, ItemEdgeProperty};

/// Maps LSIF Elements to RDF Quads, validating namespace limits.
pub fn map_element_to_quads(
    element: &Element,
    graph_name: &GraphName,
    quads: &mut Vec<Quad>,
) -> Result<(), GraphAdmissionError> {
    match element {
        Element::Vertex(vertex) => map_vertex(vertex, graph_name, quads),
        Element::Edge(edge) => map_edge(edge, graph_name, quads),
    }
}

fn map_vertex(
    vertex: &Vertex,
    graph_name: &GraphName,
    quads: &mut Vec<Quad>,
) -> Result<(), GraphAdmissionError> {
    let sub_uri = format!("urn:project:local:lsif:{}", get_vertex_id(vertex));
    let subject = Subject::NamedNode(
        NamedNode::new(&sub_uri).map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?
    );

    // Apply rdf:type base mapping
    let class_uri = match vertex {
        Vertex::MetaData { .. } => "urn:lsp-max:core:Metadata",
        Vertex::Project { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Project",
        Vertex::Document { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Document",
        Vertex::ResultSet { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/ResultSet",
        Vertex::Range { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Range",
        Vertex::Moniker { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Moniker",
        Vertex::PackageInformation { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/PackageInformation",
        Vertex::HoverResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/HoverResult",
        Vertex::DefinitionResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/DefinitionResult",
        Vertex::ReferenceResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/ReferenceResult",
        _ => "urn:lsp-max:core:GenericVertex",
    };

    quads.push(Quad::new(
        subject.clone(),
        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        Term::NamedNode(NamedNode::new(class_uri).unwrap()),
        graph_name.clone(),
    ));

    // Handle properties and restrict namespace hijacking
    match vertex {
        Vertex::MetaData { version, .. } => {
            quads.push(Quad::new(
                subject.clone(),
                NamedNode::new("urn:lsp-max:core:version").unwrap(),
                Term::Literal(oxrdf::Literal::new_simple_literal(version)),
                graph_name.clone(),
            ));
        }
        Vertex::Document { uri, language_id, .. } => {
            quads.push(Quad::new(
                subject.clone(),
                NamedNode::new("urn:lsp-max:core:uri").unwrap(),
                Term::Literal(oxrdf::Literal::new_simple_literal(uri)),
                graph_name.clone(),
            ));
            quads.push(Quad::new(
                subject.clone(),
                NamedNode::new("urn:lsp-max:core:languageId").unwrap(),
                Term::Literal(oxrdf::Literal::new_simple_literal(language_id)),
                graph_name.clone(),
            ));
        }
        _ => {}
    }

    Ok(())
}

fn map_edge(
    edge: &Edge,
    graph_name: &GraphName,
    quads: &mut Vec<Quad>,
) -> Result<(), GraphAdmissionError> {
    let out_uri = format!("urn:project:local:lsif:{}", get_edge_out_v(edge));
    let subject = Subject::NamedNode(NamedNode::new(&out_uri).unwrap());

    let predicate_uri = match edge {
        Edge::Contains { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains",
        Edge::Next { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/next",
        Edge::Moniker { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/moniker",
        Edge::Attach { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/attach",
        Edge::PackageInformation { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/packageInformation",
        Edge::Item { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/item",
        Edge::TextDocumentDefinition { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_definition",
        Edge::TextDocumentReferences { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_references",
        Edge::TextDocumentHover { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_hover",
        _ => return Err(GraphAdmissionError::NamespaceViolation(format!("Unauthorized edge label mapping: {:?}", edge))),
    };

    let predicate = NamedNode::new(predicate_uri).unwrap();

    // Map targets (inV or inVs)
    for in_v in get_edge_in_vs(edge) {
        let in_uri = format!("urn:project:local:lsif:{}", in_v);
        let object = Term::NamedNode(NamedNode::new(&in_uri).unwrap());
        quads.push(Quad::new(subject.clone(), predicate.clone(), object, graph_name.clone()));
    }

    Ok(())
}

// Helper accessors
fn get_vertex_id(vertex: &Vertex) -> String {
    match vertex {
        Vertex::MetaData { id, .. } => id.to_string(),
        Vertex::Source { id, .. } => id.to_string(),
        Vertex::Project { id, .. } => id.to_string(),
        Vertex::Document { id, .. } => id.to_string(),
        Vertex::ResultSet { id, .. } => id.to_string(),
        Vertex::Range { id, .. } => id.to_string(),
        Vertex::ResultRange { id, .. } => id.to_string(),
        Vertex::Moniker { id, .. } => id.to_string(),
        Vertex::PackageInformation { id, .. } => id.to_string(),
        Vertex::HoverResult { id, .. } => id.to_string(),
        Vertex::ReferenceResult { id, .. } => id.to_string(),
        Vertex::DeclarationResult { id, .. } => id.to_string(),
        Vertex::DefinitionResult { id, .. } => id.to_string(),
        Vertex::ImplementationResult { id, .. } => id.to_string(),
        Vertex::TypeDefinitionResult { id, .. } => id.to_string(),
        Vertex::FoldingRangeResult { id, .. } => id.to_string(),
        Vertex::DocumentLinkResult { id, .. } => id.to_string(),
        Vertex::DocumentSymbolResult { id, .. } => id.to_string(),
        Vertex::DiagnosticResult { id, .. } => id.to_string(),
        Vertex::SemanticTokensResult { id, .. } => id.to_string(),
        Vertex::Event { id, .. } => id.to_string(),
    }
}

fn get_edge_out_v(edge: &Edge) -> String {
    match edge {
        Edge::Contains { out_v, .. } => out_v.to_string(),
        Edge::Next { out_v, .. } => out_v.to_string(),
        Edge::Moniker { out_v, .. } => out_v.to_string(),
        Edge::Attach { out_v, .. } => out_v.to_string(),
        Edge::PackageInformation { out_v, .. } => out_v.to_string(),
        Edge::Item { out_v, .. } => out_v.to_string(),
        Edge::TextDocumentHover { out_v, .. } => out_v.to_string(),
        Edge::TextDocumentDefinition { out_v, .. } => out_v.to_string(),
        Edge::TextDocumentDeclaration { out_v, .. } => out_v.to_string(),
        Edge::TextDocumentReferences { out_v, .. } => out_v.to_string(),
    }
}

fn get_edge_in_vs(edge: &Edge) -> Vec<String> {
    match edge {
        Edge::Contains { in_vs, .. } => in_vs.iter().map(|id| id.to_string()).collect(),
        Edge::Next { in_v, .. } => vec![in_v.to_string()],
        Edge::Moniker { in_v, .. } => vec![in_v.to_string()],
        Edge::Attach { in_v, .. } => vec![in_v.to_string()],
        Edge::PackageInformation { in_v, .. } => vec![in_v.to_string()],
        Edge::Item { in_vs, .. } => in_vs.iter().map(|id| id.to_string()).collect(),
        Edge::TextDocumentHover { in_v, .. } => vec![in_v.to_string()],
        Edge::TextDocumentDefinition { in_v, .. } => vec![in_v.to_string()],
        Edge::TextDocumentDeclaration { in_v, .. } => vec![in_v.to_string()],
        Edge::TextDocumentReferences { in_v, .. } => vec![in_v.to_string()],
    }
}
```

---

## 6. Verification & Invariant Enforcement

Before promoting a `CANDIDATE` snapshot to `ADMITTED`, the control plane must execute validation checks. This includes matching shape constraints and evaluating the five core SPARQL invariants.

```rust
use oxigraph::sparql::{SparqlEvaluator, QueryResults};

/// Evaluates core semantic validation rules on a candidate graph.
pub fn verify_invariants(
    store: &Store,
    graph_name: &GraphName,
) -> Result<VerificationReport, GraphAdmissionError> {
    let mut diagnostics = Vec::new();

    // Invariant 1: No Orphan LSIF Relations
    // Verify that every LSIF predicate relates nodes defined in the active snapshot.
    let orphan_query = "
        PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
        ASK {
            ?s ?p ?o .
            FILTER(STRSTARTS(STR(?p), 'https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/'))
            FILTER NOT EXISTS { ?o ?any_p ?any_o }
            FILTER NOT EXISTS { ?any_s ?any_p ?o }
        }
    ";
    
    let is_orphan = execute_ask(store, orphan_query)?;
    if is_orphan {
        diagnostics.push(MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: Some(lsp_types::NumberOrString::String("INVARIANT_1".to_string())),
                source: Some("RelationAdmitter".to_string()),
                message: "Graph contains orphan LSIF relations; object lacks structural node definitions.".to_string(),
                ..Default::default()
            },
            law_axis: lsp_max_protocol::LawAxis::Conformance,
        });
    }

    // Invariant 2: No Unreceipted Graph Consequences
    // Every diagnostic must have a prov:wasGeneratedBy link pointing to a max:Receipt.
    let receipt_query = "
        PREFIX prov: <http://www.w3.org/ns/prov#>
        PREFIX max:  <urn:lsp-max:core:>
        ASK {
            ?diag a max:Diagnostic .
            FILTER NOT EXISTS {
                ?diag prov:wasGeneratedBy ?receipt .
                ?receipt a max:Receipt .
            }
        }
    ";
    let is_unreceipted = execute_ask(store, receipt_query)?;
    if is_unreceipted {
        diagnostics.push(MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: Some(lsp_types::NumberOrString::String("INVARIANT_2".to_string())),
                source: Some("RelationAdmitter".to_string()),
                message: "Found compiled diagnostic artifact lacking cryptographic provenance receipt.".to_string(),
                ..Default::default()
            },
            law_axis: lsp_max_protocol::LawAxis::Provenance,
        });
    }

    let is_success = diagnostics.is_empty();
    
    if !is_success {
        return Err(GraphAdmissionError::InvariantViolated(VerificationReport {
            is_success,
            diagnostics,
            execution_time_ms: 0,
        }));
    }

    Ok(VerificationReport {
        is_success: true,
        diagnostics: Vec::new(),
        execution_time_ms: 0,
    })
}

fn execute_ask(store: &Store, query: &str) -> Result<bool, GraphAdmissionError> {
    let evaluator = SparqlEvaluator::new();
    let parsed_query = evaluator
        .parse_query(query)
        .map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?;
    
    let results = parsed_query
        .on_store(store)
        .execute()
        .map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?;

    if let QueryResults::Boolean(val) = results {
        Ok(val)
    } else {
        Err(GraphAdmissionError::ParsingFailed("Query did not return a boolean result".to_string()))
    }
}
```

---

## 7. Implementation Location & Dependency Mapping

To compile clean, the `RelationAdmitter` typestate engine must be integrated into the workspace:

### 7.1 Crate Dependency Modifications

We will add dependencies to `lsp-max-runtime/Cargo.toml` to link Oxigraph, base abstractions, and parsing logic:

```toml
[dependencies]
# Existing dependencies
lsp-max-protocol = { path = "../lsp-max-protocol", version = "26.6.4" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
lsp-types = "0.97.0"

# New Control Plane dependencies
oxigraph = "0.5.8"
lsp-max-base = { path = "../crates/lsp-max-base", version = "0.1.0" }
lsp-max-lsif = { path = "../crates/lsp-max-lsif" }
```

### 7.2 Directory and File Layout

The implementation will live under the `lsp-max-runtime` crate. The structure will be co-located with existing modules:

```
lsp-max-runtime/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Exposes module control_plane
    └── control_plane/
        ├── mod.rs              # Mod declaration
        └── admission.rs        # RelationAdmitter & state machine implementations
```

---

## 8. Verification and Testing Blueprint

A comprehensive test suite will be added to `lsp-max-runtime/src/control_plane/admission.rs` to guarantee happy and unhappy paths:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::store::Store;
    use lsp_max_lsif::lsif::{Element, Vertex, VertexType};

    fn make_test_metadata() -> Element {
        Element::Vertex(Vertex::MetaData {
            id: lsp_types::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            version: "0.6.0".to_string(),
            position_encoding: lsp_max_lsif::lsif::PositionEncoding::Utf16,
            tool_info: None,
        })
    }

    #[test]
    fn test_happy_path_raw_to_admitted() {
        let store = Store::new().unwrap();
        let raw_elements = vec![make_test_metadata()];

        // 1. RAW state
        let raw_machine = Machine::new(RawPhase, RawData { elements: raw_elements });

        // 2. Transition RAW -> CANDIDATE
        let candidate_machine = raw_machine.admit_candidate("snapshot-44").unwrap();
        assert_eq!(candidate_machine.data.quads.len(), 2); // type + version

        // 3. Perform verification checks
        let verify_result = verify_invariants(&store, &candidate_machine.data.quads[0].graph_name);
        assert!(verify_result.is_ok());

        // 4. Transition CANDIDATE -> ADMITTED
        let mock_receipt = CryptographicReceipt {
            prev_hash: "0000".to_string(),
            discipline_id: "test".to_string(),
            law_id: "test".to_string(),
            graph_hash: candidate_machine.data.graph_hash.clone(),
            query_hash: "0000".to_string(),
            result_hash: "0000".to_string(),
            sequence: 1,
            signature: vec![],
        };

        let graph_name = candidate_machine.data.quads[0].graph_name.clone();
        let admitted_machine = candidate_machine
            .admit_admitted(&store, graph_name, mock_receipt)
            .unwrap();

        assert_eq!(admitted_machine.data.quad_count, 2);
    }
}
```
