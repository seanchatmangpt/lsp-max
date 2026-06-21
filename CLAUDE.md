# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A fork of `tower-lsp` (Tower-based LSP framework) upgraded into **lsp-max**: a "law-state runtime projected through LSP." It maximizes LSP 3.18 capability coverage and adds a custom `max/*` protocol surface (snapshots, conformance vectors, receipts, repair plans, gates). Primary clients are agents, CI, and release gates — the editor is just one client.

**Read `AGENTS.md` before substantive work. It is the project constitution and its laws are enforced by tooling** (the `crates/anti-llm-cheat-lsp` server detects violations). The most load-bearing laws:

- **Never reference plain `tower-lsp`/`tower_lsp`** in code, manifests, tests, or docs — outside explicit negative-control fixtures. Everything is `lsp-max`.
- **No victory language** in code, comments, reports, or commit messages ("done", "all clean", "fully admitted", "solved", "guaranteed"). Use bounded statuses only: `ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`, `PARTIAL`, `OPEN`, etc.
- **Test stdout / log messages are not receipts.** Claims of admission require receipt artifacts (path, digest, boundary, checkpoint, negative-control result).
- The CLI component is named exactly **`clap-noun-verb`** — never invent a "CLAP" authority.
- The LSP surface is **read-only**: it emits diagnostics/hovers/intents but never mutates files directly.
- `ConformanceVector` carries `admitted`/`refused`/`unknown` law-axis sets; **Unknown must never collapse into Admitted or Refused**.

## Versioning

Workspace version uses **CalVer (YY.M.D)**, not SemVer. The current version is `26.6.21`. Version-law violations are a diagnostic family (`ANTI-LLM-VERSION-*`).

## Sibling repo dependencies (build prerequisite)

The workspace does not build standalone. It requires sibling checkouts at:

- `../lsp-types-max` — path dependency, the LSP type authority (with `proposed` 3.18 features)
- `../wasm4pm-compat` — `[patch.crates-io]`; the **sole baseline type authority** for process-mining types
- `../wasm4pm` — `[patch.crates-io]`; the execution engine

Architectural mandate (enforced by `just dx-verify`, which greps the sibling repos): no intermediary type crates (`wasm4pm_types`, `ocel_core` are forbidden), and the words `legacy`/`deprecated`/`shim`/`facade`/`backward compatibility` must not appear anywhere in `../wasm4pm-compat` or `../wasm4pm` — including in comments you write.

## Commands

Build orchestration is via `just` (run `just` alone to list recipes):

```sh
just test               # cargo test --workspace
just test-e2e           # cargo test --test e2e
just test-pre-publish   # dx-verify + dx-polish + tests including --include-ignored
just dx-polish          # cargo fmt --all + cargo clippy --workspace --all-targets --all-features -- -D warnings
just dx-verify          # architectural boundary scan across sibling repos
```

Single test / single crate:

```sh
cargo test -p <crate-name> <test_name>
cargo test -p lsp-max-cli nouns::conformance   # run all tests in one noun module
cargo test --test test_lsp318_capabilities     # one root integration-test file in tests/
cargo test -p anti-llm-cheat-lsp --test dogfood     # example-crate integration test
```

Clippy with `-D warnings` is the bar; run `just dx-polish` before considering a change complete.

**`scripts/` directory**: Contains utility scripts for law compliance, receipt validation, and diagnostic automation:
- **`scripts/check-law-compliance.sh`** — Detects plain `tower-lsp` references and victory language.
- **`scripts/validate-receipt-chain.sh`** — Verifies SHA256 digests and boundary markers.

## ANDON Gate — PreToolUse Hook (Λ_CD^runtime)

A `PreToolUse` hook in `.claude/settings.json` runs `lsp-max-cli gate check` before every **Bash, Edit, and Write** tool call.

- **Exit 0** — gate is clear; the tool proceeds.
- **Exit 1** — ANDON is ACTIVE; the tool is blocked until the gate clears.

This enforces `Λ_CD^runtime`: no shell-side action (build, test, release, format) and no file mutation may proceed while an active ANDON signal is present. Resolve all `WASM4PM-*` and `GGEN-*` diagnostics to clear the gate.

## Workspace architecture

The five-layer model: (1) actuation grammar → (2) local LSP state surface → (3) law-state runtime → (4) knowledge hooks → (5) autonomic LSP mesh. Crates map onto it:

- **Root crate (`src/`)** — the LSP server framework: `LanguageServer` trait, `LspService`, `Server` over stdio/TCP, plus `gate.rs`, `diagnostics.rs`, `composition.rs` for the law-state surface.
- **`lsp-max-protocol`** — the `max/*` method declarations, capability vectors, `MaxDiagnostic`, `ConformanceVector`, receipts, analysis bundles. This is where protocol types live; change them here, not in consumers.
- **`lsp-max-runtime`** — typestate machine, phases, transitions, runtime snapshotting.
- **`lsp-max-agent`** — agent integration and analysis bundles.
- **`lsp-max-macros`** — internal proc macros (e.g. `#[tower_lsp::async_trait]` equivalent surface).
- **`crates/lsp-max-cli`** — actuation grammar: noun/verb CLI built on `clap-noun-verb` (filename = noun, `#[verb]` = action). 31 noun modules — see Noun/Verb Pattern section below.
- **`crates/lsp-max-client`** — LSP *client* framework (drives servers in tests).
- **`crates/lsp-max-base`**, **`-live`**, **`-lsif`**, **`-specgen`** — base protocol types, live surfaces, LSIF export/conformance, and codegen from the official LSP 3.18 `metaModel.json`.
- **`crates/lsp-max-adapters/`** — ported `auto-lsp` stack (`lsp-max-ast-core`, `lsp-max-ast-codegen`, `lsp-max-ast`): tree-sitter-driven AST/codegen layer. Tree-sitter observes; it never admits.
- **`crates/wasm4pm-lsp`**, **`crates/gc005-wasm4pm-adapter`** — process-mining LSP surfaces over the wasm4pm engine; dogfood tests (`dogfood_gc00*.rs`) validate gate conformance.
- **`crates/playground`** — dev-dependency harness with demo binaries (`dogfood_harness`, `lsif_demo`).
- **`crates/anti-llm-cheat-lsp`** — the diagnostic canary: an LSP that detects reintroduction of plain `tower-lsp`, fake receipts, fake routes, and victory language. Other examples (`pattern-lsp`, `clap-noun-verb-lsp`, `axum-lsp`, `bevy-lsp`, `tex-lsp`, `wasm4pm-compat-lsp`) are workspace members and must keep compiling.

## lsp-max-cli: Noun/Verb Pattern

The CLI is a **noun/verb actuation grammar**. Each file in `crates/lsp-max-cli/src/nouns/` is one noun; each `#[verb]`-annotated function is one action.

### Three-tier structure (every noun file follows this)

```rust
// 1. Domain Tier — domain types only (no logic)
#[derive(Debug, Clone, Serialize)]
pub struct SomeThing { pub field: String }

// 2. Service Tier — business logic against AutonomicMesh
pub struct SomeService { state_path: String }

impl SomeService {
    pub fn new() -> Self { Self { state_path: crate::nouns::get_state_path() } }

    pub fn do_thing(&self, id: &str) -> std::result::Result<SomeThing, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let inst = mesh.instances.get(id).ok_or_else(|| format!("Instance not found: {id}"))?;
        Ok(SomeThing { field: inst.conformance_score().to_string() })
    }
}

impl Default for SomeService { fn default() -> Self { Self::new() } }

// 3. CLI Tier — result wrapper + #[verb] function
#[derive(Serialize)]
pub struct DoThingResult { pub thing: SomeThing, pub status: String }

/// One-line doc shown in --help.
#[verb("do-thing")]
pub fn do_thing(instance_id: String) -> Result<DoThingResult> {
    let svc = SomeService::new();
    let thing = svc.do_thing(&instance_id).map_err(NounVerbError::execution_error)?;
    let status = "CANDIDATE".to_string();
    Ok(DoThingResult { thing, status })
}
```

### Registration

Add `pub mod your_noun;` to `crates/lsp-max-cli/src/nouns/mod.rs`. The `linkme` crate auto-discovers `#[verb]` items; no manual wiring is needed beyond the `mod` declaration.

### Current nouns (31 total, in mod.rs order)

`admission`, `agent`, `alias`, `batch`, `client`, `config`, `conformance`, `diagnostics`, `doctor`, `event`, `export`, `gate`, `history`, `hook`, `import`, `logs`, `metamodel`, `metrics`, `ocel`, `plugin`, `process`, `receipt`, `repair`, `rpc`, `server`, `snapshot`, `state`, `swarm`, `task`, `telemetry`, `workspace`

### State path resolution

`crate::nouns::get_state_path()` reads `$LSP_MAX_STATE_PATH`, falling back to `.mesh_state.json`. Services with their own `state_path` field use it directly; services that call `new()` read from the env at call time.

### `AutonomicMesh::load_from_file` — critical bootstrap behavior

When the given path does not exist but the **parent directory is writable**, `load_from_file` bootstraps a default mesh, writes it to that path, and returns `Ok`. It does **not** return an error for a nonexistent file when the directory exists. This affects test design — see Testing Patterns below.

### HookEvent serialization format

`HookEvent` is an externally-tagged Serde enum. The serialized form is:

```json
{ "VariantName": { "instance_id": "...", ... } }
```

The outer key (variant name) is the activity name used in process-mining verbs (`process dfg`, `process variants`, etc.) and in OCEL 2.0 export (`ocel export`). Extract it with:

```rust
let activity = raw.as_object().and_then(|m| m.keys().next()).unwrap_or("Unknown");
```

## Testing Patterns for Noun Modules

Two patterns apply depending on whether the service reads `state_path` from a struct field or from the env var via `get_state_path()`.

### Pattern A — service-layer tests (struct field injection)

Use when the service struct has a `state_path: String` field that you set directly.

```rust
fn make_temp_svc() -> (tempfile::NamedTempFile, MyService) {
    let mut mesh = AutonomicMesh::new();
    mesh.add_instance(LspInstance::new("test-inst"));
    let f = tempfile::NamedTempFile::new().unwrap();
    mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
    let svc = MyService { state_path: f.path().to_str().unwrap().to_string() };
    (f, svc) // hold f alive; dropping it deletes the file
}

#[test]
fn thing_returns_ok() {
    let (_f, svc) = make_temp_svc();
    assert!(svc.do_thing("test-inst").is_ok());
}
```

### Pattern B — verb-level tests (env var guard)

Use when the service calls `get_state_path()` internally (e.g. `ClientService`, `ServerService`, `PluginService`, `ConfigService`).

```rust
struct StateGuard { prev: Option<String>, _f: tempfile::NamedTempFile }

impl StateGuard {
    fn new() -> Self {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("test-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: under TEST_ENV_LOCK, single-threaded env mutation.
        unsafe { std::env::set_var("LSP_MAX_STATE_PATH", f.path()) };
        Self { prev, _f: f }
    }
}

impl Drop for StateGuard {
    fn drop(&mut self) {
        // SAFETY: restoring env under TEST_ENV_LOCK.
        unsafe {
            match self.prev.take() {
                Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                None => std::env::remove_var("LSP_MAX_STATE_PATH"),
            }
        }
    }
}

#[test]
fn thing_ok() {
    let _lock = crate::nouns::TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let _g = StateGuard::new();
    let svc = MyService::new();
    assert!(svc.do_thing().is_ok());
}
```

`crate::nouns::TEST_ENV_LOCK` (a `static Mutex<()>` in `mod.rs`) serializes all tests that mutate env vars.

### Unsafe env mutation (Rust 1.83+)

`std::env::set_var` and `remove_var` are `unsafe` in Rust 1.83+. Always wrap in `unsafe {}` and include a `// SAFETY: under TEST_ENV_LOCK.` comment.

### "Missing state file" test antipattern — critical

`load_from_file` bootstraps a mesh when the parent directory is writable. This means:

```rust
// WRONG — /tmp/ is writable, so load_from_file writes the file and returns Ok
let svc = MyService { state_path: "/tmp/nonexistent-test.json".to_string() };
assert!(svc.do_thing().is_err()); // FAILS
```

Route through a **non-existent directory** instead:

```rust
// CORRECT — /tmp/no-such-dir-lsp-max/X/ doesn't exist, bootstrap write fails → Err
let svc = MyService { state_path: "/tmp/no-such-dir-lsp-max/X/state.json".to_string() };
assert!(svc.do_thing().is_err()); // PASSES
```

### Per-verb test requirements

Each verb should have at minimum:
- **Success** — happy path returns `Ok`
- **Falsification** — verify the result contents (not just `is_ok()`)
- **Counterfactual** — error path returns `Err` (bad arg, missing state, unknown instance)

## Code layout conventions

- Keep files ≤ 500 LOC; split into submodules in a subdirectory matching the module name (`src/service/` next to `src/service.rs` is the established pattern).
- Integration tests live in root `tests/` (one file per concern, e.g. `test_lsp318_capabilities.rs`) and per-crate `tests/` for dogfood suites.
- LSP 3.18 feature claims require transcript + negative control + receipt; a feature row is `SUPPORTED_WITH_TRANSCRIPT`, `REFUSED_BY_LAW_WITH_RECEIPT`, or `BLOCKED` — never "implied" or "covered by normal LSP".

## External consumers of lsp-max (e.g. sibling LSP crates)

When a crate outside this workspace depends on `lsp-max` and also declares its own `tokio` dependency, it must include `"io-std"` in tokio features — `tokio::io::stdin` / `tokio::io::stdout` are behind that feature gate and are not inherited transitively from lsp-max.

When constructing `CodeAction` literals, always use `..Default::default()` for the trailing fields rather than exhaustive field listing — lsp-types-max may add fields as LSP 3.18 evolves and exhaustive structs break at the call site.

## Common Anti-Patterns

Avoid these patterns; they are enforced by linting, gate rules, and code review:

1. **Using half-finished features** — Don't reference proposed LSP 3.18 features unless they are marked `SUPPORTED_WITH_TRANSCRIPT` with a receipt. `CANDIDATE` or `BLOCKED` features may be removed or changed before stabilization.

2. **Error handling for impossible cases** — Don't add error branches for conditions the type system already guarantees are impossible (e.g., handling a `None` from a `let x = val.unwrap()` path). Well-typed code is self-documenting; redundant error handling obscures intent.

3. **Inventing explanatory comments** — Don't comment *what* the code does; well-named functions and variables already convey that. Comments describe *why* and *context*: design decisions, invariants, and non-obvious coupling.

4. **Exhaustive struct literals** — When constructing `CodeAction`, `Diagnostic`, `LSPRequest`, or similar types, use `..Default::default()` for trailing fields, not exhaustive field lists. As LSP 3.18 evolves, new fields may be added; exhaustive structs will break at the call site.

5. **Collapsing `Unknown` into `Admitted` or `Refused`** — `ConformanceVector` law-axis sets distinguish three states. Never coerce `Unknown` to either polarity; it signals a gap in tracing or a precondition not yet met.

6. **Using victory language** — Avoid "done", "all clean", "fully admitted", "solved", "guaranteed" in code, comments, commit messages, or reports. Use bounded statuses: `ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`, `PARTIAL`, `OPEN`, etc.

7. **Tautological test assertions** — Don't assert on return values that are always identical regardless of success (e.g., `stop()` always returns `Stopped`). Assert on observable state via a subsequent query (`status()` after `stop()` checks the OS).

8. **Writable `/tmp/` paths in error tests** — `load_from_file` bootstraps when the parent dir is writable. Use `/tmp/no-such-dir-lsp-max/X/state.json` (non-existent parent) for error-path tests.

## Debugging & Troubleshooting

### WASM4PM Build Failures

Symptom: `cargo build` fails with "unresolved import `wasm4pm_compat`" or similar.

**Check**: Verify sibling checkout:
```sh
ls ../wasm4pm ../wasm4pm-compat ../lsp-types-max
```

If missing, clone the repos into parent directory and re-run `just test`. The workspace uses `[patch.crates-io]` and path dependencies; they must be present in the filesystem, not fetched from crates.io.

### Reception Validation Failures

Symptom: Diagnostic `ANTI-LLM-CHEAT-LSP-RECEIPT-INVALID` or SHA256 chain mismatch.

**Check**: Verify the receipt artifact chain:
```sh
scripts/validate-receipt-chain.sh <receipt-path>
```

Look for boundary markers (`-----BEGIN RECEIPT-----`), valid SHA256 digests, and checkpoint closure. Receipts are not valid if they lack boundary markers or have digest mismatches.

### Conformance Score Anomalies

Symptom: A feature row shows low conformance or unexpected `REFUSED` status despite code changes.

**Check**: Trace admission law axes:
```sh
cargo test test_conformance_vector -- --nocapture
cargo test -p lsp-max-protocol test_law_axis_admission -- --nocapture
```

Use the output to identify which law axis is refusing admission (e.g., transcript missing, negative control missing, receipt not signed). Conformance is cumulative; all law axes must pass.

### Tower-LSP Reference Detection

Symptom: Clippy or `dx-verify` fails with "forbidden: plain tower-lsp reference".

**Check**: Run the compliance scanner:
```sh
scripts/check-law-compliance.sh
```

This greps for plain `tower-lsp`, `tower_lsp`, victory language, and fake receipt markers across the codebase. Fix any matches by renaming to `lsp-max` and ensuring all diagnostic claims have receipt artifacts.

### Gate State Inspection

Symptom: ANDON gate is set, blocking Bash commands; unclear what diagnostics are active.

**Check**: Query gate state:
```sh
lsp-max-cli gate check  # Exit 0 = gate clear; exit 1 = gate set
lsp-max-cli gate list   # (if available) List active WASM4PM-* / GGEN-* diagnostics
```

Or inspect the `.claude/settings.json` `PreToolUse` hook directly to see which gates are wired. Resolve all listed diagnostics to clear the ANDON signal.
