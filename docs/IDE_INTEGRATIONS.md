# Claude Code IDE Integrations

Comprehensive guide to integrating **lsp-max** with Claude Code across VS Code, JetBrains IDEs, web applications, and desktop environments. This document covers installation, configuration, keyboard shortcuts, feature matrices, debugging, performance tuning, and troubleshooting.

## Quick Navigation

- [VS Code Extension](#vs-code-extension)
- [JetBrains Plugins](#jetbrains-plugins)
- [Web App Integration](#web-app-integration)
- [Desktop App (Mac/Windows)](#desktop-app-macwindows)
- [Cross-IDE Feature Matrix](#cross-ide-feature-matrix)
- [Development Workflows](#development-workflows)

---

## VS Code Extension

### Installation

#### From Visual Studio Marketplace

1. Open VS Code
2. Navigate to **Extensions** (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "lsp-max"
4. Click **Install**

#### From GitHub Releases

1. Download the `.vsix` file from [GitHub Releases](https://github.com/seanchatmangpt/lsp-max/releases)
2. In VS Code, run **Extensions: Install from VSIX** (Cmd+Shift+P → `ext install`)
3. Select the downloaded `.vsix` file

#### From Source

```bash
# Clone the repository
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max

# Install vsce (VS Code extension build tool)
npm install -g vsce

# Build the extension
cd extensions/vscode
npm install
vsce package

# Install locally
code --install-extension lsp-max-*.vsix
```

### Configuration

#### Settings.json (User or Workspace)

Create or edit `.vscode/settings.json` in your workspace:

```json
{
  "lsp-max.enabled": true,
  "lsp-max.serverPath": "/usr/local/bin/lsp-max-server",
  "lsp-max.serverArgs": ["--log-level", "debug"],
  "lsp-max.trace.server": "verbose",
  "lsp-max.conformance.enableConformanceChecks": true,
  "lsp-max.conformance.reportUnknownAxes": true,
  "lsp-max.diagnostics.maxDiagnosticsPerDocument": 100,
  "lsp-max.diagnostics.reportVictoryLanguage": true,
  "lsp-max.diagnostics.reportVersionViolations": true,
  "lsp-max.semanticTokens.enabled": true,
  "lsp-max.inlayHints.enabled": true,
  "lsp-max.typeHierarchy.enabled": true,
  "lsp-max.callHierarchy.enabled": true,
  "lsp-max.receipt.showReceiptDigests": true,
  "lsp-max.gates.enableAndonGate": true,
  "lsp-max.performance.debounceMs": 500,
  "lsp-max.performance.maxParallelRequests": 4,
  "lsp-max.telemetry.enabled": false
}
```

#### Environment Variables

Add to `.vscode/settings.json` or use system environment:

```json
{
  "lsp-max.env": {
    "RUST_LOG": "debug,lsp_max=trace",
    "LSP_MAX_GATE_FILE": "/tmp/lsp-max.gate",
    "WASM4PM_TRACE": "true",
    "BLAKE3_VERIFY_RECEIPTS": "true"
  }
}
```

#### Launch Configuration (Debug)

Create or update `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Attach to lsp-max Server",
      "type": "lldb",
      "request": "attach",
      "pid": "${command:pickProcess}",
      "preLaunchTask": "Start LSP Server"
    },
    {
      "name": "LSP Max Extension Debug",
      "type": "extensionHost",
      "request": "launch",
      "runtimeExecutable": "${execPath}",
      "args": [
        "--extensionDevelopmentPath=${workspaceFolder}/extensions/vscode",
        "${workspaceFolder}"
      ],
      "outFiles": [
        "${workspaceFolder}/extensions/vscode/out/**/*.js"
      ],
      "preLaunchTask": "npm: watch"
    }
  ]
}
```

### Keyboard Shortcuts

Add to `.vscode/keybindings.json`:

```json
[
  {
    "key": "ctrl+shift+d",
    "command": "lsp-max.showDiagnostics",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+shift+c",
    "command": "lsp-max.checkConformance",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+shift+v",
    "command": "lsp-max.viewConformanceVector",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+shift+r",
    "command": "lsp-max.showReceipts",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+shift+g",
    "command": "lsp-max.checkAndonGate",
    "when": "editorFocus"
  },
  {
    "key": "shift+f12",
    "command": "lsp-max.typeHierarchy.superTypes",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+alt+shift+h",
    "command": "lsp-max.callHierarchy.incoming",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+alt+h",
    "command": "lsp-max.callHierarchy.outgoing",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+k ctrl+i",
    "command": "lsp-max.toggleInlayHints",
    "when": "editorFocus"
  },
  {
    "key": "ctrl+alt+l",
    "command": "lsp-max.toggleSemanticTokens",
    "when": "editorFocus"
  }
]
```

### Features Matrix (VS Code)

| Feature | Supported | Status | Notes |
|---------|-----------|--------|-------|
| Basic LSP diagnostics | ✅ | SUPPORTED | Full conformance with LSP 3.18 |
| Semantic tokens | ✅ | SUPPORTED | Full and delta variants |
| Hover information | ✅ | SUPPORTED | With receipt metadata |
| Go to definition | ✅ | SUPPORTED | Cross-document support |
| References | ✅ | SUPPORTED | Via `textDocument/references` |
| Rename | ✅ | SUPPORTED | Workspace-wide renaming |
| Code actions | ✅ | SUPPORTED | With resolution phase |
| Code lens | ✅ | SUPPORTED | Per-line metrics |
| Inlay hints | ✅ | SUPPORTED | Type and parameter hints |
| Inline values | ⚠️ | PARTIAL | Debug context only |
| Type hierarchy | ✅ | SUPPORTED | Super/subtypes navigation |
| Call hierarchy | ✅ | SUPPORTED | Incoming/outgoing calls |
| Document symbols | ✅ | SUPPORTED | Hierarchical outline |
| Workspace symbols | ✅ | SUPPORTED | Fuzzy search |
| Conformance vector | ✅ | SUPPORTED | Custom `max/*` methods |
| Receipt ledger | ✅ | SUPPORTED | Digest and history view |
| ANDON gate status | ✅ | SUPPORTED | Live gate polling |
| Snapshot export | ✅ | SUPPORTED | Via `max/snapshot` |
| Process mining | ⚠️ | PARTIAL | OCEL 2.0 view in sidebar |
| Multi-server fan-out | ✅ | SUPPORTED | Via lsp-max-compositor |

### Debugging Integration

#### Enable Server Logging

```json
{
  "lsp-max.trace.server": "messages",
  "lsp-max.logLevel": "debug"
}
```

Then access logs via **Output** panel (Ctrl+Shift+U):
- Select "lsp-max" from the dropdown

#### Debug Inspector

1. Press Ctrl+Shift+P → **Developer: Toggle Developer Tools**
2. Open **Console** tab
3. Log messages appear from the extension
4. Use `fetch()` to inspect server state:
   ```javascript
   const response = await fetch('http://localhost:3000/max/state');
   console.log(await response.json());
   ```

#### Attach Debugger to Server

1. Start the server separately:
   ```bash
   lsp-max-server --debug
   ```

2. In VS Code, create launch config:
   ```json
   {
     "name": "Attach to Server",
     "type": "lldb",
     "request": "attach",
     "pid": "${command:pickProcess}"
   }
   ```

3. Press F5 to attach

#### Diagnostic Collection

Export diagnostics for analysis:

```bash
# Via CLI
lsp-max-cli diagnostics export --format=json > diagnostics.json

# Via VS Code command palette
lsp-max: Export Diagnostics
```

### Performance Tuning

#### Reduce CPU Usage

```json
{
  "lsp-max.performance.debounceMs": 1000,
  "lsp-max.performance.maxParallelRequests": 2,
  "lsp-max.semanticTokens.enabled": false,
  "lsp-max.inlayHints.enabled": false,
  "lsp-max.diagnostics.maxDiagnosticsPerDocument": 50
}
```

#### Optimize for Large Files

```json
{
  "lsp-max.performance.fileSizeThresholdMb": 5,
  "[files.large]": {
    "lsp-max.semanticTokens.enabled": false,
    "lsp-max.typeHierarchy.enabled": false
  }
}
```

#### Memory Management

```json
{
  "lsp-max.performance.maxCachedDocuments": 50,
  "lsp-max.performance.enableCompression": true,
  "lsp-max.performance.gcIntervalMs": 60000
}
```

### Troubleshooting

#### Server Fails to Start

**Error:** `Failed to find lsp-max-server executable`

**Solution:**
```bash
# Install via cargo
cargo install lsp-max-cli

# Or set explicit path in settings.json
{
  "lsp-max.serverPath": "/path/to/lsp-max-server"
}
```

#### High CPU Usage

**Cause:** Semantic tokens or inlay hints on large files

**Solution:**
```json
{
  "lsp-max.performance.debounceMs": 2000,
  "[rust]": {
    "lsp-max.semanticTokens.enabled": false
  }
}
```

#### ANDON Gate Stuck

**Error:** Shell commands blocked indefinitely

**Solution:**
```bash
# Check gate status
lsp-max-cli gate check

# Reset gate
lsp-max-cli gate reset

# View active diagnostics
lsp-max-cli diagnostics list
```

#### Slow Conformance Checks

**Cause:** Full process-mining validation on every change

**Solution:**
```json
{
  "lsp-max.conformance.checkInterval": 5000,
  "lsp-max.conformance.enableConformanceChecks": false
}
```

Enable manually via command palette when needed.

---

## JetBrains Plugins

### Supported IDEs

- IntelliJ IDEA (2024.1+)
- CLion (2024.1+)
- RustRover (2024.1+)
- WebStorm (2024.1+)
- DataGrip (2024.1+)
- PhpStorm (2024.1+)
- GoLand (2024.1+)
- PyCharm (2024.1+)

### Installation

#### From JetBrains Marketplace

1. Open IDE → **Settings** (Cmd+, / Ctrl+Alt+S)
2. Navigate to **Plugins**
3. Search for "lsp-max"
4. Click **Install**
5. Restart IDE

#### From GitHub Releases

1. Download the `.zip` file from [GitHub Releases](https://github.com/seanchatmangpt/lsp-max/releases)
2. Open IDE → **Settings** → **Plugins**
3. Click ⚙️ → **Install Plugin from Disk**
4. Select the `.zip` file
5. Restart IDE

#### From Source

```bash
cd lsp-max/extensions/jetbrains
./gradlew buildPlugin

# Result: build/distributions/lsp-max-*.zip
# Install via IDE Settings → Plugins → Install from Disk
```

### Configuration

#### IDE Settings

**IntelliJ Settings → Languages & Frameworks → LSP-Max:**

```
Server Path: /usr/local/bin/lsp-max-server
Server Arguments: --log-level=debug

Conformance:
  ☑ Enable conformance checks
  ☑ Report unknown axes
  Max diagnostics per file: 100

Diagnostics:
  ☑ Report victory language
  ☑ Report version violations
  ☐ Report all diagnostic families

Performance:
  Debounce (ms): 500
  Max parallel requests: 4
  Enable compression: ☑

Features:
  ☑ Semantic tokens
  ☑ Inlay hints
  ☑ Type hierarchy
  ☑ Call hierarchy
```

#### Plugin Configuration File

Create `~/.config/lsp-max/jetbrains.toml`:

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

[features]
semantic_tokens = true
inlay_hints = true
type_hierarchy = true
call_hierarchy = true
code_lens = true
receipts = true
gates = true
```

#### Environment Variables

Add to IDE run configuration or `.env`:

```bash
RUST_LOG=debug,lsp_max=trace
LSP_MAX_GATE_FILE=/tmp/lsp-max.gate
WASM4PM_TRACE=true
BLAKE3_VERIFY_RECEIPTS=true
```

### Keyboard Shortcuts

**IntelliJ Settings → Keymap → Plug-ins → LSP-Max:**

| Action | VS Windows | macOS | Alternative |
|--------|-----------|-------|-------------|
| Show diagnostics | Ctrl+Shift+D | Cmd+Shift+D | Right-click → Diagnostics |
| Check conformance | Ctrl+Shift+C | Cmd+Shift+C | Tools → LSP-Max → Conformance |
| View conformance vector | Ctrl+Shift+V | Cmd+Shift+V | Hover on diagnostic |
| Show receipts | Ctrl+Shift+R | Cmd+Shift+R | Tools → LSP-Max → Receipts |
| Check ANDON gate | Ctrl+Shift+G | Cmd+Shift+G | Status bar indicator |
| Type hierarchy (supertypes) | Ctrl+H | Ctrl+H | Navigate → Type Hierarchy |
| Call hierarchy (incoming) | Ctrl+Alt+Shift+H | Cmd+Alt+Shift+H | Navigate → Call Hierarchy |
| Call hierarchy (outgoing) | Ctrl+Alt+H | Cmd+Alt+H | Navigate → Call Hierarchy |
| Toggle inlay hints | Ctrl+Alt+I | Cmd+Alt+I | View → Inlays |
| Toggle semantic tokens | Ctrl+Alt+L | Cmd+Alt+L | View → Syntax Highlighting |
| Snapshot export | Ctrl+Alt+S | Cmd+Alt+S | Tools → LSP-Max → Snapshot |

#### Custom Keybinding (Example)

1. **Settings** → **Keymap**
2. Search for "lsp-max.conformance"
3. Right-click → **Add Keyboard Shortcut**
4. Press your desired key combination
5. Click **OK**

### Features Matrix (JetBrains)

| Feature | IDEA | CLion | RustRover | WebStorm | DataGrip | Status |
|---------|------|-------|-----------|----------|----------|--------|
| Diagnostics | ✅ | ✅ | ✅ | ✅ | ✅ | SUPPORTED |
| Semantic tokens | ✅ | ✅ | ✅ | ✅ | ⚠️ | SUPPORTED |
| Hover & navigation | ✅ | ✅ | ✅ | ✅ | ✅ | SUPPORTED |
| Type hierarchy | ✅ | ✅ | ✅ | ⚠️ | ✅ | PARTIAL |
| Call hierarchy | ✅ | ✅ | ✅ | ✅ | ⚠️ | PARTIAL |
| Inlay hints | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | PARTIAL |
| Code lens | ✅ | ✅ | ✅ | ✅ | ⚠️ | PARTIAL |
| Conformance vector | ✅ | ✅ | ✅ | ✅ | ✅ | SUPPORTED |
| Receipt ledger | ✅ | ✅ | ✅ | ✅ | ✅ | SUPPORTED |
| ANDON gate | ✅ | ✅ | ✅ | ✅ | ✅ | SUPPORTED |
| Snapshot export | ✅ | ✅ | ✅ | ✅ | ✅ | SUPPORTED |
| Process mining view | ⚠️ | ⚠️ | ⚠️ | ❌ | ⚠️ | PARTIAL |

### Debugging Integration

#### Enable Plugin Logging

**Settings** → **Tools** → **LSP-Max** → **Debug**:

```
☑ Enable debug logging
☑ Log server messages
☑ Log diagnostic events
Log level: TRACE
```

#### Debug Console

1. In IDE, open **Tools** → **LSP-Max** → **Debug Console**
2. View real-time server output
3. Send test requests:
   ```
   > conformance vector
   > snapshot export
   > gate check
   ```

#### Attach Debugger (Server Process)

1. Start server separately:
   ```bash
   lsp-max-server --debug --port 5005
   ```

2. **Run** → **Attach to Process** → select `lsp-max-server`
3. Breakpoints work if server was compiled with debug symbols

#### Problem Inspector

**Tools** → **LSP-Max** → **Problem Inspector**:

- View all diagnostics hierarchically
- Filter by severity, family, or code
- Export as JSON/CSV
- Replay individual problems

### Performance Tuning

#### Reduce Indexing Overhead

**Settings** → **Tools** → **LSP-Max** → **Performance**:

```
Semantic tokens: OFF for large projects
Inlay hints: OFF initially, enable per-file
Debounce (ms): 1000
Max parallel: 2
```

#### IDE-Specific Tips

**IntelliJ IDEA (for large codebases):**
```
Settings → Appearance & Behavior → System Settings
  Auto-save: 5 seconds
  Power Save Mode: ON (disables real-time checks)

Settings → Editor → Inspections
  LSP-Max diagnostics: OFF during indexing
```

**RustRover:**
```
Settings → Languages & Frameworks → Rust
  Exclude: target/
  
Settings → Tools → LSP-Max
  Semantic tokens: OFF
  Parallel requests: 1 (on 4-core machines)
```

#### Memory Limits

Create `~/.<IDE>/idea.properties` (or similar):

```properties
# Increase heap for large projects
-Xmx4096m
-Xms2048m

# Reduce garbage collection frequency
-XX:+UseG1GC
-XX:MaxGCPauseMillis=200
```

### Troubleshooting

#### Plugin Not Showing in Marketplace

**Cause:** IDE version incompatible with plugin

**Solution:**
```bash
# Check minimum IDE version requirement
cat extensions/jetbrains/gradle.properties | grep ideaVersion

# Install older plugin version compatible with your IDE
# Download from GitHub Releases
```

#### Server Connection Fails

**Error:** `Failed to connect to LSP server`

**Solution:**
1. **Tools** → **LSP-Max** → **Server Status**
2. Click **Start Server**
3. Check paths in **Settings** → **Tools** → **LSP-Max**

#### High Memory Usage

**Cause:** Semantic token caching or large OCEL logs

**Solution:**
```toml
# In ~/.config/lsp-max/jetbrains.toml
[performance]
semantic_tokens = false
max_cached_documents = 20
compression = true
```

Restart IDE.

#### Diagnostics Not Updating

**Cause:** Debounce too high or file changes not detected

**Solution:**
```
Settings → Editor → General → Auto Save
  Save modified files: ON
  
Settings → Tools → LSP-Max
  Debounce (ms): 300
  
Restart IDE
```

---

## Web App Integration

### Installation

The web app is available at https://lsp-max.example.com (when deployed).

#### Docker Deployment

```bash
# Build Docker image
docker build -f web/Dockerfile -t lsp-max-web .

# Run container
docker run -d \
  --name lsp-max-web \
  -p 3000:3000 \
  -e LSP_MAX_SERVER_URL=http://localhost:8080 \
  -e NEXT_PUBLIC_ENV=production \
  lsp-max-web
```

#### Manual Installation

```bash
cd web

# Install dependencies
npm install

# Set environment variables
export LSP_MAX_SERVER_URL=http://localhost:8080
export NEXT_PUBLIC_ENV=production

# Build for production
npm run build

# Start server
npm start
```

### Configuration

#### Environment Variables

Create `.env.local`:

```env
# Server connection
LSP_MAX_SERVER_URL=http://localhost:8080
LSP_MAX_SERVER_TIMEOUT=30000

# Features
NEXT_PUBLIC_CONFORMANCE_ENABLED=true
NEXT_PUBLIC_RECEIPTS_ENABLED=true
NEXT_PUBLIC_GATES_ENABLED=true
NEXT_PUBLIC_OCEL_ENABLED=true

# UI
NEXT_PUBLIC_THEME=dark
NEXT_PUBLIC_MAX_LOG_ENTRIES=1000

# Auth (if applicable)
AUTH_ENABLED=false
AUTH_SECRET=your-secret-key

# Telemetry
NEXT_PUBLIC_TELEMETRY_ENABLED=false
```

#### Theming

Create `web/lib/theme.config.ts`:

```typescript
export const themeConfig = {
  colors: {
    primary: '#0066cc',
    success: '#28a745',
    warning: '#ffc107',
    error: '#dc3545',
    neutral: '#6c757d',
  },
  fonts: {
    mono: '"Courier New", Courier, monospace',
    sans: '-apple-system, BlinkMacSystemFont, "Segoe UI"',
  },
  breakpoints: {
    xs: '320px',
    sm: '576px',
    md: '768px',
    lg: '992px',
    xl: '1200px',
  },
};
```

### Features

#### Receipt Ledger

**URL:** `/receipts`

- Browse all generated receipts (BLAKE3-hashed)
- Filter by status (ADMITTED, CANDIDATE, BLOCKED)
- View receipt chain provenance
- Export receipt artifacts

#### Conformance Viewer

**URL:** `/conformance`

- Live conformance vector display
- View admitted/refused/unknown axes
- Interactive law-axis explorer
- Conformance timeline (historical)

#### ANDON Gate Dashboard

**URL:** `/gate`

- Live gate status (OPEN / ANDON)
- Active diagnostic list with severity
- Resolve diagnostics workflow
- Gate history timeline

#### OCEL Process Evidence

**URL:** `/ocel`

- Interactive OCEL 2.0 graph visualization
- Timeline and event filtering
- Object lifecycle tracking
- Export events as CSV/JSON

#### CLI Surface Explorer

**URL:** `/cli`

- Browse all noun/verb commands
- Search by command name or arguments
- View real source code (`#[verb]` attributes)
- Copy command templates

#### Coverage Gap Map

**URL:** `/coverage`

- Documentation vs. examples coverage
- Missing fixtures and transcripts
- Gap status per iteration
- Coverage trends

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Search | Cmd+K / Ctrl+K |
| Navigate to receipts | G then R |
| Navigate to conformance | G then C |
| Navigate to gate | G then G |
| Navigate to OCEL | G then O |
| Toggle dark mode | T then D |
| Refresh data | R |
| Export current view | E |
| Help | ? |

### Performance Optimization

#### Lazy Loading

```typescript
// web/app/conformance/page.tsx
const ConformanceViewer = dynamic(
  () => import('@/components/ConformanceViewer'),
  { loading: () => <LoadingSkeleton /> }
);
```

#### Caching

```typescript
// Cache receipt list for 5 minutes
export const revalidate = 300;
```

#### Image Optimization

```typescript
// Use Next.js Image component
import Image from 'next/image';

<Image
  src="/receipt-icon.svg"
  alt="Receipt"
  width={24}
  height={24}
  priority={false}
/>
```

### Troubleshooting

#### Cannot Connect to Server

**Error:** `Failed to fetch from LSP_MAX_SERVER_URL`

**Solution:**
```bash
# Check server is running
lsp-max-server --version

# Verify server URL
curl http://localhost:8080/max/state

# Update environment variable
export LSP_MAX_SERVER_URL=http://localhost:8080
```

#### Slow Page Load

**Cause:** Large OCEL files or unoptimized queries

**Solution:**
```env
# Reduce initial data load
NEXT_PUBLIC_MAX_RECEIPTS_INITIAL=20
NEXT_PUBLIC_MAX_OCEL_EVENTS=500

# Enable compression
NEXT_PUBLIC_ENABLE_COMPRESSION=true
```

#### Missing Data Display

**Cause:** Receipt artifacts not in git

**Solution:**
```bash
# Generate receipts
cargo test --test test_receipts

# Add to .gitignore exception
echo "!receipts/" >> .gitignore
git add receipts/
```

---

## Desktop App (Mac/Windows)

### Installation

#### macOS

**Via Homebrew:**
```bash
brew tap seanchatmangpt/lsp-max
brew install lsp-max-desktop
```

**Direct Download:**
1. Download from [GitHub Releases](https://github.com/seanchatmangpt/lsp-max/releases)
2. Extract `lsp-max-macos-aarch64.dmg` or `lsp-max-macos-x86_64.dmg`
3. Drag to Applications folder
4. Open Applications → Right-click `LSP Max` → Open (to bypass notarization on first run)

**From Source:**
```bash
cd desktop
cargo build --release
# Result: target/release/lsp-max.app
```

#### Windows

**Via Chocolatey:**
```powershell
choco install lsp-max-desktop
```

**Direct Download:**
1. Download from [GitHub Releases](https://github.com/seanchatmangpt/lsp-max/releases)
2. Run `lsp-max-windows-x86_64.msi` or `lsp-max-windows-arm64.msi`
3. Follow installer prompts
4. Launch from Start menu

**From Source:**
```powershell
cd desktop
cargo build --release
# Result: target\release\lsp-max.exe
```

### Configuration

#### macOS

**Configuration File:** `~/Library/Application\ Support/lsp-max/config.toml`

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
```

**Launch on Startup:**
```bash
# Create LaunchAgent
mkdir -p ~/Library/LaunchAgents

cat > ~/Library/LaunchAgents/com.seanchatmangpt.lsp-max.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.seanchatmangpt.lsp-max</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/LSP\ Max.app/Contents/MacOS/lsp-max</string>
        <string>--headless</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/com.seanchatmangpt.lsp-max.plist
```

#### Windows

**Configuration File:** `%APPDATA%\lsp-max\config.toml`

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
```

**Launch on Startup (Task Scheduler):**

```powershell
# Create scheduled task
$trigger = New-ScheduledTaskTrigger -AtLogOn
$action = New-ScheduledTaskAction `
  -Execute "C:\Program Files\lsp-max\bin\lsp-max.exe" `
  -Argument "--headless"
$principal = New-ScheduledTaskPrincipal `
  -UserId "$env:USERNAME" `
  -RunLevel Highest

Register-ScheduledTask `
  -TaskName "LSP Max" `
  -Trigger $trigger `
  -Action $action `
  -Principal $principal
```

### Keyboard Shortcuts

**macOS:**

| Action | Shortcut |
|--------|----------|
| Preferences | Cmd+, |
| Show conformance | Cmd+1 |
| Show receipts | Cmd+2 |
| Show gate | Cmd+3 |
| Show OCEL | Cmd+4 |
| Show CLI | Cmd+5 |
| Search | Cmd+K |
| Refresh | Cmd+R |
| Quit | Cmd+Q |

**Windows:**

| Action | Shortcut |
|--------|----------|
| Preferences | Ctrl+, |
| Show conformance | Ctrl+1 |
| Show receipts | Ctrl+2 |
| Show gate | Ctrl+3 |
| Show OCEL | Ctrl+4 |
| Show CLI | Ctrl+5 |
| Search | Ctrl+K |
| Refresh | Ctrl+R |
| Exit | Alt+F4 |

### Features

#### Unified Dashboard

Central hub displaying:
- Server health (CPU, memory, uptime)
- Live gate status
- Recent diagnostics
- Receipt summary

#### Embedded Server

Built-in LSP server (no separate installation):
```
~/Library/Application\ Support/lsp-max/server/lsp-max-server
```

Access via:
- UI: Automatic
- CLI: `lsp-max-cli --server-url http://localhost:8080`
- Other IDEs: Configure server path in IDE settings

#### Tray Icon

Always-available status indicator (macOS/Windows):
- Gate status
- Quick actions menu
- Open dashboard
- Server logs

#### Native File Dialogs

Open projects and export artifacts:
```
File → Open Project → Select folder
File → Export Conformance → Save location
```

### Performance

#### Memory Optimization

**macOS:**
```toml
[performance]
max_cached_documents = 50
enable_compression = true
gc_interval_ms = 30000
```

**Windows:**
```toml
[performance]
max_cached_documents = 30
enable_compression = true
gc_interval_ms = 30000
```

#### CPU Reduction

Disable real-time features on slower machines:
```toml
[features]
semantic_tokens = false
inlay_hints = false
continuous_conformance = false

[performance]
debounce_ms = 2000
max_parallel_requests = 1
```

### Troubleshooting

#### App Crashes on Startup (macOS)

**Error:** `SIGILL` or `SIGSEGV`

**Solution:**
```bash
# Ensure correct architecture
file /Applications/LSP\ Max.app/Contents/MacOS/lsp-max
# Should show "Mach-O 64-bit executable arm64" (Apple Silicon) 
# or "Mach-O 64-bit executable x86_64" (Intel)

# Reinstall for correct architecture
rm -rf /Applications/LSP\ Max.app
# Download correct .dmg from releases
```

#### Server Not Accessible from IDE

**Error:** `connection refused on port 8080`

**Solution:**
1. Open Desktop App
2. Check status bar (should be green/listening)
3. Open **Preferences** → **Server**
4. Verify port: 8080
5. IDE settings: `lsp-max.serverPath=http://localhost:8080`

#### High CPU Usage

**Cause:** Continuous conformance checking on large projects

**Solution:**
1. **Preferences** → **Performance**
2. Disable "Continuous conformance"
3. Use on-demand via keyboard shortcut (Cmd+1)
4. Adjust debounce to 2000ms

---

## Cross-IDE Feature Matrix

### Summary Table

| Feature | VS Code | JetBrains | Web | Desktop |
|---------|---------|-----------|-----|---------|
| **Core LSP** | | | | |
| Diagnostics | ✅ | ✅ | ✅ | ✅ |
| Hover | ✅ | ✅ | ⚠️ | ✅ |
| Go to definition | ✅ | ✅ | ✅ | ⚠️ |
| References | ✅ | ✅ | ✅ | ✅ |
| Rename | ✅ | ✅ | ⚠️ | ✅ |
| Code actions | ✅ | ✅ | ⚠️ | ✅ |
| Code lens | ✅ | ⚠️ | ⚠️ | ✅ |
| Document symbols | ✅ | ✅ | ⚠️ | ✅ |
| Workspace symbols | ✅ | ✅ | ✅ | ✅ |
| **Advanced** | | | | |
| Semantic tokens | ✅ | ✅ | ⚠️ | ✅ |
| Inlay hints | ✅ | ✅ | ⚠️ | ✅ |
| Type hierarchy | ✅ | ✅ | ✅ | ⚠️ |
| Call hierarchy | ✅ | ✅ | ✅ | ⚠️ |
| Inline values | ⚠️ | ⚠️ | ❌ | ⚠️ |
| **max/* Features** | | | | |
| Conformance vector | ✅ | ✅ | ✅ | ✅ |
| Receipt ledger | ✅ | ✅ | ✅ | ✅ |
| ANDON gate | ✅ | ✅ | ✅ | ✅ |
| Snapshot export | ✅ | ✅ | ✅ | ✅ |
| Process mining (OCEL) | ⚠️ | ⚠️ | ✅ | ✅ |
| **Auxiliary** | | | | |
| Debugging UI | ✅ | ✅ | ⚠️ | ✅ |
| Multi-server fan-out | ✅ | ✅ | ✅ | ✅ |
| Receipt chain viewer | ✅ | ✅ | ✅ | ✅ |

### Legend

- **✅ SUPPORTED** — Full feature parity
- **⚠️ PARTIAL** — Limited support or missing features
- **❌ UNSUPPORTED** — Not available in this environment

---

## Development Workflows

### Workflow 1: Local Development (VS Code + CLI)

**Setup:**

```bash
# Clone and build
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max
cargo build --release

# Install extension
cd extensions/vscode
npm install
npm run compile
code --install-extension out/lsp-max-*.vsix
```

**Usage:**

```bash
# Terminal 1: Run server
lsp-max-server --log-level debug

# Terminal 2: Open VS Code
code .

# Terminal 3: Run tests/build
cargo test
just dx-polish
```

**Settings (.vscode/settings.json):**

```json
{
  "lsp-max.serverPath": "/home/user/lsp-max/target/release/lsp-max-server",
  "lsp-max.serverArgs": ["--log-level", "debug"],
  "lsp-max.trace.server": "verbose"
}
```

### Workflow 2: Multi-IDE Testing (JetBrains + Web + Desktop)

**Setup:**

```bash
# Build all components
cargo build --release
npm run build:all  # Builds web app
cargo build --release -p lsp-max-desktop

# Start server
lsp-max-server --port 8080 &

# Deploy web app
cd web && npm start &

# Open desktop app
open -a "LSP Max"  # macOS
# or
./target/release/lsp-max &  # Windows/Linux
```

**Testing Checklist:**

- [ ] VS Code: Open project, check diagnostics
- [ ] JetBrains: Attach IntelliJ IDEA, navigate code
- [ ] Web: Open browser to localhost:3000, check receipts
- [ ] Desktop: Verify server status, toggle features

### Workflow 3: Gate-Driven Development

Leverage the **ANDON gate** to enforce quality gates:

**Setup:**

```bash
# Enable pre-commit hook
git config core.hooksPath .git/hooks
cp scripts/pre-commit .git/hooks/

# Run gate check before commits
lsp-max-cli gate check

# Gate blocks shell if ANDON is set (WASM4PM-* or GGEN-* diagnostics)
```

**Workflow:**

```bash
# Make changes
vim src/service.rs

# Try to test
cargo test
# Output: ANDON gate is set. Resolve diagnostics.

# Check what's blocking
lsp-max-cli diagnostics list

# Fix issues (code or diagnostic false positives)
vim src/service.rs

# Gate clears when last diagnostic resolves
lsp-max-cli gate check
# Output: Gate is OPEN. Proceeding.

cargo test  # Now proceeds
```

### Workflow 4: CI/CD Integration

**GitHub Actions (.github/workflows/lsp-max.yml):**

```yaml
name: LSP-Max CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      
      - name: Check gate
        run: cargo install --path crates/lsp-max-cli && lsp-max-cli gate check
      
      - name: Run tests
        run: cargo test --workspace
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      
      - name: Publish diagnostics
        if: always()
        run: |
          lsp-max-cli diagnostics export --format=json > diagnostics.json
          echo "::notice::Diagnostics exported"
```

### Workflow 5: Agent-Driven Development

Use Claude Code agents with lsp-max:

**Agent Integration:**

```bash
# Install Claude Code CLI
npm install -g @anthropic-ai/claude-code

# Initialize session
claude-code --init

# Set up hook to check gate before shell actions
# (Edit .claude/settings.json — see CLAUDE.md)
```

**Development with Agent:**

```
User: "Implement feature X"
Agent:
  1. Analyzes codebase
  2. Proposes changes
  3. Runs cargo test (blocked if gate ANDON)
  4. Gate check resolves diagnostics
  5. Changes committed with receipt in message
```

---

## Troubleshooting Guide (Cross-IDE)

### Server Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Server not found | Binary not in PATH | `cargo install lsp-max-cli` or set full path in IDE |
| Port already in use | Conflict with another LSP | Change port in config: `LSP_MAX_PORT=8081` |
| Slow startup | Large codebase indexing | Set `max_cached_documents=50` |
| Crashes on large files | Memory limit | Increase heap: `RUST_MAX_HEAP=4g` |

### IDE-Specific

**VS Code:**
- Extension not loaded: Reload window (Cmd+R)
- Outdated types: `npm install -g @types/vscode@latest`

**JetBrains:**
- Plugin disabled: Settings → Plugins → Enable LSP-Max
- Cache stale: Settings → Tools → LSP-Max → Clear Cache

**Web:**
- CORS issues: Add CORS headers to server: `--allow-origin='*'`
- Stale data: Hard refresh (Cmd+Shift+R)

**Desktop:**
- App won't open (macOS): `xattr -d com.apple.quarantine /Applications/LSP\ Max.app`
- Config not loaded: Check file permissions: `chmod 644 ~/Library/Application\ Support/lsp-max/config.toml`

---

## Additional Resources

- [LSP 3.18 Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/)
- [lsp-max Repository](https://github.com/seanchatmangpt/lsp-max)
- [CLAUDE.md](../CLAUDE.md) — Project constitution
- [AGENTS.md](../AGENTS.md) — Law-state runtime overview
- [docs/FEATURES.md](../docs/FEATURES.md) — Complete feature matrix
- [docs/TEST_INFRA.md](../docs/TEST_INFRA.md) — Testing and conformance

---

**Last updated:** 2026-06-14  
**Version:** 26.6.9 (CalVer)  
**Maintainers:** LSP-Max Core Team
