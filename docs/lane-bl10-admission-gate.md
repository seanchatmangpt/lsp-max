# Lane BL10 Receipt: Admission Gate + Pre-Commit Hook

Date: 2026-11-02. Repo: /Users/sac/lsp-max. Branch: main.

## Files written (owned)

- `scripts/gate-branchless-lsp.py` (140 lines) -- pre-commit admission gate
- `scripts/install-gate-hook.sh` -- idempotent pre-commit installer
- `docs/lane-bl10-admission-gate.md` -- this receipt

The shared lsp-max tree was NOT hook-installed and NO admission proofs were
written into it (contract: installer is the deliverable). All witnessing ran
in `/tmp/bl10-witness/{repo,repo2}` sandbox git repos.

## Gate contract

- Staged files: `git diff --cached --name-only --diff-filter=ACM`, filtered
  to `*.rs` (absolute paths).
- Client: imports `lsp_client` from `BRANCHLESS_LSP_GATE_CLIENT_DIR`
  (default `<repo>/python`, the BL9 deliverable location). Expected
  interface: `diagnostics_for_files(paths: list[str]) -> dict` with
  per-path diagnostic lists (`severity` in {"ERROR","error",1} = refusal)
  and optional top-level `n_refused` int.
- Refusal: any ERROR diagnostic or `n_refused > 0` -> prints
  `ANDON_REFUSAL` with file:line, exit 1. Pass: writes
  `.cgap/receipts/branchless/admission-<UTCts>.json`, exit 0.
- Unavailable server/client, import failure, interface mismatch, or client
  exception -> loud `[SKIP] ...` exit 3. Never a silent pass.
- Exit codes: 0 pass / 1 ANDON_REFUSAL / 2 installer gate-missing / 3 SKIP.

## Scenario matrix (all run this session, /tmp/bl10-witness)

| id | scenario | staged | client | exit | witness |
|----|----------|--------|--------|------|---------|
| A  | client dir missing | branchy.rs | none | 3 | `[SKIP] ... not found` |
| A2 | default path `<repo>/python` missing | clean.rs staged | none | 3 | `[SKIP] ... /python/lsp_client.py not found` |
| B  | clean client, clean file | clean.rs | clean fake | 0 | proof `admission-20261102T191419Z.json` written |
| C  | FLIP: ERROR diagnostic | clean.rs | error fake | 1 | `ANDON_REFUSAL ... clean.rs:1: branch-dense site` |
| D  | FLIP: n_refused=2, no diags | clean.rs | n_refused fake | 1 | `ANDON_REFUSAL ... (client N_refused=2 ...)` |
| E1 | installer fresh | -- | -- | 0 | hook written, chmod +x |
| E2 | installer idempotent re-run | -- | -- | 0 | `already installed ... no-op` |
| E3 | installer chains legacy hook | -- | -- | 0 | legacy hook preserved, gate appended |

Flip witnesses C and D use /tmp fake clients injected via
`BRANCHLESS_LSP_GATE_CLIENT_DIR` -- these prove the refusal path is not
vacuous independent of branchless-lsp server availability. Executing the
chained hook in repo2 directly printed `LEGACY_RAN` then the gate's
`[SKIP]` (exit 3), witnessing the chain end to end.

## Honest availability statement

The branchless-lsp LSP server binary is NOT built/installed on this machine
(`which branchless-lsp` -> not found) and the BL9 deliverable
`/Users/sac/lsp-max/python/lsp_client.py` does NOT exist at witness time.
Therefore the real-client pass scenario is UNVERIFIED, not simulated as
passing: it is the [SKIP] exit-3 path that is ALIVE (A/A2 above). When BL9
lands `python/lsp_client.py`, the same gate picks it up with no changes.

## Divergences

1. Dispatch assumed `python/lsp_client` exists (BL9); it does not at
   witness time. Gate degrades to loud [SKIP] per contract.
2. Gate defines a tolerant client interface (`diagnostics_for_files`) --
   BL9's actual API is unknown on disk; if it differs, the interface check
   fails loud [SKIP] naming the missing attribute, and the gate's expected
   interface should be reconciled with BL9's landed API.
3. A PreToolUse topology guard refused `git add`/`git commit` in the /tmp
   sandbox repos mid-witness; hook chaining was witnessed by executing the
   hook script directly (read-only) instead of a real commit.
