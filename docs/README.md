# Documentation

This directory contains lsp-max documentation organized by the **Diataxis** framework: four distinct types of documentation serving different user needs.

## Four Pillars

### 📚 [Tutorials](./tutorials/) — Learning-Oriented

**Purpose:** Get started with lsp-max by walking through a complete, working example.

**Audience:** New users, developers learning the library for the first time.

**What You'll Find:**
- [Getting Started](./tutorials/getting-started.md) — Complete agent loop that self-corrects from LSP law signals

**When to Read:** You're new to lsp-max and want to understand how everything fits together.

---

### 🎯 [How-To Guides](./how-to/) — Task-Oriented

**Purpose:** Solve a specific problem or complete a specific task.

**Audience:** Developers with some lsp-max experience looking for recipes and step-by-step instructions.

**What You'll Find:**
- [index.md](./how-to/index.md) — Standalone recipes for common integration tasks (block agent execution on ANDON gate, examine gate state, etc.)
- [release.md](./how-to/release.md) — Release workflow: versioning, pre-release checklist, dry-run publish, manual publish, GitHub release, rollback

**When to Read:** You know what you want to do (e.g., "block an agent on the ANDON gate") and want step-by-step instructions.

---

### 📖 [Reference](./reference/) — Information-Oriented

**Purpose:** Look up exact definitions, schemas, APIs, and configuration.

**Audience:** Developers building with lsp-max who need precise, complete information.

**What You'll Find:**
- [reference.md](./reference/reference.md) — Complete max/* protocol, type schemas, gate predicate, LSIF extensions, OCEL model, Declare constraints, receipt chain, virtual documents, RulePackServer trait, CLI grammar, model policies
- [CONFIGURATION_REFERENCE.md](./reference/CONFIGURATION_REFERENCE.md) — LSP and lsp-max configuration keys
- [PERFORMANCE.md](./reference/PERFORMANCE.md) — Performance tuning, benchmarks, optimization strategies
- [CANCELLATION_SAFETY.md](./reference/CANCELLATION_SAFETY.md) — Cancellation semantics, safe shutdown
- [TEST_INFRA.md](./reference/TEST_INFRA.md) — Testing patterns, receipt validation, dogfood suites
- [RELEASE.md](./reference/RELEASE.md) — Glossary of release terms and statuses (see also `how-to/release.md` for workflow)

**When to Read:** You're implementing a feature, writing tests, or need exact type signatures and configuration options.

---

### 💡 [Explanation](./explanation/) — Understanding-Oriented

**Purpose:** Understand the *why* behind architectural decisions and key concepts.

**Audience:** Developers who want to understand the reasoning, design tradeoffs, and big-picture context.

**What You'll Find:**
- [index.md](./explanation/index.md) — Why LSP is the ambient law-state substrate for coding agents; why request-response is insufficient for law enforcement; three-valued logic of ConformanceVector; how OCEL and LSIF close the gap; why the ggen + RulePackServer pattern scales

**When to Read:** You want to understand the "why" behind a design decision, or you're designing a new feature and need to ground it in the project's philosophy.

---

## Quick Navigation

| I want to... | Start here |
|---|---|
| Learn lsp-max from scratch | [Tutorials](./tutorials/) |
| Block an agent on ANDON | [How-to: Block agent on gate](./how-to/index.md#recipe-1--block-agent-execution-on-andon-gate) |
| Release a new version | [How-to: Release workflow](./how-to/release.md) |
| Look up the max/* protocol | [Reference: Protocol](./reference/reference.md#max-methods) |
| Configure lsp-max | [Reference: Configuration](./reference/CONFIGURATION_REFERENCE.md) |
| Understand why LSP? | [Explanation: Law-state substrate](./explanation/index.md) |
| Understand three-valued logic | [Explanation: ConformanceVector](./explanation/index.md) |
| See working examples | [examples/](../examples/) (source tree) |
| Check RFCs and decisions | [rfcs/](./rfcs/) — Accepted design decisions |
| Browse archive | [archive/](./archive/) — Prior versions and historical context |

---

## Other Resources

### In This Repo

- **[AGENTS.md](../AGENTS.md)** — Project constitution: release law, diagnostic tokens, required tests
- **[CLAUDE.md](../CLAUDE.md)** — Claude Code-specific guidance and conventions
- **[CONTRIBUTING.md](../CONTRIBUTING.md)** — Contribution workflow, branch naming, code review
- **[CHANGELOG.md](../CHANGELOG.md)** — Version history and release notes
- **[DEFINITION_OF_DONE.md](../DEFINITION_OF_DONE.md)** — Release admission gates for the current version

### External

- [LSP 3.18 Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/) — Official LSP spec
- [Diataxis Framework](https://diataxis.fr/) — Documentation structure philosophy
- [Tower-LSP (upstream)](https://github.com/ebkalderon/tower-lsp) — The LSP framework lsp-max is based on

---

## Contributing to Docs

### Adding a New How-To Guide

1. Create a file in `docs/how-to/<topic>.md` (or `docs/how-to/index.md` if modifying the main guide)
2. Start with a one-line summary: "Recipe: <task> — <outcome>"
3. List prerequisites (what must be true to start)
4. Provide step-by-step instructions
5. Link from the [how-to index](./how-to/index.md)

### Adding a New Tutorial

1. Create a file in `docs/tutorials/<name>.md`
2. Start with a learning-focused introduction (not a reference)
3. Walk through a complete, working example
4. Link from the [tutorials index](./tutorials/)

### Adding a New Reference

1. Create or update a file in `docs/reference/`
2. Use structured sections (tables, schemas, definitions)
3. Include code examples where relevant
4. Keep prose minimal; prioritize lookupability

### Adding Explanation

1. Modify or create a file in `docs/explanation/`
2. Start with the core question: "Why X?"
3. Explain the reasoning and tradeoffs
4. Relate to other concepts in the system

---

**Last Updated:** July 1, 2026
