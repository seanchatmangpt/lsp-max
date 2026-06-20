# Skill: /review

**Status:** AVAILABLE | **Scope:** Pull Request Review | **Category:** Validation & Verification

---

## Overview

Review a pull request comprehensively. Analyzes changes, checks correctness, assesses completeness, and generates a review summary with optional approval or change requests.

**Scope:** PR-level audit (broader than `/code-review` which focuses on code diff)

## When to Use

Use `/review` when you want to:
- Audit a PR for completeness before merge
- Check all aspects: code, tests, docs, migration steps
- Post review as PR comment
- Approve or request changes from author
- Ensure consistency with project standards

**Do NOT use `/review` for:**
- Detailed code review (use `/code-review` instead)
- Security audit (use `/security-review` instead)
- Testing behavior (use `/verify` instead)

## Parameters

```bash
/review [--approve] [--request-changes] [--comment]
```

| Parameter | Type | Optional? | Meaning |
|-----------|------|-----------|---------|
| `--approve` | Flag | Yes | Approve PR if review is clear |
| `--request-changes` | Flag | Yes | Request changes from author |
| `--comment` | Flag | Yes | Post review as PR comment |

## Invocation

```bash
# Basic review (report only)
/review

# Post as PR comment
/review --comment

# Request changes if issues found
/review --request-changes

# Approve if clear
/review --approve

# Combined
/review --comment --approve
```

## Review Scope

PR reviews check:

1. **Completeness** — All necessary files? Missing documentation? Migration steps?
2. **Correctness** — Logic errors, edge cases, null checks
3. **Testing** — Tests added? Coverage adequate? Existing tests pass?
4. **Documentation** — CHANGELOG updated? API docs? Comments clear?
5. **Consistency** — Follows CLAUDE.md standards? Style matches?
6. **Performance** — Regressions? Inefficient new code?

## Expected Output

```
📋 Pull Request Review: #42

Title: Add email validation feature
Author: @username
Status: CANDIDATE (2 findings)

Findings:

✅ Completeness
  [✓] Main feature implemented
  [✓] Tests added (8 test cases)
  [✓] CHANGELOG updated
  [?] API documentation incomplete

❌ Correctness
  [✓] Logic appears sound
  [✗] Missing null check on line 47
  [✗] Off-by-one error in loop (line 63)

✅ Testing
  [✓] All existing tests pass
  [✓] New tests: 8/8 passing
  [✓] Coverage: 95% (was 94%)

⚠️  Documentation
  [✓] CHANGELOG updated
  [?] API docs incomplete (1 new endpoint undocumented)
  [✓] Code comments clear

Status: CANDIDATE (2 correctness issues must be addressed)

Recommendation:
  - Fix null check and off-by-one error
  - Complete API documentation
  - Re-request review after fixes
```

## Integration

### Before `/review`

1. **`/verify`** — Test that PR works
2. **`/code-review`** — Find code issues
3. **`/security-review`** — Security audit

### After `/review`

- If `--approve`: PR ready to merge
- If `--request-changes`: Author addresses feedback, re-request
- If issues found: Author applies fixes, request re-review

## Examples

### Example 1: Approving a PR

```bash
$ /verify
✅ Receipt: ADMITTED

$ /code-review --comment
✅ Posted 1 minor style comment

$ /security-review
✅ No security issues found

$ /review --approve

📋 PR Review Complete
Status: APPROVED
✅ PR #42 approved (ready to merge)
```

### Example 2: Requesting Changes

```bash
$ /verify
❌ Receipt: REFUSED (feature doesn't work)

$ /review --request-changes

📋 PR Review Complete
Status: CHANGES_REQUESTED

Issues found:
  1. Feature fails when input is empty
  2. Error message not user-friendly
  3. Tests incomplete

Author is notified; waiting for changes.
```

## See Also

- [`/verify`](SKILL_VERIFY.md) — Test PR functionality first
- [`/code-review`](SKILL_CODE_REVIEW.md) — Detailed code review
- [`/security-review`](SKILL_SECURITY_REVIEW.md) — Security audit

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
