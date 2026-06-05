# ADR-0001: Bootstrap Specification Generator First

## Context

The `tower-lsp-max` workspace requires standard and extended LSP types (such as capability vectors, diagnostics, and transactional code actions). Writing these types by hand is error-prone and hard to keep updated with the fast-evolving LSP specification. The LSP project publishes a formal JSON metamodel of the protocol (`metaModel.json`).

## Decision

We bootstrap a specification generator crate `tower-lsp-max-specgen` in the workspace first. This utility reads the official LSP metamodel and generates a type-safe Rust representation of the protocol vocabulary.

## Rationale

1. **Precision:** Automating vocabulary mapping ensures generated Rust structs match the exact JSON schema defined by the LSP Working Group.
2. **Speed:** Upgrading the protocol to a newer version of the LSP spec becomes as simple as updating the `metaModel.json` fixture and re-running the generator.
3. **Safety:** Typestate structures, request/notification associations, and serialization attributes are kept in sync automatically.
