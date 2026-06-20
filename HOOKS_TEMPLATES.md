# Claude Code Hooks Templates — Ready-to-Use Examples

This document provides copy-paste hook templates for common workflows. Customize the commands to match your project structure.

---

## Template 1: ANDON Gate (Λ_CD^runtime)

Enforce a law-state gate that blocks shell actions when violations are active.

**Use when:** Your project uses lsp-max compositor with active diagnostics that must be resolved before proceeding.

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
            "command": "cd $(pwd) && lsp-max-cli diagnostic snapshot 2>/dev/null || true"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- PreToolUse: Checks if ANDON gate is set; blocks Bash/Edit/Write if true
- PostToolUse: Captures diagnostic snapshot after writes
- Exit 0 = gate clear, proceed; Exit 1 = gate blocked, tool prevented

**Customization:**
- Change matched tools by editing `"matcher"`
- Replace `lsp-max-cli` with your gate command if different
- Adjust snapshot directory path if needed

**Verification:**
```bash
# Test locally
cd your-workspace
lsp-max-cli gate check
echo "Exit code: $?"
```

---

## Template 2: Pre-commit Formatting & Linting

Enforce `cargo fmt` and `clippy` checks before edits are persisted.

**Use when:** You want all changes automatically formatted and checked for warnings.

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

**What it does:**
- Before Edit/Write: Run `cargo fmt` and `clippy`
- If either fails, tool is blocked
- Forces all code to be formatted before writing

**Customization:**
- Use `cargo fmt --check` to report without modifying (read-only):
```json
{
  "type": "command",
  "command": "cargo fmt --check"
}
```
- Replace `cargo` with your language's formatter (`black`, `prettier`, `rustfmt`, etc.)
- Add `--all-features` or `--no-default-features` as needed

**Performance note:** This blocks all edits. If 30+ seconds, move to PostToolUse:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cargo clippy --workspace --all-targets --all-features 2>&1 | grep -E '(error|warning)' || true"
          }
        ]
      }
    ]
  }
}
```

---

## Template 3: Run Test Suite After Changes

Execute test suite asynchronously after every write. Non-blocking.

**Use when:** You want to catch breaking changes quickly but not block editing.

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

**What it does:**
- After Edit/Bash: Spawn test suite in background
- Returns immediately; does not block tool
- Test results saved to `/tmp/test-results-<timestamp>.log`
- Check results manually later

**Customization:**
- Replace `cargo test` with your test command
- Change `/tmp` to your preferred log directory
- Remove `2>&1` to suppress stderr
- Use `&& echo 'Tests complete'` instead of `&` to block (not recommended)

**Viewing results:**
```bash
# List test log files
ls -lt /tmp/test-results-*.log | head -5

# View latest test output
tail -f /tmp/test-results-*.log
```

---

## Template 4: Validate Configuration on Session Start

Check configuration files exist and are valid JSON/TOML before starting session.

**Use when:** Your project requires specific configuration files to exist.

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "echo 'Validating configuration...' && [ -f '.claude/settings.json' ] && cat .claude/settings.json | jq . > /dev/null && [ -f 'Cargo.toml' ] && cat Cargo.toml | grep -q '\\[package\\]' && echo 'Configuration valid' || { echo 'Configuration validation failed'; exit 1; }"
      }
    ]
  }
}
```

**What it does:**
- Checks `.claude/settings.json` exists and is valid JSON
- Checks `Cargo.toml` exists and contains `[package]` section
- Blocks session if either check fails
- Displays validation status

**Customization:**
- Add more files by chaining `[ -f file ] &&` conditions
- Validate YAML with: `cat file | python3 -c "import yaml; yaml.safe_load(__import__('sys').stdin)"`
- Validate TOML with: `cargo metadata --format-version 1 > /dev/null`

**Simpler version (just check existence):**

```json
{
  "type": "command",
  "command": "[ -f '.claude/settings.json' ] && [ -f 'Cargo.toml' ] && echo 'Required files present' || { echo 'Missing configuration'; exit 1; }"
}
```

---

## Template 5: Check Dependencies Before Bash

Ensure required tools are installed before allowing shell commands.

**Use when:** Your project requires external tools (just, cargo, python, etc.).

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "for cmd in cargo just rustfmt clippy-driver; do which $cmd > /dev/null || { echo \"Missing dependency: $cmd\"; exit 1; }; done && echo 'All dependencies available'"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Checks `cargo`, `just`, `rustfmt`, `clippy-driver` are in PATH
- Blocks Bash if any are missing
- Lists missing dependency in error

**Customization:**
- Add your required tools to the `for` loop:
```bash
for cmd in cargo just python3 docker; do ...
```
- Or check single command:
```bash
which cargo > /dev/null || { echo 'Cargo not found'; exit 1; }
```

---

## Template 6: Archive Build Artifacts on Success

After successful build, capture release binary to archive directory.

**Use when:** You want to preserve built artifacts for release or testing.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if [ -d './target/release' ] && [ $(find ./target/release -name 'lsp-max-cli' -type f 2>/dev/null | wc -l) -gt 0 ]; then mkdir -p /tmp/artifacts && cp ./target/release/lsp-max-cli /tmp/artifacts/lsp-max-cli-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)-$(date +%s) && echo 'Build artifact archived'; fi"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- After Bash execution, checks if release binary exists
- If found, copies to `/tmp/artifacts/` with timestamp and commit hash
- Non-blocking; does not affect tool result

**Customization:**
- Change `lsp-max-cli` to your binary name
- Change `/tmp/artifacts` to your archive path
- Add versioning: `$(cat Cargo.toml | grep '^version' | cut -d'"' -f2)`
- Compress: `tar -czf /tmp/artifacts/build-$(date +%s).tar.gz ./target/release`

---

## Template 7: Verify Tests Pass Before Release

Strict gate that runs full test suite before allowing release builds.

**Use when:** You want CI-grade validation before publishing or deploying.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if grep -q 'cargo publish\\|cargo release' <<< \"$COMMAND\" 2>/dev/null || [[ $(pwd) == *'main'* ]] || [[ $(git branch --show-current 2>/dev/null) == 'main' ]]; then echo 'Running full test suite before release...' && cargo test --workspace --all-features && just dx-polish && echo 'All checks passed' || { echo 'Release validation failed'; exit 1; }; fi"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Detects release-like operations (publish, main branch)
- Runs full test suite and polish checks
- Blocks if any check fails
- Passes through on non-release branches

**Customization:**
- Replace `cargo test` with your test command
- Add `--include-ignored` for comprehensive testing
- Use `just dx-verify` if your justfile has verification tasks
- Change branch name from `main` if needed

**Note:** `$COMMAND` may not be available; use filesystem checks instead:

```bash
if grep -q 'publish' Cargo.toml && [[ $(git branch --show-current) == 'main' ]]; then
    # Run release checks
fi
```

---

## Template 8: Generate Documentation After Writes

Build documentation and syntax-check after significant edits.

**Use when:** You maintain markdown or generated docs that must stay in sync.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "if find . -name '*.md' -newer /tmp/.hook-timestamp 2>/dev/null | grep -q .; then echo 'Documentation updated; rebuilding...' && cargo doc --no-deps --document-private-items > /tmp/doc-build-$(date +%s).log 2>&1 & touch /tmp/.hook-timestamp; fi"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Detects recent markdown changes
- Triggers documentation rebuild in background
- Saves build log for inspection
- Non-blocking

**Customization:**
- Replace `cargo doc` with your doc builder (`mdbook`, `sphinx`, `make docs`, etc.)
- Track doc freshness by comparing timestamps
- Use `cargo test --doc` to verify doc examples compile
- Generate table of contents: `doctoc *.md`

---

## Template 9: Lint Markdown & Documentation

Check markdown files for common errors before commit.

**Use when:** You maintain README, guides, and want to catch formatting issues.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write",
        "hooks": [
          {
            "type": "command",
            "command": "find . -name '*.md' -type f | xargs -I {} sh -c 'grep -E \"^# .*# \" {} && { echo \"Error: Duplicate headers in {}\"; exit 1; } || true' && echo 'Markdown validation passed'"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Checks for duplicate headers (common markdown error)
- Blocks Write if found
- Returns success if all markdown is valid

**Customization:**
- Add `markdownlint` if installed: `markdownlint *.md`
- Check for long lines: `awk 'length > 100 { exit 1 }' *.md`
- Verify links: `markdown-link-check *.md`
- Check spelling: `aspell check *.md`

**Install linters:**
```bash
# Markdown lint (Node.js)
npm install -g markdownlint-cli

# Markdown link check
npm install -g markdown-link-check

# Spell check
brew install aspell  # macOS
# or: apt-get install aspell-en  # Linux
```

---

## Template 10: Enforce Commit Message Policy

Validate commit messages before allowing git operations.

**Use when:** You require conventional commits or specific message formats.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if [[ \"$*\" =~ git.*commit ]]; then echo 'Validating commit message format...'; read -p 'Commit type (feat|fix|docs|style|refactor|test|chore): ' type && [[ \"$type\" =~ ^(feat|fix|docs|style|refactor|test|chore)$ ]] || { echo 'Invalid commit type'; exit 1; }; fi"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Detects git commit operations
- Prompts for conventional commit type
- Validates type matches allowed list
- Blocks if invalid

**Better approach (use git hooks):**

This is better handled by actual git hooks (`.git/hooks/commit-msg`) rather than Claude Code hooks, since the hook runs before you see the prompt.

**Alternative (static pattern validation):**

```json
{
  "type": "command",
  "command": "echo 'Use conventional commits: feat|fix|docs|style|refactor|test|chore(scope): message'"
}
```

---

## Template 11: Synchronize State Across Workspaces

After changes, replicate state to sibling repositories or cloud storage.

**Use when:** Multiple linked projects must stay synchronized.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "if [ -d '../lsp-types-max' ]; then echo 'Syncing sibling repos...' && (cd ../lsp-types-max && git pull 2>/dev/null) && echo 'Sibling sync complete'; fi"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- After any change, pulls latest from sibling repo
- Non-blocking; does not affect tool result
- Only runs if sibling directory exists

**Customization:**
- Replace `git pull` with `git fetch && git merge`, `rsync`, or cloud sync (`aws s3 sync`, `gsutil cp`)
- Sync multiple repos:
```bash
for dir in ../lsp-types-max ../wasm4pm ../wasm4pm-compat; do
  [ -d "$dir" ] && (cd "$dir" && git pull)
done
```

---

## Template 12: Email/Slack Notification on Success

Send notification when major task completes successfully.

**Use when:** You want to track progress or alert on release events.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "if [ -f '/tmp/.release-in-progress' ] && cargo test --workspace --quiet; then echo 'Release tests passed!' && rm /tmp/.release-in-progress && echo '{\"text\":\"Release tests PASSED\"}' | curl -X POST -H 'Content-Type: application/json' -d @- $SLACK_WEBHOOK 2>/dev/null || true; fi"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Monitors for release flag file
- Runs tests
- If passed, posts to Slack webhook
- Cleans up flag

**Customization:**
- Set Slack webhook: `export SLACK_WEBHOOK="https://hooks.slack.com/services/..."`
- Use email instead of Slack: `echo 'Subject: Build passed' | mail user@example.com`
- Use HTTP notification: `curl -d 'status=passed' https://your-api.com/webhook`

**Security note:** Never hardcode secrets. Use environment variables:

```bash
# In .env (not in repo)
export SLACK_WEBHOOK="https://..."
export GITHUB_TOKEN="ghp_..."

# In hook
source .env && curl -d "..." $SLACK_WEBHOOK
```

---

## Template 13: Create Session Log File

Log all session activity to a timestamped file for audit trail.

**Use when:** You need to record what happened during the session.

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "mkdir -p .claude/logs && echo \"Session started: $(date)\" > .claude/logs/session-$(date +%Y%m%d-%H%M%S).log && echo 'Session log created'"
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "echo \"[$(date +'%Y-%m-%d %H:%M:%S')] Tool invoked\" >> .claude/logs/session-*.log"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- SessionStart: Creates timestamped log file in `.claude/logs/`
- PreToolUse: Appends tool invocation timestamps to log
- Provides audit trail of all actions

**Customization:**
- Add tool names: `echo "[$(date +'%Y-%m-%d %H:%M:%S')] Tool: $TOOL_NAME" >> ...`
- Include git commit hash: `echo ... $(git rev-parse --short HEAD) ...`
- Archive logs: `gzip .claude/logs/*.log`

---

## Template 14: Check Disk Space Before Large Operations

Gate large writes or builds if disk is low.

**Use when:** Your workspace is on a size-limited filesystem.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Write",
        "hooks": [
          {
            "type": "command",
            "command": "available=$(df . | tail -1 | awk '{print $4}') && [ \"$available\" -lt 1048576 ] && { echo \"Low disk space: $(($available / 1024))MB remaining\"; exit 1; } || echo 'Disk space OK'"
          }
        ]
      }
    ]
  }
}
```

**What it does:**
- Checks available disk space in current directory
- Blocks if less than 1GB available
- Warns with remaining space

**Customization:**
- Change threshold: `1048576` is 1GB in KB; use `5242880` for 5GB
- Alternative using `df -H`: `df -H . | tail -1 | grep -oP '\d+G' | head -1 | grep -q '[0-9]'`

---

## Template 15: Run Workspace Health Check

Multi-step validation of workspace state (files, dependencies, git).

**Use when:** You want comprehensive pre-flight checks before work begins.

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "./.claude/health-check.sh"
      }
    ]
  }
}
```

**Companion shell script (`.claude/health-check.sh`):**

```bash
#!/bin/bash
set -e

echo '=== Workspace Health Check ==='

# Check required files
echo -n 'Checking required files... '
for f in Cargo.toml justfile .claude/settings.json; do
  [ -f \"$f\" ] || { echo \"FAIL ($f missing)\"; exit 1; }
done
echo 'OK'

# Check git status
echo -n 'Checking git status... '
[ -d '.git' ] || { echo 'FAIL (not a git repo)'; exit 1; }
echo 'OK'

# Check dependencies
echo -n 'Checking dependencies... '
for cmd in cargo just rustfmt; do
  which \"$cmd\" > /dev/null || { echo \"FAIL ($cmd not found)\"; exit 1; }
done
echo 'OK'

# Check disk space
echo -n 'Checking disk space... '
available=$(df . | tail -1 | awk '{print $4}')
[ \"$available\" -gt 1048576 ] || { echo \"FAIL (< 1GB available)\"; exit 1; }
echo 'OK'

echo '=== All checks passed ==='
```

**Usage:**

```bash
chmod +x ./.claude/health-check.sh
# Hook will run on session start
```

---

## Combining Templates

You can combine multiple templates in a single hooks configuration:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "./.claude/health-check.sh"
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      },
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings"
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
      },
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cargo test --workspace > /tmp/test-results-$(date +%s).log 2>&1 & true"
          }
        ]
      }
    ]
  }
}
```

This configuration:
1. **SessionStart:** Validates workspace on session begin
2. **PreToolUse (gate):** Enforces ANDON gate before all actions
3. **PreToolUse (lint):** Enforces formatting before edits
4. **PostToolUse (snapshot):** Captures diagnostics after changes
5. **PostToolUse (test):** Runs tests asynchronously after edits

---

## Tips for Customizing Templates

1. **Test locally first:**
```bash
# Test hook command before adding to settings.json
cd /your/workspace
lsp-max-cli gate check
echo "Exit code: $?"
```

2. **Use absolute paths for safety:**
```bash
# BAD (may fail due to CWD changes)
my-command && save-to-log > ./output.log

# GOOD (absolute path)
my-command && save-to-log > $(pwd)/output.log
```

3. **Handle errors gracefully:**
```bash
# BAD (fails silently)
my-command > /dev/null

# GOOD (logs error)
my-command || { echo "Command failed: $?"; exit 1; }
```

4. **Keep hooks fast (PreToolUse):**
- Target: <500ms for blocking hooks
- Use background processes (`&`) for long operations
- Move heavy operations to PostToolUse (non-blocking)

5. **Document hook intent:**
Add a `.claude/HOOKS_README.md` explaining each hook's purpose:

```markdown
# Workspace Hooks

## PreToolUse: Gate Check
Enforces ANDON gate before shell actions. Blocks Bash/Edit/Write if violations active.

## PreToolUse: Code Quality
Runs cargo fmt + clippy before edits. Ensures code meets project standards.

## PostToolUse: Snapshot
Captures diagnostic state after changes for audit trail.
```

---

## See Also

- `HOOKS_GUIDE.md` — Comprehensive hook system documentation
- `AGENTS.md` — Agent-specific hook usage patterns
- `CLAUDE.md` — Project-specific hooks configuration
