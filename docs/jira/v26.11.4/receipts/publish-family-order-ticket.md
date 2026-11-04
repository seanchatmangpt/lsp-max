# publish-family-order-ticket

## Status

OPEN — operator decision required (crates.io publishing of the lsp-max family).

## Context

D95 RCA: `cargo publish -p lsp-max --dry-run` fails on version resolution, not on
packaging. `lsp-max` (and `lsp-max-lsif`) declare path+version deps
(`lsp-max-ast = { version = "26.11.3", path = ... }`), and `cargo publish` resolves
`^26.11.3` against the crates.io index even in `--dry-run`. Registry state:
`lsp-max-ast` latest is `26.7.3`.

## Measured (2026-11-04, lane FIX, nightly-2026-04-15)

- `cargo +nightly-2026-04-15 publish -p lsp-max-macros --dry-run` — packaging OK (dry-run aborts upload).
- `... -p lsp-max-protocol --dry-run` — packaging OK.
- `... -p lsp-max-ast --dry-run` — packaging OK.
- `... -p lsp-max-lsif --dry-run` — FAILS: `lsp-max-ast ^26.11.3` not on registry.
- `... -p lsp-max --dry-run` — FAILS: internal deps at `^26.11.3` absent from registry.

## Smallest correct path to green

1. Publish in dependency order, with the same pinned toolchain:
   `lsp-max-macros → lsp-max-protocol → lsp-max-ast → lsp-max-lsif → lsp-max`.
   Each step gates the next; the parent dry-run goes green mechanically once
   `26.11.3` of the four deps exist on the index.
2. Internal-only crates NOT in the parent's closure already carry `publish = false`
   (`branchless-lsp`, `ra-max`, `playground`).
   Marking closure members `publish = false` is NOT lawful: cargo refuses to publish
   a crate whose dependency is registry-excluded.

## Lawful boundary

Real `cargo publish` is outside automated execution authority
(`README.md` line 91), so the final step is an operator actuation, not a lane actuation.
