# Lane F5 — branchless-lsp workspace integration receipt

Repo: /Users/sac/lsp-max (canonical checkout, branch main). Date: 2026-11-02.

## Ownership
- `/Users/sac/lsp-max/Cargo.toml` (workspace members)
- `crates/branchless-lsp/Cargo.toml` (no changes needed — see below)
- `crates/branchless-lsp/src/main.rs` (task-3 scope: warning fix, see concurrency note)
- `docs/lane-f5-workspace-integration.md` (this receipt)

## Standing: ALIVE

## Commands / exits (this session)
- `cargo check -p branchless-lsp` → exit 0 (zero warnings after main.rs fix)
- `cargo check --workspace` → exit 0 (two runs, before and after main.rs edit)
- `cargo test -p branchless-lsp` → exit 0, 21 passed / 0 failed (final run)
- `cargo build -p branchless-lsp --release` → exit 0
- Binary witnessed: `/Users/sac/lsp-max/target/release/branchless-lsp`
  (Mach-O 64-bit arm64, 14,409,616 bytes, mtime Nov 2 11:29) — closes the
  gate's permanent-exit-3 (subject-missing) blocker.
- Live JSON-RPC over stdio: initialize → capabilities
  (textDocumentSync=1, diagnosticProvider identifier=branchless-lsp) +
  `max/conformanceVector` (id=2) → real ConformanceVector result
  (score=100.0, strict_mode=true, admitted/refused/unknown axes). The
  BL9 "0 server hits = dead request" finding is CLOSED: the request
  answers on the wire via the `lsp_max::LanguageServer` trait default
  (`src/language_server.rs:412` → `impls/snapshot.rs:96`), which
  BranchlessLspServer inherits by not overriding.

## Compile-fix findings (dispatch expectations vs. reality)
The dispatch anticipated tree-sitter 0.26-vs-0.23 clash, capstone/gimli
feature resolution, edition inheritance issues. NONE materialized:
- tree-sitter: crate uses workspace 0.26.5 (`tree-sitter.workspace = true`);
  the dev-dep `tree-sitter-rust = "0.23"` grammar version is API-compatible
  with the 0.26 runtime — resolved cleanly in Cargo.lock.
- capstone/gimli: resolved from crates.io without feature conflicts.
- edition: inherited from workspace (2021), no issue.

## Test failures encountered and resolution (Chicago, real collaborators)
`cargo test` initially FAILED 2/21 (disasm_court host-object courts):
- "court saw no functions (vacuous)" — Mach-O probe object exposed only
  `ltmp0` scaffolding; the real fn symbol was filtered.
- `COURT-SKIP capstone init failed: Must specify mode for arm64` —
  capstone 0.12 requires an explicit mode.
These were repaired mid-session by a concurrently running lane
(concurrent edits observed on disasm_court.rs three times; fixes landed:
`#[no_mangle] pub extern "C" fn f` probes, capstone `.mode(ArchMode::Arm)`,
ltmp symbol filter, symbol-gap extent derivation). Final state: 21/21 pass.
My own edits overlapped once: my `let _ =` fix collided with the other
lane's identical fix producing `let _ = let _ =` (compile error) — I
collapsed it to a single binding; disclosed here per law.

## Divergences from dispatch
1. Workspace members line was already present (uncommitted +1 in
   Cargo.toml) when I started — W2/W6's "never compiled / absent from
   members" state had already been superseded before my first command.
   I verified it via `cargo metadata` (branchless-lsp listed) and kept it.
2. main.rs was already wired (as W16 found); no handler wiring was
   missing — the `max/conformanceVector` handler lives upstream in the
   lsp-max crate and is inherited, so task-3 resolved to verification
   only (live stdio round-trip witness above) plus the unused_must_use
   warning fix.
