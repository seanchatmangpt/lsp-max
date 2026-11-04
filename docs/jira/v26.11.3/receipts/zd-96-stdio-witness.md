# ZD-96 — stdio framing witness: portable binary resolution (ZD-47 pattern)

## Subject

- Repo: `/Users/sac/lsp-max` @ bd97e5c (ZD-54)
- Files: `tests/test_stdio_framing_witness.rs` (only file modified)
- Class: ZD-34 environmental finding — witness test hardcoded
  `/Users/sac/lsp-max/target/debug/examples/stdio`, absent in isolated target
  dirs; test BLOCKed at the assert.

## Diff

`stdio_example_bin()` rewritten on the ZD-47 pattern
(`crates/lsp-max-compositor/tests/diagnostic_relay_probe.rs::resolve_child_bin`):

1. Env override `LSP_MAX_STDIO_BIN` (must exist when set).
2. Fallback search over roots (manifest dir, `$CARGO_TARGET_DIR`, manifest
   parent) × profiles (`debug`, `release`) for `target/<profile>/examples/stdio`.
3. `None` → caller prints a loud typed `[SKIP]` naming the build command
   (`cargo build --example stdio` or `LSP_MAX_STDIO_BIN`) and returns ok.
   Never a silent pass, never a hardcoded path.

Note: `CARGO_BIN_EXE_<name>` is bins-only — cargo does not provide it for
`[[example]]` targets, hence the manual resolution.

## Verification ladder

| # | Command | Exit | Outcome |
|---|---|---|---|
| 1 | `cargo build --example stdio` | 0 | binary at `target/debug/examples/stdio` |
| 2 | `cargo test --test test_stdio_framing_witness` | 0 | `1 passed; 0 failed` |
| 3 | same, second run | 0 | `1 passed; 0 failed` (0.01s) |
| 4 | Shim court A: `mv` binary away, `cargo test` | 0 | test PASSED — court did NOT flip: cargo itself rebuilds the example during `cargo test`, un-hiding the binary |
| 5 | Shim court B: `mv` binary away, run compiled `target/debug/deps/test_stdio_framing_witness-*` directly, `--nocapture` | 0 | loud `[SKIP]` line witnessed verbatim |
| 6 | restore `mv` back | 0 | `sha256 ce7ce1d74e5c1616dd5b24039d7d382a5e11c2ea78ca345c970cfffbc19f102a` byte-exact |

## Findings

1. **Cargo-example rebuild defeats binary-hiding under `cargo test`.** Any
   skip-path court that hides `target/debug/examples/*` must run the compiled
   test binary from `target/debug/deps/` directly — cargo's own example
   compilation during `cargo test` un-hides the shim. (Court A is the
   documented non-vacuity trap.)
2. The skip is an early-return `ok` in libtest terms: a SKIP is not
   distinguishable from PASS in the `test result:` line alone — the `[SKIP]`
   line on stderr is the receipt. This matches the ZD-47 precedent.
3. The auto-build removed with the hardcoded path also removed the workspace
   `target/` assumption; `CARGO_TARGET_DIR` / isolated target-dir runs now
   resolve or skip loudly instead of BLOCKing.

## Standing

- Witness test with binary present: ALIVE (commands 1–3 above).
- Loud typed SKIP when absent: ALIVE (court B, `--nocapture`).
- Court A "cargo un-hides the example" behavior: observed, documented.
