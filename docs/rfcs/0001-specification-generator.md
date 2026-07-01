# RFC 0001: Specification Generator and Protocol Vocabulary

**Status:** Accepted

## Context

The `lsp-max` workspace requires standard and extended LSP types (such as capability vectors, diagnostics, and transactional code actions). Writing these types by hand is error-prone and hard to keep updated with the fast-evolving LSP specification. The LSP Working Group publishes a formal JSON metamodel of the protocol.

Additionally, the traditional LSP server assumes it is an IDE helper or developer assistance tool meant to improve human typing and display visual diagnostics inside a text editor. This assumption is obsolete under autonomous machine agent workflows. Autonomous agents do not require interactive hints; they require a deterministic protocol to query state, calculate conformance, apply atomic edits, run correctness gates, and log cryptographic receipts.

In standard Language Server Protocol implementations, the type vocabulary (structures, enums, requests, and notifications) is often maintained manually. Developers replicate the schema defined by the LSP specifications in their target programming languages by copying fields, annotations, and documentation comments. This manual replication introduces errors, inconsistencies, and protocol drift.

## Decision

We have made the decision to repurpose the workspace framework as `lsp-max` — a **post-human project-state enforcement server** rather than a traditional IDE helper.

To achieve this, we bootstrap a specification generator crate `lsp-max-specgen` in the workspace. This utility reads the official LSP metamodel (`metaModel.json`) and generates a type-safe Rust representation of the protocol vocabulary. Standard LSP types are generated automatically; extended vocabulary structures for custom `max/` endpoints are layered on top.

The generated Rust modules (e.g., `crates/lsp-max-protocol/src/lsp_3_18.rs`) are declared to be the **absolute source of truth** for all protocol data models. No manual edits are permitted on these generated structures. Custom protocol extensions are built on top of this generated vocabulary by nesting standard types or providing explicit wrappers.

This transforms the LSP endpoint from an editor backend into a state transition gateway for autonomous machine agents.

## Rationale

1. **Protocol Doctrine Alignment:** By declaring `lsp-max` a post-human project-state protocol, we formalize the server as an admission controller. Automating standard type generation allows developers and agents to focus exclusively on custom state enforcement logic rather than mapping boilerplate editor types.

2. **Precision & Consistency:** Automating vocabulary mapping ensures generated Rust structs match the exact JSON schema defined by the LSP Working Group. Manual drift is prevented, securing a reliable substrate.

3. **Speed of Evolution:** Upgrading the protocol to a newer version of the LSP spec is simplified: update the `metaModel.json` fixture and re-run the generator. The agent logic immediately benefits from new features without manual typing.

4. **Typestate Security:** Associations between LSP requests, notifications, and custom metadata are synchronized automatically, enabling zero-cost typestate validation inside `lsp-max-runtime`.

5. **Absolute Truth & Prevention of Drift:** By deriving types directly from the JSON schema published by the LSP Working Group, we establish a single, verifiable source of truth. Any change in protocol requirements must be initiated by updating the metamodel and re-running the generator.

6. **Deterministic Schemas:** The generator automatically maps the 11 type kinds defined by the metamodel, guaranteeing that serialization and deserialization boundaries conform exactly to the spec.

## Consequences

**Positive:**
- Generated types are always in sync with the official LSP specification.
- Silent protocol drift is eliminated by construction.
- Developers can focus on semantic law plugins and runtime typestate transitions.
- Automated verification of schema compliance.

**Negative:**
- Generator maintenance: updating `metaModel.json` and re-running codegen is a development overhead.
- Generated code is opaque to developers (read-only); debugging schema issues requires understanding the generator, not the generated output.

**Neutral:**
- Existing LSP clients (editors) are unaffected; they see generated types that conform to the spec they already understand.

## Reference

- **Generator crate:** `crates/lsp-max-specgen`
- **Generated modules:** `crates/lsp-max-protocol/src/lsp_*.rs`
- **Metamodel source:** `external/metaModel.json` (LSP official specification)
- **Build:** Codegen runs as a `build.rs` script during `cargo build`
