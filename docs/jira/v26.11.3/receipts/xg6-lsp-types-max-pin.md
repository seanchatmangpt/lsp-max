# xg6 — lsp-types-max pin decision (G3, lane LM-1)

Date: 2026-11-03. Lane: IMPLEMENTATION (LM-1), Wave 2. Repo: /Users/sac/lsp-max (no git run).

## Decision

**Document, do NOT bump.** Workspace stays at `lsp-types-max = "26.6.24"`.

## Pin law

- crates.io ground truth (re-read at use time, 2026-11-03):
  `GET https://crates.io/api/v1/crates/lsp-types-max` → `max_version: 26.6.24`,
  published versions `[26.6.24, 26.6.8]`. **26.11.3 is unpublished upstream.**
- Bumping the workspace to `26.11.3` would break every external consumer
  resolve (`cargo` cannot resolve a version that does not exist on crates.io)
  while `lsp-max` itself remains unpublished at 26.11.3. The workspace-internal
  CalVer bump (all 7 lsp-max crates at 26.11.3) is internal path-dep wiring and
  does not require the external pin to move.
- The pin moves only when `lsp-types-max` actually publishes a version > 26.6.24
  AND lsp-max consumes it via crates.io. Until then, "bumped all crates" claims
  stop at the internal workspace — the external pin is deliberately NOT bumped.

## The 4 manifest sites (verified on disk, 2026-11-03)

| site | line | pin |
|---|---|---|
| `/Users/sac/lsp-max/Cargo.toml` | 47 | `lsp-types-max = { version = "26.6.24", features = ["proposed"] }` (workspace deps) |
| `/Users/sac/lsp-max/Cargo.toml` | 154 | `lsp-types-max = { version = "26.6.24", features = ["proposed"] }` (root crate deps) |
| `/Users/sac/lsp-max/lsp-max-protocol/Cargo.toml` | 22 | `lsp-types-max = { version = "26.6.24" }` |
| `/Users/sac/lsp-max/crates/lsp-max-lsif/Cargo.toml` | 19 | `lsp-types-max = { version = "26.6.24" }` |
| `/Users/sac/lsp-max/crates/playground/Cargo.toml` | 39 | `lsp-types-max = { version = "26.6.24", features = ["proposed"] }` |

(Note: the contract named 4 sites; disk shows 5 lines across 4 manifests —
root Cargo.toml carries the pin twice. Playground's feature list includes
`"proposed"` — transcribed verbatim from disk above.)

## Adjacent stale externals (so "bumped all crates" stops overreaching)

| dep | pin | site |
|---|---|---|
| `affidavit` | `26.6.22` | root Cargo.toml:68 |
| `wasm4pm` | `26.6.11` | crates/lsp-max-compositor/Cargo.toml:51 (optional), crates/playground/Cargo.toml:46 |
| `wasm4pm-compat` | `26.6.29` | root Cargo.toml:153 (non-workspace copy), workspace deps |
| `wasm4pm-cognition` | `26.6.10` | root Cargo.toml:69 |
| `clap-noun-verb` | `26.6.2` | crates/lsp-max-cli/Cargo.toml:54 |

None of these moved in the 26.11.3 bump; the bump moved only the 7 internal
lsp-max crates and their intra-workspace path-dep pins.

## Standing

- ADMITTED: pin decision + manifest-site inventory re-read from disk this
  session (grep receipts in session transcript).
- UNVERIFIED beyond crates.io: whether upstream lsp-types-max will publish
  26.11.3 is UNKNOWN; this receipt is as-of-date 2026-11-03.
