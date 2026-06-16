# Claude Code Hooks — Advanced Patterns & Debugging

This guide covers advanced hook patterns, debugging techniques, and integration strategies for complex workflows.

---

## Advanced Patterns

### Pattern 1: Conditional Gates Based on Branch

Run stricter checks only on main branch; allow faster iteration on feature branches.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown') && if [[ \"$branch\" == 'main' ]] || [[ \"$branch\" == 'master' ]]; then echo \"On $branch: running strict checks\" && just dx-verify && just dx-polish; else echo \"On feature branch $branch: fast checks only\" && cargo check --workspace; fi"
          }
        ]
      }
    ]
  }
}
```

**Benefits:**
- Fast feedback on feature branches (cargo check only)
- Full validation on main before merge
- Prevents accidental merges of unformatted code

**Customization:**
```bash
# Different checks per branch
if [[ "$branch" == "main" ]]; then
    cargo test --workspace --all-features
elif [[ "$branch" == "develop" ]]; then
    cargo check --workspace
else
    echo "Feature branch; skipping checks"
    exit 0
fi
```

---

### Pattern 2: Tiered Validation (Quick → Full)

Run fast checks first; only run full suite if fast checks pass.

```bash
#!/bin/bash
# .claude/tiered-check.sh

set -e

echo "=== Tier 1: Syntax & Format ==="
cargo check --workspace || { echo "Syntax error"; exit 1; }
cargo fmt --check || { echo "Format check failed"; exit 1; }

echo "=== Tier 2: Lint ==="
cargo clippy --workspace --all-targets -- -D warnings || { echo "Clippy failed"; exit 1; }

echo "=== Tier 3: Tests ==="
cargo test --workspace || { echo "Tests failed"; exit 1; }

echo "=== All tiers passed ==="
exit 0
```

Hook configuration:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "bash ./.claude/tiered-check.sh"
          }
        ]
      }
    ]
  }
}
```

**Benefits:**
- Fast failure on syntax errors (stop early, save time)
- Incremental complexity; easier to debug
- Clear separation of concerns

**Metrics:**
- Tier 1: ~5 seconds (syntax check)
- Tier 2: ~10 seconds (clippy)
- Tier 3: ~30 seconds (tests)

---

### Pattern 3: Parallel Hook Execution

Run multiple independent checks in parallel; fail fast if any fails.

```bash
#!/bin/bash
# .claude/parallel-checks.sh

# Run checks in background
cargo check --workspace &
PID_CHECK=$!

cargo fmt --check &
PID_FMT=$!

cargo clippy --workspace --all-targets -- -D warnings &
PID_CLIPPY=$!

# Wait for all and collect results
errors=0
wait $PID_CHECK || errors=$((errors + 1))
wait $PID_FMT || errors=$((errors + 1))
wait $PID_CLIPPY || errors=$((errors + 1))

# Fail if any failed
[ $errors -eq 0 ] || { echo "$errors checks failed"; exit 1; }
echo "All parallel checks passed"
exit 0
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/parallel-checks.sh"
}
```

**Benefits:**
- 3 checks run simultaneously instead of sequentially
- Reduces total wait time significantly
- Graceful failure collection

**Performance:**
- Sequential: ~45 seconds (5 + 10 + 30)
- Parallel: ~30 seconds (max of 3)

---

### Pattern 4: Hook with Retry Logic

Retry flaky operations (network calls, file system) up to N times.

```bash
#!/bin/bash
# .claude/retry-hook.sh

max_retries=3
retry_count=0

while [ $retry_count -lt $max_retries ]; do
    echo "Attempt $((retry_count + 1)) of $max_retries..."
    
    if lsp-max-cli gate check; then
        echo "Gate check passed"
        exit 0
    fi
    
    retry_count=$((retry_count + 1))
    if [ $retry_count -lt $max_retries ]; then
        echo "Retrying in 1 second..."
        sleep 1
    fi
done

echo "Gate check failed after $max_retries attempts"
exit 1
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/retry-hook.sh"
}
```

**Use cases:**
- Network timeouts (registry unavailable)
- Transient file locks
- Race conditions in parallel systems

---

### Pattern 5: Hook with Fallback Strategy

Try preferred check; fall back to simpler check if unavailable.

```bash
#!/bin/bash
# .claude/fallback-check.sh

# Try lsp-max gate check if available
if command -v lsp-max-cli &> /dev/null; then
    echo "Using lsp-max gate check"
    lsp-max-cli gate check
    exit $?
fi

# Fall back to git status check
if [ -d '.git' ]; then
    echo "Falling back to git status"
    status=$(git status --short)
    if [ -z "$status" ]; then
        echo "Working directory clean"
        exit 0
    else
        echo "Working directory has uncommitted changes"
        exit 1
    fi
fi

# Final fallback: allow (open gate)
echo "No checks available; allowing operation"
exit 0
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/fallback-check.sh"
}
```

**Benefits:**
- Graceful degradation when tools unavailable
- Works across different project setups
- Self-documenting fallback chain

---

### Pattern 6: Hook with Contextual Data Passing

Use temporary files to pass complex data between hook invocations.

```bash
#!/bin/bash
# .claude/gate-with-context.sh

context_file="/tmp/.claude-hook-context-$$"

# Initialize context on first run
if [ ! -f "$context_file" ]; then
    cat > "$context_file" <<EOF
{
  "session_start": $(date +%s),
  "checks_run": 0,
  "last_gate_state": "unknown"
}
EOF
fi

# Read current context
context=$(cat "$context_file")
checks_run=$(echo "$context" | jq -r '.checks_run // 0')

# Increment check counter
checks_run=$((checks_run + 1))

# Check gate
if lsp-max-cli gate check; then
    gate_state="open"
else
    gate_state="blocked"
fi

# Update context
jq -r ".checks_run = $checks_run | .last_gate_state = \"$gate_state\" | .last_check = $(date +%s)" \
    "$context_file" > "${context_file}.tmp"
mv "${context_file}.tmp" "$context_file"

# Log context for debugging
echo "Context after check #$checks_run: gate=$gate_state"

# Clean up on session end (would be in SessionEnd hook if it existed)
[ $checks_run -gt 100 ] && rm -f "$context_file"

# Exit with gate state
[ "$gate_state" = "open" ] || exit 1
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/gate-with-context.sh"
}
```

**Benefits:**
- Tracks hook invocation history
- Detects patterns (repeated failures)
- Debugging via persistent context

---

### Pattern 7: Hook with Adaptive Behavior

Change hook behavior based on detected environment (CI, local, Docker).

```bash
#!/bin/bash
# .claude/adaptive-check.sh

# Detect environment
if [ -n "$CI" ] || [ -n "$GITHUB_ACTIONS" ] || [ -n "$GITLAB_CI" ]; then
    environment="CI"
elif [ -f "/.dockerenv" ]; then
    environment="DOCKER"
else
    environment="LOCAL"
fi

echo "Running in: $environment"

case "$environment" in
    CI)
        echo "CI mode: full validation + upload results"
        cargo test --workspace --all-features || exit 1
        # Upload results to CI system
        [ -n "$CI_BUILD_ID" ] && upload_results "$CI_BUILD_ID"
        ;;
    DOCKER)
        echo "Docker mode: fast checks only (limited resources)"
        cargo check --workspace || exit 1
        ;;
    LOCAL)
        echo "Local mode: standard checks"
        cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings || exit 1
        ;;
esac

exit 0
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/adaptive-check.sh"
}
```

**Benefits:**
- One script works across all environments
- Respects resource constraints (Docker)
- Integrates with CI systems

---

## Advanced Debugging Techniques

### Technique 1: Hook Execution Tracing

Enable bash tracing to see exactly what the hook is executing.

```json
{
  "type": "command",
  "command": "bash -x ./.claude/my-hook.sh 2>&1 | tee /tmp/hook-trace.log"
}
```

Output example:

```
+ lsp-max-cli gate check
+ echo 'Gate state: open'
+ exit 0
```

The `set -x` flag echoes all commands. `tee` captures to file for later analysis.

---

### Technique 2: Hook Health Monitoring

Periodically run and log hook results to detect degradation.

```bash
#!/bin/bash
# .claude/hook-monitor.sh

log_file=".claude/hook-monitor.log"

run_check() {
    local check_name=$1
    local check_cmd=$2
    local start=$(date +%s%N)
    
    if eval "$check_cmd" > /tmp/check-$check_name.log 2>&1; then
        local status="PASS"
        local exit_code=0
    else
        local status="FAIL"
        local exit_code=$?
    fi
    
    local end=$(date +%s%N)
    local duration=$(( (end - start) / 1000000 ))  # Convert to ms
    
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $check_name: $status (${duration}ms)" >> "$log_file"
    
    return $exit_code
}

# Run all checks
run_check "gate" "lsp-max-cli gate check"
run_check "format" "cargo fmt --check"
run_check "lint" "cargo clippy --workspace --all-targets -- -D warnings"

# Report
echo "=== Hook Monitor ==="
tail -10 "$log_file"
```

**Usage:**

```bash
# Run periodically (daily)
0 0 * * * cd /workspace && bash ./.claude/hook-monitor.sh
```

---

### Technique 3: Hook Failure Analysis

Collect diagnostic data when hooks fail for analysis.

```bash
#!/bin/bash
# .claude/hook-with-diagnostics.sh

diagnostic_dir="/tmp/hook-diagnostics-$(date +%Y%m%d-%H%M%S)"

trap_error() {
    echo "Hook failed; collecting diagnostics..."
    mkdir -p "$diagnostic_dir"
    
    # Capture environment
    env > "$diagnostic_dir/env.txt"
    
    # Capture system state
    whoami > "$diagnostic_dir/whoami.txt"
    pwd > "$diagnostic_dir/pwd.txt"
    git status > "$diagnostic_dir/git-status.txt" 2>&1 || true
    df -h > "$diagnostic_dir/df.txt"
    
    # Capture last command output
    history 1 > "$diagnostic_dir/last-command.txt" 2>&1 || true
    
    echo "Diagnostics saved to: $diagnostic_dir"
    exit 1
}

trap trap_error ERR

# Run check
lsp-max-cli gate check || exit 1
echo "Check passed"
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/hook-with-diagnostics.sh"
}
```

---

### Technique 4: Hook Timing & Performance Analysis

Profile hook execution to identify slow operations.

```bash
#!/bin/bash
# .claude/profile-hook.sh

declare -A timings

measure() {
    local name=$1
    local cmd=$2
    
    local start=$(date +%s%N)
    eval "$cmd" || return $?
    local end=$(date +%s%N)
    
    timings["$name"]=$(( (end - start) / 1000000 ))  # ms
}

# Profile each operation
measure "cargo_check" "cargo check --workspace"
measure "cargo_fmt" "cargo fmt --check"
measure "cargo_clippy" "cargo clippy --workspace --all-targets -- -D warnings"

# Report timings
echo "=== Hook Timing Profile ==="
for check in "${!timings[@]}"; do
    echo "$check: ${timings[$check]}ms"
done | sort -t: -k2 -rn
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/profile-hook.sh"
}
```

---

### Technique 5: Interactive Hook Debugging

Pause hook execution for manual inspection.

```bash
#!/bin/bash
# .claude/debug-hook.sh

set -e

echo "=== Debug Mode Enabled ==="
echo "Working directory: $(pwd)"
echo "Git branch: $(git rev-parse --abbrev-ref HEAD)"
echo ""

read -p "Continue with checks? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted by user"
    exit 1
fi

echo "Running gate check..."
lsp-max-cli gate check

echo "Running cargo check..."
cargo check --workspace

echo "All checks passed"
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/debug-hook.sh"
}
```

**Warning:** Interactive hooks block user action. Use only during development.

---

### Technique 6: Hook Logging & Audit Trail

Centralized logging of all hook activity.

```bash
#!/bin/bash
# .claude/logged-hook.sh

log_file=".claude/logs/hooks.log"
mkdir -p "$(dirname "$log_file")"

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] [$$] $*" >> "$log_file"
}

log "Hook started: $0"
log "Working directory: $(pwd)"
log "User: $(whoami)"
log "Git branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"

# Run check with logging
log "Running lsp-max-cli gate check..."
if lsp-max-cli gate check; then
    log "Gate check: PASSED"
    exit_code=0
else
    log "Gate check: FAILED"
    exit_code=1
fi

log "Hook finished with exit code: $exit_code"
exit $exit_code
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/logged-hook.sh"
}
```

**View logs:**

```bash
tail -f .claude/logs/hooks.log
```

---

## Integration Patterns

### Integration 1: GitHub Actions + Hooks

Sync CI checks with local hooks to ensure parity.

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: rust-lang/rust-toolchain@v1
      - name: Run checks
        run: bash ./.claude/tiered-check.sh
```

Hook (same as CI):

```json
{
  "type": "command",
  "command": "bash ./.claude/tiered-check.sh"
}
```

**Benefits:**
- Local environment matches CI
- Developers catch issues before push
- No "it works locally" surprises

---

### Integration 2: Pre-commit Framework

Use standard pre-commit hooks alongside Claude Code hooks.

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
  - repo: https://github.com/rust-lang/rust-clippy
    rev: v1.0.0
    hooks:
      - id: clippy
```

Hook that runs pre-commit:

```json
{
  "type": "command",
  "command": "pre-commit run --all-files"
}
```

**Benefits:**
- Single source of truth for lint rules
- Works with git, IDE, and Claude Code
- Framework-agnostic

---

### Integration 3: LSP Server Gates

Use LSP diagnostics as gate input.

```bash
#!/bin/bash
# .claude/lsp-gate-check.sh

# Query LSP server for active diagnostics
error_count=$(lsp-max-cli diagnostic list --severity error | wc -l)

if [ $error_count -gt 0 ]; then
    echo "Gate BLOCKED: $error_count active errors"
    exit 1
else
    echo "Gate CLEAR"
    exit 0
fi
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/lsp-gate-check.sh"
}
```

---

### Integration 4: Database State Validation

Check database/service dependencies before operations.

```bash
#!/bin/bash
# .claude/service-gate-check.sh

check_service() {
    local service=$1
    local endpoint=$2
    
    if ! nc -z ${endpoint%:*} ${endpoint##*:} 2>/dev/null; then
        echo "ERROR: $service unavailable at $endpoint"
        return 1
    fi
}

# Check all required services
check_service "postgres" "localhost:5432" || exit 1
check_service "redis" "localhost:6379" || exit 1
check_service "elasticsearch" "localhost:9200" || exit 1

echo "All services available"
exit 0
```

Hook:

```json
{
  "type": "command",
  "command": "bash ./.claude/service-gate-check.sh"
}
```

---

## Troubleshooting Advanced Hooks

### Problem: Hook works locally but fails in Claude Code

**Diagnosis:**

```bash
# Run from Claude Code directory (may differ from local CWD)
cd /actual/claude/code/directory
bash ./.claude/my-hook.sh
```

**Solution:** Use absolute paths:

```bash
#!/bin/bash
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir/../.." || exit 1
# Now in workspace root; safe to proceed
```

---

### Problem: Hook hangs or times out

**Diagnosis:**

```bash
# Add timeouts
timeout 30 bash ./.claude/my-hook.sh
```

**Solution:** Use explicit timeouts and background jobs:

```bash
#!/bin/bash
timeout 5 lsp-max-cli gate check || exit 1
timeout 10 cargo check --workspace || exit 1
# If any command exceeds timeout, exit 1
```

---

### Problem: Environment variables not available in hook

**Diagnosis:**

```bash
# Check what's inherited
bash -c 'env | grep MY_VAR'
```

**Solution:** Source `.env` or use full paths:

```bash
#!/bin/bash
if [ -f '.env' ]; then
    set -a
    source .env
    set +a
fi

# Now MY_VAR is available
echo "MY_VAR: $MY_VAR"
```

---

### Problem: Hooks interfere with each other

**Diagnosis:**

```bash
# Log all hook invocations
bash -x ./.claude/my-hook.sh 2>&1 | tee -a /tmp/hook-debug.log
```

**Solution:** Use mutex locks:

```bash
#!/bin/bash
lock_file="/tmp/.claude-hook.lock"

# Wait up to 10 seconds for lock
for i in {1..10}; do
    if mkdir "$lock_file" 2>/dev/null; then
        trap "rmdir $lock_file" EXIT
        break
    fi
    sleep 1
done

[ -d "$lock_file" ] || { echo "Lock timeout"; exit 1; }

# Critical section (only one hook runs at a time)
lsp-max-cli gate check
```

---

## Performance Optimization

### Caching Hook Results

Skip expensive checks if inputs haven't changed.

```bash
#!/bin/bash
# .claude/cached-check.sh

cache_file="/tmp/.claude-check-cache"
cache_key=$(git rev-parse HEAD)

if [ -f "$cache_file" ]; then
    cached_key=$(cat "$cache_file")
    if [ "$cache_key" = "$cached_key" ]; then
        echo "Using cached result"
        exit 0
    fi
fi

# Run expensive check
cargo test --workspace || exit 1

# Cache result
echo "$cache_key" > "$cache_file"
```

**Benefits:**
- Avoid redundant tests on same commit
- Significant speedup for repeated operations

---

### Lazy Hook Initialization

Defer expensive setup until first use.

```bash
#!/bin/bash
# .claude/lazy-init-hook.sh

init_file=".claude/.hook-initialized"

if [ ! -f "$init_file" ]; then
    echo "First run; initializing..."
    cargo build --release  # Expensive
    touch "$init_file"
fi

# Quick check (reuses build from init)
cargo check --workspace
```

---

## See Also

- `HOOKS_GUIDE.md` — Comprehensive hook documentation
- `HOOKS_TEMPLATES.md` — Ready-to-use templates
- `AGENTS.md` — Agent-specific hook patterns
