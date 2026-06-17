# ADR-0003: Composition Over tower-lsp Fork

**Status:** ACCEPTED

## Context

The original lsp-max codebase was a fork of `tower-lsp`, the standard Rust LSP framework. `tower-lsp` is a solid, battle-tested foundation that handles stdio/TCP transport, message routing, and request/response dispatch.

The design question was: should lsp-max extend tower-lsp through composition (wrapper traits, middlewares, protocol-level extensions) or fork it to embed law-state machinery directly?

**Composition challenges:**
- tower-lsp's internal state machine and trait bounds are tightly coupled to stateless request dispatch.
- Adding receipt chains and ConformanceVector state requires re-architecting the entire `LanguageServer` trait lifecycle.
- Middleware-based approaches force users to repeat receipt threading at the call site, creating friction and error-prone boilerplate.
- tower-lsp's message dispatch is synchronous-within-async; law-state phases (typestate transitions) cannot be grafted onto it without breaking LSP semantics.

**Forking rationale:**
- Full control over the execution model: law-state machines can be the primary abstraction, not bolted on top.
- No version lock to upstream: lsp-max can move at its own pace and adopt LSP 3.18 features independently.
- Cleaner separation of concerns: the five-layer model (actuation → LSP surface → law-state runtime → knowledge hooks → mesh) is architecturally enforced.
- Explicit law visibility: developers cannot accidentally bypass receipt generation or conformance checks.

## Decision

Fork tower-lsp into the root crate of lsp-max. Refactor the `LanguageServer` trait and `LspService` to be **law-state aware**: every method invocation threads through the runtime's typestate machine and generates receipts as a side effect of dispatch.

The five-layer isolation model is enforced:

1. **Actuation grammar** (`lsp-max-cli`, `clap-noun-verb`): command-line interface to the runtime.
2. **Local LSP state surface** (`src/`, root crate): `LanguageServer` trait, `LspService`, transport (stdio/TCP).
3. **Law-state runtime** (`lsp-max-runtime`): phases, transitions, receipt generation, gate checks.
4. **Knowledge hooks** (`lsp-max-agent`): analysis bundles, agent integration.
5. **Autonomic LSP mesh** (sibling repos): wasm4pm, lsp-types-max, wasm4pm-compat.

Never reference plain `tower-lsp` or `tower_lsp` in code, docs, tests, or commit messages outside of explicit negative-control fixtures (e.g., examples that demonstrate what *not* to do).

## Consequences

**Positive:**
- Unified law-state model: receipts and conformance vectors are first-class in the dispatch path.
- No adaptation layer needed: law enforcement is built in, not retrofit.
- Independent versioning: lsp-max can track LSP 3.18 without waiting for upstream.
- Compiler safety: the typestate machine is enforced at compile time; invalid state transitions are type errors.

**Negative:**
- Forking burden: lsp-max must track upstream LSP 3.18 changes and port them independently.
- Larger codebase: the fork adds ~500 LOC of law-state machinery on top of tower-lsp's ~2000 LOC.
- Maintenance cost: bug fixes to upstream LSP handling must be ported manually.

**Neutral:**
- Existing LSP clients (editors) are unaffected; the wire protocol is unchanged.
- The fork does not prevent using lsp-max as a library in other projects.

## Alternatives Considered

1. **Pure composition (trait wrappers)**: Would require users to manually thread receipts; high friction.
2. **Patching tower-lsp at build time**: Fragile; requires post-hoc bytecode patching or procedural macros.
3. **Vendoring tower-lsp (monorepo)**: Moves maintenance burden but loses upstream updates.
4. **Switching to a different LSP framework**: Language-specific (e.g., lsp-client in C++ doesn't exist in Rust in the same form).

Forking was chosen because it:
- Cleanly separates law-state machinery from transport machinery.
- Enables the five-layer model without retrofitting.
- Allows lsp-max to evolve independently while remaining a superset of tower-lsp semantics.

## Reference

- **Root crate**: `src/` (contains LanguageServer trait, LspService, gate.rs, diagnostics.rs)
- **Anti-tower-lsp checks**: `examples/anti-llm-cheat-lsp` (detects re-introduction of plain tower-lsp)
- **Related law**: AGENTS.md Section 2 ("Never reference plain tower-lsp/tower_lsp")
- **Five-layer model**: CLAUDE.md Section "Workspace architecture"
