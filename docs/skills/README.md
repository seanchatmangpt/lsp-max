# Claude Code Skills Documentation

**Version:** 26.6.9 | **Status:** ADMITTED | **Directory:** `/docs/skills/`

This directory contains detailed documentation for each Claude Code skill. Start here to find the skill you need or understand how to use a particular skill effectively.

---

## Quick Navigation

### By Use Case

**I want to run and test my app:**
- [`/run`](SKILL_RUN.md) — Launch the app
- [`/verify`](SKILL_VERIFY.md) — Validate behavior and generate receipt
- [`/loop`](SKILL_LOOP.md) — Repeat checks on interval

**I want to review and improve code:**
- [`/code-review`](SKILL_CODE_REVIEW.md) — Find bugs and inefficiencies
- [`/simplify`](SKILL_SIMPLIFY.md) — Refactor for clarity and reuse
- [`/security-review`](SKILL_SECURITY_REVIEW.md) — Identify vulnerabilities
- [`/review`](SKILL_REVIEW.md) — Comprehensive PR review

**I want to configure my environment:**
- [`/init`](SKILL_INIT.md) — Initialize CLAUDE.md
- [`/update-config`](SKILL_UPDATE_CONFIG.md) — Manage settings and permissions
- [`/keybindings-help`](SKILL_KEYBINDINGS_HELP.md) — Customize keyboard shortcuts
- [`/session-start-hook`](SKILL_SESSION_START_HOOK.md) — Setup web session hooks
- [`/fewer-permission-prompts`](SKILL_FEWER_PERMISSION_PROMPTS.md) — Reduce dialogs

**I want to research or learn:**
- [`/deep-research`](SKILL_DEEP_RESEARCH.md) — Multi-source fact-checked research
- [`/claude-api`](SKILL_CLAUDE_API.md) — Claude API reference

---

## Skill Directory

All 14 skills documented:

### Execution & Validation

| Skill | Purpose | Quick Link |
|-------|---------|-----------|
| `/run` | Launch app | [SKILL_RUN.md](SKILL_RUN.md) |
| `/verify` | Validate behavior with receipt | [SKILL_VERIFY.md](SKILL_VERIFY.md) |
| `/loop` | Recurring task automation | [SKILL_LOOP.md](SKILL_LOOP.md) |

### Code Quality

| Skill | Purpose | Quick Link |
|-------|---------|-----------|
| `/code-review` | Find bugs and inefficiencies | [SKILL_CODE_REVIEW.md](SKILL_CODE_REVIEW.md) |
| `/simplify` | Refactor for clarity | [SKILL_SIMPLIFY.md](SKILL_SIMPLIFY.md) |
| `/security-review` | Identify vulnerabilities | [SKILL_SECURITY_REVIEW.md](SKILL_SECURITY_REVIEW.md) |
| `/review` | PR-level comprehensive review | [SKILL_REVIEW.md](SKILL_REVIEW.md) |

### Configuration

| Skill | Purpose | Quick Link |
|-------|---------|-----------|
| `/init` | Initialize CLAUDE.md | [SKILL_INIT.md](SKILL_INIT.md) |
| `/update-config` | Manage settings/permissions | [SKILL_UPDATE_CONFIG.md](SKILL_UPDATE_CONFIG.md) |
| `/keybindings-help` | Customize keyboard | [SKILL_KEYBINDINGS_HELP.md](SKILL_KEYBINDINGS_HELP.md) |
| `/session-start-hook` | Web session hooks | [SKILL_SESSION_START_HOOK.md](SKILL_SESSION_START_HOOK.md) |
| `/fewer-permission-prompts` | Reduce permission dialogs | [SKILL_FEWER_PERMISSION_PROMPTS.md](SKILL_FEWER_PERMISSION_PROMPTS.md) |

### Research & Reference

| Skill | Purpose | Quick Link |
|-------|---------|-----------|
| `/deep-research` | Multi-source research | [SKILL_DEEP_RESEARCH.md](SKILL_DEEP_RESEARCH.md) |
| `/claude-api` | API reference | [SKILL_CLAUDE_API.md](SKILL_CLAUDE_API.md) |

---

## Common Workflows

### Workflow 1: Feature Development & Validation

```bash
/run                    # Launch app
# (manual testing)
/verify                 # Validate feature works
/code-review --fix      # Find bugs and auto-fix
/verify                 # Re-test after fixes
/simplify               # Clean up code
/security-review        # Audit for security
# (commit and push)
```

See: [SKILL_RUN.md](SKILL_RUN.md) → [SKILL_VERIFY.md](SKILL_VERIFY.md) → [SKILL_CODE_REVIEW.md](SKILL_CODE_REVIEW.md)

### Workflow 2: Pull Request Review

```bash
/verify                 # Test PR works
/code-review --comment  # Find issues + post comments
/security-review        # Security audit
/review --approve       # Approve if clear
# (merge)
```

See: [SKILL_VERIFY.md](SKILL_VERIFY.md) → [SKILL_REVIEW.md](SKILL_REVIEW.md)

### Workflow 3: Project Onboarding

```bash
/init                           # Create CLAUDE.md
/update-config "allow npm"      # Set permissions
/session-start-hook             # Web session hooks
/fewer-permission-prompts       # Reduce dialogs
/keybindings-help               # Customize keyboard
```

See: [SKILL_INIT.md](SKILL_INIT.md) → [SKILL_UPDATE_CONFIG.md](SKILL_UPDATE_CONFIG.md)

### Workflow 4: Continuous Monitoring

```bash
/loop 5m /verify        # Poll every 5 minutes
# (Ctrl+C to stop)
```

See: [SKILL_LOOP.md](SKILL_LOOP.md)

### Workflow 5: Research-Driven Development

```bash
/deep-research "Is library X secure?"
/claude-api "What models are available?"
# (code based on research)
/verify                 # Validate
/code-review --fix      # Improve code
```

See: [SKILL_DEEP_RESEARCH.md](SKILL_DEEP_RESEARCH.md) → [SKILL_CLAUDE_API.md](SKILL_CLAUDE_API.md)

---

## Skill Statuses

All skills in this directory are **AVAILABLE** and production-ready unless otherwise noted.

| Status | Meaning | Action |
|--------|---------|--------|
| AVAILABLE | Fully functional, documented | Use freely |
| CANDIDATE | Experimental, may change | Use with caution; feedback welcome |
| BLOCKED | Unavailable | Check prerequisites |
| PARTIAL | Some features available | Check docs for limitations |

---

## Key Concepts

### Receipt (Verification Evidence)

Skills like `/verify` generate **receipts** with status:
- **ADMITTED** — Change works as expected
- **REFUSED** — Unexpected behavior detected
- **UNKNOWN** — Inconclusive result; requires investigation

Example receipt:
```
✅ Receipt: ADMITTED
  Changed: src/auth.rs
  Tests: 8 passed, 0 failed
  Status: Ready to proceed
```

### Auto-fixes (Code Modifications)

Skills like `/code-review --fix` and `/simplify` modify your code by creating **new commits** (never amending):
- Safe to revert with `git revert`
- Always review before pushing
- Creates commit message with summary

### Authority Levels

- **AUTHORITATIVE** — Single source of truth (e.g., `/update-config` for settings)
- **HIGH** — High-confidence findings (e.g., `/verify` for behavior validation)
- **MEDIUM** — Requires human judgment (e.g., `/review` for PR approval)
- **ADVISORY** — Informational (e.g., `/deep-research`)

---

## Best Practices

✓ **Do:**
- Run `/verify` before `/code-review` (verify works, then review code)
- Use skills in order: `run` → `verify` → `code-review` → `simplify` → `security-review`
- Post comments with `--comment` in PR workflows
- Use `--fix` for safe auto-fixes; review before pushing
- Use `/loop` for polling, not tight loops

✗ **Don't:**
- Skip `/verify` before code review
- Use `git commit --amend` after skill auto-fixes (breaks history)
- Ignore `/security-review` findings
- Use `/run` for validation (use `/verify` instead)
- Use `/code-review` for security (use `/security-review` instead)

---

## Troubleshooting

**Skill not working?**
1. Check skill documentation for prerequisites
2. Verify project type is detected correctly
3. Check `.claude/settings.json` for configuration issues
4. Run `/update-config` to validate setup

**Unexpected findings?**
1. Review the skill's output carefully
2. Check if effort level is appropriate
3. Read the suggested fixes or changes
4. Use `/verify` to validate after applying fixes

**Permission prompt?**
1. Run `/fewer-permission-prompts` to reduce dialogs
2. Or use `/update-config` to pre-approve specific tools

---

## File Structure

```
docs/skills/
├── README.md                        (this file)
├── SKILL_RUN.md                    (launch app)
├── SKILL_VERIFY.md                 (validate behavior)
├── SKILL_CODE_REVIEW.md            (find bugs)
├── SKILL_SIMPLIFY.md               (refactor)
├── SKILL_SECURITY_REVIEW.md        (security audit)
├── SKILL_REVIEW.md                 (PR review)
├── SKILL_LOOP.md                   (recurring tasks)
├── SKILL_INIT.md                   (initialize CLAUDE.md)
├── SKILL_UPDATE_CONFIG.md          (configure settings)
├── SKILL_KEYBINDINGS_HELP.md       (customize keyboard)
├── SKILL_SESSION_START_HOOK.md     (web session hooks)
├── SKILL_FEWER_PERMISSION_PROMPTS.md (reduce dialogs)
├── SKILL_DEEP_RESEARCH.md          (multi-source research)
└── SKILL_CLAUDE_API.md             (API reference)
```

---

## Related Documentation

- **[SKILLS_REGISTRY.md](../SKILLS_REGISTRY.md)** — Comprehensive registry with all details
- **[SKILLS_TAXONOMY.md](../SKILLS_TAXONOMY.md)** — Organized by category, dependency, and decision matrix
- **[CLAUDE.md](../CLAUDE.md)** — Project constitution (required reading)
- **[AGENTS.md](../AGENTS.md)** — Agent architecture

---

## Questions?

Each skill documentation includes:
- **When to Use** — Right and wrong times to invoke
- **Parameters** — What arguments to pass
- **How It Works** — Step-by-step execution
- **Examples** — Real-world usage scenarios
- **Troubleshooting** — Common issues and fixes
- **Integration Points** — How skills work together
- **See Also** — Related skills and docs

---

## Contributing

To improve skill documentation:
1. Edit the relevant `SKILL_*.md` file
2. Update status if needed
3. Add examples if discovering new patterns
4. Submit PR with improvements

---

**Last Updated:** 2026-06-14 | **Maintained by:** Claude Code Skills Registry
