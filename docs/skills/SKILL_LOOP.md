# Skill: /loop

**Status:** AVAILABLE | **Scope:** Recurring Task Automation | **Category:** Code Execution & Automation

---

## Overview

Run a prompt, skill, or command on a recurring interval. Perfect for polling deployment status, running tests repeatedly, or continuous monitoring during development.

**Key feature:** Defaults to 10-minute interval; specify custom intervals with syntax `<number><unit>` (e.g., "5m", "30s", "1h").

## When to Use

Use `/loop` when you want to:
- Poll for deployment status every N seconds/minutes
- Run tests repeatedly during TDD
- Monitor a service continuously
- Check build status on an interval
- Watch a metric until it reaches target

**Do NOT use `/loop` for:**
- One-time checks (use individual skill instead)
- Tight loops (< 1 second; defeats purpose)
- Blocking workflows (loop runs in background)

## Parameters

```bash
/loop [interval] <command>
```

| Parameter | Type | Optional? | Examples |
|-----------|------|-----------|----------|
| `interval` | string: `<number><unit>` | Yes | "5m", "30s", "1h", "2d" |
| `command` | skill or bash command | No | `/verify`, `/run`, `cargo test` |

**Default interval:** 10 minutes (if omitted)

## Interval Syntax

```
5s       → every 5 seconds
30s      → every 30 seconds
1m       → every 1 minute
5m       → every 5 minutes (common)
10m      → every 10 minutes (default)
1h       → every 1 hour
2d       → every 2 days
```

## Invocation

```bash
# Default interval (10 minutes)
/loop /verify

# Custom interval
/loop 5m /verify
/loop 30s "cargo test"
/loop 1h /run

# With flags
/loop 5m "/code-review --comment"
```

## How It Works

### Phase 1: Initialization

1. Parse interval and command
2. Validate command syntax
3. Setup recurring timer
4. Report: "Loop starting: run [command] every [interval]"

### Phase 2: Execution Loop

1. Execute command
2. Capture output and status
3. Log result with timestamp
4. Wait for interval
5. Repeat (unless user stops with Ctrl+C)

### Phase 3: Exit

User can exit by:
- **Ctrl+C** — Stop the loop manually
- **Condition met** — (if loop has exit condition) automatically stop
- **Error** — Stop if command fails (depends on loop configuration)

## Expected Output

### Starting

```
🔄 Loop started: run /verify every 5 minutes
Press Ctrl+C to stop

[2026-06-14 10:00:00] Starting iteration 1/∞
```

### Each Iteration

```
[2026-06-14 10:00:15] ✅ ADMITTED (took 15s)
[2026-06-14 10:05:15] ✅ ADMITTED (took 12s)
[2026-06-14 10:10:15] ⚠️  UNKNOWN (took 8s)
[2026-06-14 10:15:15] ✅ ADMITTED (took 14s)
```

### Stopping

```
[2026-06-14 10:20:15] Loop stopped by user (4 iterations completed)
Summary: 3 ADMITTED, 1 UNKNOWN, 0 REFUSED
```

## Integration with Other Skills

### Wraps `/verify`

Most common use case: loop `/verify` to poll behavior.

```bash
/loop 5m /verify

# Each iteration runs:
# - Launch app
# - Validate behavior
# - Generate receipt
# - Wait 5 minutes
# - Repeat
```

### Wraps `/run`

Restart app on interval (useful during development).

```bash
/loop 10s /run

# Rebuilds and restarts app every 10 seconds
# (for testing crash recovery, hot reload, etc.)
```

### Wraps other skills

Any skill or bash command can be looped:

```bash
/loop 1m /code-review         # Run code review every minute
/loop 30s "cargo test --lib"  # Run tests every 30 seconds
/loop 1h /security-review     # Security audit hourly
```

### Works in background

Loop runs without blocking other work:

```bash
/loop 5m /verify              # Loop starts in background
# (you can continue coding, running other skills, etc.)
```

## Examples

### Example 1: Poll Deployment

```bash
$ /loop 30s /verify

🔄 Loop started: run /verify every 30 seconds

[10:00:00] Checking deployment status...
  ✅ ADMITTED (service responding, tests passed)

[10:00:30] Checking deployment status...
  ✅ ADMITTED (service responding, tests passed)

[10:01:00] Checking deployment status...
  ✅ ADMITTED (service responding, tests passed)

[10:01:30] Loop stopped by user
Summary: 3 ADMITTED, 0 REFUSED
Deployment verified as stable.
```

### Example 2: Test-Driven Development

```bash
$ /loop 10s "cargo test --lib"

🔄 Loop started: run cargo test --lib every 10 seconds

[10:00:00] Running tests...
  ❌ FAILED: test_validation
  Failures: 1

[10:00:10] Running tests...
  ❌ FAILED: test_validation
  Failures: 1

[10:00:20] Running tests...
  ❌ FAILED: test_validation
  Failures: 1

[10:00:30] Running tests...
  ✅ PASSED: all tests passed
  Failures: 0

[10:00:40] Running tests...
  ✅ PASSED: all tests passed
  Failures: 0

[10:00:50] Loop stopped by user
Tests are now passing. Development ready.
```

### Example 3: Monitor Service Health

```bash
$ /loop 1m /run

🔄 Loop started: run /run every 1 minute

[10:00:00] Launching server...
  ✅ Ready on http://localhost:3000

[10:01:00] Restarting server...
  ✅ Ready on http://localhost:3000

[10:02:00] Restarting server...
  ✅ Ready on http://localhost:3000

[10:03:00] Restarting server...
  ❌ FAILED: Port 3000 already in use
  
[10:03:30] (Attempting recovery)

[10:03:30] Restarting server...
  ✅ Ready on http://localhost:3000

[10:04:00] Restarting server...
  ✅ Ready on http://localhost:3000

[10:05:00] Loop stopped by user
Server stability: 4/5 restarts successful
```

### Example 4: Babysit Pull Requests

```bash
$ /loop 5m /review

🔄 Loop started: run /review every 5 minutes

[10:00:00] Checking PR status...
  Status: CANDIDATE (1 comment requesting changes)

[10:05:00] Checking PR status...
  Status: CANDIDATE (author responded to feedback)

[10:10:00] Checking PR status...
  Status: ADMITTED (ready to approve)

[10:10:05] Approving PR...
  ✅ PR #42 approved

[10:10:06] Loop stopped (condition met: PR approved)
```

## Interval Selection Guide

**How often should you loop?**

| Use Case | Recommended Interval | Reasoning |
|----------|---------------------|-----------|
| **Deploy status** | 30s-1m | Quick feedback |
| **Service health** | 1-5m | Balance resource/responsiveness |
| **Test runs** | 10-30s | TDD quick feedback |
| **Security audit** | 1h+ | Expensive; run infrequently |
| **Linter checks** | 5-10m | Medium cost |
| **Manual poll** | 5m-1h | Quick checks; not blocking |

## Troubleshooting

### "Loop is consuming too much CPU"

Solution: Increase interval
```bash
/loop 1m /verify        # Instead of /loop 10s /verify
```

### "Command keeps failing; loop won't continue"

Check the command:
```bash
# Test the command outside loop first
/verify              # Does this work?

# If it works once, it should loop OK
/loop 5m /verify
```

### "Loop doesn't stop automatically"

Loop requires manual stop (Ctrl+C) unless built-in exit condition:

```bash
/loop 5m /verify        # Runs forever until you Ctrl+C
```

To exit automatically, you may need to script it:

```bash
# Shell alternative (outside loop skill):
while true; do
  /verify
  if [ $? -eq 0 ]; then break; fi
  sleep 5m
done
```

## Performance Considerations

| Interval | Frequency/Hour | Resource Use | Best For |
|----------|----------------|--------------|----------|
| **5s** | 720 | Very high | Testing crash recovery |
| **30s** | 120 | High | Rapid feedback loops |
| **1m** | 60 | Medium-high | Deploy monitoring |
| **5m** | 12 | Medium | Regular checks |
| **1h** | 1 | Low | Periodic audits |
| **1d** | 0.04 | Very low | Nightly checks |

**Default (10m):** 6 executions/hour; reasonable for most use cases.

## Differences from Similar Patterns

| Method | Use | Notes |
|--------|-----|-------|
| **`/loop`** | Recurring Claude skills | Integrated, easy to setup |
| **Shell loop** | Bash commands | `while true; do cmd; sleep X; done` |
| **Cron** | System scheduling | For persistent, background tasks |
| **CI/CD** | Automated checks | For PR/commit-triggered actions |

## See Also

- [`/verify`](SKILL_VERIFY.md) — Validation skill (commonly looped)
- [`/run`](SKILL_RUN.md) — App execution (can be looped)
- [`/code-review`](SKILL_CODE_REVIEW.md) — Code review (can be looped)

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
