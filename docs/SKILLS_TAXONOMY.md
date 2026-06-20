# Claude Code Skills Taxonomy & Classification System

**Version:** 26.6.9 | **Generated:** 2026-06-14 | **Status:** ADMITTED

---

## Overview

This document organizes Claude Code skills along multiple dimensions for faster discovery and contextual skill selection. Use this taxonomy to:

1. **Find skills** by use case, project stage, or problem category
2. **Understand relationships** between skills
3. **Make decisions** about which skill to invoke in a given situation
4. **Design workflows** by following natural execution chains

---

## Taxonomy 1: By Lifecycle Stage

Skills organized by where they appear in the development and release cycle.

### Project Initialization (Pre-coding)

Skills for setting up a new project or onboarding to an existing one.

| Skill | Purpose | Duration | Authority |
|-------|---------|----------|-----------|
| **`/init`** | Initialize CLAUDE.md with project docs | 2-5 min | Authoritative |
| **`/session-start-hook`** | Configure web session startup hooks | 3-10 min | Contextual |
| **`/update-config`** | Setup permissions, env vars, hooks | 1-5 min | Authoritative |
| **`/keybindings-help`** | Customize keyboard shortcuts | 2-10 min | Optional |
| **`/fewer-permission-prompts`** | Auto-generate permission allowlist | <1 min | Convenience |

**Chain:**
```
/init → /session-start-hook → /update-config → /fewer-permission-prompts
```

**Next phase:** Development cycle

---

### Development & Testing (During coding)

Skills for running, validating, and testing code during active development.

| Skill | Purpose | Frequency | Authority |
|-------|---------|-----------|-----------|
| **`/run`** | Launch and observe app | Frequent | Essential |
| **`/verify`** | Validate behavior against spec | Per feature | Essential |
| **`/loop`** | Repeat skill/command on interval | Frequent | Convenience |
| **`/deep-research`** | Fact-check assumptions | As needed | Reference |
| **`/claude-api`** | Reference API/pricing/models | As needed | Reference |

**Chain (repeated):**
```
/run → manual testing → /run (again) → /verify → (proceed or fix)
```

**Next phase:** Code review cycle

---

### Code Quality & Review (Before commit)

Skills for ensuring code quality before merging.

| Skill | Purpose | Severity | Authority |
|-------|---------|----------|-----------|
| **`/code-review`** | Find bugs and inefficiencies | Must-do | Essential |
| **`/simplify`** | Refactor for clarity and reuse | Should-do | Optional |
| **`/security-review`** | Identify vulnerabilities | Must-do | Essential |
| **`/review`** | Comprehensive PR review | Must-do | Essential |

**Chain (sequential):**
```
/verify
  ↓
/code-review --fix
  ↓
/verify (re-test after fixes)
  ↓
/simplify
  ↓
/security-review
  ↓
/review --comment
  ↓
commit
```

**Next phase:** Merge/release

---

### Release & Deployment (Post-commit)

Skills for monitoring and validating releases.

| Skill | Purpose | Use Case |
|-------|---------|----------|
| **`/loop`** | Poll deployment status | Continuous monitoring |
| **`/verify`** | Validate production behavior | Post-release check |

---

## Taxonomy 2: By Problem Domain

Skills grouped by the type of problem they solve.

### Configuration & Environment

**Skills for setting up tools, permissions, and project structure.**

| Skill | Scope | Modifies |
|-------|-------|----------|
| **`/init`** | Project documentation and architecture | CLAUDE.md (project-wide) |
| **`/update-config`** | Permissions, env vars, hooks, automation | .claude/settings.json, .claude/settings.local.json |
| **`/keybindings-help`** | Keyboard customization | ~/.claude/keybindings.json |
| **`/session-start-hook`** | Web session initialization | .claude/settings.json (hooks) |
| **`/fewer-permission-prompts`** | Reduce permission dialogs | .claude/settings.json (allowlist) |

**Authority:** `update-config` is the primary tool; others are specialized helpers.

**Typical workflow:**
```
/init (establish baseline)
  ↓
/update-config (configure per needs)
  ↓
/session-start-hook (if web-based)
  ↓
/fewer-permission-prompts (reduce friction)
```

---

### Execution & Validation

**Skills for running code, observing behavior, and generating evidence.**

| Skill | Launches? | Validates? | Generates Receipt? |
|-------|-----------|-----------|-------------------|
| **`/run`** | ✓ Yes | ✗ No | ✗ No |
| **`/verify`** | ✓ Yes | ✓ Yes | ✓ Yes |
| **`/loop`** | Via target | Via target | Via target |

**Relationships:**
- `/run` → `/verify` (most common chain)
- `/verify` → everything else (must verify before reviewing code)
- `/loop` wraps `/run` or `/verify` for recurring checks

**Typical workflow:**
```
/run                    (app started)
  ↓ (manual testing)
/verify                 (validate behavior)
  ↓ (if ADMITTED)
/code-review            (review code)
```

---

### Code Quality & Analysis

**Skills for finding bugs, improving efficiency, and simplifying code.**

| Skill | Finds Bugs? | Auto-fixes? | For Review? |
|-------|------------|------------|-------------|
| **`/code-review`** | ✓ Yes | ✓ Optional | ✓ PR or diff |
| **`/simplify`** | ✗ No | ✓ Always | ✓ Code quality |
| **`/security-review`** | ✓ Yes (security) | ✗ No | ✓ Security audit |
| **`/review`** | ✓ Yes (all categories) | ✗ No | ✓ PR-level |

**Severity hierarchy:**
```
CORRECTNESS (bugs, logic errors)
  ↓
EFFICIENCY (performance, resource use)
  ↓
STYLE (naming, consistency, clarity)
```

**Always prioritize CORRECTNESS > EFFICIENCY > STYLE**

**Typical workflow:**
```
/code-review low        (find bugs)
  ↓
/code-review --fix      (auto-fix safe issues)
  ↓
/verify                 (re-validate after fixes)
  ↓
/simplify               (further cleanup)
  ↓
/security-review        (audit for vulnerabilities)
```

---

### Research & Reference

**Skills for looking up information and conducting investigations.**

| Skill | Type | Authority | Sources |
|-------|------|-----------|---------|
| **`/deep-research`** | Multi-source fact-check | High | Web, academic, blogs |
| **`/claude-api`** | API reference | Authoritative | Official docs, training cutoff |

**Use cases:**
- `/deep-research` — When you need cited evidence or contested claims
- `/claude-api` — When your question is about Claude API (model IDs, pricing, etc.)

**Note:** System reminder auto-triggers `/claude-api` when Claude/Anthropic is mentioned.

---

## Taxonomy 3: By Skill Autonomy

How much automation each skill provides vs. requiring human decisions.

### Fully Automated

Skills that run end-to-end with no human intervention.

| Skill | Inputs | Outputs | No User Action Required? |
|-------|--------|---------|--------------------------|
| **`/run`** | Auto-detect project | App running | ✓ Yes |
| **`/verify`** | Changed files + app | Receipt (ADMITTED/REFUSED) | ✓ Yes |
| **`/loop`** | Target command + interval | Recurring execution | ✓ Yes |
| **`/fewer-permission-prompts`** | Session transcript | Updated settings | ✓ Yes |

### Semi-Automated (Guided)

Skills that provide automation but require human decisions at key points.

| Skill | Automation | Human Decision | Output |
|-------|-----------|-----------------|--------|
| **`/code-review`** | Analyze and find issues | Which fixes to apply? | Report + optional auto-fixes |
| **`/simplify`** | Refactor + apply | Review changes before push | Refactored code |
| **`/security-review`** | Identify vulns | Which to fix first? | Categorized findings |
| **`/review`** | Comprehensive analysis | Approve/request-changes? | PR review with flags |

### Manual (Advisory)

Skills that provide guidance but don't modify code.

| Skill | Advisory | Output | Human Action |
|-------|----------|--------|---------------|
| **`/deep-research`** | Multi-source research | Cited report | Apply learnings or decide |
| **`/claude-api`** | API reference | Documentation + examples | Use knowledge to code |

### Configuration (One-time)

Skills for setup; typically run once per project or session.

| Skill | Setup | Persistence | Frequency |
|-------|-------|-------------|-----------|
| **`/init`** | Initialize CLAUDE.md | Project root (checked in) | Once per project |
| **`/update-config`** | Permissions, hooks, env | `.claude/settings.json` | Multiple times; incremental |
| **`/keybindings-help`** | Keyboard shortcuts | `~/.claude/keybindings.json` | Once; then editing as needed |
| **`/session-start-hook`** | Web session hooks | `.claude/settings.json` | Once per project |

---

## Taxonomy 4: By Input/Output Type

How skills consume and produce information.

### Reads

What each skill examines to make decisions:

| Skill | Reads |
|-------|-------|
| **`/run`** | Cargo.toml, package.json, Dockerfile, entry points |
| **`/verify`** | Git diff, changed files, app behavior, console output |
| **`/code-review`** | Git diff, source code, test files, documentation |
| **`/simplify`** | Git diff, source code, patterns |
| **`/security-review`** | Git diff, dependencies, environment handling |
| **`/review`** | PR metadata, commits, diff, test results |
| **`/deep-research`** | Web (via search) |
| **`/claude-api`** | Stored documentation, training data |

### Writes

What each skill modifies:

| Skill | Writes | Rollback Possible? |
|-------|--------|-------------------|
| **`/code-review --fix`** | Working tree (new commit) | ✓ Yes (git revert) |
| **`/simplify`** | Working tree (new commit) | ✓ Yes (git revert) |
| **`/code-review --comment`** | PR comments | ✓ Yes (delete comments) |
| **`/review --approve`** | PR approval | ✓ Yes (dismiss) |
| **`/update-config`** | .claude/settings.json | ✓ Yes (git checkout) |
| **`/keybindings-help`** | ~/.claude/keybindings.json | ✓ Yes (restore from backup) |
| **`/session-start-hook`** | .claude/settings.json | ✓ Yes (git checkout) |
| **`/fewer-permission-prompts`** | .claude/settings.json | ✓ Yes (git checkout) |

### Outputs (Receipts)

What evidence each skill generates:

| Skill | Receipt Type | Authority |
|-------|--------------|-----------|
| **`/verify`** | Behavior receipt (ADMITTED/REFUSED/UNKNOWN) | High |
| **`/code-review`** | Code analysis report + optional PR comments | High |
| **`/simplify`** | Refactored code + commit message | High |
| **`/security-review`** | Vulnerability report (categorized by severity) | High |
| **`/review`** | PR review summary + optional approval/changes | High |
| **`/run`** | App running + console output | Informational |
| **`/deep-research`** | Cited research report | High |
| **`/init`** | CLAUDE.md documentation | Baseline |

---

## Taxonomy 5: By Trigger Pattern

When to invoke each skill based on context.

### "I want to see if it works"

```
Trigger: User wants to run the app
→ /run                  (launch)
→ (manual testing)
→ /verify               (validate behavior)
```

### "I made a change and want to verify it"

```
Trigger: Feature or bug fix completed
→ /run                  (launch)
→ /verify               (test change)
→ /code-review --fix    (improve code)
→ /verify               (re-test after fixes)
→ /security-review      (audit)
→ commit
```

### "I need to review a PR"

```
Trigger: New pull request ready
→ /verify               (test PR works)
→ /code-review --comment (find issues)
→ /security-review      (security audit)
→ /review --approve     (approve if clear)
```

### "I want to understand a concept"

```
Trigger: Need to research or look up info
→ /deep-research "question" (multi-source fact-check)
OR
→ /claude-api "api question" (if Claude API related)
```

### "I want to set up my environment"

```
Trigger: New project or onboarding
→ /init                 (create CLAUDE.md)
→ /update-config        (set permissions)
→ /session-start-hook   (if web-based)
→ /fewer-permission-prompts (reduce dialogs)
```

### "I need to keep checking status"

```
Trigger: Polling scenario (deploy, test run, etc.)
→ /loop 5m /verify      (repeat every 5 min)
OR
→ /loop 30s /run        (restart app every 30s)
```

---

## Taxonomy 6: Skill Dependency Graph

Directed graph of skill dependencies and typical chains.

### Simple Chains

**Linear, sequential workflows:**

```
/init → /session-start-hook → /update-config → /fewer-permission-prompts
(Project setup)

/run → /verify → (commit or loop back)
(Simple validation)

/code-review → /simplify → /security-review
(Code quality)
```

### Complex Chains

**Convergent/divergent workflows:**

```
                    /code-review --fix
                   /                  \
/verify ────────►  /simplify          → /security-review
                   \                  /
                    \ ─────────────────
                    
(Full quality pipeline)

                      /code-review --comment
                    /
/verify ───────────┤
                    \
                     /security-review → /review --approve
                    
(PR review workflow)
```

### Cyclical Patterns

**Iterative development:**

```
    ┌─────────────┐
    │   /run      │
    └──────┬──────┘
           │
    ┌──────▼──────┐
    │ Manual test │
    └──────┬──────┘
           │
    ┌──────▼──────┐
    │   /verify   │──REFUSED──┐
    └──────┬──────┘           │
           │ ADMITTED         │
           │                  │
    ┌──────▼──────────────────┤
    │ /code-review --fix      │
    └──────┬──────────────────┘
           │
    ┌──────▼──────┐
    │   /verify   │──REFUSED──┘
    └──────┬──────┘
           │ ADMITTED
    ┌──────▼──────────────────┐
    │ commit & push           │
    └─────────────────────────┘
```

### Polling Pattern

```
    /loop 5m /verify
           │
    ┌──────┴──────┐
    │             │
    │  ADMITTED   │ (continue)
    │      or     │
    │  REFUSED    │ (alert)
    │             │
    └──────┬──────┘
    (repeat every 5 min)
```

---

## Taxonomy 7: Decision Matrix

Quick reference for choosing the right skill.

### Question: "What's happening?"

| Scenario | Skill | Why |
|----------|-------|-----|
| Need to start the app | `/run` | Launches with auto-detection |
| App running; need to validate behavior | `/verify` | Generates receipt with evidence |
| Need to poll status repeatedly | `/loop` + `/verify` | Recurring checks |

### Question: "Is the code good?"

| Scenario | Skill | Why |
|----------|-------|-----|
| Find bugs and inefficiencies | `/code-review` | Comprehensive analysis |
| Refactor for clarity (no bugs) | `/simplify` | Auto-refactoring |
| Check for security issues | `/security-review` | Specialized for vulns |
| Full PR audit | `/review` | Covers completeness + correctness |

### Question: "How do I set this up?"

| Scenario | Skill | Why |
|----------|-------|-----|
| Initialize new project | `/init` | Creates CLAUDE.md |
| Configure permissions/env | `/update-config` | Authoritative config tool |
| Customize keyboard | `/keybindings-help` | Specialized for keybindings |
| Reduce permission prompts | `/fewer-permission-prompts` | Auto-generates safe allowlist |

### Question: "I need to research something"

| Scenario | Skill | Why |
|----------|-------|-----|
| Technical question with multiple sources | `/deep-research` | Fact-checks across sources |
| Claude API question | `/claude-api` | Authoritative API reference |
| General knowledge | Claude directly | If not API/LLM-related |

---

## Taxonomy 8: Risk & Authority Levels

How much trust and caution each skill requires.

### Authority: AUTHORITATIVE

**Definitive source; use as single source of truth.**

| Skill | Authority | Reason |
|-------|-----------|--------|
| **`/init`** | CLAUDE.md generation | Project constitution |
| **`/update-config`** | Configuration | Single point of control |
| **`/verify`** | Behavior validation | Direct observation |
| **`/code-review`** | Bug detection | Systematic analysis |
| **`/security-review`** | Vulnerability finding | Specialized tooling |
| **`/claude-api`** | API reference | Official documentation |

### Authority: HIGH

**High-confidence; minimal review before action.**

| Skill | Authority | Caution |
|-------|-----------|---------|
| **`/run`** | App execution | May fail if dependencies missing |
| **`/simplify`** | Auto-refactoring | Review changes; test afterward |
| **`/code-review --fix`** | Auto-fixes | Apply only safe suggestions |

### Authority: MEDIUM

**Requires human judgment; advisory.**

| Skill | Authority | Caution |
|-------|-----------|---------|
| **`/review`** | PR review | Author has final say |
| **`/deep-research`** | Multi-source research | Verify critical claims independently |
| **`/loop`** | Polling | May not detect all state changes |

### Authority: LOW

**Informational; verify independently.**

| Skill | Authority | Caution |
|-------|-----------|---------|
| **`/keybindings-help`** | Keyboard customization | Personal preference |
| **`/fewer-permission-prompts`** | Auto-allowlist | Review for safety |
| **`/session-start-hook`** | Hook setup | May need debugging |

---

## Taxonomy 9: Performance Characteristics

Time and resource usage for each skill.

| Skill | Time | Resources | Scalability |
|-------|------|-----------|-------------|
| **`/run`** | 5-30s (app dependent) | Low | Fair (depends on app) |
| **`/verify`** | 10s-2min | Low | Fair (depends on test scope) |
| **`/code-review low`** | 10-30s per file | Low | Good (incremental) |
| **`/code-review medium`** | 30-90s per file | Low | Good (incremental) |
| **`/code-review high`** | 2-5 min per file | Medium | Fair (detailed analysis) |
| **`/code-review max`** | 5-15 min per file | Medium | Poor (exhaustive) |
| **`/simplify`** | 1-3 min per file | Low | Good (incremental) |
| **`/security-review`** | 2-5 min per file | Low | Good (focused scope) |
| **`/review`** | 5-15 min per PR | Medium | Fair (depends on PR size) |
| **`/deep-research`** | 2-10 min | Medium (web) | Poor (research dependent) |
| **`/loop`** | 1+ intervals (recurring) | Variable | Fair (depends on target) |

**Optimization tips:**
- Use `low` effort first, increase if needed
- Batch multiple skills (e.g., `/code-review --fix && /simplify`)
- Use `/loop` for polling, not tight polling loops
- Cache `/deep-research` results (don't re-research same topic)

---

## Taxonomy 10: Conflict & Interaction Matrix

How skills interact when used together.

### Complementary (use together)

| Skill Pair | Pattern | Notes |
|-----------|---------|-------|
| `/run` + `/verify` | /run → /verify | Standard chain |
| `/verify` + `/code-review` | /verify → /code-review | Verify works, then review |
| `/code-review` + `/simplify` | /code-review --fix → /simplify | Fix bugs, then clean up |
| `/simplify` + `/security-review` | /simplify → /security-review | Clean up, then audit |

### Sequential (order matters)

| Skill Pair | Correct Order | Why |
|-----------|---------------|-----|
| `/run` and `/verify` | `/run` first | Can't verify without running |
| `/verify` and `/code-review` | `/verify` first | Can't review broken code |
| `/code-review --fix` and `/verify` | `/code-review` first | Fix, then test |

### Conflicting (use one or other)

| Skill Pair | Use Instead Of | Reason |
|-----------|----------------|--------|
| `/simplify` | `/code-review --fix` | simplify is specialized refactoring |
| `/security-review` | `/code-review` | For security-specific audit |
| `/review` | `/code-review` | For PR-level audit vs diff |

### Independent (can run in parallel)

| Skills | Notes |
|--------|-------|
| `/loop` + other work | Loop runs in background |
| `/deep-research` + coding | Research doesn't block development |
| `/update-config` + other skills | Configuration independent |

---

## Appendix: Quick Skill Selection Guide

### I want to...

**...run my app**
→ `/run`

**...test that a change works**
→ `/run` then `/verify`

**...find bugs in code**
→ `/code-review`

**...check for vulnerabilities**
→ `/security-review`

**...review a pull request**
→ `/verify` (test it) then `/review`

**...clean up my code**
→ `/simplify` (auto-refactor) or `/code-review --fix` (fix + refactor)

**...configure my environment**
→ `/update-config`

**...initialize a project**
→ `/init` then `/session-start-hook` then `/update-config`

**...research something**
→ `/deep-research` (multi-source) or `/claude-api` (if API-related)

**...poll for status**
→ `/loop 5m /verify`

**...reduce permission prompts**
→ `/fewer-permission-prompts`

---

## Appendix: Skill Maturity Timeline

Expected skill evolution (informational):

```
Q3 2026: v26.9+
  - /verify adds timeout params
  - /code-review adds diff-caching for performance

Q4 2026: v26.12+
  - /review adds auto-approve conditions
  - /loop adds conditional exit (e.g., "stop when tests pass")

Q1 2027: v27.3+
  - /deep-research adds interactive source vetting
  - /simplify learns project-specific patterns
```

---

## See Also

- [SKILLS_REGISTRY.md](SKILLS_REGISTRY.md) — Full skill documentation
- [CLAUDE.md](CLAUDE.md) — Project constitution
- [AGENTS.md](AGENTS.md) — Agent architecture

---

**End of Skills Taxonomy**
