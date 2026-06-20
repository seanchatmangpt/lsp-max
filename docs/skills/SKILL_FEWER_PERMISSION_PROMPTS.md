# Skill: /fewer-permission-prompts

**Status:** AVAILABLE | **Scope:** Permission Management | **Category:** Configuration & Setup

---

## Overview

Scan transcripts for common read-only Bash and MCP tool calls, then add a prioritized allowlist to `.claude/settings.json` to reduce permission prompts during sessions.

## When to Use

Use `/fewer-permission-prompts` when you want to:
- Reduce permission dialogs during active development
- Auto-generate safe allowlist from transcript
- Improve session flow by pre-approving common tools
- Balance security (allowlist) with convenience

**Typical:** Run once per session after discovering repeated permission prompts.

## Parameters

**None** — Automatic analysis of transcript.

```bash
/fewer-permission-prompts
```

## How It Works

### Phase 1: Transcript Analysis

Examine session transcript for:
- Repeated Bash commands (git status, ls, find, etc.)
- MCP tool calls (GitHub search, calendar lookups, etc.)
- Pattern of read-only vs. write operations

### Phase 2: Tool Categorization

- **Read-only (safe)** — git status, ls, grep, cat, etc.
- **Write (careful)** — git push, file modification, git commit
- **Dangerous (excluded)** — git push --force, rm -rf, etc.

### Phase 3: Allowlist Generation

Create prioritized list ordered by frequency:

```json
{
  "permissions": {
    "bash": [
      "git status",      // Called 12 times
      "git log",         // Called 8 times
      "ls",              // Called 5 times
      "grep"             // Called 3 times
    ],
    "mcp": [
      "github:search_code",      // Called 6 times
      "github:list_issues"       // Called 2 times
    ]
  }
}
```

### Phase 4: Validation

- Check for security conflicts
- Ensure allowlist is safe
- Report changes to user

## Expected Output

```
🔐 Permission Allowlist Generation

Analyzing transcript: 47 tool calls
  Read-only Bash: 31 calls
  MCP calls: 12 calls
  Write operations: 4 calls

Tool frequency:
  [12x] git status
  [8x]  git log
  [6x]  github:search_code
  [5x]  ls
  [4x]  grep
  [3x]  find
  [2x]  github:list_issues
  [1x]  cargo build

Generated allowlist (7 items):
  bash: git status, git log, ls, grep, find, cargo
  mcp: github:search_code, github:list_issues

Added to: .claude/settings.json

Security check:
  ✅ No write operations in allowlist
  ✅ No dangerous commands
  ✅ Safe to enable

Status: ADMITTED
Estimated prompt reduction: ~90% fewer permission dialogs
```

## Integration

### Works with `/update-config`

After `/fewer-permission-prompts` generates allowlist:

```bash
/fewer-permission-prompts

# Review the output

/update-config "review permissions"  # If needed to adjust
```

## Example Output

```json
{
  "permissions": {
    "bash": [
      "git status",
      "git log",
      "git branch",
      "ls",
      "ls -la",
      "find",
      "grep",
      "cat",
      "head",
      "tail",
      "which"
    ],
    "mcp": [
      "github:search_code",
      "github:list_issues",
      "github:list_pull_requests"
    ]
  }
}
```

## Best Practices

✓ **Do:**
- Run after a productive session
- Review the generated allowlist
- Understand what's being whitelisted
- Regenerate periodically as patterns change

✗ **Don't:**
- Auto-approve without review
- Add write operations to allowlist blindly
- Use if you're uncomfortable with the tools listed

## See Also

- [`/update-config`](SKILL_UPDATE_CONFIG.md) — Manual permission management
- [`/security-review`](SKILL_SECURITY_REVIEW.md) — Audit permission safety

---

**Last Updated:** 2026-06-14 | **Status:** AVAILABLE
