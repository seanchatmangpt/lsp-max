# Compositor: Multi-Server LSP Composition

This chapter describes the `lsp-max-compositor` — the fan-out/merge multiplexer that allows multiple LSP servers to operate on the same file extension and synchronizes their diagnostics.

## Overview

Most editors bind a single LSP server per file extension. The compositor breaks this limitation: multiple servers can register for `.rs`, `.tsx`, etc., and their diagnostics (and other features) are merged at the protocol level.

```
Editor Session
       |
       | LSP 3.18 (stdio/TCP)
       v
   Compositor
    |    |    |
    +----+----+--- Fan-out (didOpen, didChange, etc.)
         |
    +----+----+---- Merge (publishDiagnostics, receipts)
    |    |    |
    v    v    v
Rust-Analyzer  Clippy-LSP  Test-Analyzer
(Primary)       (DiagnosticsOnly)  (DiagnosticsOnly)
```

## Architecture

### Tier-Stratified Routing

Each child server is registered with a **tier** and a list of **file extensions**:

- **Primary tier:** Full LSP support (navigation, formatting, refactoring). Used for FirstSuccess dispatch (navigation methods return the first Primary server's response).
- **Secondary tier:** Partial support. Included in FanAll merges but excluded from FirstSuccess.
- **DiagnosticsOnly tier:** Diagnostics only. Never serves navigation or document mutation requests.
- **Lsif tier:** Read-only LSIF index; used as a fallback for navigation when Primary servers are unavailable.

Example configuration in `lsp-max.toml`:

```toml
[[server]]
id = "rust-analyzer"
command = "rust-analyzer"
args = []
primary_extensions = [".rs"]
secondary_extensions = []
priority = "full"  # Maps to Primary tier

[[server]]
id = "clippy-lsp"
command = "clippy-lsp"
args = []
primary_extensions = [".rs"]
secondary_extensions = []
priority = "diagnostics-only"  # Maps to DiagnosticsOnly tier
```

### Dispatch Strategies

LSP methods are classified into dispatch strategies:

| Strategy | Methods | Behavior |
|----------|---------|----------|
| **FirstSuccess** | `hover`, `definition`, `completion`, `references`, etc. | Use the first Primary server's non-null response; skip secondaries. |
| **FanAll** | `publishDiagnostics`, `diagnostic` | Collect responses from all servers and merge. |
| **Notify** | `didOpen`, `didChange`, `didClose`, `didSave` | Send notification to all servers; no merge. |
| **PrimaryOnly** | `formatting`, `codeAction`, etc. | Send only to Primary-tier servers. |

### Diagnostic Merging

Diagnostics are collected from all servers and merged by the **FlushCoordinator**:

1. **Collection:** Each server's diagnostics are buffered (not immediately published).
2. **Dedup strategy:** By default, diagnostics are merged without deduplication. REFUSED_BY_LAW diagnostics (law violations) are always preserved.
3. **Flush:** On a fixed schedule or on explicit `max/flushDiagnostics` request, all buffered diagnostics are published in a single `publishDiagnostics` call.

Each flush produces a **CompositorReceipt** that:

- Attributes each diagnostic to its originating server.
- Binds diagnostics to code symbols via moniker join keys (RFC-B: per-server speciation receipt chains).
- Records the flush state (count of admitted diagnostics, presence of ANDON refusals) in a cryptographic receipt.

## Integration with the Law-State Runtime

The compositor is itself a law-state consumer:

1. **Session initialization:** When an editor opens a session, `lsp-max` runs SessionStart hooks to discover and spawn child servers.
2. **Pre-dispatch:** Before fanning out a request, the gate predicate (§III.4 in `01-architecture.md`) is evaluated. If the predicate refuses, the compositor emits an ANDON diagnostic and blocks the dispatch.
3. **Post-merge:** After merging diagnostics, the compositor runs a conformance check: are the merged diagnostics consistent with the law-state runtime's expectations? Any violation is reported.

## Receipt Chains (RFC-B)

Each child server's diagnostic stream is bound to a cryptographic receipt chain. The compositor:

1. Observes each child's diagnostics.
2. Computes a `ChildEvidence` record (server_id, receipt, symbol_object_id, has_andon_contribution).
3. Builds a `CompositorReceipt` that links the per-child evidence to the merged verdict.

This enables **attribution**: if a release fails, you can trace which server emitted which diagnostic and which law gate refused the transition.

## Example: Using the Compositor

**Configuration** (`lsp-max.toml`):

```toml
[[server]]
id = "rust-analyzer"
command = "target/debug/rust-analyzer"
primary_extensions = [".rs"]
priority = "full"

[[server]]
id = "test-analyzer"
command = "target/debug/test-analyzer"
primary_extensions = [".rs"]
priority = "diagnostics-only"
andon_code_prefixes = ["TEST-ANALYSIS-"]
```

**Editor behavior:**
- Hover → routed to rust-analyzer (FirstSuccess).
- Diagnostics → collected from rust-analyzer + test-analyzer and merged.
- ANDON diagnostics (TEST-ANALYSIS-*) block gates and are always preserved in merges.

## Crate Structure

- `crates/lsp-max-compositor/src/registry.rs` — ChildServer, ChildTier, ExtensionRouter.
- `crates/lsp-max-compositor/src/fanout.rs` — Dispatch strategy classification.
- `crates/lsp-max-compositor/src/flush_coordinator.rs` — Buffering and flushing logic.
- `crates/lsp-max-compositor/src/receipt_chain.rs` — RFC-B per-server receipt binding.
- `crates/lsp-max-compositor/src/config.rs` — Parsing lsp-max.toml server configuration.

## Further Reading

- **Architecture overview:** `docs/book/01-architecture.md` (Section IV: Multi-Server Composition)
- **RFC-B (Speciation Receipt Chains):** Referenced in `01-architecture.md` Section V.4
- **max/* Protocol:** `docs/reference/max-protocol-law.md`
