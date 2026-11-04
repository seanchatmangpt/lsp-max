# Lane BL9 — Python LSP client (branchless-lsp)

Standing: ALIVE (stub-server witness + 3 typed-error courts + mutation
flip, all run this session against the exact files below).

## Files written

- `python/lsp_client/__init__.py` — exports `analyze`, `analyze_text`,
  typed errors (`LspError`, `LspProtocolError`, `LspServerCrashed`,
  `LspTimeout`), `CLIENT_VERSION`.
- `python/lsp_client/branchless_client.py` — stdlib-only async client
  (asyncio + json + subprocess via `asyncio.create_subprocess_exec`).
  LSP Content-Length framing, initialize handshake (`initialized`),
  didOpen/didSave, `textDocument/publishDiagnostics` consumption,
  `max/conformanceVector` request, shutdown/exit, typed failures.

## API

`analyze(path, source=None, server_bin=None, timeout=30.0)` (and
`analyze_text(source, server_bin)`) -> dict:

- `diagnostics`: list from the publishDiagnostics notification for the
  document URI
- `conformance`: parsed `max/conformanceVector` result, or None
- `receipt`: `conformance["receipt"]` when present, else None
- `uri`: canonical `file://` URI sent to the server

`server_bin` accepts a string (whitespace-split argv) or list; falls
back to env `BRANCHLESS_LSP_BIN`. No Process.sleep anywhere: all waits
are `asyncio.wait_for` with explicit deadlines against real observables
(queue items, response map, process exit).

## Protocol sequence (witnessed)

initialize -> (server: capabilities) -> initialized -> didOpen ->
publishDiagnostics (consumed) -> didSave -> max/conformanceVector ->
shutdown -> exit.

## Courts and exits (this session)

- `python3 /tmp/bl9/witness.py` -> exit 0:
  `WITNESS_PASS diagnostics=1 receipt=bl9-stub-receipt-0xA20C0FFEE5A7E000`
  (real subprocess stub at /tmp/bl9/stub_lsp_server.py, real frames;
  asserts canned diagnostic code E-STUB-001, conformance admitted=[3,7]
  refused=[11], receipt round-trip).
- `python3 /tmp/bl9/courts.py` -> exit 0:
  - `COURT_CRASH_PASS typed=LspServerCrashed` (server aborts mid-frame
    with a lying Content-Length 200; request path fails typed, not with
    a raw TimeoutError)
  - `COURT_TIMEOUT_PASS` (server reads requests, never answers;
    LspTimeout at the stated 2.0s deadline)
  - `COURT_MALFORMED_PASS detail=bad Content-Length: 'notanumber'`

## Anti-vacuity mutation (sandbox copy, sha256 restore)

Mutated /tmp/bl9/stub_lsp_server.py diagnostic code E-STUB-001 ->
E-MUTATED: witness FAILED (exit 1) -> flip proven. Restored
sha256-exact (`sha_match=yes`); post-restore witness exit 0.

## Defects found and fixed by courts (same session)

1. Pump crash propagation: a server dying mid-frame left the reader
   task dead with an unhandled IncompleteReadError and the request path
   hanging until a generic TimeoutError. Fixed: pump poisons the
   notification queue with the typed error; `_request` and
   `_next_notification` re-raise it.
2. Poison swallowing: `_request` consumed the poison queue item and
   discarded it, still timing out. Fixed by checking `_crash` in the
   request wait loop.
3. Raw TimeoutError leaks translated to `LspTimeout` in `_read_frame`
   and `_request`.
4. Pump indentation corruption from a partial patch left the routing
   body dead (dedented outside the `while True:`); caught by the
   witness regressing to LspTimeout; re-indentation restored routing.

## Real binary

[SKIP: branchless-lsp binary not built yet] — searched
`/Users/sac/lsp-max/target`, PATH (`which branchless-lsp`), and the
tree; no binary this session. The client takes `server_bin`, so the
run is one command once BL1 lands:
`python3 /tmp/bl9/witness.py` with `BRANCHLESS_LSP_BIN=<real-bin>`.

## Divergences

- The stub server lives in /tmp/bl9/ (witness fixture), not in the
  owned file set; the lane contract named only the two python files
  plus this receipt.
- `lsp-max`'s `max/conformanceVector` result carries
  admitted/refused/unknown law-axis sets; the client passes the vector
  through verbatim (no collapsing of unknown into admitted/refused).
