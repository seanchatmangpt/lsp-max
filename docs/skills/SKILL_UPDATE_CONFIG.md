# Skill: /update-config

**Status:** AVAILABLE | **Scope:** Configuration Management | **Category:** Configuration & Setup

---

## Overview

Configure the Claude Code harness via `settings.json` and `settings.local.json`. Manages permissions, environment variables, hooks, and behavioral automation rules. **The authoritative tool for all configuration changes.**

## When to Use

Use `/update-config` when you want to:
- Grant or revoke permissions (npm, git, bash, mcp tools)
- Set environment variables for the session
- Configure automation hooks (PreToolUse, PostToolUse, etc.)
- Move permissions between global and user-local scope
- Troubleshoot hook failures
- Setup ANDON gate enforcement

**Do NOT use `/update-config` for:**
- Simple theme/model changes (use `/config` instead)
- Keyboard shortcuts (use `/keybindings-help` instead)

## Parameters

```bash
/update-config "intent"
```

| Intent Type | Examples |
|------------|----------|
| **Permissions** | "allow npm commands", "add bq permission", "move permission to user settings" |
| **Environment** | "set DEBUG=true", "export DATABASE_URL=postgres://..." |
| **Hooks** | "when tests fail, show summary", "before commit, lint" |
| **Troubleshooting** | "fix hook X", "debug permissions" |

## Invocation

```bash
# Grant a permission
/update-config "allow npm commands"

# Set environment variable
/update-config "set DEBUG=true"

# Configure a hook
/update-config "when claude stops show X"

# Move permission to user scope
/update-config "move permission to user settings"

# Add a permission
/update-config "add bq permission to global settings"
```

## Configuration Scopes

### Global (`.claude/settings.json`)

Shared across all sessions in this project. Checked into version control.

```json
{
  "permissions": {
    "bash": ["git status", "ls"],
    "npm": ["*"]
  },
  "hooks": {
    "PreToolUse": "lsp-max-cli gate check"
  }
}
```

### User-Local (`.claude/settings.local.json`)

User-specific, session-scoped overrides. Git-ignored.

```json
{
  "permissions": {
    "bash": ["lsof"]
  }
}
```

## Configuration Categories

### 1. Permissions

Grant or deny access to tools.

```bash
/update-config "allow npm commands"
/update-config "allow git push"
/update-config "add bq permission"
/update-config "deny bash wildcard"
```

**Permission types:**
- `bash` — Shell commands
- `npm` — Node package manager
- `git` — Git version control
- `mcp` — MCP tools (GitHub, Google Calendar, etc.)
- `file-system` — File read/write

### 2. Environment Variables

Set session-wide variables.

```bash
/update-config "set NODE_ENV=development"
/update-config "set DEBUG=true"
/update-config "set DATABASE_URL=postgres://localhost/mydb"
```

Variables are available to all executed commands in the session.

### 3. Hooks

Setup automation behaviors.

```bash
/update-config "when tests fail, show summary"
/update-config "before commit, run lint"
/update-config "PreToolUse hook: lsp-max-cli gate check"
```

**Hook types:**
- `PreToolUse` — Before any Bash/MCP call (ANDON gate enforcement)
- `PostToolUse` — After Bash/MCP call completes
- `PreCommit` — Before git commit
- Custom behaviors — "whenever X, do Y"

### 4. Scoping

Move permissions between global and user-local.

```bash
/update-config "move permission to global settings"
/update-config "move permission to user settings"
```

## Expected Output

### Success

```
✅ Configuration Updated

Change: Allow npm commands globally

Modified: .claude/settings.json
  permissions.npm: ["*"]

Status: ADMITTED
Permissions updated. npm commands now allowed.
```

### Validation Error

```
⚠️  Configuration Change Blocked

Requested: allow git push

Check: Is this safe?
  [!] Permission granted: allow git push to any branch

Suggestion: 
  /update-config "allow git push to feature/* only"
  (more restrictive)

Status: CANDIDATE (requires confirmation)
```

## Integration

### Works with Other Skills

- **`/session-start-hook`** — After hook setup, configure via `/update-config`
- **`/fewer-permission-prompts`** — Auto-generates allowlist
- **`/keybindings-help`** — Separate tool for keyboard (not `/update-config`)

### ANDON Gate Pattern

The `PreToolUse` hook enforces law-state runtime gates:

```bash
/update-config "PreToolUse: lsp-max-cli gate check"

# Now, before EVERY bash tool call:
# - lsp-max-cli gate check runs
# - Exit 0 → proceed
# - Exit 1 → ANDON set; block until gate clears
```

## Examples

### Example 1: Setup Node.js Project

```bash
$ /update-config "allow npm commands"
✅ npm permission granted

$ /update-config "set NODE_ENV=development"
✅ NODE_ENV=development set

$ /update-config "allow git status, git log"
✅ git read-only commands allowed
```

### Example 2: Configure Permissions

```bash
$ /update-config "add database permission to global"
✅ bq permission added (global scope)

$ /update-config "move permission to user settings"
✅ Moved to user-local (.claude/settings.local.json)

$ /update-config "deny bash wildcard"
⚠️  Restricting shell access to specific commands only
✅ Wildcard disabled (whitelist only)
```

### Example 3: Setup Git Hooks

```bash
$ /update-config "before commit, run lint"
✅ PreCommit hook configured

$ /update-config "when tests fail, notify user"
✅ PostTest hook configured
```

## Configuration Files Reference

### `.claude/settings.json` (Shared)

```json
{
  "permissions": {
    "bash": ["git status", "git log", "ls", "find"],
    "npm": ["*"],
    "git": ["clone", "pull", "status"],
    "mcp": ["github:search_code"]
  },
  "environment": {
    "NODE_ENV": "development",
    "LOG_LEVEL": "debug"
  },
  "hooks": {
    "PreToolUse": "lsp-max-cli gate check",
    "PostToolUse": "echo 'Tool executed'"
  },
  "automation": {
    "on_test_failure": "show summary",
    "before_commit": "run lint"
  }
}
```

### `.claude/settings.local.json` (User-specific, Git-ignored)

```json
{
  "permissions": {
    "bash": ["lsof", "ps aux"]
  },
  "environment": {
    "DEBUG": "true"
  }
}
```

## Troubleshooting

### "Permission denied for tool X"

```bash
# Check current permissions
/update-config

# Allow the tool
/update-config "allow X"
```

### "Hook not working"

```bash
# Verify hook configuration
/update-config "fix hook PreToolUse"

# Or reconfigure
/update-config "PreToolUse: /path/to/hook/script"
```

### "Accidentally granted too broad permission"

```bash
# Revoke and re-grant more narrowly
/update-config "revoke npm wildcard"
/update-config "allow npm install, npm start only"
```

## Best Practices

✓ **Do:**
- Use global settings for project-wide config (checked in)
- Use user-local for personal preferences (not checked in)
- Review permission changes before confirming
- Use specific command whitelists over wildcards

✗ **Don't:**
- Grant wildcard permissions to all bash commands
- Hardcode secrets in settings (use env var references)
- Change settings during critical operations
- Forget to test after configuration change

## See Also

- [`/keybindings-help`](SKILL_KEYBINDINGS_HELP.md) — Configure keyboard (not settings)
- [`/session-start-hook`](SKILL_SESSION_START_HOOK.md) — Setup hooks first
- [`/fewer-permission-prompts`](SKILL_FEWER_PERMISSION_PROMPTS.md) — Auto-generate safe allowlist
- [CLAUDE.md](../CLAUDE.md) — Project configuration reference

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
