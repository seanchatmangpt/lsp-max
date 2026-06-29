# Claude Code as an LSP Client

Research synthesized 2026-06-29. Sources: Microsoft LSP 3.17–3.18 spec, LSIF 0.5.0–0.6.0 spec,
Neovim client PRs, VS Code LSP extension guide, tower-lsp/async-lsp Rust crates. 103 agents,
21 sources fetched, 96 claims extracted, 25 adversarially verified (12 confirmed, 13 killed).

---

## 1. Notification Model

### didOpen

- Claude Code sends `textDocument/didOpen` when a document enters memory.
- From this point forward **the client is the authoritative source for document content**, not
  the filesystem. The server must not read the document via its URI — it must rely on change
  notifications.
- Source: LSP 3.18 §3.16.1 — "From now on, the truth about the contents of the document is no
  longer on the file system but kept by the tool in memory."

### didChange

- Sent whenever document content mutates.
- **Two sync modes** (negotiated at initialization via `TextDocumentSyncKind`):
  - `Full` (kind=1) — sends the complete document text on every change.
  - `Incremental` (kind=2) — sends only the changed ranges (`TextDocumentContentChangePartial`).
- Version field in `DidChangeTextDocumentParams` represents the version **after** all content
  changes have been applied, not before. Servers validate pre/post state using this invariant.
  Source: LSP 3.18 spec + GitHub issue #1706.

### didClose

- Sent when the document leaves memory. The server may release resources and revert to the
  filesystem as source of truth.

### publishDiagnostics

- A **server-to-client** push notification (`textDocument/publishDiagnostics`).
- Unidirectional: the server fires this after analysis, no request-response cycle.
- Only sent after server initialization completes.
- LSP 3.16+ also introduces pull-based diagnostics (`textDocument/diagnostic`) as an alternative,
  but push-based `publishDiagnostics` remains the primary mechanism.

---

## 2. Change Notification Mechanics

### Ordering guarantee

Notifications **must be processed in order** (synchronously), while requests may be concurrent.
This is critical: `didChange` notifications alter document state and affect the semantics of
subsequent requests.

### Debouncing — per-buffer segmentation

Clients commonly debounce `didChange` to avoid flooding servers with high-frequency edits. The
key correctness requirement:

> **Debounce must segment pending changes by URI/buffer.**

If pending changes are stored in a flat list without URI keys, edits from multiple buffers can
be mixed into a single notification, causing syntax errors and broken undo on the server side.

Reference implementation: VS Code uses a `Delayer` with ~250ms debounce; Neovim fixed this in
PR #16431 (Nov 2021) by changing `pending_changes: list` → `pending_changes: table (uri → list)`.

### Incremental sync details

- `TextDocumentContentChangePartial`: `{range, text}` — sends only the mutated range.
- `TextDocumentContentChangeWholeDocument`: `{text}` (no range field) — full document replacement.
- Clients can send multiple change events in a single `didChange` notification (array of changes).

---

## 3. LSIF Integration

### What LSIF is

LSIF (Language Server Index Format) is a pre-computed offline index for IDE navigation features:
go-to-definition, find-all-references, hover information. Stored as line-delimited JSON — each
line is a vertex or edge in the index graph.

- GitHub indexes 200+ billion lines with LSIF for cross-repo navigation.
- Sourcegraph's precise code search is built on LSIF.

### What LSIF does NOT do

**LSIF has no incremental update mechanism.** Document mutations invalidate the index entirely;
affected code must be re-indexed from scratch. LSIF explicitly excludes "requests used when
mutating a document" because the pre-computed data would be stale.

This is a design limitation, not a bug. SCIP (LSIF's successor, developed by Sourcegraph) was
created specifically to address this and other LSIF limitations including incremental support.

### LSIF vs. live LSP

| Property | Live LSP server | LSIF |
|---|---|---|
| Real-time diagnostics | Yes (`publishDiagnostics`) | No |
| Incremental sync | Yes (`didChange`) | No — full re-index |
| go-to-definition | Yes (request/response) | Yes (pre-computed) |
| find-all-references | Yes | Yes (pre-computed) |
| Requires running server | Yes | No |
| Suitable for CI/PR review | Limited | Yes |

---

## 4. Open Questions (Not Resolved by Available Sources)

The research found no primary sources specific to Claude Code's own LSP client implementation.
The following are unanswered:

1. **Does Claude Code implement its own debounce?** If yes, does it segment by URI? Or does it
   rely on the transport layer / editor host to batch?
2. **Concurrent edit version validation**: How does Claude Code handle incremental `didChange`
   from out-of-order or concurrent agent edits? Does it validate that the server's current version
   matches the pre-change version before applying?
3. **LSIF reconciliation**: Does Claude Code consume LSIF indexes? If so, how does it reconcile
   static pre-computed navigation with live LSP diagnostics?
4. **Agent-driven edits**: How do multi-file automated transformations (e.g., refactoring across
   N files) interact with incremental sync — are edits bundled into one `didChange` per file, or
   do individual agent tool calls each fire separate notifications?

---

## 5. Implications for lsp-max

The compositor's `Notify` routing (fan all servers on `didOpen`/`didChange`/`didClose`) is
consistent with the protocol's ordering guarantee — notifications must reach all servers in
order. Key considerations:

- **Version tracking in multi-server fan-out**: When `didChange` is fanned to N servers, each
  server independently tracks document versions. If servers respond at different rates,
  `publishDiagnostics` from server A may reference version V while server B is still at V-1.
  The compositor's `DiagnosticBuffer` last-write-wins per `(server_id, uri)` partially mitigates
  this but does not version-gate diagnostics.
- **LSIF as a compositor input**: The compositor could consume LSIF dumps as a static
  `DiagnosticsOnly` tier — providing navigation data without a running server. Since LSIF
  excludes mutation-related requests, it would only serve `definition`/`references`/`hover`,
  consistent with the `PrimaryOnly` routing for those methods.
- **Debounce at the compositor layer**: The `FlushCoordinator`'s adaptive quorum debounce
  (`clamp(2×spread, 1ms, 30ms)`) is a flush-side debounce for diagnostics aggregation. This is
  distinct from a notification-side debounce — the compositor should not add latency to
  `didChange` fan-out, only to the diagnostic flush.

---

## Sources

| URL | Angle | Quality |
|---|---|---|
| https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ | Notification protocol | Primary |
| https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/ | Notification protocol | Primary |
| https://microsoft.github.io/language-server-protocol/specifications/lsif/0.5.0/specification/ | LSIF format | Primary |
| https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/ | LSIF format | Primary |
| https://github.com/microsoft/language-server-protocol/issues/1706 | Version semantics | Primary |
| https://github.com/neovim/neovim/pull/16431 | Per-buffer debounce fix | Primary |
| https://github.com/neovim/neovim/issues/16424 | Per-buffer debounce bug | Primary |
| https://github.com/neovim/neovim/pull/16908 | Debounce algorithms | Primary |
| https://code.visualstudio.com/api/language-extensions/language-server-extension-guide | Client implementation | Primary |
| https://github.com/microsoft/vscode-languageserver-node | VSCode LSP client | Primary |
| https://github.com/ebkalderon/tower-lsp | Rust async LSP | Primary |
| https://lib.rs/crates/async-lsp | Rust async LSP | Primary |
| https://sourcegraph.com/blog/evolution-of-the-precise-code-intel-backend | LSIF→SCIP | Secondary |
