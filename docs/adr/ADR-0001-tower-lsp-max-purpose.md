# ADR-0001: Bootstrap Specification Generator First (Refined)

## Status
Accepted

## Context

The `tower-lsp-max` workspace requires standard and extended LSP types (such as capability vectors, diagnostics, and transactional code actions). Writing these types by hand is error-prone and hard to keep updated with the fast-evolving LSP specification. The LSP project publishes a formal JSON metamodel of the protocol (`metaModel.json`).

Furthermore, standard LSP servers are built under the assumption that they are IDE helpers or developer assistance tools meant to improve human typing and display visual diagnostics inside a text editor. This assumption is obsolete under autonomous machine agent workflows. 

Autonomous agents do not require interactive hints; they require a deterministic protocol to query state, calculate conformance, apply atomic edits, run correctness gates, and log cryptographic receipts.

## Decision

We have made the decision to repurpose the workspace framework as `tower-lsp-max` — a **post-human project-state enforcement server** rather than a traditional IDE helper. 

To achieve this, we bootstrap a specification generator crate `tower-lsp-max-specgen` in the workspace first. This utility reads the official LSP metamodel and generates a type-safe Rust representation of the protocol vocabulary. Standard LSP types are generated automatically, while extended vocabulary structures for custom `max/` endpoints are layered on top of this generated vocabulary.

This transforms the LSP endpoint from an editor backend into a state transition gateway for autonomous machine agents.

## Rationale

1. **Protocol Doctrine Alignment:** By declaring `tower-lsp-max` a post-human project-state protocol, we formalize the server as an admission controller. Automating standard type generation allows developers and agents to focus exclusively on custom state enforcement logic rather than mapping boilerplate editor types.
2. **Precision & Consistency:** Automating vocabulary mapping ensures generated Rust structs match the exact JSON schema defined by the LSP Working Group. Manual drift is prevented, securing a reliable substrate.
3. **Speed of Evolution:** Upgrading the protocol to a newer version of the LSP spec is simplified: update the `metaModel.json` fixture and re-run the generator. The agent logic immediately benefits from new features without manual typing.
4. **Typestate Security:** Associations between LSP requests, notifications, and custom metadata are synchronized automatically, enabling zero-cost typestate validation inside `tower-lsp-max-runtime`.
