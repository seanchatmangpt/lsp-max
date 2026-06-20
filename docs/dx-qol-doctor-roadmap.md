# DX / QoL / Doctor — Innovation Roadmap

**Status: PARTIAL.** A 5-agent fan-out explored the developer-experience,
quality-of-life, and doctor surfaces. The verifiable shell/Justfile wins are
applied below; the deeper Rust/CLI ideas are recorded as CANDIDATE moonshots
ranked by effort. North star (AGENTS.md): raise admitted velocity
`v_eff = dA_admitted/dt`. Primary clients are agents, CI, and release gates.

## Applied this iteration (verified locally)

| Surface | Recipe / artifact | Notes |
|---|---|---|
| Doctor | `just doctor` → `scripts/doctor.sh` | Full-spectrum read-only oracle: toolchain-pin match, per-sibling version + git-clean (gc006), workspace-resolve, ANDON gate (UNKNOWN if CLI unbuilt — never compiled, never collapsed), config completeness, target size. Bounded overall status. |
| Doctor | `just doctor-json` | Same oracle as one machine-readable object for agents/CI. |
| Inner loop | `just check [base]` + `scripts/changed-crates.sh` | fmt → clippy → test only on crates changed since `base`; fast-fail. Manifest/Justfile/toolchain change ⇒ `__ALL__` (no unsafe narrowing). A pre-filter, **not** the admission authority. |
| Inner loop | `just watch [base]` | Continuous loop via bacon/cargo-watch when present. |
| QoL | `just status` | One-glance dashboard across all 4 ecosystem repos (branch / dirty / ahead-behind / last commit). |
| QoL | `just qol-sync` (fixed) | Now includes `../lsp-types-max` (previously dropped) and reports divergence. |
| QoL | `just qol-deps` | Surfaces duplicate dependency versions (currently 37) as a drift signal. |
| Admission | `just verify` + `scripts/validate-receipt-chain.sh` | Walks receipt artifacts; bounded status per receipt; emits the validator CLAUDE.md references but which was **absent**. |

### Prerequisite bug fixed: `just` did not parse at all

`release-version-bump` contained a multi-line `git commit -m "…"` whose
continuation lines sat at column 0, which `just` reads as the end of the recipe —
so **every** `just` recipe failed to parse. It went unnoticed because CI invokes
`cargo` directly and `just` was not installed here. Converted to multiple `-m`
flags (same three-paragraph message). Without this, none of the recipes above —
new or old — would run.

## Verification (local runs — not signed receipt artifacts)

| Check | Result |
|---|---|
| `just --list` | parses; all recipes listed |
| `just doctor` | overall PARTIAL; toolchain ADMITTED (`nightly-2026-04-15`), 3 siblings ADMITTED+clean, resolve ADMITTED, gate UNKNOWN (not collapsed) |
| `just doctor-json` | one JSON object; `jq`-parseable; bounded `overall` |
| `just status` | 4 repos reported (lsp-max + 3 siblings) |
| `just verify` | exit 0; ~95 receipts ADMITTED (incl. one `status=REFUSED` well-formed refusal), 5 playground summaries UNKNOWN |
| receipt validator | discriminates: real → ADMITTED, bad-marker negative control → REFUSED, non-marker shape → UNKNOWN |
| `just check` | manifest/Justfile guard → `__ALL__` path taken |

## Moonshots (CANDIDATE — require a build to verify)

### Agent-native DX (flagship — DX for the primary clients)
- **`gate check --format=agent-context`** (touches `nouns/gate.rs`): on BLOCKED, emit the governing set as JSON instead of a dead 1-bit + an error lost to stderr. This is AGENTS.md RFC-1 (D_t PUSH). Effort: M.
  ```json
  {"andon_blocked":true,"status":"BLOCKED","since_seq":1487,
   "active_andon_codes":[{"code":"WASM4PM-…","uri":"…","severity":"ERROR"}],
   "governing_axes":{"refused":["Receipt"],"unknown":["Security"]},
   "available_repairs":[{"action_id":"emit-receipt","verb":"diagnostics repair-plan …"}],
   "compositor_active":true}
  ```
- **`lsp-max-cli doctor` noun** (new `nouns/doctor.rs`): the Rust counterpart of `scripts/doctor.sh`, composing gate ⊕ conformance ⊕ D_t ⊕ config-source into one bounded envelope (`--format=agent-context`). Replaces an agent's multi-call fan-out + prose parsing. Effort: M-L.
- **`config schema` / `config doctor`**: list known keys/types/defaults and the effective value + source (default/file/env). Effort: M.
- **`agent next`**: given the current failset, return the next admissible verb-invocations. Effort: L.

### Doctor
- **Receipt-emitting doctor** (`doctor --receipt`): write a BLAKE3-signed env-readiness artifact so "READY" becomes a receipt, not stdout. Effort: M.
- **`doctor --fix`**: run only the idempotent safe subset of each check's one-line fix, behind a gate check. Effort: M.

### Inner loop
- **Affected-graph engine**: `git diff` → `cargo metadata` reverse-dep closure, so the loop term goes from O(workspace) to O(touched). `scripts/changed-crates.sh` is the first step; the closure (dependents of changed crates) is the next. Effort: M.
- **Receipt-bearing inner loop**: `just check` emits a scoped receipt (scope set + per-crate verdict + negative-control note). Effort: M.

### QoL
- **`ecosystem-snapshot.receipt.json`**: `just status` emits a signed 4-repo situational receipt so release gates consume fleet state, not stdout. Effort: M.
- **Dep-drift gate**: ratchet the `qol-deps` duplicate count with a budget (ADMITTED/PARTIAL/BLOCKED). Effort: M.

### Admission
- **`receipt` noun as the engine**: one verb `receipt emit --claim … --cmd … --boundary …` that runs, digests, binds all fields, and writes the artifact in Rust (reusing `CryptographicReceipt`/`Keystore` ed25519), retiring the duplicated bash emitters. Effort: L.
- **`receipt walk`**: wrap `lsp_max_runtime::verify_receipt_chain` for per-link bounded status over ledger receipts. Effort: M.

## Risks / what stays UNKNOWN until built
- `--format`/flag mechanics of `clap-noun-verb` are UNKNOWN until a build; the agent-context envelopes assume an explicit `Option<String>` param.
- Two receipt schemas coexist (marker-style file receipts vs runtime `CryptographicReceipt`); unifying them is CANDIDATE, not assumed. `scripts/validate-receipt-chain.sh` validates only the marker-style shape and reports UNKNOWN for others.
- `just check` uses per-crate feature resolution, which differs from `--all-features --workspace`; the full gate (`dx-polish` / CI) must remain whole-workspace — `check` is a pre-filter, never the authority.
- Loop-time reduction is qualitative until measured.
