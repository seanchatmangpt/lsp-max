# Requests for Comments (RFCs)

This directory contains accepted design decisions for `lsp-max`. Each RFC is a durable record of a significant architectural choice, the rationale, and its consequences.

RFCs are numbered sequentially (0001, 0002, ...) and assigned a status:

- **Accepted:** The decision is settled and implemented.
- **Superseded:** A later RFC has replaced this one; see the newer RFC for details.
- **Withdrawn:** The decision was considered but rejected; kept for historical context.

## Accepted RFCs

| RFC | Title | Status |
|-----|-------|--------|
| [0001](0001-specification-generator.md) | Specification Generator and Protocol Vocabulary | Accepted |
| [0002](0002-law-enforcement-via-receipt-chains.md) | Law Enforcement via Receipt Chains | Accepted |
| [0003](0003-conformance-vector-three-valued-logic.md) | Conformance Vector Three-Valued Logic | Accepted |
| [0004](0004-composition-over-tower-lsp-fork.md) | Composition Over tower-lsp Fork | Accepted |
| [0005](0005-calver-versioning-over-semver.md) | CalVer Versioning Over SemVer | Accepted |

## Reading Order

For first-time readers, start with:

1. `docs/book/01-architecture.md` — Comprehensive system overview
2. **RFC 0001** — Why code generation is the foundation
3. **RFC 0002–0005** — The specific decisions that shape the architecture

For deep dives:

- **RFC 0002–0003** are prerequisites for understanding the law-state runtime (layers 3 of the architecture).
- **RFC 0004** explains the five-layer model and why tower-lsp was forked rather than extended.
- **RFC 0005** governs versioning and deployment semantics.

## Contributing

When a significant architectural decision is needed:

1. Write an RFC in the format above (status, context, decision, rationale, consequences, alternatives).
2. Number it sequentially (the next available number).
3. Submit for approval via PR.
4. Update this README with a row in the table above.

Never edit an existing RFC; instead, write a new RFC that supersedes it and link back.
