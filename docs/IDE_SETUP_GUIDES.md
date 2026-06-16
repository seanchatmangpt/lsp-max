# IDE Setup Guides for lsp-max

Step-by-step setup instructions for different development workflows and IDE combinations.

## Quick Setup (5 minutes)

### For VS Code Users

```bash
# 1. Install extension
code --install-extension seanchatmangpt.lsp-max

# 2. Install server
cargo install lsp-max-cli

# 3. Open any Rust project
code ~/my-rust-project

# Done! Diagnostics should appear automatically.
```

### For JetBrains Users

```bash
# 1. Open Settings → Plugins → Marketplace
# 2. Search "lsp-max" and click Install
# 3. Restart IDE
# 4. Install server:
cargo install lsp-max-cli

# Done! Features available via Tools → LSP-Max
```

### For Desktop Users (No IDE Required)

```bash
# macOS
brew install lsp-max-desktop
open -a "LSP Max"

# Windows
choco install lsp-max-desktop
# or download .msi from GitHub releases
```

---

## Full Setup Guides

### 1. Rust Project with VS Code + CLI

**Target:** Rust developers wanting LSP + CLI tools

**Time:** 15 minutes

#### Step 1: Install Tools

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install lsp-max CLI
cargo install lsp-max-cli

# Install VS Code extension
code --install-extension seanchatmangpt.lsp-max

# Verify installation
lsp-max-server --version
lsp-max-cli --version
```

#### Step 2: Create Workspace Config

```bash
# Navigate to your Rust project
cd /path/to/rust-project

# Create VS Code settings
mkdir -p .vscode

cat > .vscode/settings.json << 'EOF'
{
  "lsp-max.enabled": true,
  "lsp-max.conformance.enableConformanceChecks": true,
  "lsp-max.trace.server": "messages",
  "editor.formatOnSave": false,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  }
}
EOF

cat > .vscode/keybindings.json << 'EOF'
[
  {
    "key": "ctrl+shift+d",
    "command": "lsp-max.showDiagnostics",
    "when": "editorFocus && editorLangId == rust"
  },
  {
    "key": "ctrl+shift+c",
    "command": "lsp-max.checkConformance",
    "when": "editorFocus"
  }
]
EOF
```

#### Step 3: Verify Setup

```bash
# Open project in VS Code
code .

# Open any .rs file
# Expected: Diagnostics appear in gutter, hover shows type info

# Test CLI
lsp-max-cli gate check
# Expected: "Gate status: OPEN"

lsp-max-cli conformance vector
# Expected: Conformance vector JSON output
```

#### Step 4: Enable Additional Features

**Inlay Hints:**
```json
{
  "lsp-max.inlayHints.enabled": true,
  "[rust]": {
    "editor.inlayHints.enabled": true
  }
}
```

**Semantic Tokens:**
```json
{
  "lsp-max.semanticTokens.enabled": true,
  "editor.semanticHighlighting.enabled": true
}
```

**Type Hierarchy:**
```json
{
  "lsp-max.typeHierarchy.enabled": true
}
```

---

### 2. Multi-Language Project with JetBrains IDE

**Target:** Teams with mixed Rust/Java/Kotlin/Go codebases

**Time:** 20 minutes

#### Step 1: Install Plugin

**IntelliJ IDEA:**
1. Open **Settings** (Cmd+, on macOS, Ctrl+Alt+S on Windows/Linux)
2. Navigate to **Plugins**
3. Click **Marketplace**
4. Search for "lsp-max"
5. Click **Install**
6. Restart IDE

**RustRover / CLion / GoLand:**
- Same steps as IntelliJ IDEA
- No additional configuration needed

#### Step 2: Configure Server

```bash
# Install server
cargo install lsp-max-cli

# Find installation path
which lsp-max-server
# Example output: /Users/user/.cargo/bin/lsp-max-server
```

Now configure in IDE:

1. **Settings** → **Languages & Frameworks** → **LSP-Max**
2. Set **Server Path:** `/Users/user/.cargo/bin/lsp-max-server` (your path)
3. Leave **Server Arguments** empty (or add `--log-level debug`)
4. Click **Apply** and **OK**

#### Step 3: Create Project Config

```bash
cd /path/to/project

# Create lsp-max config file
mkdir -p ~/.config/lsp-max

cat > ~/.config/lsp-max/jetbrains.toml << 'EOF'
[server]
path = "/Users/user/.cargo/bin/lsp-max-server"
args = ["--log-level", "info"]
enable = true

[conformance]
enabled = true
report_unknown = true
max_per_file = 100

[diagnostics]
victory_language = true
version_violations = true
gate_violations = false

[performance]
debounce_ms = 500
max_parallel = 4
compression = true

[features]
semantic_tokens = true
inlay_hints = true
type_hierarchy = true
call_hierarchy = true
code_lens = true
receipts = true
gates = true
EOF
```

#### Step 4: Customize Keybindings

**Settings** → **Keymap** → **Plug-ins** → **LSP-Max**

Right-click each action to bind custom keys:
- Show diagnostics
- Check conformance
- View conformance vector
- Show receipts
- Check ANDON gate

#### Step 5: Verify Setup

1. Open a Rust/Java/Go file
2. Hover over identifiers → Should show type information
3. **Tools** → **LSP-Max** → **Server Status** → Should say "Connected"
4. **Ctrl+Shift+D** (or your bound key) → Should show diagnostics panel

---

### 3. Full-Stack Development (Web App + Server)

**Target:** Frontend + backend developers building on lsp-max

**Time:** 30 minutes

#### Step 1: Install Backend Server

```bash
# Clone lsp-max
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max

# Build server
cargo build --release

# Install globally
cargo install --path . lsp-max-cli
```

#### Step 2: Install Frontend Tools

```bash
# Node.js (if not installed)
curl -sL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs

# OR macOS:
brew install node

# Install web dependencies
cd web
npm install
```

#### Step 3: Setup Development Environment

Create `.env.development.local` in `web/`:

```env
LSP_MAX_SERVER_URL=http://localhost:8080
LSP_MAX_SERVER_TIMEOUT=30000
NEXT_PUBLIC_CONFORMANCE_ENABLED=true
NEXT_PUBLIC_RECEIPTS_ENABLED=true
NEXT_PUBLIC_GATES_ENABLED=true
NEXT_PUBLIC_OCEL_ENABLED=true
NEXT_PUBLIC_THEME=dark
```

#### Step 4: Run Development Servers

**Terminal 1 — Backend:**
```bash
cd /path/to/lsp-max
cargo run --bin lsp-max-server --release -- --port 8080
```

**Terminal 2 — Frontend:**
```bash
cd /path/to/lsp-max/web
npm run dev
```

**Terminal 3 — IDE (optional):**
```bash
code .
```

#### Step 5: Open in Browser

Navigate to: `http://localhost:3000`

Check each section:
- `/receipts` — Should load receipt artifacts
- `/conformance` — Should show live conformance vector
- `/gate` — Should show gate status
- `/ocel` — Should show process evidence (if available)
- `/cli` — Should show all CLI commands

---

### 4. Team Collaboration (Shared Server)

**Target:** Teams running centralized lsp-max server

**Time:** 15 minutes per developer

#### Step 1: Network Setup (Admin)

```bash
# On server machine (e.g., 192.168.1.100)
cargo install lsp-max-cli

# Start server accessible on network
lsp-max-server --host 0.0.0.0 --port 8080 &

# Test from another machine
curl http://192.168.1.100:8080/max/state
```

#### Step 2: Developer Setup (VS Code)

```bash
# Edit .vscode/settings.json
{
  "lsp-max.serverPath": "http://192.168.1.100:8080",
  "lsp-max.serverArgs": [],
  "[rust]": {
    "editor.defaultFormatter": "lsp-max"
  }
}
```

#### Step 3: Developer Setup (JetBrains)

**Settings** → **Tools** → **LSP-Max**:
- Server Path: `http://192.168.1.100:8080`
- Enable: checked

#### Step 4: Verify Connection

```bash
# From any developer machine
curl http://192.168.1.100:8080/max/state
# Should return JSON server state
```

---

### 5. CI/CD Pipeline Integration

**Target:** Automated testing and quality gates

**Time:** 20 minutes

#### Step 1: Local Pre-Commit Hook

```bash
cd /path/to/project

mkdir -p .git/hooks

cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Checking lsp-max gate..."
lsp-max-cli gate check

if [ $? -ne 0 ]; then
  echo "Gate is ANDON. Resolve diagnostics before committing."
  exit 1
fi

echo "Running tests..."
cargo test --workspace

echo "Checking code style..."
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings

echo "All checks passed!"
EOF

chmod +x .git/hooks/pre-commit

# Test hook
git commit --allow-empty -m "test" 2>&1 | head -20
```

#### Step 2: GitHub Actions Workflow

```bash
mkdir -p .github/workflows

cat > .github/workflows/lsp-max-ci.yml << 'EOF'
name: LSP-Max CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  conformance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install lsp-max-cli
        run: cargo install lsp-max-cli
      
      - name: Check gate
        run: lsp-max-cli gate check
      
      - name: Test
        run: cargo test --workspace
      
      - name: Lint
        run: cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings
      
      - name: Export diagnostics
        if: always()
        run: |
          lsp-max-cli diagnostics export --format=json > diagnostics.json
          cat diagnostics.json

  conformance-vector:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install lsp-max-cli
        run: cargo install lsp-max-cli
      
      - name: Check conformance
        run: lsp-max-cli conformance vector --output=json | jq .
EOF
```

#### Step 3: GitLab CI (if using GitLab)

```yaml
# .gitlab-ci.yml
stages:
  - conform
  - test

conformance:
  stage: conform
  image: rust:latest
  script:
    - cargo install lsp-max-cli
    - lsp-max-cli gate check
    - lsp-max-cli conformance vector
  only:
    - merge_requests
    - main

test:
  stage: test
  image: rust:latest
  script:
    - cargo test --workspace
  needs:
    - conformance
```

---

### 6. Desktop + IDE Integration (Unified Workflow)

**Target:** Users wanting centralized LSP server + IDE integration

**Time:** 25 minutes

#### Step 1: Install Desktop App

**macOS:**
```bash
brew tap seanchatmangpt/lsp-max
brew install lsp-max-desktop
open -a "LSP Max"
```

**Windows:**
```powershell
choco install lsp-max-desktop
# or download .msi and run
```

#### Step 2: Verify Desktop App

1. Open **LSP Max** application
2. Check **Preferences** → **Server**
3. Verify port is **8080** (default)
4. Status should show **"Listening on 127.0.0.1:8080"**

#### Step 3: Configure VS Code

```json
// .vscode/settings.json
{
  "lsp-max.serverPath": "http://localhost:8080",
  "lsp-max.trace.server": "off"
}
```

#### Step 4: Configure JetBrains IDE

**Settings** → **Tools** → **LSP-Max**:
- Server Path: `http://localhost:8080`
- Arguments: (leave empty)

#### Step 5: Workflow

```bash
# 1. Open LSP Max desktop app (always running)
open -a "LSP Max"

# 2. Open VS Code — automatically connects
code /path/to/project

# 3. Open JetBrains IDE — automatically connects
idea /path/to/project

# 4. Optional: Open web dashboard
open http://localhost:3000

# 5. Use CLI for automation
lsp-max-cli conformance vector
lsp-max-cli gate check
```

---

### 7. Advanced: Custom Server Configuration

**Target:** Advanced users needing custom server parameters

**Time:** 10 minutes

#### Create Custom Server Binary

```bash
# Create custom wrapper script
cat > ~/bin/lsp-max-custom << 'EOF'
#!/bin/bash
exec lsp-max-server \
  --log-level=debug \
  --port=8081 \
  --host=127.0.0.1 \
  --max-cache=100 \
  --debounce=1000 \
  "$@"
EOF

chmod +x ~/bin/lsp-max-custom
```

#### Use in VS Code

```json
{
  "lsp-max.serverPath": "/Users/user/bin/lsp-max-custom"
}
```

#### Use in JetBrains

**Settings** → **Tools** → **LSP-Max**:
- Server Path: `/Users/user/bin/lsp-max-custom`

#### Environment Variables

Create `~/.config/lsp-max/env`:

```bash
export RUST_LOG=debug
export LSP_MAX_GATE_FILE=/tmp/lsp-max.gate
export WASM4PM_TRACE=true
export BLAKE3_VERIFY_RECEIPTS=true
```

Source before running:
```bash
source ~/.config/lsp-max/env
lsp-max-server
```

---

## Troubleshooting Setup

### Cannot Find lsp-max-server

```bash
# Check if installed
which lsp-max-server

# If not found, install via cargo
cargo install lsp-max-cli

# Verify
lsp-max-server --version
```

### Version Mismatch (Extension ≠ Server)

```bash
# Check extension version
# VS Code: Extensions → lsp-max → version number

# Check server version
lsp-max-server --version

# If mismatch, update both:
code --install-extension seanchatmangpt.lsp-max  # VS Code
cargo install --force lsp-max-cli  # Server
```

### IDE Not Detecting Server on Network

```bash
# Check server is accessible
curl http://192.168.1.100:8080/max/state

# Check firewall
sudo ufw allow 8080/tcp  # Linux
# macOS: System Preferences → Security & Privacy → Firewall

# Ensure server started with --host 0.0.0.0
lsp-max-server --host 0.0.0.0 --port 8080
```

### Gate Check Fails During CI

```bash
# Check for active diagnostics
lsp-max-cli diagnostics list

# View specific diagnostic
lsp-max-cli diagnostics view ANTI-LLM-VERSION-001

# Resolve (often false positives from auto-generated code)
# Then retry
lsp-max-cli gate check
```

---

## Configuration Reference

### VS Code `settings.json`

```json
{
  // Server
  "lsp-max.enabled": true,
  "lsp-max.serverPath": "/path/to/lsp-max-server",
  "lsp-max.serverArgs": ["--log-level", "debug"],
  
  // Trace/Debug
  "lsp-max.trace.server": "verbose",
  "lsp-max.logLevel": "debug",
  
  // Conformance
  "lsp-max.conformance.enableConformanceChecks": true,
  "lsp-max.conformance.reportUnknownAxes": true,
  "lsp-max.conformance.checkInterval": 5000,
  
  // Diagnostics
  "lsp-max.diagnostics.maxDiagnosticsPerDocument": 100,
  "lsp-max.diagnostics.reportVictoryLanguage": true,
  "lsp-max.diagnostics.reportVersionViolations": true,
  
  // Features
  "lsp-max.semanticTokens.enabled": true,
  "lsp-max.inlayHints.enabled": true,
  "lsp-max.typeHierarchy.enabled": true,
  "lsp-max.callHierarchy.enabled": true,
  "lsp-max.codeLens.enabled": true,
  "lsp-max.receipt.showReceiptDigests": true,
  "lsp-max.gates.enableAndonGate": true,
  
  // Performance
  "lsp-max.performance.debounceMs": 500,
  "lsp-max.performance.maxParallelRequests": 4,
  "lsp-max.performance.maxCachedDocuments": 50,
  "lsp-max.performance.enableCompression": true,
  "lsp-max.performance.gcIntervalMs": 60000,
  
  // Telemetry
  "lsp-max.telemetry.enabled": false
}
```

### JetBrains `jetbrains.toml`

```toml
[server]
path = "/usr/local/bin/lsp-max-server"
args = ["--log-level", "debug"]
enable = true

[conformance]
enabled = true
report_unknown = true
max_per_file = 100

[diagnostics]
victory_language = true
version_violations = true
gate_violations = false
max_total = 200

[performance]
debounce_ms = 500
max_parallel = 4
compression = true
gc_interval_ms = 60000
max_cached_documents = 50

[features]
semantic_tokens = true
inlay_hints = true
type_hierarchy = true
call_hierarchy = true
code_lens = true
receipts = true
gates = true
```

### Desktop App `config.toml`

**macOS:** `~/Library/Application\ Support/lsp-max/config.toml`
**Windows:** `%APPDATA%\lsp-max\config.toml`

```toml
[server]
port = 8080
host = "127.0.0.1"
enable_stdio = false
log_level = "info"

[ui]
theme = "dark"
font_size = 12
window_width = 1200
window_height = 800

[features]
conformance = true
receipts = true
gates = true
ocel = true

[performance]
max_cached_documents = 50
enable_compression = true
gc_interval_ms = 30000
```

---

## Next Steps

- [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md) — Detailed feature documentation
- [FEATURES.md](FEATURES.md) — Complete LSP 3.18 feature matrix
- [AGENTS.md](../AGENTS.md) — Law-state runtime and gate system
- [CLAUDE.md](../CLAUDE.md) — Project guidelines and conventions

---

**Last updated:** 2026-06-14  
**Version:** 26.6.9 (CalVer)
