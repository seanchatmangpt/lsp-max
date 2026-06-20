# Contributing to lsp-max

**Before opening a PR, read `AGENTS.md` and `CLAUDE.md` in full.** They are the project constitution and enforce laws that are validated by automated tooling. This guide operationalizes them.

---

## Quick Start

```bash
# Clone and set up sibling repos (build prerequisite)
git clone <your-fork>
cd lsp-max

# Ensure siblings exist at these paths (absolute requirement)
../lsp-types-max       # LSP type authority with proposed 3.18 features
../wasm4pm-compat      # Sole baseline type authority for process-mining
../wasm4pm             # Execution engine

# Install just for command orchestration
cargo install just

# List available commands
just
```

---

## PR Workflow

### 1. Branch Naming: `claude/*`

All contribution branches must follow the `claude/*` naming convention:

```bash
git checkout -b claude/feature-name
git checkout -b claude/fix-issue-123
git checkout -b claude/docs-update
```

**Why?** It signals agent-oriented work and prevents collision with human-driven branches. Non-`claude/*` branches are not eligible for merge without maintainer override.

### 2. Commit Conventions

#### 2.1 No Victory Language

**Forbidden words in commits, code, and comments:**
- "done", "all clean", "fully admitted", "solved", "guaranteed", "fixed", "works", "passes", "success"

**Allowed statuses** (from AGENTS.md):
- `ADMITTED` — receipt artifact proves compliance
- `CANDIDATE` — under evaluation, receipt pending
- `BLOCKED` — cannot proceed; explicit reason required
- `REFUSED` — rejected by law or receipt failure
- `UNKNOWN` — state unresolved (never collapse to admitted/refused)
- `PARTIAL` — subset achieved; boundary stated explicitly
- `OPEN` — decision pending

**Examples:**

Bad:
```
commit: Fix LSP 3.18 support — all tests pass
comment: // This is finally working
```

Good:
```
commit: Add LSP 3.18 textDocument/diagnostic with receipt
comment: // textDocument/diagnostic is CANDIDATE pending receipt validation
```

#### 2.2 Commit Message Template

Use the format below. No preamble; start with the message.

```
<type>: <subject> — <receipt-or-constraint>

<body>

<sign-off>
```

**Type:** `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

**Subject:** ≤ 50 characters, lowercase, imperative ("add" not "adds")

**Receipt or constraint:** State the admission form:
- `with receipt <path>` — claim backed by BLAKE3-hashed artifact
- `CANDIDATE` — pending receipt
- `BLOCKED: <reason>` — explicit blocker
- `REFUSED: <reason>` — rejected by law or gate
- `no victory language` — commit contains no forbidden words

**Body:** Why, not what. Reference AGENTS.md laws where relevant.

**Sign-off:** Add your sign-off (required):

```
Signed-off-by: Your Name <your.email@example.com>
```

You can automate this:

```bash
git config user.name "Your Name"
git config user.email "your.email@example.com"
git commit -s  # -s adds the sign-off automatically
```

**Example commits:**

```
feat: add max/conformance-vector diagnostic with receipt

Implement ConformanceVector as per AGENTS.md §2.3. Adds OCEL-driven
admitted/refused/unknown tristate. Receipt written at receipts/gc005_conformance.blake3.

Signed-off-by: Jane Doe <jane@example.com>
```

```
test: add negative control for victory language detection — CANDIDATE

Adds dogfood test for anti-llm-cheat-lsp. Receipt generation pending
bench-admit run.

Signed-off-by: John Smith <john@example.com>
```

### 3. Pre-submission Checklist

Before `git push`:

```bash
# 1. Run the full pre-publish gate
just test-pre-publish

# 2. Verify architectural boundaries (mandatory)
just dx-verify

# 3. Polish code: format + strict clippy
just dx-polish

# 4. Run single-crate tests if you changed a specific crate
cargo test -p <crate-name> <optional-test-name>

# 5. Verify sibling repos comply with architectural mandate
# (dx-verify does this, but check manually if uncertain)
```

**If any step fails:** Fix the issue locally and create a **new commit**. Never amend a commit that has already been pushed.

### 4. Pull Request (GitHub)

Open your PR at: https://github.com/seanchatmangpt/lsp-max/compare

**PR Title:** ≤ 70 characters; lowercase; no victory language

**PR Body:** Use this template:

```markdown
## Summary

- <one-liner: what and why>
- <second bullet if needed>
- <reference relevant AGENTS.md laws>

## Receipt Status

- [ ] All receipts present (if claiming ADMITTED)
- [ ] Receipt paths listed: `receipts/<name>.blake3`
- [ ] Negative controls run and passed
- [ ] No victory language in code or comments

## Testing

- [ ] Unit tests added (inline in `.rs` files)
- [ ] Integration tests added (in `tests/`)
- [ ] Slow tests marked `#[ignore]`
- [ ] `just test-pre-publish` passes locally
- [ ] `just dx-verify` passes (architectural boundaries intact)

## Code Quality

- [ ] `just dx-polish` passes (fmt + clippy -D warnings)
- [ ] Files ≤ 500 LOC (or split into submodules)
- [ ] No comments except "why" — avoid "what"
- [ ] No plain `tower-lsp` outside negative-control fixtures
- [ ] `ConformanceVector` logic is disjoint (admitted/refused/unknown never collapse)

## Deployment

- [ ] CalVer version bumped (if applicable)
- [ ] All sibling repos (`../lsp-types-max`, `../wasm4pm-compat`, `../wasm4pm`) remain compliant
- [ ] CI passes
```

---

## Code Standards

### 1. File Size ≤ 500 LOC

Split large files into submodules in a directory matching the module name:

```
src/service.rs       (≤ 500 LOC)
src/service/        (submodule directory)
├── handler.rs
├── state.rs
└── mod.rs           (re-exports from this dir)
```

Check file size as you go:

```bash
wc -l src/service.rs  # Should be ≤ 500
```

### 2. Comments: "Why" Only

Comments must explain intent, not re-state code.

**Bad:**
```rust
// Increment counter
counter += 1;

// Check if vector is empty
if vec.is_empty() {
  return Err("empty");
}
```

**Good:**
```rust
// Bump counter to reflect newly admitted gate transitions
counter += 1;

// Refuse admission if no law-set observations recorded
if vec.is_empty() {
  return Err("empty");
}
```

Comments must never contain victory language or make unfounded claims.

### 3. Error Handling: Minimal, Explicit

- Use `Result<T, E>` for fallible operations
- Provide context in error messages; no generic "error"
- Do not swallow errors; propagate or log with full context
- Use `?` operator for early return

**Bad:**
```rust
match operation() {
  Ok(_) => {},
  Err(_) => {} // Silently ignore
}
```

**Good:**
```rust
operation().map_err(|e| {
  tracing::error!("Gate check REFUSED: {}", e);
  GateError::Blocked(e)
})?;
```

### 4. No Half-Finished Features

Every feature must be:
- Tested (unit + integration)
- Documented (rustdoc + CONTRIBUTING.md if user-facing)
- Backed by a receipt if it claims LSP 3.18 admission

Do not merge stubs or TODOs marked "finish later". Features must reach ADMITTED or BLOCKED state before merge.

### 5. Forbidden Words and Patterns

#### 5.1 Never Plain `tower-lsp`

Forbidden in code, Cargo.lock, docs, tests, comments (except negative-control fixtures):

```rust
// ✗ Never this
use tower_lsp::{Client, LanguageServer};

// ✓ Use this
use lsp_max::LanguageServer;  // from lsp-max crate
```

Negative-control fixtures in `examples/anti-llm-cheat-lsp` may reference `tower_lsp` explicitly to test detection.

#### 5.2 Environment Variables: `LSP_MAX_*`

All new env vars must use the `LSP_MAX_` prefix:

```rust
std::env::var("LSP_MAX_GATE_LEVEL")?
std::env::var("LSP_MAX_COMPOSITOR_WORKERS")?
```

#### 5.3 ConformanceVector: Disjoint Logic

`admitted`, `refused`, and `unknown` law-axes must never overlap:

```rust
// ✗ Wrong: unknown can flip to admitted
if unknown.is_empty() {
  return ADMITTED;
}

// ✓ Right: keep them disjoint
match (admitted.is_empty(), refused.is_empty(), unknown.is_empty()) {
  (false, true, true) => ADMITTED,
  (_, false, _) => REFUSED,
  (true, true, false) => UNKNOWN,
  _ => PARTIAL,  // Mixed state requires explicit boundary
}
```

### 6. LSP Surface: Read-Only

The LSP server emits diagnostics, hovers, and code intents but **never mutates user files directly**:

```rust
// ✓ Emit diagnostic
server.client.publish_diagnostics(uri, diags, None)?;

// ✗ Never this
fs::write(path, modified_content)?;  // LSP servers don't mutate
```

Code actions may suggest edits; the editor applies them. The server observes, not acts.

---

## Testing Requirements

### 1. Unit Tests: Inline in Modules

Place unit tests at the bottom of the same `.rs` file in a `#[cfg(test)]` module:

```rust
// File: src/gate.rs

pub fn check_gate() -> Result<()> { /* ... */ }

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_gate_admits_clean_state() {
    // ...
  }

  #[test]
  fn test_gate_blocks_andon() {
    // ...
  }
}
```

### 2. Integration Tests: `tests/` Directory

Place integration tests in the root `tests/` directory, one file per concern:

```
tests/
├── test_lsp318_capabilities.rs      (root harness)
├── lsp318_capabilities/
│   ├── mod.rs                       (test body)
│   ├── hover.rs                     (submodule)
│   └── diagnostics.rs               (submodule)
├── test_compositor_perf_admission.rs
├── compositor_perf_admission/
│   ├── mod.rs
│   └── ...
```

Use descriptive test names that mirror the feature being tested:

```rust
#[test]
fn test_max_conformance_vector_admitted_disjoint() {
  // ...
}

#[test]
fn test_lsp318_textDocument_diagnostic_receipt_required() {
  // ...
}
```

### 3. Slow Tests: Mark with `#[ignore]`

Tests that take > 5 seconds should be marked `#[ignore]` and gated behind `just test-pre-publish`:

```rust
#[test]
#[ignore]  // Runs only in pre-publish gate
fn test_stress_compositor_100_servers() {
  // ...
}
```

Run them explicitly:

```bash
cargo test --workspace -- --include-ignored
```

### 4. Negative Controls

For any LSP 3.18 feature claim, provide:
- A **positive test**: server exhibits the feature
- A **negative control**: feature is explicitly refused or blocked (transcript of refusal)
- A **receipt**: BLAKE3-hashed proof of both

Example for `textDocument/diagnostic`:

```rust
#[test]
fn test_textDocument_diagnostic_supported() {
  // Positive: server reports capability
  assert!(caps.text_document.diagnostic.is_some());
  // Negative: verify via transcript that diagnostic is actually emitted
}

#[test]
fn test_textDocument_diagnostic_receipt() {
  // Load receipt artifact and verify BLAKE3 hash
  let receipt = std::fs::read("receipts/gc005_diagnostic.blake3")?;
  assert_eq!(receipt.boundary, "textDocument/diagnostic");
}
```

### 5. Test Execution

```bash
# Run all tests
just test

# Run tests for a specific crate
cargo test -p lsp-max-protocol

# Run a specific test
cargo test test_lsp318_capabilities

# Run including slow tests (pre-publish gate)
just test-pre-publish

# Run a single integration test file
cargo test --test test_lsp318_capabilities
```

---

## Law Compliance: Architectural Boundaries

### 1. Reference AGENTS.md and CLAUDE.md

The project has non-negotiable laws (from AGENTS.md):

- **No plain `tower-lsp`** — only `lsp-max`
- **No victory language** — use bounded statuses
- **Receipts decide admissibility** — test output is not proof
- **`ConformanceVector` is tristate** — never collapse unknown
- **LSP surface is read-only** — no file mutations

Violating these triggers automated diagnostics from `examples/anti-llm-cheat-lsp`:

```
GC004B_NO_TOWER_LSP_LOCK = BLOCKED
ANTI-LLM-SURFACE-001: Victory language detected: "solved"
ANTI-LLM-SURFACE-002: Unknown collapsed into admitted
```

### 2. Sibling Repo Compliance

The workspace requires three sibling repos checked out as build prerequisites:

```
../lsp-types-max
  └─ LSP type authority; defines proposed 3.18 features
  └─ Must not contain "legacy", "deprecated", "shim", "facade", "backward compatibility"
  └─ Referenced via path dependency in Cargo.toml

../wasm4pm-compat
  └─ Sole baseline type authority for process-mining types
  └─ FORBIDDEN: intermediary type crates (wasm4pm_types, ocel_core)
  └─ FORBIDDEN: legacy terminology
  └─ Referenced via [patch.crates-io]

../wasm4pm
  └─ Execution engine
  └─ FORBIDDEN: legacy terminology
  └─ Referenced via [patch.crates-io]
```

Before merging, run:

```bash
just dx-verify
```

This automatically scans sibling repos for forbidden terminology and intermediary type crates.

### 3. Workspace Architecture Mapping

Understand where your code belongs (from CLAUDE.md):

| Crate | Purpose | Location | Example |
|-------|---------|----------|---------|
| Root `lsp-max` | LSP framework, `LanguageServer` trait, gate, composition | `src/` | `gate.rs`, `service.rs` |
| `lsp-max-protocol` | `max/*` methods, `MaxDiagnostic`, `ConformanceVector`, receipts | `lsp-max-protocol/` | Protocol type definitions |
| `lsp-max-runtime` | Typestate machine, phases, transitions, snapshots | `lsp-max-runtime/` | State transitions |
| `lsp-max-agent` | Agent integration, analysis bundles | `lsp-max-agent/` | Agent hooks |
| `lsp-max-cli` | Noun/verb CLI, actuation grammar | `crates/lsp-max-cli/` | Commands and verbs |
| `lsp-max-client` | LSP client for testing | `crates/lsp-max-client/` | Test harness |
| `anti-llm-cheat-lsp` | Diagnostic canary (detects violations) | `examples/anti-llm-cheat-lsp/` | Negative controls |

### 4. Version Management: CalVer (YY.M.D)

`lsp-max` uses **CalVer**, not SemVer:

- Current version: `26.6.9` (2026, June, 14th)
- Date-based; reflects release date
- Update `version.workspace` in `Cargo.toml` when releasing

Version violations trigger `ANTI-LLM-VERSION-*` diagnostics.

---

## External Consumer Guidelines

If your crate depends on `lsp-max` from outside the workspace:

### 1. Tokio Feature: `io-std`

If you also declare `tokio` as a dependency, include `"io-std"` in features:

```toml
[dependencies]
lsp-max = { version = "26.6", features = ["..."] }
tokio = { version = "1", features = ["io-std"] }  # io-std is required
```

Reason: `tokio::io::stdin` / `tokio::io::stdout` are behind `io-std` and not inherited transitively.

### 2. CodeAction Literals: Use `Default`

When constructing `CodeAction` literals, always use `..Default::default()`:

```rust
// ✗ Never exhaustive field listing
let action = CodeAction {
  title: "Fix".into(),
  kind: Some(CodeActionKind::QUICKFIX),
  // ... all fields listed
};

// ✓ Use Default for trailing fields
let action = CodeAction {
  title: "Fix".into(),
  kind: Some(CodeActionKind::QUICKFIX),
  ..Default::default()  // Future LSP 3.18 fields won't break this
};
```

Reason: `lsp-types-max` evolves as LSP 3.18 spec grows.

---

## ANDON Gate (Lambda_CD^runtime)

A `PreToolUse` hook in `.claude/settings.json` runs `lsp-max-cli gate check` before every shell action.

| Exit Code | Meaning |
|-----------|---------|
| 0 | Gate is clear; shell actions proceed |
| 1 | ANDON is set; shell is blocked |

**ANDON is triggered by active diagnostics:**
- `WASM4PM-*` — process-mining conformance violations
- `GGEN-*` — gateway generation errors

**To clear the gate:**
1. Review all diagnostics from `anti-llm-cheat-lsp`
2. Fix violations (code changes, receipt generation, etc.)
3. Verify: `lsp-max-cli gate check` returns 0

No merges, builds, or tests proceed while ANDON is active.

---

## Workflow Summary

1. **Fork, clone, set up siblings**
   ```bash
   git clone <your-fork>
   # Ensure ../lsp-types-max, ../wasm4pm-compat, ../wasm4pm exist
   ```

2. **Create a `claude/*` branch**
   ```bash
   git checkout -b claude/your-feature
   ```

3. **Code, test, commit with sign-off**
   - Write tests (unit + integration)
   - Run `just test-pre-publish` and `just dx-verify` locally
   - Commit with sign-off: `git commit -s`
   - No victory language; use bounded statuses

4. **Push and open PR**
   ```bash
   git push origin claude/your-feature
   # Open PR at https://github.com/seanchatmangpt/lsp-max/compare
   ```

5. **Await review**
   - Resolve CI failures
   - Address reviewer comments (new commits, no amends)
   - Maintainer merges when ready

---

## Getting Help

- **Read AGENTS.md first.** It's the constitution; laws there are enforced by tooling.
- **Check CLAUDE.md.** It explains architecture, commands, and versioning.
- **Run `just`** to list all available commands.
- **Grep for examples.** The `examples/` directory contains working templates.
- **Check CI output.** Failed checks point to specific violations.

---

## Code of Conduct

Please note that this project is released with a [Contributor Code of Conduct](./CODE_OF_CONDUCT.md).
By participating in this project, you agree to abide by its terms.

---

**Last updated:** 2026-06-14  
**For:** lsp-max version 26.6.9
