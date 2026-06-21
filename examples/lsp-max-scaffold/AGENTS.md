# lsp-max-scaffold — Agent Constitution

This scaffold demonstrates the five-layer lsp-max architecture for new LSP
projects. Read this file before modifying any source file.

## What this is NOT

This is **not** a hexagonal-architecture scaffold:

| Hexagonal concept | lsp-max replacement |
|---|---|
| Domain entities (Item, Id<T>) | Law axes (Protocol, Receipt, Gate, Ontology) |
| Ports/adapters (inbound/outbound) | LSP surface (read-only) + max/* methods |
| Application services | Noun/verb service tier |
| Infrastructure (SQL, HTTP) | Receipt chain + ANDON gate |
| Error hierarchy (CoreError) | ConformanceVector (tri-state) |
| SemVer | CalVer (YY.M.D) |
| Stable toolchain | `nightly-2026-04-15` |

## The Five-Layer Model

```
Layer 1 — Actuation Grammar     src/nouns/         clap-noun-verb CLI
Layer 2 — Local LSP State       src/server.rs      LanguageServer impl
Layer 3 — Law-State Runtime     src/law.rs         ConformanceVector, AxisState
          Diagnostics           src/diagnostics.rs  ScaffoldDiagnostic, codes
Layers 4/5 — (extend here)     lsp-max-runtime     AutonomicMesh, mesh routing
```

## Bounded Status Vocabulary

**Allowed:** `ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`,
`PARTIAL`, `OPEN`

**Forbidden (victory language):** `done`, `solved`, `guaranteed`, `complete`,
`supported`, `fully`, `all clean`

Violations trigger `ANTI-LLM-CHEAT-*` diagnostics from the canary server.

## Law #1 — UNKNOWN Must Not Collapse

`ScaffoldConformanceVector` has three disjoint sets: `admitted`, `refused`,
`unknown`. An axis begins in `unknown`. It may only move to `admitted` via
`admit_axis()` — and only after all three receipt-chain preconditions are met:

1. A receipt exists in `receipts/<method>.json` with a valid BLAKE3 digest
2. A transcript exists in `transcripts/<method>.jsonl`
3. A negative-control exists in `fixtures/negative_controls/<method>.rs`

Calling `admit_axis()` before these artifacts exist is a law violation.
The negative control in `fixtures/negative_controls/unknown_collapse.rs`
demonstrates the violation pattern.

## Law #2 — LSP Surface is Read-Only

`ScaffoldServer` (Layer 2) emits diagnostics, hovers, and code actions. It
never writes files, never runs processes, never mutates workspace state. The
ANDON gate check in `did_open`/`did_change`/`did_save` enforces this boundary.

## Law #3 — ANDON Gate

Before any shell action, the gate file must be clear. The `gate` noun reads the
workspace-specific gate file (`/tmp/lsp-max-gate-<hash>`) and returns exit 1
if blocked. The `server.rs` gate check runs on every document event.

Gate file path formula (must match `lsp-max-compositor/src/gate_file.rs`):
```
XDG_RUNTIME_DIR/lsp-max-gate-<fnv1a(workspace_path)>
```

## Law #4 — Receipt Chains

Every method claim requires a receipt. A receipt has:
- `"boundary": "-----BEGIN RECEIPT-----"`
- `"checkpoint": "-----END RECEIPT-----"`
- `"digest_algorithm": "BLAKE3"`
- `"digest": "<hex of transcript>"` (not a placeholder)
- `"status": "CANDIDATE"` (promoted to `"ADMITTED"` after `admit promote`)

Run `lsp-max-scaffold admit receipt --method <method>` to generate a skeleton.
Run `lsp-max-scaffold admit check --method <method>` to verify preconditions.
Run `lsp-max-scaffold admit promote --method <method>` when all three exist.

## Law #5 — CalVer

Version is `version.workspace = true` — inherits the workspace CalVer
(`YY.M.D`). Never introduce a SemVer bump or a `version = "0.x"` in any crate.

## Noun/Verb Pattern

Each file in `src/nouns/` is a noun with three tiers:

```rust
// 1. Domain tier — serialisable value types
#[derive(Debug, Serialize)]
pub struct MyResult { pub status: &'static str }

// 2. Service tier — pure logic, no I/O side-effects
pub struct MyService;
impl MyService { pub fn new() -> Self { Self } }
impl Default for MyService { fn default() -> Self { Self::new() } }

// 3. Verb tier — #[verb] entry points
#[verb("do-thing")]
pub fn do_thing() -> clap_noun_verb::Result<MyResult> {
    Ok(MyService::new().do_thing())
}
```

## CI

The workspace CI (`ci.yml`) covers this crate. It uses `nightly-2026-04-15`
and checks out all three sibling repos before running `cargo fmt -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
`cargo test --workspace`.

## Anti-Patterns (Enforced)

1. `tower-lsp` / `tower_lsp` in code → ANTI-LLM-CHEAT-* (blocked by canary)
2. `Unknown` coerced to `Admitted` without receipts → SCAFFOLD-AXIS-001
3. Victory language in messages/statuses → ANTI-LLM-CHEAT-VICTORY-*
4. Exhaustive struct literals for LSP types → breaks when lsp-types-max adds fields
5. `cargo test` output as admission evidence → ANTI-LLM-CHEAT-RECEIPT-INVALID
