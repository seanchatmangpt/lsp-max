# Lane F6 — BL9↔BL10 adapter (3-way break closure)

Status: PARTIAL_ALIVE — adapter surface ALIVE, refusal witness
BLOCKED on F5 server landing (see Falsifier). No Process.sleep:
polling used a bounded 20-minute deadline loop on the observable
(server probe), with 45s poll steps.

## Diff (owned files only)

### python/lsp_client/branchless_client.py

- Added sync batch adapter `diagnostics_for_files(paths, server_bin=None, timeout=...)`:
  one client session for ALL files (never per-file servers), didOpen+didSave
  each path, waits per-URI publishDiagnostics, aggregates
  `{"<path>": [diags], ..., "n_refused": n}`; `n_refused` = ERROR-severity
  count across files. Raises typed LspError subclasses on transport failure.
- None-guard narrowing (pyright ✘s at :73-127/:140/:148/:212):
  new `_stdin()`/`_stdout()`/`_returncode()` narrowed accessors raising
  `LspServerCrashed` when the process is not live; stop() narrows
  `proc = self._proc; if proc is None: raise LspServerCrashed(None)`.
  All `self._proc.stdin/stdout` uses route through the accessors.
- pyright: 0 errors, 0 warnings (was 5 errors + the known ✘s).

### scripts/gate-branchless-lsp.py

- `load_client()` now accepts the `python/lsp_client/` package dir
  (checked with `is_dir()`) OR legacy `lsp_client.py`; skip message names
  both. Break 1 fixed.
- Submodule fallback: since the sync API lives in
  `lsp_client.branchless_client` and `__init__.py` (not mine) does not
  re-export it, the gate imports `lsp_client.branchless_client` directly
  when the package attr is absent. Break 2's import side fixed.
- Break 3 (`result.get("n_refused")`) needed no gate change — the
  wrapper emits it; gate logic verified end-to-end.
- Empty staged set: exit 0 + proof written (pre-existing behavior,
  unchanged).

## Witness commands (this session)

Sandbox: `/Users/sac/scratch-f6/repo` (git init; topology guard refuses
/tmp; home-dir scratch allowed). Env:
`BRANCHLESS_LSP_BIN=/Users/sac/lsp-max/target/debug/branchless-lsp`,
`BRANCHLESS_LSP_GATE_CLIENT_DIR=/Users/sac/lsp-max/python`.

1. Clean scenario: staged `clean.rs` → gate exit **0**, proof
   `.cgap/receipts/branchless/admission-20261102T192717Z.json`,
   `admission: ADMITTED`, `n_refused: 0`. First real pass witness.
2. Sync batch probe: `diagnostics_for_files([branch.rs, clean.rs])` →
   keys = both paths + `n_refused`, single server session, exit 0.
3. py_compile + pyright: both exit 0, pyright 0 errors.
4. Server probe (manual JSON-RPC): initialize → publishDiagnostics
   `diagnostics: []` for branchy file.

## Falsifier (flip court) — UNVERIFIED/BLOCKED, not faked

The refusal flip requires the server to publish an ERROR diagnostic on a
branchy staged file. The current `branchless-lsp` binary publishes
`diagnostics: []` deterministically: `BranchlessLspServer::new` builds an
EMPTY rule-pack set (`crates/branchless-lsp/src/lib.rs:57`) and
`did_save` is a documented no-op stub ("Tier 2 court runs off the saved
file in a later lane", `crates/branchless-lsp/src/branchless-lsp/src/
lib.rs:123-126`); Tier 1 ast_linter is advisory by design. None of these
are F6-owned files. A court that cannot flip without faking is reported,
not faked: flip scenario run against the real binary → exit 0 (no flip)
— recorded as a real result, not a pass.

Bounded poll (20 min deadline, 45s poll steps, /tmp/f6-poll.log,
2026-11-02 11:34:47-11:54:04 local) ran to deadline exhaustion: F5's
server landing (Tier 2 wiring in did_save and/or max/conformanceVector)
did NOT arrive within the window — 14 consecutive probes returned
`diagnostics: 0, conformance: False`. Refusal flip remains BLOCKED on
F5; the flip court against the real binary exits 0 (no refusal), which
is reported as the real result rather than faked. F6 standing:
PARTIAL_ALIVE — pass witness real (exit 0, ADMITTED proof), refusal
witness UNVERIFIED pending F5.
