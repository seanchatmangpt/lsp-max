# xg6 — gate-path byte/row-equality court (G6, lane LM-1)

Date: 2026-11-03. Lane: IMPLEMENTATION (LM-1), Wave 2. Repo: /Users/sac/lsp-max (no git run).

## Subject

- Authoritative formula: `/Users/sac/lsp-max/src/primitives/gate_path.rs` —
  sole lawful derivation of the Λ_CD gate file path (FNV-1a workspace hash →
  `$XDG_RUNTIME_DIR|/tmp` → `lsp-max-gate-{hash:016x}` [+ `-agent-{id}` when
  `LSP_MAX_AGENT_ID` is set]).
- Hand-maintained copies now courted:
  - `/Users/sac/lsp-max/examples/lsp-max-scaffold/src/nouns/gate.rs`
  - `/Users/sac/lsp-max/examples/lsp-max-scaffold/src/server.rs`
- New court: `/Users/sac/lsp-max/tests/test_gate_path_byte_equal_court.rs`

## Fix landed before the court could pass

Both scaffold copies had ALREADY DIVERGED from the authoritative formula: they
lacked the `LSP_MAX_AGENT_ID` branch (the drift class the court exists to kill,
witnessed pre-landing). Both copies were updated to the authoritative
derivation; the court's positive control now refuses an all-copies-agree-by-
dropping-the-branch regression.

## Court shape (4 tests)

- row-extraction of the `gate_file_path` body from all three include_str!ed
  files; normalization limited to the lawful spellings (`pub ` fn modifier,
  `std::path::` qualification, comment/blank lines)
- row-by-row equality authoritative vs each scaffold copy
- positive control: all three must still carry the `LSP_MAX_AGENT_ID` and
  `XDG_RUNTIME_DIR` branches (all-copies-drift non-vacuity)
- structural assert on the authoritative rows (file-name formats + fnv1a arg)

## Gate (executed this session)

`cargo test --test test_gate_path_byte_equal_court` → 4 passed; 0 failed (exit 0).

## Standing

- ADMITTED: court green over the real three files this session.
- OPEN: compositor-internal copy `crates/lsp-max-compositor/src/gate_file.rs`
  (`GateFile::for_workspace`) is not yet in the court's include_str! set —
  contract scoped the court to the scaffold copies; adding gate_file.rs is the
  natural next widening.
