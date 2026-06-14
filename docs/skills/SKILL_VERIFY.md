# Skill: /verify

**Status:** AVAILABLE | **Scope:** Change Validation | **Category:** Validation & Verification

---

## Overview

Verify that a code change actually does what it's supposed to by running the app and observing behavior. The `/verify` skill is a hands-on validation tool: it launches your app in a real environment, performs the relevant behavior tests, and generates a **receipt** with findings.

**Key distinction from `/run`:** `/run` launches the app; `/verify` launches AND validates behavior, then issues a receipt.

## When to Use

Use `/verify` when you want to:
- Confirm a bug fix actually resolves the reported issue
- Test a feature end-to-end before committing
- Validate behavior against a specification
- Generate evidence (receipt) that the change works
- Check for regressions in related functionality

**Do NOT use `/verify` for:**
- Simple app launch (use `/run` instead)
- Automated unit testing (use `cargo test` or equivalent)
- Quick syntax check (use linter instead)

## Parameters

**None** — Interactive guided validation based on project type and changes detected.

## Invocation

```bash
/verify
```

## How It Works

### Phase 1: Context Analysis

The skill examines:

1. **Changed files** — What code changed in this commit/branch
2. **Change summary** — What feature/bug fix was attempted
3. **Project type** — Detect app type (CLI, server, TUI, web, etc.)
4. **Related tests** — Are there test files for changed code?

### Phase 2: App Launch

Same as `/run`:
- Builds if necessary
- Starts the app
- Waits for readiness (port binding, output signal, etc.)

### Phase 3: Behavior Validation

The skill performs **observational tests**:

#### For a Bug Fix:

```
Issue: "Login fails with empty password"
Changed: src/auth.rs (added validation)

Test 1: POST /login with empty password
  ✓ Returns 400 Bad Request (expected)
  ✓ Error message mentions "password required" (expected)

Test 2: POST /login with valid credentials
  ✓ Returns 200 OK (expected)
  ✓ Session token issued (expected)

Receipt: ADMITTED — Bug fix validated
```

#### For a Feature:

```
Feature: "Dark mode toggle"
Changed: src/ui/theme.rs, styles/dark.css

Test 1: Page loads in light mode
  ✓ Light colors applied (expected)

Test 2: Click theme toggle
  ✓ Dark colors applied (expected)
  ✓ LocalStorage updated (expected)

Test 3: Refresh page
  ✓ Theme persists (expected)

Receipt: ADMITTED — Feature working as specified
```

#### For a Configuration Change:

```
Config: "Increase cache TTL from 60s to 300s"
Changed: src/config.rs

Test 1: Cache behavior with new TTL
  ✓ First request: cache miss (OK)
  ✓ Requests within 300s: from cache (expected)
  ✓ Request after 300s: cache miss, fresh data (expected)

Receipt: ADMITTED — Configuration change applied
```

### Phase 4: Receipt Generation

The skill issues a **receipt** with:

| Field | Example |
|-------|---------|
| **Status** | ADMITTED / REFUSED / UNKNOWN |
| **Summary** | What was tested and result |
| **Evidence** | Console logs, network requests, screenshots |
| **Observations** | What worked, what didn't |
| **Next steps** | If REFUSED: debugging suggestions |

## Expected Output

### Success (ADMITTED)

```
✅ Verification: ADMITTED

Changed: src/auth.rs (added email validation)

Tests run:
  ✓ Valid email: accepted (POST /signup)
  ✓ Invalid email: rejected with 400
  ✓ Existing email: rejected with 409
  ✓ No email: rejected with 400

Evidence:
  - Console: "Validation successful: 4 tests passed"
  - Response headers: Content-Type: application/json
  - Database: New user record created

Status: The change works as expected.
```

### Failure (REFUSED)

```
❌ Verification: REFUSED

Changed: src/auth.rs (added email validation)

Tests run:
  ✓ Valid email: accepted (POST /signup)
  ✗ Invalid email: NOT rejected (expected 400, got 200) ← ISSUE
  ✗ Database: New user created with invalid email ← ISSUE
  ✗ Existing email: NOT rejected (expected 409, got 200) ← ISSUE

Evidence:
  - Console shows validation bypassed: "Email validation DISABLED"
  - Network: POST returned 200 when expecting 400

Problems identified:
  1. Email validation code not loaded (missing import?)
  2. Logic inverted? (accepting instead of rejecting)

Next steps:
  - Check: is validation code being called? (add log statement)
  - Run: /run with LOG_LEVEL=debug
  - Review: /code-review for logic errors
```

### Inconclusive (UNKNOWN)

```
⚠️  Verification: UNKNOWN

Changed: src/cache.rs (TTL optimization)

Tests run:
  ✓ Cache enabled
  ? TTL behavior inconclusive (requests inconsistently cached)
  ? Hard to validate in local environment (depends on system clock)

Problems:
  - Behavior varies based on timing
  - Cannot reproduce reliably in manual testing

Next steps:
  - Run integration tests: cargo test --test cache_integration
  - Or: Use /loop to poll behavior over time
  - Or: Check CI logs (automated tests may have clearer result)
```

## Integration with Other Skills

### Before `/verify`

- **`/run`** — Launch the app first (verify assumes it's running)
- **Manual testing** — Understand the change you're testing

### After `/verify`

- **`/code-review`** — If ADMITTED, proceed to code review
- **`/simplify`** — Clean up code after verification
- **`/security-review`** — Audit for security issues
- **Commit** — If all verifications pass, commit the change

### Typical Chain

```
/run → /verify → /code-review → /simplify → /security-review → commit
```

## Examples

### Example 1: Bug Fix

```bash
$ git diff
# File: src/payment.rs
# - Changed: amount validation
# - Added: check for zero or negative amounts

$ /verify

🔍 Analyzing changes: src/payment.rs (validation added)
▶️  Launching app...
✅ App ready

Testing bug fix: "Payment fails with negative amount"

Test 1: POST /api/payment -d '{"amount": -100}'
  Response: 400 Bad Request
  Error: "Amount must be positive"
  ✓ PASS (expected behavior)

Test 2: POST /api/payment -d '{"amount": 0}'
  Response: 400 Bad Request
  Error: "Amount must be positive"
  ✓ PASS (expected behavior)

Test 3: POST /api/payment -d '{"amount": 100}'
  Response: 200 OK
  Payment ID: pay_12345
  ✓ PASS (valid payment accepted)

Test 4: Database consistency check
  Transaction recorded correctly
  ✓ PASS

✅ Receipt: ADMITTED

Bug fix verified. Ready for code review.
```

### Example 2: Feature Addition

```bash
$ git diff
# File: src/ui/alerts.rs
# - Added: dismissable alert component

$ /verify

🔍 Analyzing changes: src/ui/alerts.rs (new component)
▶️  Launching app...
✅ App ready

Testing feature: "Dismissable alert component"

Test 1: Alert displays
  Visual: Alert banner visible
  ✓ PASS

Test 2: Dismiss button present
  Visual: X button in top-right corner
  ✓ PASS

Test 3: Click dismiss button
  Visual: Alert slides out and disappears
  DOM: Alert removed from document
  ✓ PASS

Test 4: Multiple alerts
  Created 3 alerts; each can be dismissed independently
  ✓ PASS

✅ Receipt: ADMITTED

Feature works as expected. Ready for code review.
```

### Example 3: Configuration Change

```bash
$ git diff
# File: config.json
# - Changed: max_connections from 50 to 200

$ /verify

🔍 Analyzing changes: config.json (max_connections)
▶️  Launching app...
✅ App ready

Testing config: "Increased max_connections"

Test 1: Load config
  max_connections = 200
  ✓ PASS (config loaded)

Test 2: Concurrent connections
  Established 150 concurrent connections
  Server accepting new connections
  ✓ PASS

Test 3: Graceful degradation
  Established 200 concurrent connections
  Connection 201: queued (server respects limit)
  ✓ PASS (limit enforced)

Test 4: Memory/resource usage
  Memory: 256MB (baseline: 250MB, +2% acceptable)
  CPU: <30% (normal)
  ✓ PASS (no resource issues)

✅ Receipt: ADMITTED

Configuration change validated. No resource issues detected.
```

## Troubleshooting

### "App won't start" (Can't verify)

```
❌ App failed to launch
⚠️  Cannot verify behavior if app doesn't start

Fix:
1. Run /run to diagnose launch issue
2. Address any build errors
3. Then run /verify again
```

### "Behavior not as expected" (REFUSED)

```
❌ Verification: REFUSED

Expected: POST /api/test returns 200
Got: 404 Not found

Debugging steps:
1. Check if endpoint exists: grep -r "/api/test" src/
2. Check if route registered: /run (see server logs)
3. Review /code-review for logic errors
```

### "Can't reproduce issue reliably" (UNKNOWN)

```
⚠️  Verification: UNKNOWN

Issue appears intermittent. Options:

1. Run /loop 10s /verify
   (repeat test multiple times)

2. Run cargo test --test behavior_integration
   (automated tests may catch it)

3. Check /code-review for race conditions
```

## Configuration

### Custom Verification Steps

If you want more detailed validation, you can manually run tests after `/verify`:

```bash
/verify

# Then, if you want more details:
cargo test --test integration_tests
./scripts/e2e.sh
```

### Verification Timeout

For slow apps or comprehensive tests:

```bash
/verify --timeout 120s
```

## Differences from Similar Skills

| Skill | Launches App? | Validates Behavior? | Generates Receipt? |
|-------|---------------|--------------------|--------------------|
| **`/run`** | ✓ Yes | ✗ No | ✗ No |
| **`/verify`** | ✓ Yes | ✓ Yes | ✓ Yes |
| **`cargo test`** | ✗ No | ✓ Yes | ✗ No |
| **`/code-review`** | ✗ No | ✗ No | ✓ Yes |

## Verification Receipt Format

Each `/verify` invocation produces a receipt with:

```
✅ Receipt: ADMITTED
  Changed: src/feature.rs (added X)
  Tests: 8 observations, 8 passed, 0 failed
  Evidence: Console logs, network requests, screenshots
  Confidence: HIGH
  Status: Ready to proceed to code review
```

Or:

```
❌ Receipt: REFUSED
  Changed: src/feature.rs (added X)
  Tests: 8 observations, 6 passed, 2 failed
  Issues:
    1. Endpoint returns 404 (expected 200)
    2. Database transaction not committed
  Next: Fix issues, run /run, then /verify again
```

## See Also

- [`/run`](SKILL_RUN.md) — Launch app without validation
- [`/code-review`](SKILL_CODE_REVIEW.md) — Review code after verification
- [`/loop`](SKILL_LOOP.md) — Repeat verification on interval
- [`/security-review`](SKILL_SECURITY_REVIEW.md) — Security validation
- [Test Infra](../TEST_INFRA.md) — Automated test patterns

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
