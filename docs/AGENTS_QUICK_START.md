# Quick Start: lsp-max-anti-cheat for Claude Code Agents

**TL;DR** — Prevent LLM-generated anti-patterns in 5 minutes.

---

## Install

```bash
cargo install --path crates/lsp-max-anti-cheat-cli
# Or: cargo install lsp-max-anti-cheat-cli
```

---

## Use

```bash
# Check your project for violations
lsp-max-anti-cheat check all --path .

# Exit code: 0 = clean, 1 = violations
```

---

## Integrate (Pick One)

### Option A: Pre-Command Hook (Blocking)

Add to `.claude/settings.json`:
```json
{
  "hooks": {
    "PreToolUse": {
      "command": "lsp-max-anti-cheat check all --path . >/dev/null 2>&1",
      "blocking": true
    }
  }
}
```

**Effect**: Bash commands are blocked if violations exist.

### Option B: Pre-Commit Hook (Git)

Add to `.git/hooks/pre-commit`:
```bash
#!/bin/bash
lsp-max-anti-cheat check all --path . || exit 1
```

**Effect**: Commits are blocked if violations exist.

### Option C: CI/CD Gate (GitHub Actions)

```yaml
- name: Anti-LLM Check
  run: cargo run -p lsp-max-anti-cheat-cli -- check all --path .
```

**Effect**: PRs fail if violations exist.

---

## Common Violations & Fixes

| Violation | Fix |
|-----------|-----|
| `tower_lsp` reference | Replace with `lsp_max` |
| "done", "solved", "guaranteed" | Remove term or add to config |
| Test stdout as receipt | Add BLAKE3 receipt artifact |
| `1.0.0` version in CalVer project | Update to `YY.M.D` |
| Hardcoded metrics | Parameterize from config |

---

## Configuration (Optional)

Create `anti-llm.toml`:

```toml
[claim]
# Terms that are OK in your project
domain_terms = ["fully admitted"]

[surface]
# Paths where tower-lsp refs are warnings, not errors
non_blocking_path_prefixes = ["docs/archive/"]
```

---

## Verify

```bash
# List all rules
lsp-max-anti-cheat rules list

# Get detailed info about a rule
lsp-max-anti-cheat rules describe --code ANTI-LLM-SURFACE-001

# Scan and see raw observations
lsp-max-anti-cheat scan directory --path .
```

---

## Exit Codes

| Code | Meaning | Agent Action |
|------|---------|--------------|
| **0** | No violations | Proceed (commit, push, release) |
| **1** | Violations found | Fix before proceeding |

---

## Troubleshooting

**Q: "Command not found"**
```bash
# Re-install
cargo install --path crates/lsp-max-anti-cheat-cli
```

**Q: "Exit code 1 but no violations shown"**
```bash
# Run without redirection
lsp-max-anti-cheat check all --path . | jq '.diagnostics'
```

**Q: "Config error"**
```bash
# Re-initialize
lsp-max-anti-cheat config init --path .
```

---

## Parse Output (Agents)

```bash
# Exit code
lsp-max-anti-cheat check all --path .
echo $?  # 0 or 1

# JSON parsing
result=$(lsp-max-anti-cheat check all --path .)
echo "$result" | jq '.summary.blocking'  # Count violations
echo "$result" | jq '.diagnostics[].code'  # List violation codes
```

---

## Status

**Ready for production use**: CANDIDATE (awaiting sibling repo deps for full compilation)

**Integrates with**: 
- ✅ Claude Code agents (hooks)
- ✅ Git pre-commit hooks
- ✅ GitHub Actions
- ✅ GitLab CI
- ✅ Custom tools (JSON output)

---

## Full Documentation

- **[Getting Started](./TUTORIAL_getting_started.md)** — Complete walkthrough
- **[Agent Integration](./TUTORIAL_agent_integration.md)** — Hooks, subprocess, auto-fix
- **[CLI Reference](./REFERENCE_cli_commands.md)** — All commands
- **[Overview](./AGENTS_ANTI_CHEAT_INTEGRATION.md)** — Full Diataxis guide

---

**Ready?** Run `lsp-max-anti-cheat check all --path .` now.
