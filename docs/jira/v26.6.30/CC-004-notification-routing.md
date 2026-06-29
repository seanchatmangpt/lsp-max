# CC-004 — Notification Routing: didOpen/didChange/didClose Fan-Out

**Status**: OPEN  
**Component**: `crates/lsp-max-compositor/src/routing.rs`, `crates/lsp-max-compositor/src/server.rs`  
**Depends on**: CC-003  
**Blocked by**: nothing

## Problem

The compositor must correctly fan out the three document lifecycle notifications from Claude Code
to all registered child servers. The current routing table is method-keyed but has no special
handling for the ordering guarantee LSP imposes on notifications, and no per-URI serialization
of `didChange` events.

## LSP Constraints (from research)

1. **Notifications must be processed in order.** Requests may be concurrent; notifications are
   sequential and stateful. A `didChange` at version N must reach each child before any request
   that references version N.

2. **Client is document authority after `didOpen`.** Child servers must not re-read the
   filesystem once the compositor has forwarded `didOpen`. The compositor must forward `didOpen`
   to all children before forwarding any `didChange`.

3. **Per-URI serialization.** Fan-out of `didChange` must be serialized *per URI* across children.
   Changes to file A must not be interleaved with changes to file B in the same child's notification
   queue (though different URIs may be sent concurrently to different children).

4. **Version field is post-change.** The compositor must forward the version field from Claude Code
   unchanged. Never re-sequence or increment versions.

## Current Routing Table

```
textDocument/didOpen   → Notify (fan all, no response)
textDocument/didChange → Notify (fan all, no response)
textDocument/didClose  → Notify (fan all, no response)
```

This is correct at the method level. The gaps are:

- No ordering enforcement — fan-out is fire-and-forget with no channel serialization per URI
- No version validation — the compositor could theoretically forward `didChange` before `didOpen`
  completes if child startup is slow
- No state tracking — the compositor does not know which URIs are currently open in which children

## Acceptance Criteria

- [ ] The compositor maintains a `DocumentState` map: `uri → { version: u32, open_in: Set<ServerId> }`.
- [ ] `didOpen` fan-out: dispatched to all children concurrently, but the compositor does not
      forward `didChange` for a URI until all children have confirmed `didOpen` processing
      (i.e., fan-out completes before the URI enters the change queue). Use a one-shot channel per
      URI to signal completion.
- [ ] `didChange` fan-out: per-URI serialized via a `tokio::sync::Mutex`-keyed channel or a
      dedicated per-URI task. Changes to the same URI are sent to each child in order.
- [ ] `didClose` fan-out: sent to all children that have the URI in `open_in`. Removes the URI
      from `DocumentState`.
- [ ] The compositor validates that `didChange` version > last seen version for the URI. If not,
      emit a `COMPOSITOR-VERSION-REGRESSION` diagnostic (not ANDON, severity Warning).
- [ ] `DocumentState` is accessible via `lsp-max-cli snapshot` output.

## Per-URI Serialization Design

```rust
// One send-half per open URI, per child server
type UriKey = (ServerId, DocumentUri);
struct FanoutCoordinator {
    // channel per (server_id, uri) — serializes sends to that child for that file
    uri_channels: DashMap<UriKey, mpsc::Sender<DidChangeParams>>,
    // set of open URIs per server — gates didChange until didOpen confirmed
    open_set: DashMap<UriKey, ()>,
    doc_versions: DashMap<DocumentUri, u32>,
}
```

## Files to Modify

- `crates/lsp-max-compositor/src/routing.rs` — add `Notify` ordering metadata
- `crates/lsp-max-compositor/src/server.rs` — implement `FanoutCoordinator`
- `crates/lsp-max-compositor/src/flush_coordinator.rs` — integrate `DocumentState` read

## Test Plan

- [ ] Send `didOpen` then `didChange` to compositor with two children; verify each child receives
      them in order with correct version field
- [ ] Send `didChange` before `didOpen` completes (mock slow child); verify `didChange` is held
      until `didOpen` fan-out finishes
- [ ] Send `didChange` for URI A and URI B concurrently; verify no interleaving per child
- [ ] Send `didChange` with version regression; verify `COMPOSITOR-VERSION-REGRESSION` warning
      diagnostic, no ANDON
- [ ] `lsp-max-cli snapshot` shows open URIs and their current versions
