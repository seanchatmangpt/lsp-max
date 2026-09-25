# ADR-0002: Generated Protocol Vocabulary as Source of Protocol Truth

## Status
Accepted

## Context

In standard Language Server Protocol implementations, the type vocabulary (structures, enums, requests, and notifications) is often maintained manually. Developers replicate the schema defined by the LSP specifications in their target programming languages by copying fields, annotations, and documentation comments.

This manual replication is problematic for several reasons:
- It introduces errors and inconsistencies due to human oversight.
- It is difficult to keep updated as the LSP specifications evolve (e.g., transition from 3.16 to 3.17, and now 3.18).
- It leads to silent protocol drift where custom extensions or interpretations diverge from the official spec.

Because `tower-lsp-max` is a post-human project-state enforcement server, its components rely on precise type safety, cryptographic verification, and deterministic schemas. Any manual drift in protocol types could cause validation gates or state admission checks to fail silently or crash the machine state runtime.

## Decision

We have decided to generate standard and extended protocol vocabulary using the custom specification generator crate `tower-lsp-max-specgen` directly from the official LSP `metaModel.json` schema.

The generated Rust modules (e.g., `crates/tower-lsp-max-protocol/src/lsp_3_18.rs`) are declared to be the absolute source of truth for all protocol data models. No manual edits are permitted on these generated structures. Custom protocol extensions are built on top of this generated vocabulary by nesting standard types or providing explicit wrappers.

## Rationale

1. **Absolute Truth & Prevention of Drift:** By deriving types directly from the JSON schema published by the LSP Working Group, we establish a single, verifiable source of truth. Any change in protocol requirements must be initiated by updating the metamodel and re-running the generator.
2. **Deterministic Schemas:** The generator automatically maps the 11 type kinds defined by the metamodel, guaranteeing that serialization and deserialization boundaries conform exactly to the spec.
3. **Automated Verification:** Autonomous agents and validation engines can depend on the generated vocabulary to evaluate state conformance.
4. **Developer/Agent Focus:** Eliminating manually written type models allows implementation work to focus on semantic law plugins and runtime typestate transitions.
