# Skill: /init

**Status:** AVAILABLE | **Scope:** Project Documentation | **Category:** Project-Specific & Specialized

---

## Overview

Initialize a new `CLAUDE.md` file with codebase documentation. Establishes project constitution, coding standards, and architectural guidelines for Claude Code sessions.

**Output:** `CLAUDE.md` in project root with comprehensive project documentation.

## When to Use

Use `/init` when you want to:
- Setup a new project for Claude Code
- Create baseline project documentation
- Establish coding standards and architecture overview
- Document build commands and validation gates
- Setup hooks and configuration expectations

**Typical:** Run once per project, at the beginning.

## Parameters

**None** — Interactive generation based on project introspection.

```bash
/init
```

## How It Works

### Phase 1: Project Introspection

Examine:
- `Cargo.toml`, `package.json`, `setup.py` (tech stack)
- Directory structure (crates, src, components, etc.)
- Build files (`Justfile`, `Makefile`, etc.)
- `.github/workflows` (CI/CD patterns)
- Existing documentation

### Phase 2: Generate CLAUDE.md Sections

1. **What this is** — Project name, purpose, status
2. **Versioning** — CalVer or SemVer; version law violations
3. **Sibling dependencies** — Path or patch dependencies
4. **Commands** — Build, test, development recipes
5. **Architecture** — Layer/crate model
6. **Code conventions** — File structure, naming, size limits
7. **External consumers** — Downstream project guidelines

### Phase 3: Output

- Create/overwrite `CLAUDE.md`
- Status: ADMITTED (generated and valid)
- Next: Review and customize

## Expected Output

```
📝 Project Documentation: CLAUDE.md

Introspection:
  [✓] Detected: Rust workspace (5 crates)
  [✓] Build system: Just recipes
  [✓] Test setup: cargo test + integration tests
  [✓] CI/CD: GitHub Actions (3 workflows)
  [✓] Versioning: CalVer (YY.M.D)

Generated sections:
  [✓] What this is
  [✓] Versioning strategy
  [✓] Dependencies (3 sibling repos)
  [✓] Commands (8 recipes documented)
  [✓] Architecture (5-layer model)
  [✓] Code layout conventions
  [✓] External consumer guidelines

CLAUDE.md created: /home/user/lsp-max/CLAUDE.md (1,247 lines)

Status: ADMITTED
Next: Review CLAUDE.md; customize as needed
```

## CLAUDE.md Structure

Generated file includes:

```markdown
# CLAUDE.md

## What this is
[Project name, purpose, brief description]

## Versioning
[Version strategy and laws]

## Sibling repo dependencies
[Path dependencies, patch dependencies]

## Commands
[Build, test, dev commands]

## Workspace architecture
[Layer/crate model, component map]

## Code layout conventions
[File structure, naming patterns, size limits]

## External consumers
[Guidelines for downstream projects]

## [Custom sections as needed]
[Anti-patterns, special validation, etc.]
```

## Integration

### First Step in Project Setup

```
/init                           (create CLAUDE.md)
  ↓
/session-start-hook            (setup hooks)
  ↓
/update-config                 (configure)
  ↓
(start development)
```

### Referenced by Other Skills

Every other skill reads CLAUDE.md to understand:
- Project architecture
- Build commands
- Code standards
- Validation requirements

## Example Output

```markdown
# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## What this is

A Rust LSP framework called **lsp-max**: a "law-state runtime projected through LSP". 
It maximizes LSP 3.18 capability coverage and adds custom max/* protocol extensions.

## Versioning

Workspace version is **CalVer (YY.M.D)**, not SemVer.

## Commands

```sh
just test               # cargo test --workspace
just test-e2e           # cargo test --test e2e
just dx-verify          # architectural boundary scan
just dx-polish          # cargo fmt + clippy
```

## Architecture

Five-layer model:
1. Actuation grammar (CLI)
2. Local LSP state
3. Law-state runtime
4. Knowledge hooks
5. Autonomic mesh

Crates: root (`src/`), lsp-max-protocol, lsp-max-runtime, ...

## Code conventions

- Files ≤ 500 LOC; split into subdirectories
- Integration tests in `tests/` (one per concern)
- No plain `tower-lsp` references (use `lsp-max`)
- Use bounded statuses: ADMITTED, CANDIDATE, BLOCKED, REFUSED, UNKNOWN
```

## See Also

- [`/update-config`](SKILL_UPDATE_CONFIG.md) — Configure after initialization
- [`/session-start-hook`](SKILL_SESSION_START_HOOK.md) — Setup hooks next
- [CLAUDE.md Format Reference](../CLAUDE.md) — Template and guidelines

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
