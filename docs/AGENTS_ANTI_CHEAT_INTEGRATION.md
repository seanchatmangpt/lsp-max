# Claude Code Agents & lsp-max-anti-cheat: Integration Lifecycle

Complete Diataxis documentation for agents using anti-cheat compliance tools across development, CI, and release.

## Quick Navigation

### 📚 **Tutorials** (Learning)
- [Getting Started with Anti-Cheat CLI](./TUTORIAL_getting_started.md) — First scan, interpreting output, fixing violations
- [Agent Integration Tutorial](./TUTORIAL_agent_integration.md) — Hook setup, subprocess dispatch, error handling

### 🎯 **How-To Guides** (Tasks)
- [How to Check for Tower-LSP References](./HOWTO_tower_lsp_check.md)
- [How to Scan Victory Language](./HOWTO_victory_language.md)
- [How to Set Up Pre-Command Hooks](./HOWTO_precommand_hooks.md)
- [How to Integrate with CI/CD](./HOWTO_cicd_integration.md)
- [How to Configure Domain Exemptions](./HOWTO_configure_exemptions.md)
- [How to Debug Failed Scans](./HOWTO_debug_failures.md)

### 📖 **Reference** (Lookup)
- [CLI Command Reference](./REFERENCE_cli_commands.md) — All verbs, flags, exit codes
- [Diagnostic Codes Reference](./REFERENCE_diagnostics.md) — All 80+ codes, categories, fixes
- [Configuration Schema](./REFERENCE_config_schema.md) — anti-llm.toml fields and defaults
- [Exit Code Reference](./REFERENCE_exit_codes.md) — Semantics for agent dispatch
- [Output Formats](./REFERENCE_output_formats.md) — JSON structure, SARIF, text

### 💡 **Explanation** (Understanding)
- [Architecture Overview](./EXPLANATION_architecture.md) — Library, CLI, noun/verb pattern
- [Rule Categories Deep Dive](./EXPLANATION_rule_categories.md) — What each rule prevents, why
- [Integration Patterns](./EXPLANATION_integration_patterns.md) — Agents, hooks, subprocess lifecycle
- [Design Decisions](./EXPLANATION_design_decisions.md) — Why clap-noun-verb, JSON output, exit code gating

---

## Lifecycle Integration Points

```
┌─────────────────────────────────────────────────────────────────┐
│ Development (Local Agent)                                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Pre-Command Hook (PreToolUse)                              │
│     lsp-max-anti-cheat check all --path .                      │
│     → Exit 0: Proceed with command                             │
│     → Exit 1: Block (violations present)                       │
│                                                                 │
│  2. Continuous Scan (Agent Loop)                               │
│     Agent periodically invokes: lsp-max-anti-cheat scan dir    │
│     Parses JSON output, tracks violations                      │
│                                                                 │
│  3. Interactive Fixes (Agent Assist)                           │
│     Agent interprets diagnostics, suggests corrections         │
│     Re-runs check to validate                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ CI/CD (Automated Gate)                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  4. Pre-Merge Gate                                             │
│     cargo run -p lsp-max-anti-cheat-cli -- check all --path . │
│     → Exit 0: Merge allowed                                    │
│     → Exit 1: Merge blocked (diagnostic review required)       │
│                                                                 │
│  5. Conformance Report                                         │
│     lsp-max-anti-cheat check all --path . | jq '.summary'     │
│     → Track compliance over time                               │
│     → Archive violations per commit                            │
│                                                                 │
│  6. SARIF Upload (GitHub Advanced Security)                    │
│     lsp-max-anti-cheat check all --path . \                   │
│       --format sarif | gh code-scanning upload --sarif -      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Release (Admission Layer)                                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  7. Release Validation                                         │
│     All blocking violations must be resolved before release    │
│     Final scan confirms 0 BLOCKING diagnostics                 │
│                                                                 │
│  8. Archive & Audit Trail                                      │
│     Scan results stored in OCEL event log with receipts        │
│     Proves admissibility at release time                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Integration Decision Tree

**Start here to find the right integration for your use case:**

```
Are you a...

├─ Claude Code Agent (Local Dev)
│  └─ See: TUTORIAL_getting_started.md → HOWTO_precommand_hooks.md
│
├─ CI/CD Pipeline (GitHub Actions, GitLab CI)
│  └─ See: HOWTO_cicd_integration.md → REFERENCE_cli_commands.md
│
├─ Release Gating System
│  └─ See: EXPLANATION_integration_patterns.md → REFERENCE_exit_codes.md
│
├─ Custom Tool (Non-Agent)
│  └─ See: REFERENCE_output_formats.md → REFERENCE_cli_commands.md
│
└─ Debugger (Troubleshooting)
   └─ See: HOWTO_debug_failures.md → REFERENCE_diagnostics.md
```

---

## Key Concepts

### Exit Code Semantics (All Agents Must Respect)

```rust
// Subprocess exit codes
0 => {
    // No blocking violations
    // Agent may proceed (commit, push, release)
}

1 => {
    // Blocking violations detected
    // Agent must stop, review diagnostics, fix violations
    // This is NOT a tool failure — it's a compliance gate
}
```

### JSON Output Structure

Every command returns structured JSON:

```json
{
  "path": ".",
  "observations_count": 24,
  "diagnostics": [...],      // AntiLlmDiagnostic[]
  "summary": {
    "total": 8,
    "blocking": 3,            // Exit code 1 if > 0
    "warnings": 5
  },
  "exit_code": 1
}
```

Agents parse this to:
- Count violations
- Extract file paths for remediation
- Track compliance trends
- Drive retry logic

### Configuration (anti-llm.toml)

Agents can customize rule enforcement per repo:

```toml
[claim]
# Domain vocabulary that's NOT victory language
domain_terms = ["fully admitted", "candidate"]

[surface]
# Paths where tower-lsp refs are warnings, not errors
non_blocking_path_prefixes = ["docs/archive/"]
```

Agents should:
1. Detect existing config
2. Offer to initialize if missing
3. Validate config syntax
4. Respect domain exemptions

---

## Common Agent Patterns

### Pattern 1: Pre-Command Gate (Blocking)

```bash
#!/bin/bash
# Called before any Bash/shell command

lsp-max-anti-cheat check all --path . >/dev/null 2>&1
if [ $? -ne 0 ]; then
  echo "⛔ Blocking violations detected"
  echo "Run: lsp-max-anti-cheat check all --path . | jq ."
  exit 1
fi
```

**Agent responsibility:**
- Display this error to user
- Offer to show violations
- Offer to attempt auto-fixes
- Offer to add domain exemption

### Pattern 2: Continuous Monitoring (Non-Blocking)

```bash
#!/bin/bash
# Runs periodically; violations are warnings, not gates

result=$(lsp-max-anti-cheat check all --path .)
blocking=$(echo "$result" | jq '.summary.blocking')

if [ "$blocking" -gt 0 ]; then
  echo "⚠️  $blocking blocking violations (will fail on merge)"
  echo "Violations:"
  echo "$result" | jq '.diagnostics[] | select(.blocking == true)'
fi
```

**Agent responsibility:**
- Surface violations in UI
- Suggest fixes
- Update on each code change
- Don't block user (warning only)

### Pattern 3: CI/CD Gate (Deterministic)

```yaml
# GitHub Actions
- name: Anti-LLM Admissibility Check
  run: |
    cargo run -p lsp-max-anti-cheat-cli -- check all --path . \
      --format sarif > results.sarif
    gh code-scanning upload --sarif results.sarif
    exit $?
```

**CI responsibility:**
- Run on every PR
- Fail the build if violations present
- Upload to security scanning (SARIF)
- Require fix before merge

### Pattern 4: Agent Assist (Interactive)

```rust
// Pseudo-code
fn auto_fix_violation(diag: &AntiLlmDiagnostic) {
  match diag.code.as_str() {
    "ANTI-LLM-SURFACE-001" => {
      // Replace tower-lsp with lsp-max
      agent.read_file(diag.file_path)?;
      agent.replace_in_file(
        "use tower_lsp",
        "use lsp_max",
        diag.file_path
      )?;
      // Re-run check to validate
      let result = subprocess::run("lsp-max-anti-cheat", &["check", "all"])?;
      if result.exit_code == 0 {
        agent.commit("Fix: replace tower-lsp with lsp-max")?;
      }
    }
    _ => {
      // Offer manual intervention
      agent.prompt_user(&format!("Fix required: {}", diag.message))?;
    }
  }
}
```

**Agent responsibility:**
- Attempt auto-fixes for known patterns
- Re-validate after fixes
- Fall back to user prompts
- Provide clear reasoning

---

## Violation Triage Matrix

When a violation is detected, agents should respond as follows:

| Violation | Blocking | Auto-Fix? | User Intervention | Document? |
|-----------|----------|-----------|------------------|-----------|
| **SURFACE-001** (tower-lsp ref) | Yes | Yes (replace import) | If auto-fix fails | No |
| **CLAIM-004** (victory language) | Yes | Yes (remove term) | Review context | Maybe |
| **RECEIPT-001** (test stdout) | Yes | No | Rewrite test | Yes |
| **ROUTE-001** (log as proof) | Yes | No | Add proper receipt | Yes |
| **VERSION-001** (SemVer in CalVer) | Yes | Yes (update version) | Review intention | Maybe |
| **CHEAT-001** (hardcoded metrics) | Yes | No | Parameterize | Yes |
| **Other** | Varies | Case-by-case | — | — |

---

## Status & Limitations

**Status**: CANDIDATE (compliant with CLAUDE.md law-state guarantees)

**Current Scope**:
- ✅ 20+ rule modules
- ✅ 80+ diagnostic codes
- ✅ Clap-noun-verb CLI
- ✅ JSON output for agents
- ✅ Pre-command gate support
- ✅ Configuration via anti-llm.toml

**Limitations**:
- ⏳ Awaiting sibling repo dependencies for full compilation
- ⏳ SARIF output format not yet validated with GitHub Advanced Security
- ⏳ No built-in auto-fix service (agents must implement)
- ⏳ No cloud-hosted compliance dashboard (local only)

---

## Next Steps for Agents

1. **Read**: [TUTORIAL_getting_started.md](./TUTORIAL_getting_started.md)
2. **Install**: `cargo install lsp-max-anti-cheat-cli`
3. **Test**: `lsp-max-anti-cheat check all --path .`
4. **Integrate**: Choose a pattern above (gate, monitor, or assist)
5. **Automate**: Wire into your agent's lifecycle hooks

---

**For full details, see the documentation directory structure above.**
