# Claude Code Hooks Documentation Index

This directory contains comprehensive documentation for the Claude Code hooks system. Use this index to navigate to the right resource for your needs.

---

## Quick Start (5 minutes)

1. **First time with hooks?** → Start with [HOOKS_GUIDE.md](#hooksguidemd) "Overview" and "Hook Types" sections
2. **Need a working example?** → Copy a template from [HOOKS_TEMPLATES.md](#hookstemplatesmd)
3. **Hooks not working?** → See [HOOKS_GUIDE.md "Troubleshooting"](#troubleshooting)

---

## Documents

### HOOKS_GUIDE.md

**Comprehensive reference** for the Claude Code hooks system.

**What it covers:**
- Hook types: PreToolUse, PostToolUse, SessionStart
- Configuration structure and schema
- Execution environment and variable passing
- Error handling and debugging
- Best practices
- Common use cases (linting, testing, validation, archival, CI)
- Shell scripting best practices
- Troubleshooting guide

**When to read:**
- Learning how hooks work
- Understanding exit codes and execution flow
- Debugging hooks that aren't behaving as expected
- Best practices for hook design

**Sections:**
- Overview
- Hook Types (PreToolUse, PostToolUse, SessionStart)
- Configuration Structure
- Execution Environment
- Variable Passing Strategies
- Error Handling
- Debugging Hooks
- Best Practices
- Hook Composition Patterns
- Common Use Cases & Templates
- Shell Scripting Best Practices
- Troubleshooting
- Reference: ANDON Gate Hook
- Summary

**File size:** ~24 KB, 1200+ lines

---

### HOOKS_TEMPLATES.md

**Copy-paste ready** hook configurations for 15 common scenarios.

**What it covers:**
15 production-ready templates:
1. ANDON Gate (Λ_CD^runtime)
2. Pre-commit Formatting & Linting
3. Run Test Suite After Changes
4. Validate Configuration on Session Start
5. Check Dependencies Before Bash
6. Archive Build Artifacts on Success
7. Verify Tests Pass Before Release
8. Generate Documentation After Writes
9. Lint Markdown & Documentation
10. Enforce Commit Message Policy
11. Synchronize State Across Workspaces
12. Email/Slack Notification on Success
13. Create Session Log File
14. Check Disk Space Before Large Operations
15. Run Workspace Health Check

**When to read:**
- Looking for a ready-to-use hook for your use case
- Need a starting point for customization
- Want examples of different hook patterns
- Building a combined hooks configuration

**Each template includes:**
- JSON configuration
- Plain English description
- What it does
- Customization options
- Performance notes or tips

**File size:** ~21 KB, 800+ lines

---

### HOOKS_ADVANCED.md

**Advanced patterns, debugging**, and integration strategies.

**What it covers:**
- 7 advanced patterns:
  1. Conditional gates based on branch
  2. Tiered validation (quick → full)
  3. Parallel hook execution
  4. Retry logic for flaky operations
  5. Fallback strategies
  6. Contextual data passing
  7. Adaptive behavior per environment

- 6 debugging techniques:
  1. Hook execution tracing
  2. Health monitoring
  3. Failure analysis with diagnostics
  4. Timing & performance analysis
  5. Interactive debugging
  6. Logging & audit trails

- Integration patterns:
  1. GitHub Actions + Hooks
  2. Pre-commit framework
  3. LSP server gates
  4. Database/service validation

- Troubleshooting advanced scenarios
- Performance optimization techniques

**When to read:**
- Your hooks need conditional logic
- Debugging complex hook behavior
- Integrating hooks with external systems (CI, version control)
- Optimizing hook performance
- Understanding hook composition

**File size:** ~18 KB, 900+ lines

---

## By Use Case

### I want to enforce quality standards (formatting, linting)

**Start here:**
1. HOOKS_TEMPLATES.md — Template 2 (Pre-commit Formatting & Linting)
2. HOOKS_GUIDE.md — "Best Practices" section
3. HOOKS_ADVANCED.md — Pattern 2 (Tiered Validation)

**Key concepts:**
- PreToolUse hooks block edits if checks fail
- Trade-off: speed vs. strictness
- Solution: Use PostToolUse for non-blocking checks

---

### I want to run tests after changes

**Start here:**
1. HOOKS_TEMPLATES.md — Template 3 (Run Test Suite After Changes)
2. HOOKS_GUIDE.md — "PostToolUse Hooks" section
3. HOOKS_ADVANCED.md — Pattern 2 (Tiered Validation)

**Key concepts:**
- PostToolUse hooks run non-blocking
- Use `&` to background long operations
- Save results to files for later inspection

---

### I want to implement a gate (ANDON, conformance)

**Start here:**
1. HOOKS_TEMPLATES.md — Template 1 (ANDON Gate)
2. HOOKS_GUIDE.md — "PreToolUse Hooks" and "Reference: ANDON Gate Hook"
3. HOOKS_ADVANCED.md — Integration Pattern 3 (LSP Server Gates)

**Key concepts:**
- Gates block operations until conditions are met
- Exit code 0 = proceed; exit 1 = block
- Fast checks only (target <500ms)

---

### My hooks are not working; I need to debug

**Start here:**
1. HOOKS_GUIDE.md — "Debugging Hooks" section
2. HOOKS_GUIDE.md — "Troubleshooting" section
3. HOOKS_ADVANCED.md — "Advanced Debugging Techniques"

**Key concepts:**
- Test hooks locally before integrating
- Use `bash -x` to trace execution
- Check exit codes carefully
- Capture output to temporary files

---

### I want to integrate hooks with CI/CD

**Start here:**
1. HOOKS_TEMPLATES.md — Template 7 (Verify Tests Pass Before Release)
2. HOOKS_ADVANCED.md — "Integration Patterns" section
3. HOOKS_ADVANCED.md — Integration Pattern 1 (GitHub Actions + Hooks)

**Key concepts:**
- Same checks locally and in CI
- Prevent "works locally" surprises
- Use environment detection for conditional behavior

---

### I want to set up a complete workspace validation

**Start here:**
1. HOOKS_TEMPLATES.md — Template 4 (Validate Configuration on Session Start)
2. HOOKS_TEMPLATES.md — Template 15 (Run Workspace Health Check)
3. HOOKS_ADVANCED.md — Pattern 1 (Conditional Gates Based on Branch)

**Key concepts:**
- SessionStart hooks run once per session
- Can perform multi-step initialization
- Blocks session if validation fails

---

### I want to improve hook performance

**Start here:**
1. HOOKS_GUIDE.md — "Best Practices" (Keep Hooks Fast)
2. HOOKS_ADVANCED.md — "Performance Optimization" section
3. HOOKS_ADVANCED.md — Pattern 2 (Tiered Validation)

**Key concepts:**
- PreToolUse target: <500ms
- Parallelization can reduce time
- Caching frequently repeated checks
- Background PostToolUse operations

---

## Configuration Reference

### Minimal Configuration

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Hook running'"
          }
        ]
      }
    ]
  }
}
```

### Full Configuration Structure

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "initialization-command"
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Tool1|Tool2|Tool3",
        "hooks": [
          {
            "type": "command",
            "command": "check-command"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Tool1|Tool2",
        "hooks": [
          {
            "type": "command",
            "command": "post-action-command"
          }
        ]
      }
    ]
  }
}
```

---

## Hook Type Quick Reference

| Hook Type | Timing | Blocks Tool | Use Case |
|-----------|--------|-------------|----------|
| **PreToolUse** | Before tool | Yes | Gates, validation, permission checks |
| **PostToolUse** | After tool | No | Observability, archival, snapshots |
| **SessionStart** | On session init | Yes | Environment setup, pre-flight checks |

---

## Common Hook Commands

### Gate Checks
```bash
lsp-max-cli gate check              # Check ANDON gate
lsp-max-cli diagnostic list         # List active diagnostics
lsp-max-cli diagnostic snapshot      # Capture state snapshot
```

### Build & Lint
```bash
cargo check --workspace             # Fast syntax check
cargo fmt --check                   # Check formatting
cargo clippy --all-targets -- -D warnings  # Lint
cargo test --workspace              # Full test suite
```

### Quality Assurance
```bash
just dx-verify                      # Architecture verification
just dx-polish                      # Format + lint
```

### Git & Version Control
```bash
git branch --show-current           # Get current branch
git rev-parse --short HEAD          # Get short commit hash
git status --short                  # Check working directory
```

### File Validation
```bash
[ -f file.json ]                    # Check file exists
cat file.json | jq .                # Validate JSON
cargo metadata --format-version 1   # Validate Cargo.toml
```

---

## File Organization

```
.claude/
├── settings.json          # Hook configuration
├── HOOKS_GUIDE.md         # Main documentation (this directory)
├── HOOKS_TEMPLATES.md     # Copy-paste templates
├── HOOKS_ADVANCED.md      # Advanced patterns & debugging
├── HOOKS_INDEX.md         # This file
├── hook-lib.sh            # Shared hook functions (optional)
├── tiered-check.sh        # Example: multi-tier validation
├── health-check.sh        # Example: workspace validation
└── logs/
    └── hooks.log          # Hook execution log (generated)
```

---

## Creating Helper Scripts

For complex hooks, create a reusable script:

```bash
# .claude/hook-lib.sh
check_gate() {
    lsp-max-cli gate check
}

run_fast_checks() {
    cargo check --workspace && cargo fmt --check
}

run_full_suite() {
    cargo test --workspace && just dx-verify
}
```

Then reference in settings.json:

```json
{
  "type": "command",
  "command": "source .claude/hook-lib.sh && check_gate"
}
```

---

## Frequently Asked Questions

**Q: Can I pass tool parameters to hooks?**  
A: Directly, no. Hooks receive no parameters. Workaround: inspect filesystem for recently modified files.

**Q: Do hooks run in subagents?**  
A: No. Hooks are specific to the current session. Subagents inherit no hooks and run in isolation.

**Q: How long can a PreToolUse hook take?**  
A: Target <500ms. Typical timeout is 30 seconds, but user experience degrades significantly above 5 seconds.

**Q: Can hooks read environment variables?**  
A: Yes. Hooks inherit the user's environment and can source `.env` files.

**Q: What if a hook times out?**  
A: Treated as failure (exit 1). PreToolUse hook failure blocks the tool.

**Q: Can I disable a hook temporarily?**  
A: Modify settings.json to remove/comment the hook, or change the command to `exit 0` (always allow).

**Q: How do I test hooks locally?**  
A: Run the exact command from your shell: `cd /workspace && bash ./.claude/my-hook.sh && echo "Exit: $?"`

---

## Next Steps

1. **Pick a template** from HOOKS_TEMPLATES.md that matches your use case
2. **Customize it** for your project (tool names, file paths, commands)
3. **Test locally** using the exact command from HOOKS_GUIDE.md "Debugging" section
4. **Add to .claude/settings.json** and verify it works
5. **Document the purpose** in a companion comment or `.claude/HOOKS_README.md`

---

## See Also

- `.claude/settings.json` — Your actual hook configuration
- `CLAUDE.md` — Project-specific hooks and guidelines
- `AGENTS.md` — Agent-specific hook considerations
- Claude Code documentation — General tool and session information

---

**Last Updated:** 2026-06-14  
**Version:** Complete Reference  
**Scope:** Claude Code Hooks System (all types, configurations, patterns, debugging)
