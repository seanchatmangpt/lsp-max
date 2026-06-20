# Skill: /run

**Status:** AVAILABLE | **Scope:** App Execution | **Category:** Code Execution & Automation

---

## Overview

Launch and drive this project's app to see a change working. The `/run` skill automatically detects your project type and executes the appropriate launch command, with support for CLI, server, TUI, Electron, and browser-based applications.

## When to Use

Use `/run` when you want to:
- See the app start and verify it launches successfully
- Test changes in a live environment (not automated tests)
- Observe console output or app behavior
- Validate that build artifacts work as expected

**Do NOT use `/run` for:**
- Validating behavior against a specification (use `/verify` instead)
- Automated integration testing (use `cargo test` or equivalent)
- Generating receipts/evidence of behavior (use `/verify` instead)

## Parameters

**None** — The skill auto-detects project type and launch command.

## Invocation

```bash
/run
```

## How It Works

### Step 1: Project Type Detection

The skill examines your project to identify its type:

| Project Type | Detection | Launch Method |
|--------------|-----------|----------------|
| **CLI** | `Cargo.toml` with `[[bin]]` or `bin/` dir; Node with `#!/usr/bin/env node` | Executes binary: `./target/release/app` or `node bin/cli.js` |
| **Server** | `Cargo.toml` with `[dependencies]` tokio; Node with express/fastify; Python with Flask/Django | Starts server: `cargo run` or `npm start` |
| **TUI** | Rust with `termion`/`crossterm`; Node with `blessed` | Launches terminal UI: `cargo run` or `node tui.js` |
| **Electron** | `package.json` with `electron` dependency and `main` field | Launches desktop: `npm start` |
| **Web (Browser)** | `package.json` with webpack/vite/next; `Cargo.toml` with wasm-pack | Dev server: `npm run dev` or starts browser |
| **Library** | `Cargo.toml` or `package.json` with no entry point; tests present | Runs test harness or docs site |

### Step 2: Command Execution

Once the project type is detected, the skill:

1. **Resolves the launch command** — Checks for project-specific skill, falls back to built-in patterns
2. **Installs dependencies** if needed (npm install, cargo build, etc.)
3. **Executes the launch** with the appropriate runner
4. **Waits for readiness** — Polls for port binding, file watch signal, or console output

### Step 3: Observation

The skill captures and reports:
- **App state** — RUNNING, READY, FAILED, BLOCKED
- **Console output** — Startup logs, errors, debug messages
- **Network activity** — Ports listening, connections established
- **Window/browser** — Desktop or browser window opened

## Expected Output

### Success

```
🚀 Project type detected: Node.js server
📦 Dependencies: installed (package-lock.json OK)
▶️  Launching: npm start
📍 Server ready: http://localhost:3000
✅ Status: RUNNING

App is ready. You can now:
- Visit http://localhost:3000 in browser
- Send requests (curl http://localhost:3000/api/test)
- Check console output below
- Test manually or use /verify for automated validation
```

### Failure

```
❌ Project type: Rust CLI
⚠️  Build failed: error: linker `cc` not found
📋 Suggestion: Install build tools (gcc, clang)
   - Ubuntu: sudo apt-get install build-essential
   - macOS: xcode-select --install
   - Windows: Install Visual Studio Build Tools

Next: Fix error, then run /run again
```

## Integration with Other Skills

### Before `/run`

- `/init` — Initialize project docs (recommended first time)
- `/update-config` — Set environment variables or build flags

### After `/run`

- **`/verify`** — Validate that the running app behaves as expected
  - Chain: `/run` → `/verify` (most common)
- **Manual testing** — Explore the app in browser, terminal, etc.
- **Debugging** — Check logs, use browser DevTools, attach debugger

### Parallel Work

- **`/loop 10s /run`** — Restart app every 10 seconds (during development)
- **Other terminal** — Run linter, tests, etc. while app is running

## Examples

### Example 1: Rust Binary

```bash
$ /run

🚀 Project type detected: Rust CLI (binary target)
📦 Building: cargo build --release
   Compiling my-cli v1.0.0...
   Finished release [optimized] target(s)

▶️  Launching: ./target/release/my-cli --help

Usage: my-cli [OPTIONS] COMMAND

Commands:
  run     Run something
  check   Check configuration
  help    Print help

✅ Status: READY
Binary is functional. Try: ./target/release/my-cli run --help
```

### Example 2: Node.js Server

```bash
$ /run

🚀 Project type detected: Node.js server
📦 Dependencies: npm install (2.3s)

▶️  Launching: npm start
   [nodemon] starting `node src/server.js`
   Server listening on http://localhost:3000

✅ Status: RUNNING
Ready for requests. Test with:
  curl http://localhost:3000/api/status
```

### Example 3: Browser-Based Web App

```bash
$ /run

🚀 Project type detected: React (Webpack)
📦 Dependencies: npm install

▶️  Launching: npm run dev
   webpack serve --mode development
   <i> [webpack-dev-server] Project is running at:
   <i> http://localhost:8080/

📱 Opening browser: http://localhost:8080

✅ Status: RUNNING
App loaded in browser. Make changes; hot reload active.
```

## Troubleshooting

### "Port already in use"

```
❌ Failed to start server
⚠️  Port 3000 is already in use

Actions:
1. Kill existing process: lsof -i :3000 | grep LISTEN | awk '{print $2}' | xargs kill -9
2. Change port: PORT=3001 npm start
3. Run again: /run
```

### "Dependency not installed"

```
❌ Failed to launch
⚠️  Missing dependency: gcc (required for Rust build)

Fix:
- Ubuntu: sudo apt-get install build-essential
- macOS: xcode-select --install
- Windows: Install Visual Studio Build Tools

Then: /run
```

### "Project type not detected"

```
⚠️  Could not auto-detect project type

Manual launch:
1. Identify your project type (CLI, server, web, etc.)
2. Run the launch command directly:
   cargo run
   npm start
   python app.py
3. Or update-config with custom launch command:
   /update-config "set LAUNCH_CMD=cargo run"
```

## Configuration

### Custom Launch Command

If auto-detection doesn't work, configure explicitly:

```bash
/update-config "set LAUNCH_CMD=cargo run --release"
```

### Environment Variables

Set variables for the running app:

```bash
/update-config "set DATABASE_URL=postgres://localhost/mydb"
/update-config "set LOG_LEVEL=debug"
```

## Differences from Similar Skills

| Skill | Purpose | Output |
|-------|---------|--------|
| **`/run`** | Launch app, see it start | App running; console visible |
| **`/verify`** | Launch AND validate behavior | Receipt: ADMITTED or REFUSED |
| **`/loop`** | Restart app on interval | Recurring restarts with status |
| **Built-in test** | Automated unit/integration tests | Test pass/fail counts |

## Related Concepts

- **CLAUDE.md** — Project constitution (defines launch commands)
- **Port binding** — Server must listen on a port to be "RUNNING"
- **Hot reload** — Some frameworks auto-reload on file changes (webpack, nodemon)
- **Build step** — Some projects require compilation before run (Rust, TypeScript)

## See Also

- [`/verify`](SKILL_VERIFY.md) — Validate behavior after running
- [`/loop`](SKILL_LOOP.md) — Restart app on interval
- [`/init`](SKILL_INIT.md) — Initialize project docs
- [CLAUDE.md](../CLAUDE.md) — Project architecture (defines launch patterns)

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
