# CC-005 — publishDiagnostics Merge for Claude Code Consumer

**Status**: OPEN  
**Component**: `crates/lsp-max-compositor/src/merge.rs`, `crates/lsp-max-compositor/src/flush_coordinator.rs`  
**Depends on**: CC-004  
**Blocked by**: nothing

## Problem

Claude Code receives `textDocument/publishDiagnostics` notifications from LSP servers and uses
them to surface inline errors, drive agent repair loops, and feed the ANDON gate. When lsp-max
is the sole LSP endpoint, **all diagnostics from all child servers must be merged into a single
`publishDiagnostics` notification** per URI that Claude Code can consume.

Currently, `FlushCoordinator` accumulates diagnostics from children and merges them, but:

1. The merged notification is not sent back to the **upstream client** (Claude Code) — it is only
   logged or emitted as OCEL events.
2. Deduplication does not account for Claude Code's expected format: Claude Code may show duplicate
   markers if two child servers emit the same diagnostic (e.g., both rust-analyzer and
   anti-llm-cheat-lsp flag a missing import).
3. `REFUSED_BY_LAW` diagnostics (severity=Hint, code prefixed `ANTI-LLM-`) must survive dedup
   and appear in Claude Code's diagnostic view.
4. The flush debounce (adaptive quorum, 1–30ms) may be too aggressive — Claude Code's repair
   agent reacts to diagnostics, and a 30ms delay is acceptable, but a stale flush that omits a
   new child's diagnostics could cause a repair loop to terminate prematurely.

## Acceptance Criteria

- [ ] After every flush, the compositor sends `textDocument/publishDiagnostics` to Claude Code
      (the upstream client) with the merged diagnostic list for the flushed URI.
- [ ] Merge deduplication: two diagnostics are considered duplicates if they share `(range.start,
      range.end, code)`. When duplicate, keep the one with higher severity. `REFUSED_BY_LAW`
      codes always survive regardless of severity.
- [ ] Claude Code sees diagnostics tagged with their source server via `relatedInformation`:
      ```json
      { "location": {"uri": "lsp-max://server/rust-analyzer"}, "message": "source: rust-analyzer" }
      ```
- [ ] The flush debounce quorum fires at: all N children have deposited OR 30ms since first
      deposit (current behavior). New requirement: if a child deposits after the 30ms window
      (late), the compositor sends a follow-up `publishDiagnostics` within 5ms of the late
      deposit without waiting for another full quorum.
- [ ] ANDON diagnostic codes (`andon_code_prefixes` in `lsp-max.toml`) that are ERROR severity
      are forwarded with severity=Error so Claude Code's inline diagnostics show them as errors.
- [ ] `lsp-max-cli diagnostic list --uri <file>` shows the merged list that was last sent to
      Claude Code.

## Merge Logic

```rust
pub fn merge_for_upstream(
    contributions: &[(ServerId, Vec<Diagnostic>)],
    andon_prefixes: &[String],
) -> Vec<Diagnostic> {
    let mut seen: HashMap<(Range, Option<NumberOrString>), Diagnostic> = HashMap::new();

    for (server_id, diags) in contributions {
        for diag in diags {
            let key = (diag.range, diag.code.clone());
            let is_refused_by_law = andon_prefixes.iter()
                .any(|p| diag.code.as_ref().map_or(false, |c| c.to_str().starts_with(p)));

            match seen.entry(key) {
                Entry::Vacant(e) => { e.insert(diag_with_source(diag, server_id)); }
                Entry::Occupied(mut e) => {
                    if is_refused_by_law || diag.severity > e.get().severity {
                        e.insert(diag_with_source(diag, server_id));
                    }
                }
            }
        }
    }
    seen.into_values().collect()
}
```

## Files to Modify

- `crates/lsp-max-compositor/src/merge.rs` — implement `merge_for_upstream`
- `crates/lsp-max-compositor/src/flush_coordinator.rs` — send merged list upstream; add late-deposit follow-up
- `crates/lsp-max-compositor/src/server.rs` — upstream client handle for `publish_diagnostics`
- `crates/lsp-max-cli/src/nouns/diagnostics.rs` — extend `list` verb with `--uri` filter

## Test Plan

- [ ] Two children emit identical diagnostic at same range/code → merged list has one entry
- [ ] Two children emit same range, different codes → both appear in merged list
- [ ] ANTI-LLM- code emitted at Hint → survives dedup even when rust-analyzer emits same range at Warning
- [ ] Late deposit (child N arrives after 30ms) → follow-up notification sent within 5ms
- [ ] Claude Code (mock) receives `publishDiagnostics` after every flush, not just logged
- [ ] `lsp-max-cli diagnostic list --uri file:///foo.rs` returns merged list
