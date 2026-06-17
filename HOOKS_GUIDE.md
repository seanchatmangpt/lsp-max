# Claude Code Hooks System — Comprehensive Guide

## Overview

Claude Code hooks are event-driven automation layers that execute custom commands or scripts in response to tool lifecycle events. They enable you to enforce policies, validate changes, run tests, and integrate with external systems without manual intervention.

The hooks system is configured via `.claude/settings.json` and consists of:
- **Hook types**: PreToolUse, SessionStart, PostToolUse
- **Matchers**: Tool name patterns that trigger hooks
- **Hook definitions**: Command-based actions executed in the trigger flow
- **Execution environment**: Isolated shell sessions with access to workspace context

---

## Hook Types

### 1. PreToolUse Hooks

Executed **before** a tool is invoked. The exit code controls whether the tool proceeds.

**Characteristics:**
- Runs synchronously before the tool invocation
- Exit code determines tool execution: 0 (allow), 1 (block)
- Blocks tool immediately if hook fails
- No output capture; logs appear in user transcript
- Can access tool parameters indirectly via environment variables

**Use cases:**
- Gate enforcement (ANDON checks)
- Pre-commit validation
- Permission checks
- Resource availability checks
- Dependency verification

**Example:**
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      }
    ]
  }
}
```

**Behavior:**
```
Tool requested → PreToolUse hooks run → Exit code checked
  ├─ Exit 0: Tool proceeds
  └─ Exit 1: Tool blocked; error message shown
```

---

### 2. PostToolUse Hooks

Executed **after** a tool completes successfully. Used for observability and follow-up actions.

**Characteristics:**
- Runs asynchronously after tool invocation
- Does not block subsequent operations
- Success or failure does not affect prior tool result
- Can read tool output or file system changes
- Useful for non-blocking side effects

**Use cases:**
- Diagnostic snapshots
- Log aggregation
- Artifact generation
- State synchronization
- Cleanup operations

**Example:**
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cd /workspace && lsp-max-cli diagnostic snapshot 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

**Behavior:**
```
Tool executes → Tool completes → PostToolUse hooks run → Tool result returned
                                  (asynchronously, non-blocking)
```

---

### 3. SessionStart Hooks

Executed once when a Claude Code session begins, before any tools are called.

**Characteristics:**
- Runs exactly once at session initialization
- Can prepare environment, validate prerequisites, display startup info
- Blocks session start if fails
- Full access to workspace filesystem
- Ideal for multi-step initialization

**Use cases:**
- Environment validation
- Dependency checks
- Repository state inspection
- Setup scripts
- Pre-session diagnostics

**Example:**
```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "just test && echo 'Session ready'"
      }
    ]
  }
}
```

**Behavior:**
```
Session initialized → SessionStart hooks run → User can invoke tools
                      (blocks if any fails)
```

---

## Configuration Structure

### Settings.json Schema

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Tool1|Tool2|Tool3",
        "hooks": [
          {
            "type": "command",
            "command": "shell command here"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "follow-up command"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "type": "command",
        "command": "initialization command"
      }
    ]
  }
}
```

### Configuration Elements

#### Matcher

Regex-style pipe-separated tool names:

```
"matcher": "Bash|Edit|Write"        # Matches Bash, Edit, or Write tools
"matcher": "Bash"                   # Matches only Bash
"matcher": ".*"                     # Matches all tools (NOT recommended)
```

**Available tools:**
- `Bash` — Execute shell commands
- `Edit` — Modify file contents
- `Write` — Create/overwrite files
- `Read` — Read file contents
- `Glob` — Find files by pattern
- `Grep` — Search file contents
- `TaskCreate` — Create tasks
- `NotebookEdit` — Edit notebooks

#### Hook Type

Currently only `"type": "command"` is supported:

```json
{
  "type": "command",
  "command": "actual shell command"
}
```

---

## Execution Environment

### Working Directory

Hooks execute in the **current working directory** at the time the hook runs. This is typically the workspace root.

```json
{
  "type": "command",
  "command": "pwd"  # Outputs workspace directory
}
```

### Shell Selection

Hooks run in the user's configured shell (bash, zsh, fish, etc.). Shell state does **not** persist between separate hook invocations:

```json
{
  "type": "command",
  "command": "export VAR=value && echo $VAR"  # OK; same invocation
}
```

```json
{
  "type": "command",
  "command": "export VAR=value"  # First hook
},
{
  "type": "command",
  "command": "echo $VAR"  # Second hook; $VAR is undefined
}
```

**Solution:** Use subshells or chain commands:

```json
{
  "type": "command",
  "command": "export VAR=value && echo $VAR"
}
```

Or use a shell script:

```json
{
  "type": "command",
  "command": "bash -c 'export VAR=value && echo $VAR'"
}
```

### Environment Variables

Hooks inherit the user's environment. You can pass additional context via:

1. **Workspace-local environment files** (`.env`, `.envrc`):
```bash
# In hook command
source .env && my-command
```

2. **Workspace directory context** (via `pwd`, relative paths):
```bash
# No need to pass; CWD is workspace root
lsp-max-cli gate check  # Works from any subdirectory
```

3. **Tool-specific variables** (limited):
For `Bash` tools, some context may be available via environment, but this is **not guaranteed** across tool types. Always pass parameters explicitly.

### Exit Codes

- **Exit 0 (SUCCESS):** Tool proceeds (PreToolUse); no blocking
- **Exit 1 (FAILURE):** Tool blocked (PreToolUse); error shown
- **Exit codes 2+:** Treated as failure (exit 1 semantics)

```bash
#!/bin/bash
if ! lsp-max-cli gate check; then
    exit 1  # Block the tool
fi
exit 0  # Allow the tool
```

---

## Variable Passing Strategies

### 1. Environment Variables (Indirect)

Access workspace state via environment. The hook cannot receive tool parameters directly, but can infer from filesystem:

```json
{
  "type": "command",
  "command": "if [ -f 'Cargo.toml' ]; then cargo check; fi"
}
```

### 2. File System Inspection

Read state files to infer tool parameters:

```json
{
  "type": "command",
  "command": "if [ -f '.claude/settings.json' ]; then echo 'Config found'; fi"
}
```

### 3. Workspace Artifact Passing

Create intermediate files that capture state:

```json
{
  "type": "command",
  "command": "lsp-max-cli diagnostic snapshot > /tmp/diagnostic-state.json && echo 'Snapshot created'"
}
```

### 4. Shell Function Wrappers

Define reusable logic in a sourced script:

```bash
# .claude/hook-lib.sh
check_gate() {
    lsp-max-cli gate check
}

run_tests() {
    cargo test --workspace
}
```

```json
{
  "type": "command",
  "command": "source .claude/hook-lib.sh && check_gate"
}
```

---

## Error Handling

### Exit Code Contracts

**PreToolUse:**
- Hook fails (exit ≠0) → Tool is **blocked**
- Hook succeeds (exit 0) → Tool **proceeds**

```json
{
  "type": "command",
  "command": "lsp-max-cli gate check || exit 1"
}
```

**PostToolUse:**
- Hook fails → Logged as warning; tool result **not affected**
- Hook succeeds → No change to tool result

```json
{
  "type": "command",
  "command": "lsp-max-cli diagnostic snapshot 2>/dev/null || true"
}
```

The `|| true` suppresses errors for non-critical PostToolUse hooks.

### Error Messages

**PreToolUse blocking messages:**

When a PreToolUse hook exits with code 1, Claude Code shows:

```
Hook failed: <command>
Exit code: 1
Stdout: <captured output>
Tool blocked.
```

**Best practice:** Make hook error messages actionable:

```bash
#!/bin/bash
if ! lsp-max-cli gate check; then
    echo "Error: ANDON gate is BLOCKED"
    echo "Active violations:"
    lsp-max-cli diagnostic list --severity error
    exit 1
fi
```

### Timeout Handling

Hooks run with a default timeout (typically 30 seconds). Long-running hooks should provide progress:

```json
{
  "type": "command",
  "command": "echo 'Starting check...' && cargo check && echo 'Check complete'"
}
```

If a hook times out, it is treated as a failure (exit 1).

---

## Debugging Hooks

### 1. Test Hook Commands Locally

Run the exact command from your shell to debug:

```bash
cd /path/to/workspace
lsp-max-cli gate check
echo "Exit code: $?"
```

### 2. Add Debug Output

Modify hooks to emit detailed logs:

```json
{
  "type": "command",
  "command": "set -x; lsp-max-cli gate check; set +x"
}
```

The `set -x` flag echoes all commands and their arguments to stderr.

### 3. Check Hook Execution in Transcript

The Claude Code transcript shows hook execution:

```
[Hook] PreToolUse matched on Bash
[Hook] Running: lsp-max-cli gate check
[Hook] Exit code: 0
[Tool] Bash proceeding...
```

### 4. Inspect Gate File (lsp-max-specific)

For the ANDON gate hook, you can manually check gate state:

```bash
# Find the gate file
ls -la /tmp/lsp-max-gate-*

# Read gate byte
od -c /tmp/lsp-max-gate-<hash>
# Output: 0 = OPEN, 1 = ANDON blocked
```

### 5. Create a Hook Test Script

```bash
#!/bin/bash
# .claude/test-hooks.sh

echo "Testing PreToolUse hooks..."

echo -n "Gate check: "
if lsp-max-cli gate check; then
    echo "PASS"
else
    echo "FAIL (ANDON blocking)"
fi

echo -n "Cargo check: "
if cargo check --workspace; then
    echo "PASS"
else
    echo "FAIL"
fi

echo "All tests complete"
```

Run before using hooks in production:

```bash
chmod +x .claude/test-hooks.sh
./.claude/test-hooks.sh
```

---

## Best Practices

### 1. Keep Hooks Fast

PreToolUse hooks block user actions. Target execution time: <500ms.

**Bad (slow):**
```json
{
  "type": "command",
  "command": "cargo test --workspace"  # 30+ seconds
}
```

**Good (fast):**
```json
{
  "type": "command",
  "command": "lsp-max-cli gate check"  # ~10ms
}
```

For longer checks, use PostToolUse (non-blocking):

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "cargo test --workspace > /tmp/test-log 2>&1 &"
          }
        ]
      }
    ]
  }
}
```

### 2. Make Hooks Idempotent

Hooks may run multiple times in quick succession. Ensure repeated invocations are safe:

**Bad (not idempotent):**
```bash
echo "test" >> /tmp/hook-log  # Appends every time
```

**Good (idempotent):**
```bash
echo "test" > /tmp/hook-log  # Overwrites; safe to repeat
```

### 3. Use Explicit Error Handling

Avoid silent failures. Make hook intent and errors clear:

**Bad:**
```json
{
  "type": "command",
  "command": "cargo check && echo 'OK'"
}
```

**Good:**
```json
{
  "type": "command",
  "command": "cargo check || { echo 'Check failed'; exit 1; }"
}
```

### 4. Document Hook Purpose in Settings

Add comments (JSON5 style, if supported):

```json5
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            // ANDON gate enforcement: blocks shell actions while law violations active
            "command": "lsp-max-cli gate check"
          }
        ]
      }
    ]
  }
}
```

Or use a separate documentation file:

```bash
# .claude/HOOKS_README.md
# PreToolUse: ANDON Gate Check
# Runs lsp-max-cli gate check before Bash/Edit/Write
# Prevents shell actions while WASM4PM-* or GGEN-* violations are active
```

### 5. Test Hooks on Session Start

Use SessionStart to validate hook setup:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "echo 'Validating hooks...' && ./.claude/test-hooks.sh"
      }
    ]
  }
}
```

---

## Hook Composition Patterns

### Pattern 1: Sequential Gate Checks

Multiple gates that must all pass before proceeding:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          },
          {
            "type": "command",
            "command": "[ -f 'justfile' ] || { echo 'Missing justfile'; exit 1; }"
          }
        ]
      }
    ]
  }
}
```

Hooks run left-to-right. If any fails, tool is blocked.

### Pattern 2: Conditional Hooks Based on File Type

Use matcher to target specific tools, then gate on file patterns:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "if [[ $FILE == *.rs ]]; then cargo clippy --all-targets --all-features -- -D warnings; fi"
          }
        ]
      }
    ]
  }
}
```

**Limitation:** Hook commands don't directly receive tool parameters (like file paths). Workaround: check filesystem for recently modified files.

### Pattern 3: Staged Validation

Different hooks for different tool types:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      },
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cargo fmt --check"
          }
        ]
      },
      {
        "matcher": "Write",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Write permitted' && exit 0"
          }
        ]
      }
    ]
  }
}
```

### Pattern 4: Snapshot + Gate

PreToolUse blocks on gate; PostToolUse snapshots state:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli diagnostic snapshot 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

### Pattern 5: Archival + Cleanup

Post-session operations:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if [ -f './target/debug/main' ]; then tar -czf /tmp/build-artifact-$(date +%s).tar.gz ./target/debug/main; fi"
          }
        ]
      }
    ]
  }
}
```

---

## Common Use Cases & Templates

### Use Case 1: Linting Before Commits

Enforce `cargo fmt` and `clippy` before writes:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings"
          }
        ]
      }
    ]
  }
}
```

**Trade-off:** Slower edits. Alternative: use PostToolUse to flag issues without blocking:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cargo clippy --workspace --all-targets --all-features 2>&1 | grep -q error && echo 'Clippy warnings detected' || true"
          }
        ]
      }
    ]
  }
}
```

### Use Case 2: Running Tests After Changes

Non-blocking test suite on every write:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Bash",
        "hooks": [
          {
            "type": "command",
            "command": "cargo test --workspace > /tmp/test-results-$(date +%s).log 2>&1 & echo 'Tests running in background'"
          }
        ]
      }
    ]
  }
}
```

The `&` background the command; the hook returns immediately.

### Use Case 3: Validating Configuration

Gate on config file presence/syntax:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "if [ ! -f '.claude/settings.json' ]; then echo 'Missing .claude/settings.json'; exit 1; fi && cat .claude/settings.json | jq . > /dev/null || { echo 'Invalid JSON'; exit 1; }"
      }
    ]
  }
}
```

### Use Case 4: Archival Workflows

Capture build artifacts on success:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if [ $? -eq 0 ] && [ -d './target/release' ]; then echo 'Archiving release build...' && tar -czf /archive/build-$(git rev-parse --short HEAD).tar.gz ./target/release; fi"
          }
        ]
      }
    ]
  }
}
```

### Use Case 5: CI Integration

Pre-release gate that mirrors CI checks:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if [[ $(pwd) == *'/main' ]] || [[ $(git branch --show-current) == 'main' ]]; then just dx-verify && just dx-polish; else exit 0; fi"
          }
        ]
      }
    ]
  }
}
```

Only runs full checks on main branch; other branches skip (exit 0).

### Use Case 6: Custom Gates (ANDON Pattern)

Generalized gate for any diagnostic family:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check || { echo 'Gate blocked. Active diagnostics:'; lsp-max-cli diagnostic list --severity error; exit 1; }"
          }
        ]
      }
    ]
  }
}
```

---

## Shell Scripting Best Practices for Hooks

### 1. Fail Fast

Use `set -e` to exit on first error:

```bash
#!/bin/bash
set -e

echo "Step 1..."
step1_command

echo "Step 2..."
step2_command

echo "All steps succeeded"
```

Or use `&&` chaining:

```bash
step1_command && step2_command && echo "Done"
```

### 2. Defensive Programming

Check file/directory existence before using:

```bash
if [ ! -f 'Cargo.toml' ]; then
    echo "Not a Rust project"
    exit 0  # Non-blocking for unrelated projects
fi

cargo check
```

### 3. Conditional Execution

Use `[[ ... ]]` for pattern matching; `[ ... ]` for POSIX:

```bash
# POSIX (portable)
if [ -f 'file.rs' ]; then
    echo "Rust file found"
fi

# Bash (more readable)
if [[ $FILE == *.rs ]]; then
    echo "Rust file found"
fi
```

### 4. Error Context

Capture and report why a hook failed:

```bash
set -e

cargo check || {
    echo "Error: Cargo check failed"
    cargo check --message-format=short 2>&1 | head -20
    exit 1
}
```

### 5. Output Redirection

Suppress noise or capture for later inspection:

```bash
# Suppress stdout/stderr
lsp-max-cli gate check > /dev/null 2>&1

# Capture to file
command_output=$(cargo check 2>&1)
if [[ $command_output == *"error"* ]]; then
    echo "Errors detected:"
    echo "$command_output"
fi
```

### 6. Temporary Files

Use `mktemp` for safe temporary storage:

```bash
tmpfile=$(mktemp)
trap "rm -f $tmpfile" EXIT  # Clean up on exit

lsp-max-cli diagnostic snapshot > "$tmpfile"
cat "$tmpfile"
```

---

## Troubleshooting

### Problem: Hook runs but doesn't block

**Cause:** Hook exited with 0 (success), even though error occurred.

**Solution:** Use explicit error handling:

```bash
# Before (wrong)
cargo check  # If fails, still exits 0

# After (correct)
cargo check || exit 1
```

Or use `set -e`:

```bash
set -e
cargo check  # Now exits 1 if command fails
```

---

### Problem: Hook takes too long

**Cause:** PreToolUse hook running heavy operations (tests, full builds).

**Solution:** Move to PostToolUse (non-blocking) or create a faster gate:

```json
{
  "type": "command",
  "command": "lsp-max-cli gate check"  # ~10ms, not 30 seconds
}
```

---

### Problem: Environment variables undefined in hook

**Cause:** Shell state not persistent across hook invocations.

**Solution:** Use a shell script file that is sourced:

```bash
# .claude/hook-lib.sh
export MY_VAR="value"
my_function() { echo $MY_VAR; }
```

```json
{
  "type": "command",
  "command": "source .claude/hook-lib.sh && my_function"
}
```

---

### Problem: Hook works locally but fails in Claude Code

**Cause:** Different $PATH, shell, or working directory in Claude Code.

**Solution:** Use absolute paths and explicit shell:

```json
{
  "type": "command",
  "command": "/bin/bash -c 'set -e; cd $(pwd) && lsp-max-cli gate check'"
}
```

Or verify the environment:

```json
{
  "type": "command",
  "command": "echo \"PWD: $(pwd)\" && echo \"PATH: $PATH\" && which lsp-max-cli"
}
```

---

### Problem: Gate file not found

**Cause:** Compositor process has not written the gate file yet (first session).

**Solution:** Gracefully handle missing gate file:

```bash
if [ -f "$gate_file" ]; then
    # Check gate
    lsp-max-cli gate check
else
    # Compositor not active yet; allow tool
    exit 0
fi
```

The `lsp-max-cli gate check` command returns "clear" when compositor is absent (exit 0).

---

## Reference: ANDON Gate Hook (lsp-max Example)

The lsp-max project uses hooks to enforce the `Λ_CD` runtime gate. This is the reference implementation:

### Configuration

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write|TaskCreate|NotebookEdit",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cd /Users/sac/lsp-max && lsp-max-cli diagnostic snapshot 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

### How It Works

1. **PreToolUse:** Before Bash/Edit/Write, run `lsp-max-cli gate check`
   - If gate is BLOCKED (exit 1), tool is prevented
   - If gate is OPEN (exit 0), tool proceeds

2. **Gate Check Implementation:**
   - Reads a single-byte file at `/tmp/lsp-max-gate-<workspace-hash>`
   - File contains: `0` (OPEN) or `1` (ANDON)
   - O(1) filesystem check; no IPC or subprocesses

3. **PostToolUse:** After each write, snapshot diagnostics
   - Non-blocking; does not affect tool result
   - Captures current conformance state for later analysis
   - `|| true` suppresses errors

### Why This Matters

The gate enforces **Λ_CD^runtime**: no shell-side action (build, test, format, release) may proceed while law violations are active. The hook is the enforcement mechanism — the agent cannot bypass it without modifying settings.json.

---

## Summary

Claude Code hooks enable powerful automation:

- **PreToolUse:** Fast, synchronous gates that block actions
- **PostToolUse:** Non-blocking side effects and observability
- **SessionStart:** One-time initialization and validation

Use them to:
- Enforce project policies (linting, formatting, testing)
- Implement law-state runtimes (gates, conformance)
- Integrate with external systems (CI, archives, notifications)
- Improve developer experience with automated checks

Always prioritize **speed** for PreToolUse and **clarity** in error messages.

---

## See Also

- `.claude/settings.json` — Hook configuration for your workspace
- `AGENTS.md` — Agent-specific hook considerations
- `CLAUDE.md` — Project-specific laws and gates
- Claude Code CLI documentation — Additional tool invocation details
