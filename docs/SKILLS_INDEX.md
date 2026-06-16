# Claude Code Skills Index

**Version:** 26.6.9 | **Generated:** 2026-06-14 | **Status:** ADMITTED

Complete index and navigation guide for the Claude Code Skills Registry. This document provides a quick overview and links to all skill documentation, reference materials, and guides.

---

## 📚 Main Registry Documents

### [SKILLS_REGISTRY.md](SKILLS_REGISTRY.md)
**Comprehensive authoritative registry of all 14 skills**
- Full documentation for each skill
- Parameters, trigger patterns, and use cases
- Integration patterns and best practices
- Skill lifecycle and maturity levels
- **Status:** ADMITTED | **Scope:** Complete reference

### [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md)
**Organization of skills by category, lifecycle, and decision logic**
- Organized by 10 different dimensions (lifecycle, problem domain, autonomy, etc.)
- Decision matrices for skill selection
- Dependency graphs and interaction patterns
- Conflict and complementary skill combinations
- **Status:** ADMITTED | **Scope:** Classification and discovery

### [skills/README.md](skills/README.md)
**Directory guide to individual skill documentation**
- Quick navigation by use case
- Skill directory with status
- Common workflows and examples
- Troubleshooting and best practices
- **Status:** ADMITTED | **Scope:** Quick reference

---

## 🎯 Individual Skill Documentation

### Execution & Validation (3 skills)

| Skill | Purpose | Doc | Status |
|-------|---------|-----|--------|
| **`/run`** | Launch app | [SKILL_RUN.md](skills/SKILL_RUN.md) | AVAILABLE |
| **`/verify`** | Validate behavior with receipt | [SKILL_VERIFY.md](skills/SKILL_VERIFY.md) | AVAILABLE |
| **`/loop`** | Recurring task automation | [SKILL_LOOP.md](skills/SKILL_LOOP.md) | AVAILABLE |

### Code Quality & Review (4 skills)

| Skill | Purpose | Doc | Status |
|-------|---------|-----|--------|
| **`/code-review`** | Find bugs and inefficiencies | [SKILL_CODE_REVIEW.md](skills/SKILL_CODE_REVIEW.md) | AVAILABLE |
| **`/simplify`** | Refactor for clarity | [SKILL_SIMPLIFY.md](skills/SKILL_SIMPLIFY.md) | AVAILABLE |
| **`/security-review`** | Identify vulnerabilities | [SKILL_SECURITY_REVIEW.md](skills/SKILL_SECURITY_REVIEW.md) | AVAILABLE |
| **`/review`** | PR-level comprehensive review | [SKILL_REVIEW.md](skills/SKILL_REVIEW.md) | AVAILABLE |

### Configuration & Setup (5 skills)

| Skill | Purpose | Doc | Status |
|-------|---------|-----|--------|
| **`/init`** | Initialize CLAUDE.md | [SKILL_INIT.md](skills/SKILL_INIT.md) | AVAILABLE |
| **`/update-config`** | Manage settings/permissions | [SKILL_UPDATE_CONFIG.md](skills/SKILL_UPDATE_CONFIG.md) | AVAILABLE |
| **`/keybindings-help`** | Customize keyboard | [SKILL_KEYBINDINGS_HELP.md](skills/SKILL_KEYBINDINGS_HELP.md) | AVAILABLE |
| **`/session-start-hook`** | Web session hooks | [SKILL_SESSION_START_HOOK.md](skills/SKILL_SESSION_START_HOOK.md) | AVAILABLE |
| **`/fewer-permission-prompts`** | Reduce dialogs | [SKILL_FEWER_PERMISSION_PROMPTS.md](skills/SKILL_FEWER_PERMISSION_PROMPTS.md) | AVAILABLE |

### Research & Reference (2 skills)

| Skill | Purpose | Doc | Status |
|-------|---------|-----|--------|
| **`/deep-research`** | Multi-source fact-checked research | [SKILL_DEEP_RESEARCH.md](skills/SKILL_DEEP_RESEARCH.md) | AVAILABLE |
| **`/claude-api`** | Claude API reference | [SKILL_CLAUDE_API.md](skills/SKILL_CLAUDE_API.md) | AVAILABLE |

---

## 🗺️ Navigation by Use Case

### I want to run and test my app

**Workflow:** Launch → Validate → Improve

1. [`/run`](skills/SKILL_RUN.md) — Launch the app
2. [`/verify`](skills/SKILL_VERIFY.md) — Validate behavior
3. [`/loop`](skills/SKILL_LOOP.md) — Repeat checks (optional)

**Also see:** [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md#i-want-to-see-if-it-works)

---

### I want to review and improve code

**Workflow:** Verify → Review → Simplify → Audit → Approve

1. [`/verify`](skills/SKILL_VERIFY.md) — Test it works
2. [`/code-review`](skills/SKILL_CODE_REVIEW.md) — Find bugs
3. [`/simplify`](skills/SKILL_SIMPLIFY.md) — Refactor (optional)
4. [`/security-review`](skills/SKILL_SECURITY_REVIEW.md) — Security audit
5. [`/review`](skills/SKILL_REVIEW.md) — PR approval

**Also see:** [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md#i-want-to-review-a-pr)

---

### I want to configure my environment

**Workflow:** Initialize → Configure → Optimize

1. [`/init`](skills/SKILL_INIT.md) — Create CLAUDE.md
2. [`/update-config`](skills/SKILL_UPDATE_CONFIG.md) — Manage settings
3. [`/session-start-hook`](skills/SKILL_SESSION_START_HOOK.md) — Setup hooks
4. [`/keybindings-help`](skills/SKILL_KEYBINDINGS_HELP.md) — Customize keyboard
5. [`/fewer-permission-prompts`](skills/SKILL_FEWER_PERMISSION_PROMPTS.md) — Reduce dialogs

**Also see:** [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md#i-want-to-set-this-up)

---

### I want to research or learn

**Workflow:** Research → Learn → Apply

1. [`/deep-research`](skills/SKILL_DEEP_RESEARCH.md) — Multi-source research
2. [`/claude-api`](skills/SKILL_CLAUDE_API.md) — API reference (if Claude-related)

**Also see:** [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md#i-need-to-research-something)

---

### I want to poll for status

**Workflow:** Setup → Monitor

1. [`/loop`](skills/SKILL_LOOP.md) — Recurring automation
   - Wraps `/verify` for polling behavior
   - Wraps `/run` for restarting on interval
   - Works with any skill

**Also see:** [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md#i-want-to-keep-checking-status)

---

## 📊 Quick Decision Matrix

**Choose the right skill in seconds:**

| Question | Skill | Why |
|----------|-------|-----|
| "Run my app?" | `/run` | Launches with auto-detection |
| "Does it work?" | `/verify` | Generates receipt with evidence |
| "Find bugs?" | `/code-review` | Comprehensive analysis |
| "Check security?" | `/security-review` | Vulnerability audit |
| "Refactor?" | `/simplify` | Auto-refactoring (no bugs) |
| "Review PR?" | `/review` | PR-level comprehensive audit |
| "Setup project?" | `/init` | Create CLAUDE.md |
| "Configure?" | `/update-config` | Permissions, env, hooks |
| "Research?" | `/deep-research` | Multi-source fact-check |
| "API docs?" | `/claude-api` | Official reference |
| "Repeat?" | `/loop` | Polling/recurring |

**Full decision matrix:** [SKILLS_TAXONOMY.md § Decision Matrix](SKILLS_TAXONOMY.md#taxonomy-7-decision-matrix)

---

## 🔗 Integration & Workflow Chains

### Standard Development Workflow

```
/run → /verify → /code-review --fix → /verify → /simplify → /security-review → commit
```

**Detailed guide:** [SKILLS_REGISTRY.md § Integration Pattern 1](SKILLS_REGISTRY.md#pattern-1-development-cycle)

---

### Pull Request Review Workflow

```
/verify → /code-review --comment → /security-review → /review --approve
```

**Detailed guide:** [SKILLS_REGISTRY.md § Integration Pattern 2](SKILLS_REGISTRY.md#pattern-2-code-review-process)

---

### Project Onboarding Workflow

```
/init → /session-start-hook → /update-config → /fewer-permission-prompts
```

**Detailed guide:** [SKILLS_REGISTRY.md § Integration Pattern 4](SKILLS_REGISTRY.md#pattern-4-configuration--setup)

---

### Continuous Monitoring Workflow

```
/loop 5m /verify
(Ctrl+C to stop)
```

**Detailed guide:** [SKILLS_REGISTRY.md § Integration Pattern 3](SKILLS_REGISTRY.md#pattern-3-continuous-monitoring)

---

## 📋 Skills by Execution Context

When to invoke skills based on project stage:

### Before Coding
- `/init` — Initialize project docs
- `/session-start-hook` — Setup web session hooks
- `/update-config` — Configure environment

### During Development
- `/run` — Launch and observe app
- `/verify` — Validate behavior
- `/loop` — Repeat checks on interval

### Before Committing
- `/code-review` — Find bugs and inefficiencies
- `/simplify` — Refactor for clarity
- `/security-review` — Audit security

### Before Merging
- `/review` — Comprehensive PR review
- `/verify` — Final behavior check
- `/security-review` — Final security audit

### For Research
- `/deep-research` — Multi-source fact-checking
- `/claude-api` — API reference (Claude-specific)

### For Configuration
- `/update-config` — Manage settings
- `/keybindings-help` — Customize keyboard
- `/fewer-permission-prompts` — Reduce prompts

**Detailed lifecycle:** [SKILLS_TAXONOMY.md § By Lifecycle Stage](SKILLS_TAXONOMY.md#taxonomy-1-by-lifecycle-stage)

---

## ✅ Best Practices

### Do's

✓ Verify before reviewing (run `/verify` before `/code-review`)  
✓ Follow the chain: `run` → `verify` → `code-review` → `simplify` → `security-review`  
✓ Post comments with `--comment` in PR workflows  
✓ Use `--fix` for safe auto-fixes; review before pushing  
✓ Use `/loop` for polling, not tight loops  
✓ Use `low` effort first in `/code-review`, increase if needed  

### Don'ts

✗ Skip `/verify` before code review  
✗ Use `git commit --amend` after skill auto-fixes (breaks history)  
✗ Ignore `/security-review` findings  
✗ Use `/run` for validation (use `/verify` instead)  
✗ Use `/code-review` for security (use `/security-review` instead)  
✗ Assume victory language ("done", "solved"); use bounded statuses only  

**Full best practices:** [SKILLS_REGISTRY.md § Best Practices](SKILLS_REGISTRY.md#best-practices)

---

## 🆘 Troubleshooting

### Skill not working?

1. Check skill documentation for prerequisites
2. Verify project type is detected correctly
3. Check `.claude/settings.json` for configuration issues
4. Run `/update-config` to validate setup

**Troubleshooting guide:** [skills/README.md § Troubleshooting](skills/README.md#troubleshooting)

---

### Permission prompt appearing?

1. Run `/fewer-permission-prompts` to auto-generate safe allowlist
2. Or use `/update-config` to pre-approve specific tools

**Permission management:** [SKILL_UPDATE_CONFIG.md](skills/SKILL_UPDATE_CONFIG.md)

---

### Unexpected findings in code review?

1. Review the skill's output carefully
2. Check if effort level is appropriate (`/code-review low/medium/high/max`)
3. Read the suggested fixes or changes
4. Use `/verify` to validate after applying fixes

**Code review guide:** [SKILL_CODE_REVIEW.md § Troubleshooting](skills/SKILL_CODE_REVIEW.md#troubleshooting)

---

## 📖 Related Documentation

- **[CLAUDE.md](CLAUDE.md)** — Project constitution (required reading)
- **[AGENTS.md](AGENTS.md)** — Agent architecture and enforcement
- **[FEATURES.md](FEATURES.md)** — LSP 3.18 feature coverage
- **[TEST_INFRA.md](TEST_INFRA.md)** — Testing infrastructure and patterns
- **[ROADMAP.md](ROADMAP.md)** — Future direction

---

## 📊 Registry Statistics

| Metric | Count |
|--------|-------|
| **Total Skills** | 14 |
| **Fully Documented Skills** | 14 |
| **Categories** | 4 |
| **Status: AVAILABLE** | 14 |
| **Integration Patterns** | 5 |
| **Taxonomy Dimensions** | 10 |
| **Total Pages** | 20+ |

---

## 🎓 Learning Path

**Recommended progression for new users:**

1. **Start here:** [skills/README.md](skills/README.md) (5 min)
2. **Learn your workflow:** Pick matching use case above (5-10 min)
3. **Deep dive:** Read individual skill docs (10-30 min per skill)
4. **Reference:** Use taxonomy for decisions ([SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md))
5. **Master:** Use full registry for comprehensive details ([SKILLS_REGISTRY.md](SKILLS_REGISTRY.md))

---

## 🔄 Versioning

| Version | Date | Status | Changes |
|---------|------|--------|---------|
| **26.6.9** | 2026-06-14 | ADMITTED | Initial registry creation |

Skills version independently. Check individual skill docs for per-skill versioning.

---

## 📝 Contributing

To improve skill documentation:

1. Edit the relevant `SKILL_*.md` file in `/docs/skills/`
2. Update status if needed
3. Add examples if discovering new patterns
4. Update this index if adding new sections
5. Submit PR with improvements

---

## 🔗 Quick Links

| Resource | Purpose | Link |
|----------|---------|------|
| **Main Registry** | Comprehensive skill reference | [SKILLS_REGISTRY.md](SKILLS_REGISTRY.md) |
| **Taxonomy** | Skill organization & discovery | [SKILLS_TAXONOMY.md](SKILLS_TAXONOMY.md) |
| **Skills Directory** | Quick reference & navigation | [skills/README.md](skills/README.md) |
| **Individual Skills** | Full documentation per skill | [skills/](skills/) |

---

## ✨ Summary

The Claude Code Skills Registry provides:

- **14 production-ready skills** across 4 categories
- **20+ pages of documentation** with examples and workflows
- **10-dimensional taxonomy** for skill discovery and selection
- **Integration patterns** for common workflows
- **Best practices** and troubleshooting guides
- **Quick reference** tools and decision matrices

**Status:** ADMITTED — Registry is complete, authoritative, and ready for use.

---

**Generated:** 2026-06-14 | **Version:** 26.6.9 | **Maintained by:** Claude Code Skills Registry
