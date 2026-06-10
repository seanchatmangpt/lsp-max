//! Core abstractions for the lsp-max framework.
//! These traits enforce the unidirectional data flow from raw observation to static semantic index.

/// Represents a raw observation of source code from an editor or filesystem.
pub trait SourceObservation {
    type Uri;
    type Text;
    type Version;

    /// The unique resource identifier for this observation.
    fn uri(&self) -> &Self::Uri;

    /// The raw text content of the source code.
    fn text(&self) -> &Self::Text;

    /// The version or state counter of this observation.
    fn version(&self) -> Self::Version;
}

/// Ingests a `SourceObservation` and parses it into an opaque domain-specific representation.
/// This is the boundary where a vendor like `auto-lsp` (Tree-sitter/Salsa) operates.
pub trait ParseIngress {
    type Observation: SourceObservation;
    type Parsed;
    type Error;

    /// Parses the raw observation into a syntax tree or semantic database state.
    fn parse(&self, observation: Self::Observation) -> Result<Self::Parsed, Self::Error>;
}

/// Elevates a `Parsed` state into the LiveLSP's strict, meaning-bearing relation graph.
/// This trait enforces the framework's admission and refusal laws.
pub trait RelationAdmitter {
    type Parsed;
    type RelationGraph;
    type Refusal;

    /// Determines if the parsed state is structurally and semantically valid enough
    /// to be admitted into the global semantic relation graph.
    fn admit(&self, parsed: Self::Parsed) -> Result<Self::RelationGraph, Self::Refusal>;
}

/// Translates the admitted `RelationGraph` into an LSIF 0.6.0 compliant dump.
/// This allows the system to act as a static query substrate.
pub trait StaticIndexEmitter {
    type RelationGraph;
    type LsifDump;
    type Error;

    /// Emits the strict LSIF NDJSON format graph from the internal semantic representation.
    fn emit_lsif_060(&self, graph: Self::RelationGraph) -> Result<Self::LsifDump, Self::Error>;
}
