# IDE Quick Reference Card

**Print this or bookmark it for fast lookup.**

---

## Installation (One-Liners)

### VS Code
```bash
code --install-extension seanchatmangpt.lsp-max && cargo install lsp-max-cli
```

### JetBrains
```bash
# Settings → Plugins → Marketplace → Search "lsp-max" → Install
cargo install lsp-max-cli
```

### Web
```bash
cd lsp-max/web && npm install && npm run dev
# Open http://localhost:3000
```

### Desktop
```bash
# macOS
brew install lsp-max-desktop && open -a "LSP Max"

# Windows
choco install lsp-max-desktop
```

---

## Keyboard Shortcuts (Universal)

| Action | VS Code | JetBrains | macOS Desktop | Windows Desktop |
|--------|---------|-----------|---------------|-----------------|
| **Show Diagnostics** | `Ctrl+Shift+D` | `Ctrl+Shift+D` | `Cmd+2` | `Ctrl+2` |
| **Check Conformance** | `Ctrl+Shift+C` | `Ctrl+Shift+C` | `Cmd+1` | `Ctrl+1` |
| **View Gate Status** | `Ctrl+Shift+G` | `Ctrl+Shift+G` | `Cmd+3` | `Ctrl+3` |
| **Show Receipts** | `Ctrl+Shift+R` | `Ctrl+Shift+R` | `Cmd+2` | `Ctrl+2` |
| **Type Hierarchy** | `Shift+F12` | `Ctrl+H` | `Ctrl+H` | `Ctrl+H` |
| **Call Hierarchy (In)** | `Ctrl+Alt+Shift+H` | `Ctrl+Alt+Shift+H` | `Cmd+Alt+Shift+H` | `Ctrl+Alt+Shift+H` |
| **Call Hierarchy (Out)** | `Ctrl+Alt+H` | `Ctrl+Alt+H` | `Cmd+Alt+H` | `Ctrl+Alt+H` |
| **Toggle Inlay Hints** | `Ctrl+K Ctrl+I` | `Ctrl+Alt+I` | `Cmd+Alt+I` | `Ctrl+Alt+I` |
| **Search** | `Ctrl+K` (palette) | `Ctrl+N` | `Cmd+K` | `Ctrl+K` |

---

## Configuration (Settings.json / Config.toml)

### VS Code Quick Config
```json
{
  "lsp-max.enabled": true,
  "lsp-max.conformance.enableConformanceChecks": true,
  "lsp-max.performance.debounceMs": 500,
  "lsp-max.semanticTokens.enabled": true,
  "lsp-max.trace.server": "messages"
}
```

### JetBrains Quick Config
```toml
[server]
path = "/path/to/lsp-max-server"
enable = true

[performance]
debounce_ms = 500
max_parallel = 4
```

### Web Quick Config
```env
LSP_MAX_SERVER_URL=http://localhost:8080
NEXT_PUBLIC_CONFORMANCE_ENABLED=true
NEXT_PUBLIC_THEME=dark
```

### Desktop Quick Config
```toml
[server]
port = 8080
enable = true

[features]
conformance = true
gates = true
```

---

## Common Tasks

### Check Server Status
```bash
lsp-max-server --version          # Is it installed?
curl http://localhost:8080/max/state  # Is it running?
lsp-max-cli gate check            # What's the gate status?
```

### View Diagnostics
```bash
lsp-max-cli diagnostics list      # All diagnostics
lsp-max-cli diagnostics list --family=ANTI-LLM  # Filter by family
lsp-max-cli diagnostics export > diag.json      # Export for analysis
```

### Export State
```bash
lsp-max-cli snapshot export --format=json > snapshot.json
lsp-max-cli snapshot export --format=ocel > evidence.ocel.json
```

### Reset Gate
```bash
lsp-max-cli gate reset            # Force gate OPEN
lsp-max-cli gate check            # Check current state
```

### Check Conformance
```bash
lsp-max-cli conformance vector    # Full vector
lsp-max-cli conformance vector --format=json | jq  # Parse
```

---

## Performance Quick Fixes

### High CPU Usage
```json
{
  "lsp-max.semanticTokens.enabled": false,
  "lsp-max.inlayHints.enabled": false,
  "lsp-max.performance.debounceMs": 2000
}
```

### High Memory Usage
```json
{
  "lsp-max.performance.maxCachedDocuments": 20,
  "lsp-max.performance.enableCompression": true
}
```

### Large File Slowness
```json
{
  "[rust]": {
    "lsp-max.semanticTokens.enabled": false
  }
}
```

---

## Troubleshooting Checklist

- [ ] Server installed: `which lsp-max-server`
- [ ] Server running: `curl http://localhost:8080/max/state`
- [ ] Port free: `lsof -i :8080` (macOS/Linux)
- [ ] Firewall open: `sudo ufw allow 8080` (Linux)
- [ ] IDE extension/plugin enabled
- [ ] Gate status: `lsp-max-cli gate check`
- [ ] Active diagnostics: `lsp-max-cli diagnostics list`

### If Server Not Found
```bash
cargo install lsp-max-cli
export PATH="$HOME/.cargo/bin:$PATH"
which lsp-max-server
```

### If Port Conflict
```bash
lsof -i :8080 | grep LISTEN
kill -9 <PID>
# OR use different port:
lsp-max-server --port 8081
```

### If IDE Can't Connect
```bash
# Test manually
curl http://localhost:8080/max/state

# If error, check:
ps aux | grep lsp-max-server
lsp-max-server --log-level debug
```

---

## Feature Support Matrix (Quick)

| Feature | VS Code | JetBrains | Web | Desktop |
|---------|---------|-----------|-----|---------|
| **Diagnostics** | ✅ | ✅ | ✅ | ✅ |
| **Go to Definition** | ✅ | ✅ | ✅ | ⚠️ |
| **Type Hierarchy** | ✅ | ✅ | ✅ | ⚠️ |
| **Call Hierarchy** | ✅ | ✅ | ✅ | ⚠️ |
| **Conformance Vector** | ✅ | ✅ | ✅ | ✅ |
| **Receipt Ledger** | ✅ | ✅ | ✅ | ✅ |
| **ANDON Gate** | ✅ | ✅ | ✅ | ✅ |
| **OCEL Graph** | ⚠️ | ⚠️ | ✅ | ✅ |
| **Semantic Tokens** | ✅ | ✅ | ⚠️ | ✅ |
| **Inlay Hints** | ✅ | ✅ | ⚠️ | ✅ |

**Legend:** ✅ Supported | ⚠️ Partial | ❌ Unsupported

---

## File Paths

### Configuration Locations

**VS Code:** `.vscode/settings.json` (in project root)

**JetBrains:** 
- IDE Settings (Cmd+, or Ctrl+Alt+S)
- `~/.config/lsp-max/jetbrains.toml`

**Web:** `web/.env.local`

**Desktop:**
- macOS: `~/Library/Application\ Support/lsp-max/config.toml`
- Windows: `%APPDATA%\lsp-max\config.toml`

### Key Files

**Server logs:**
- VS Code: Output panel → lsp-max
- JetBrains: Tools → LSP-Max → Debug Console
- Desktop: Preferences → Debug

**Diagnostics:**
- CLI: `lsp-max-cli diagnostics export`
- All IDEs: Shown in UI

**Receipts:**
- Location: `receipts/*.receipt.json`
- View in: VS Code panel, JetBrains panel, Web app

---

## Common CLI Commands

```bash
# Status
lsp-max-server --version
lsp-max-cli --version
lsp-max-cli gate check

# Diagnostics
lsp-max-cli diagnostics list
lsp-max-cli diagnostics export
lsp-max-cli diagnostics view <ID>

# Conformance
lsp-max-cli conformance vector
lsp-max-cli conformance vector --format=json

# Snapshots
lsp-max-cli snapshot export --format=json
lsp-max-cli snapshot export --format=ocel
lsp-max-cli snapshot export --format=custom

# Gate Management
lsp-max-cli gate check
lsp-max-cli gate reset

# Server
lsp-max-server --port 8080
lsp-max-server --log-level debug
lsp-max-server --host 0.0.0.0
```

---

## Environment Variables

```bash
# Logging
export RUST_LOG=debug,lsp_max=trace

# Gate File Location
export LSP_MAX_GATE_FILE=/tmp/lsp-max.gate

# Process Mining
export WASM4PM_TRACE=true

# Receipt Verification
export BLAKE3_VERIFY_RECEIPTS=true

# Server Options
export LSP_MAX_PORT=8080
export LSP_MAX_HOST=127.0.0.1
```

---

## Help & Support

| Issue | Command/Link |
|-------|--------------|
| **Installation help** | [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md) |
| **Feature question** | [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md) |
| **Troubleshooting** | [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md) |
| **Full reference** | [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md) |
| **Navigation** | [IDE_INDEX.md](IDE_INDEX.md) |
| **GitHub issues** | https://github.com/seanchatmangpt/lsp-max/issues |
| **GitHub discussions** | https://github.com/seanchatmangpt/lsp-max/discussions |

---

## Version Info

- **lsp-max:** 26.6.9 (CalVer)
- **Minimum IDEs:**
  - VS Code: 1.85+
  - IntelliJ: 2024.1+
  - JetBrains: 2024.1+
  - Node.js: 18+ (web)

---

## Quick Links

- **📍 Start here:** [IDE_INDEX.md](IDE_INDEX.md)
- **🚀 Setup:** [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md)
- **📊 Features:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md)
- **🔧 Configure:** [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md)
- **🐛 Fix issues:** [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md)

---

**Print me! Bookmark me!**

Last updated: 2026-06-14 | Version: 26.6.9
