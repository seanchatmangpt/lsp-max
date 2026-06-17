# Skill: /session-start-hook

**Status:** AVAILABLE | **Scope:** Web Session Setup | **Category:** Configuration & Setup

---

## Overview

Create and develop startup hooks for Claude Code on the web. Ensures a repository can run tests and linters during web sessions without manual configuration.

**Availability:** Contextual (Claude Code on Web)

## When to Use

Use `/session-start-hook` when you want to:
- Setup automatic test/lint runs at session start (web)
- Ensure reproducible environment for each session
- Auto-install dependencies on session startup
- Configure build artifacts caching
- Setup pre-session validation

**Typical:** Run once per project that uses Claude Code on web.

## Parameters

**None** — Interactive guidance for web session setup.

```bash
/session-start-hook
```

## How It Works

### Phase 1: Environment Detection

Detect:
- Web session context (Claude Code on web)
- Project type (Node, Rust, Python, etc.)
- Build system (npm, cargo, pip, etc.)
- Test framework (jest, pytest, cargo test, etc.)

### Phase 2: Hook Generation

Create hook script that:
1. Installs dependencies (npm install, cargo build, etc.)
2. Runs validation (tests, linters)
3. Reports status
4. Caches artifacts for reuse

### Phase 3: Hook Registration

Register in `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": "scripts/session-start.sh"
  }
}
```

## Expected Output

```
🔧 Session Start Hook Setup

Environment: Claude Code on Web
Project type: Node.js (React + Webpack)
Build system: npm
Tests: Jest (configured)
Linter: ESLint (configured)

Generated hook: scripts/session-start.sh
  [✓] Install dependencies (npm install)
  [✓] Build project (npm run build)
  [✓] Run tests (npm test)
  [✓] Lint code (npm run lint)
  [✓] Report status

Registered in: .claude/settings.json

Status: ADMITTED
Next: Test hook with /verify
```

## Integration

### Follows `/init`

```
/init                   (create CLAUDE.md)
  ↓
/session-start-hook    (setup hooks)
  ↓
/update-config         (configure)
```

### Precedes `/verify`

```
/session-start-hook    (setup)
  ↓
/verify                (test hooks work)
```

## Example Hook Script

```bash
#!/bin/bash
# scripts/session-start.sh
# Runs at Claude Code web session start

set -e

echo "🔄 Starting session..."

# 1. Install dependencies
echo "📦 Installing dependencies..."
npm install --quiet

# 2. Build
echo "🔨 Building project..."
npm run build

# 3. Run tests
echo "🧪 Running tests..."
npm test -- --passWithNoTests

# 4. Lint
echo "✨ Linting code..."
npm run lint || true  # Non-blocking

echo "✅ Session ready"
exit 0
```

## See Also

- [`/init`](SKILL_INIT.md) — Initialize project first
- [`/update-config`](SKILL_UPDATE_CONFIG.md) — Configure after hook setup
- [`/verify`](SKILL_VERIFY.md) — Test hook works

---

**Last Updated:** 2026-06-14 | **Status:** AVAILABLE
