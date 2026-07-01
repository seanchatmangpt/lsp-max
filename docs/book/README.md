# The lsp-max Book

This is the authoritative guide to `lsp-max` — a post-human LSP 3.18 runtime that enforces architectural laws via cryptographic receipt chains, three-valued conformance vectors, and deterministic gates.

## Chapters

### 1. [Architecture: Law-State Runtime via LSP](01-architecture.md)

The core system design. Covers:
- The post-human paradigm (LSP as law enforcer, not IDE helper)
- Five-layer stack (Actuation → LSP Surface → Law-State Runtime → Knowledge Hooks → Mesh)
- Typestate machine (five phases, unidirectional transitions)
- ConformanceVector (admitted/refused/unknown three-axis logic)
- Receipt chains (cryptographic proof of every state transition)
- Gate predicates (Λ_CD formal logic for law enforcement)
- Multi-server composition (tier-stratified routing, fan-out/merge)

**Read first if:** You're new to lsp-max or want to understand how the system works.

### 2. [Compositor: Multi-Server LSP Composition](02-compositor.md)

Detailed description of the `lsp-max-compositor` — the multiplexer that allows multiple LSP servers to operate on the same file extension.

Covers:
- Tier-stratified routing (Primary/Secondary/DiagnosticsOnly/Lsif)
- Dispatch strategies (FirstSuccess, FanAll, Notify, PrimaryOnly)
- Diagnostic merging and flushing
- Per-server receipt chains (RFC-B speciation)
- Integration with the law-state runtime

**Read if:** You're configuring multiple servers or want to understand the fan-out/merge architecture.

### 3. [Getting Started](03-getting-started.md)

Hands-on guide to building, running, and extending lsp-max.

Covers:
- Building from source
- Running the server
- Building a custom LSP server (via RulePackServer trait)
- Project structure
- Debugging
- Common tasks

**Read if:** You want to get the code running or start implementing a new server.

### 4. [Contributing to lsp-max](04-contributing.md)

How to contribute code, report issues, and participate in the project.

Covers:
- Workflow (clone, branch, commit, PR)
- Style guide (naming, comments, unsafe code policy)
- Testing (unit tests, integration tests, property-based testing)
- Documentation standards
- Code review guidelines
- Release process

**Read if:** You're making changes to the codebase or want to understand project standards.

## Design Decisions (RFCs)

Major architectural decisions are documented as Requests for Comments (RFCs) in [`docs/rfcs/README.md`](../rfcs/README.md). Read these if you want to understand the rationale behind specific choices:

- **RFC 0001:** Specification Generator and Protocol Vocabulary
- **RFC 0002:** Law Enforcement via Receipt Chains
- **RFC 0003:** ConformanceVector Three-Valued Logic
- **RFC 0004:** Composition Over tower-lsp Fork
- **RFC 0005:** CalVer Versioning Over SemVer

## Technical References

Quick references and deep-dive documents:

- [`docs/reference/`](../reference/) — Technical reference docs (max/* protocol, config keys, performance notes)
- [`docs/archive/`](../archive/) — Historical and superseded documentation

## Reading Guide

**For beginners:**
1. Start with [01-architecture.md](01-architecture.md) to understand the big picture.
2. Then [03-getting-started.md](03-getting-started.md) to build and run the code.
3. Check [`docs/rfcs/README.md`](../rfcs/README.md) if you want to know why things are designed the way they are.

**For implementers:**
1. [02-compositor.md](02-compositor.md) if you're configuring multiple servers.
2. [04-contributing.md](04-contributing.md) for coding standards and workflow.
3. Reference docs in [`docs/reference/`](../reference/) for specific APIs.

**For architects:**
1. [01-architecture.md](01-architecture.md) for the full system model.
2. All RFCs in [`docs/rfcs/`](../rfcs/) for decision rationale.
3. Consider writing new RFCs for major changes (see `docs/rfcs/README.md` for template).

## Structure

```
docs/
├── book/                          # This directory
│   ├── README.md                  # You are here
│   ├── 01-architecture.md         # System design
│   ├── 02-compositor.md           # Multi-server composition
│   ├── 03-getting-started.md      # Build & run guide
│   └── 04-contributing.md         # Contribution guide
├── rfcs/                          # Design decisions
│   ├── README.md                  # RFC index
│   ├── 0001-specification-generator.md
│   ├── 0002-law-enforcement-via-receipt-chains.md
│   ├── 0003-conformance-vector-three-valued-logic.md
│   ├── 0004-composition-over-tower-lsp-fork.md
│   └── 0005-calver-versioning-over-semver.md
├── reference/                     # Technical references
│   ├── reference.md               # LSP 3.18 reference
│   ├── how-to.md                  # Task recipes
│   ├── tutorial.md                # Walkthrough examples
│   └── ...
├── archive/                       # Historical docs
│   ├── README.md                  # What's archived and why
│   ├── adr/                       # Old ADRs
│   ├── law/                       # Theoretical foundations
│   ├── reports/                   # Research and exploration
│   └── ...
└── jira/                          # Current feature epics
    └── v26.6.30/                  # Active backlog
```

## Contributing to Docs

Documentation is as important as code. To contribute:

1. **Fix typos or clarify wording** → Submit a PR with the fix.
2. **Update docs after code changes** → Ensure docs stay in sync with implementation.
3. **Add a new chapter or reference** → First write an RFC explaining why, get approval, then write the doc.
4. **Report confusing docs** → Open an issue; we take clarity seriously.

See [04-contributing.md](04-contributing.md) (Documentation section) for details.

## Version

This documentation is current as of **lsp-max v26.7.1** (July 1, 2026, CalVer format: YY.M.D).

Version history: [`CHANGELOG.md`](../../CHANGELOG.md)
