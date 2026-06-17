# IDE Troubleshooting & Performance Guide

Comprehensive troubleshooting, diagnostics, and performance optimization guide for lsp-max across all IDE environments.

---

## Quick Diagnostics

### Check Server Status

**All IDEs:**
```bash
# Is server running?
lsp-max-server --version

# Can you connect?
curl http://localhost:8080/max/state | jq .

# Check gate status
lsp-max-cli gate check

# View active diagnostics
lsp-max-cli diagnostics list
```

**VS Code:**
- Open **Output** panel (Ctrl+Shift+U)
- Select **lsp-max** from dropdown
- Look for "Server started" or error messages

**JetBrains:**
- **Tools** → **LSP-Max** → **Server Status**
- Check connection indicator at bottom of window
- View server logs: **Tools** → **LSP-Max** → **Debug Console**

**Web:**
- Open browser console (F12)
- Check Network tab for `/max/state` requests
- Look for 200 responses (success) or errors

**Desktop:**
- Check window title for "Listening on..."
- Server status in **Preferences** → **Server**
- View logs in **Preferences** → **Debug**

---

## Common Issues & Solutions

### Server Issues

#### ❌ "lsp-max-server: command not found"

**Cause:** Binary not in PATH

**Solution:**
```bash
# Option 1: Install via cargo
cargo install lsp-max-cli

# Option 2: Add to PATH
export PATH="$HOME/.cargo/bin:$PATH"
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc

# Option 3: Set full path in IDE
# VS Code: "lsp-max.serverPath": "/full/path/to/lsp-max-server"
# JetBrains: Settings → Tools → LSP-Max → Server Path
```

**Verify:**
```bash
which lsp-max-server
lsp-max-server --version
```

#### ❌ "Port 8080 already in use"

**Cause:** Another process listening on port 8080

**Solution:**
```bash
# Find process using port
lsof -i :8080          # macOS/Linux
netstat -ano | grep :8080  # Windows

# Kill process (if safe)
kill -9 <PID>

# OR use different port
lsp-max-server --port 8081

# Update IDE settings
# VS Code: "lsp-max.serverPath": "http://localhost:8081"
# JetBrains: Settings → Port: 8081
```

#### ❌ "Failed to connect to server"

**Cause:** Server not running or unreachable

**Solution:**
```bash
# Check if server is running
ps aux | grep lsp-max-server

# Start server manually (for testing)
lsp-max-server --log-level debug

# Test connection
curl http://localhost:8080/max/state

# If remote, verify network
ping <server-host>
telnet <server-host> 8080
```

**For network issues:**
```bash
# Check firewall (macOS)
sudo lsof -i -P | grep LISTEN

# Check firewall (Linux)
sudo ufw status
sudo ufw allow 8080/tcp

# Check firewall (Windows)
# Settings → Firewall & network → Advanced settings
```

#### ⏱️ "Server takes >10 seconds to start"

**Cause:** Large codebase indexing on startup

**Solution:**
```bash
# Option 1: Disable on-startup features
lsp-max-server --lazy-load

# Option 2: Increase timeout in IDE
# VS Code: "lsp-max.serverTimeout": 30000  # 30 seconds
# JetBrains: Settings → Timeout: 30s

# Option 3: Check system resources
top          # macOS/Linux
Task Manager # Windows

# Option 4: Use SSD (if on mechanical drive)
# Clone project to SSD for faster indexing
```

---

### VS Code Extensions Issues

#### ❌ "Extension fails to activate"

**Check output:**
```
1. Open Output panel: Ctrl+Shift+U
2. Select "lsp-max" from dropdown
3. Look for error messages
```

**Solutions:**

```bash
# Reinstall extension
code --uninstall-extension seanchatmangpt.lsp-max
code --install-extension seanchatmangpt.lsp-max

# Clear extension cache
rm -rf ~/.vscode/extensions/seanchatmangpt*
# Then reinstall

# Check VS Code version compatibility
code --version
# Should be 1.85+ for lsp-max 26.6.9
```

**If still failing, debug:**
```json
// .vscode/settings.json
{
  "lsp-max.trace.server": "messages",
  "[json]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  }
}
```

#### ⚠️ "Diagnostics not appearing"

**Checklist:**
- [ ] Server is running: `lsp-max-server --version`
- [ ] Extension is enabled: Check Extensions panel
- [ ] File type supported: Only .rs, .ts, .js, .py, etc.
- [ ] Project is open as folder (not file)
- [ ] No syntax errors that prevent parsing

**Diagnostic steps:**
```bash
# 1. Check if server sees the file
lsp-max-cli snapshot export | jq '.open_documents'

# 2. Check if diagnostics are being published
lsp-max-cli diagnostics list

# 3. Check server logs
# VS Code Output → lsp-max (see above)
```

**Solutions:**
```json
// .vscode/settings.json
{
  "lsp-max.enabled": true,
  "lsp-max.trace.server": "messages",
  "lsp-max.diagnostics.maxDiagnosticsPerDocument": 100,
  "[rust]": {
    "editor.formatOnSave": false  // Avoid conflicts
  }
}
```

#### 🔴 "High CPU usage (100%)"

**Cause:** Real-time semantic token computation

**Solutions:**

```json
// .vscode/settings.json — Disable heavy features
{
  "lsp-max.semanticTokens.enabled": false,
  "lsp-max.inlayHints.enabled": false,
  "lsp-max.performance.debounceMs": 2000  // Increase wait time
}
```

**For large files specifically:**
```json
{
  "[rust]": {
    "lsp-max.semanticTokens.enabled": false,
    "editor.wordWrap": "off",
    "editor.links": false
  }
}
```

**Monitor CPU:**
```bash
# On macOS/Linux
top -p <server_pid>
```

---

### JetBrains Plugin Issues

#### ❌ "Plugin not found in marketplace"

**Cause:** IDE version incompatible

**Solution:**
```bash
# Check minimum version requirement
cat extensions/jetbrains/gradle.properties | grep ideaVersion

# Download compatible version from GitHub releases
# Settings → Plugins → Install from Disk → Select .zip

# Or build from source
cd extensions/jetbrains
./gradlew buildPlugin
# Install: build/distributions/lsp-max-*.zip
```

#### ❌ "Plugin crashes on startup"

**Clear plugin cache:**
```bash
# macOS
rm -rf ~/Library/Application\ Support/JetBrains/IntelliJ*/caches/plugins*

# Linux
rm -rf ~/.cache/JetBrains/IntelliJ*/plugins*

# Windows
rmdir /s %AppData%\JetBrains\IntelliJ*\caches\plugins
```

**Restart IDE from safe mode:**
```
Help → Restart in Safe Mode
```

#### ⚠️ "Server not connecting"

**Check in IDE:**
1. **Settings** → **Tools** → **LSP-Max**
2. Verify **Server Path** is set
3. Click **Test Connection** button (if available)
4. Check server logs: **Tools** → **LSP-Max** → **Server Status**

**Manual test:**
```bash
# If using local path
/path/to/lsp-max-server --version

# If using network URL
curl http://192.168.1.100:8080/max/state
```

#### 🔴 "High memory usage (>2GB)"

**Solutions:**

```toml
# ~/.config/lsp-max/jetbrains.toml
[performance]
max_cached_documents = 20  # Default: 50
compression = true
gc_interval_ms = 30000     # More frequent GC
```

**IDE heap settings:**
```properties
# ~/.<IDE>/idea.properties
# Increase heap allocation
-Xmx4096m
-Xms2048m

# Use G1GC for better performance
-XX:+UseG1GC
-XX:MaxGCPauseMillis=200
```

#### ⚠️ "Diagnostics lagging behind typing"

**Solutions:**

```toml
# ~/.config/lsp-max/jetbrains.toml
[performance]
debounce_ms = 1000  # Wait 1s after typing stops
max_parallel = 2    # Reduce concurrent requests
```

**IDE-specific:**
```
Settings → Editor → General → Auto Save
  Save modified files: ON
  
Settings → Tools → LSP-Max
  Debounce: 1000ms
```

---

### Web App Issues

#### ❌ "Cannot connect to server"

**Error in browser console:**
```
Failed to fetch from http://localhost:8080
```

**Solutions:**

```bash
# 1. Check if server is running
lsp-max-server --version
ps aux | grep lsp-max-server

# 2. Start server if needed
lsp-max-server &

# 3. Verify URL in .env
cat web/.env.local | grep LSP_MAX_SERVER_URL
# Should match actual server address

# 4. Check CORS headers
curl -H "Origin: http://localhost:3000" \
  http://localhost:8080/max/state
# Should have Access-Control-Allow-Origin header
```

**For production deployment:**
```bash
# Add CORS headers to server startup
lsp-max-server \
  --cors-origin=https://example.com \
  --cors-methods=GET,POST
```

#### ⚠️ "Receipts page shows empty"

**Cause:** Receipt artifacts not in repository

**Solution:**
```bash
# Check if receipts exist
ls -la receipts/

# Generate receipts
cargo test --test test_receipts

# Add to git (create .gitignore exception)
echo "!receipts/" >> .gitignore
git add receipts/
git commit -m "Add receipt artifacts"
```

**If using Docker:**
```dockerfile
# Dockerfile
FROM rust:latest
WORKDIR /app
COPY . .
RUN cargo test --test test_receipts
RUN cargo build --release
```

#### 🔴 "OCEL viewer slow to render"

**Cause:** Large OCEL file (>100k events)

**Solutions:**

```env
# web/.env.local
NEXT_PUBLIC_MAX_OCEL_EVENTS=1000  # Limit rendered events
NEXT_PUBLIC_ENABLE_COMPRESSION=true
```

**Browser optimization:**
```javascript
// Hard refresh to clear cache
Ctrl+Shift+R (Windows/Linux)
Cmd+Shift+R (macOS)
```

**Server-side:**
```bash
# Export smaller OCEL file
lsp-max-cli snapshot export \
  --format=ocel \
  --limit=5000 \
  > evidence-limited.ocel.json
```

#### ❌ "Build fails with npm"

**Cause:** Node.js version or dependency issue

**Solution:**
```bash
# Check Node version (should be 18+)
node --version

# Clear npm cache
npm cache clean --force

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install

# Try build again
npm run build
```

---

### Desktop App Issues

#### ❌ "App crashes on startup (macOS)"

**Cause:** Signature/notarization issue

**Solution:**
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /Applications/LSP\ Max.app

# Or allow in Security settings:
# System Preferences → Security & Privacy → General
# Click "Open Anyway" next to LSP Max
```

**If rebuilt from source:**
```bash
# Ensure correct architecture
file /Applications/LSP\ Max.app/Contents/MacOS/lsp-max
# Should show: Mach-O 64-bit executable arm64 (Apple Silicon)
# or: Mach-O 64-bit executable x86_64 (Intel)

# If wrong, rebuild for correct architecture
cargo build --release --target aarch64-apple-darwin  # Apple Silicon
cargo build --release --target x86_64-apple-darwin   # Intel
```

#### ❌ "App crashes on startup (Windows)"

**Cause:** Missing DLL dependencies

**Solution:**
```powershell
# Install Visual C++ Redistributable
# Download from: https://support.microsoft.com/en-us/help/2977003

# OR rebuild from source
cargo build --release --target x86_64-pc-windows-msvc
```

#### ⚠️ "Cannot access from IDE"

**Error:** `connection refused on port 8080`

**Solutions:**

1. **Verify app is running:**
   - Check in System Tray / Menu Bar
   - App should show "Listening on 127.0.0.1:8080"

2. **Check preferences:**
   - Open app
   - **Preferences** → **Server**
   - Verify port is 8080
   - Verify host is 127.0.0.1 (local only)

3. **Configure IDE for local server:**
   ```json
   // VS Code
   {
     "lsp-max.serverPath": "http://localhost:8080"
   }
   ```
   
   ```toml
   // JetBrains jetbrains.toml
   [server]
   path = "http://localhost:8080"
   ```

#### 🔴 "High CPU/Memory usage"

**Cause:** Continuous conformance checking or large OCEL

**Solutions:**

1. **Disable on-startup features:**
   - **Preferences** → **Features**
   - Uncheck "Continuous conformance"
   - Uncheck "Auto-load OCEL"

2. **Increase performance:**
   ```toml
   # macOS: ~/Library/Application\ Support/lsp-max/config.toml
   # Windows: %APPDATA%\lsp-max\config.toml
   
   [performance]
   max_cached_documents = 20
   enable_compression = true
   gc_interval_ms = 30000
   ```

3. **Reduce UI updates:**
   ```toml
   [ui]
   refresh_interval_ms = 2000  # Update UI every 2s instead of 1s
   ```

---

## Performance Optimization Guide

### Baseline Measurements

**Before optimization, measure current performance:**

```bash
# CPU usage (macOS/Linux)
time cargo test --workspace
top -p $(pgrep lsp-max-server)

# Memory usage
ps aux | grep lsp-max-server
# Look at RSS column

# Diagnostics latency
lsp-max-cli conformance vector
# Note response time
```

### Tier 1: Quick Wins (< 5 min)

1. **Increase debounce time:**
   ```json
   // VS Code
   { "lsp-max.performance.debounceMs": 1000 }  // Default: 500
   ```

2. **Disable semantic tokens:**
   ```json
   { "lsp-max.semanticTokens.enabled": false }
   ```

3. **Disable inlay hints:**
   ```json
   { "lsp-max.inlayHints.enabled": false }
   ```

**Expected impact:** 30-50% CPU reduction

### Tier 2: Medium Optimizations (15 min)

1. **Limit diagnostics:**
   ```json
   {
     "lsp-max.diagnostics.maxDiagnosticsPerDocument": 50,  // Default: 100
     "lsp-max.conformance.enableConformanceChecks": false
   }
   ```

2. **Reduce cache:**
   ```json
   { "lsp-max.performance.maxCachedDocuments": 30 }  // Default: 50
   ```

3. **Increase GC interval:**
   ```json
   { "lsp-max.performance.gcIntervalMs": 120000 }  // Default: 60000
   ```

**Expected impact:** 40-60% memory reduction

### Tier 3: Heavy Optimizations (30 min)

1. **Use network server:**
   - Run `lsp-max-server` on separate machine
   - All IDEs connect to network URL
   - Frees local CPU/RAM

2. **Enable compression:**
   ```json
   { "lsp-max.performance.enableCompression": true }
   ```

3. **Reduce parallel requests:**
   ```json
   { "lsp-max.performance.maxParallelRequests": 1 }
   ```

4. **Disable process mining:**
   ```json
   { "lsp-max.features.processMinining": false }
   ```

**Expected impact:** 60-80% reduction in resource usage

### Tier 4: Architectural Changes (1+ hour)

1. **Use compositor for multi-server:**
   ```bash
   lsp-max-compositor start \
     --merge-mode=quorum \
     --debounce=2000
   ```

2. **Implement server caching layer:**
   ```bash
   # Cache frequently accessed data (receipts, OCEL)
   lsp-max-server \
     --enable-redis \
     --redis-url=redis://localhost:6379
   ```

3. **Use workspace filtering:**
   - Exclude large directories
   - `.vscode/settings.json`:
     ```json
     {
       "files.exclude": {
         "**/target": true,
         "**/node_modules": true
       }
     }
     ```

### Profiling

**VS Code:**
```json
{
  "lsp-max.telemetry.enabled": true,
  "lsp-max.performance.exportMetrics": true
}
```

Then export metrics:
```bash
lsp-max-cli metrics export > metrics.json
```

**JetBrains:**
1. **Run** → **Profile CPU**
2. Navigate code with LSP-Max active
3. Stop profiling and analyze hot spots

**Desktop/CLI:**
```bash
# Flamegraph
lsp-max-server --profile=flamegraph
# Generates flamegraph.svg

# Perf record (Linux)
perf record -p $(pgrep lsp-max-server)
perf report
```

---

## Diagnostic Export & Analysis

### Export Full Diagnostics

```bash
# All diagnostics
lsp-max-cli diagnostics export > diagnostics.json

# Specific family
lsp-max-cli diagnostics export --family=ANTI-LLM > anti-llm.json

# With context
lsp-max-cli diagnostics export --with-context=5  # 5 lines before/after
```

### Analyze Diagnostics

```bash
# Count by family
jq -r '.diagnostics[].family' diagnostics.json | sort | uniq -c

# Group by severity
jq 'group_by(.severity)' diagnostics.json | jq 'map({severity: .[0].severity, count: length})'

# Find blockers (ANDON-triggering)
jq '.diagnostics[] | select(.family | test("WASM4PM|GGEN"))' diagnostics.json
```

### Share Diagnostics

```bash
# For bug reports
lsp-max-cli diagnostics export | gzip > diagnostics.json.gz
# Attach to GitHub issue
```

---

## Gate Troubleshooting

### Gate Stuck on ANDON

**Problem:** Gate won't clear despite fixing code

**Debug:**
```bash
# Check active diagnostics
lsp-max-cli diagnostics list | grep -E "WASM4PM|GGEN"

# Check specific diagnostic
lsp-max-cli diagnostics view WASM4PM-001

# Check gate file
cat /tmp/lsp-max.gate
# 0 = OPEN, 1 = ANDON

# Force reset (careful!)
lsp-max-cli gate reset
```

**Root causes:**
- Diagnostic is false positive → Fix diagnostic detector
- Diagnostic not cleared by server → Restart server
- Gate file corruption → Delete and resync

### Gate Not Blocking Shell (vs Code)

**Problem:** Gate check returns 0 (OPEN) but .claude hook doesn't block

**Solution:**
```json
// .claude/settings.json
{
  "hooks": {
    "PreToolUse": {
      "command": "lsp-max-cli gate check",
      "onFailure": "block",  // Must be "block"
      "timeout": 5000
    }
  }
}
```

Restart Claude Code IDE after changing.

---

## Network Troubleshooting

### Remote Server Connection Issues

**Test connectivity:**
```bash
# From client machine
curl -v http://server-ip:8080/max/state

# Check response headers
curl -H "Accept: application/json" http://server-ip:8080/max/state | jq .
```

**Enable verbose logging:**
```bash
# On server
RUST_LOG=debug,lsp_max=trace lsp-max-server --host 0.0.0.0 --port 8080

# Monitor logs
tail -f /tmp/lsp-max.log
```

**Common network issues:**

```
ECONNREFUSED
  → Server not running on that port/host
  
ETIMEDOUT  
  → Firewall blocking, or route unreachable
  
ENOTFOUND
  → DNS not resolving hostname
  
403 Forbidden
  → CORS or authentication issue
```

**Solutions:**
```bash
# Check firewall on server
sudo ufw allow 8080/tcp  # Linux

# Check from client
telnet server-ip 8080

# Add CORS if needed
lsp-max-server --allow-origin='*'
```

---

## Support & Escalation

### When to Collect Info

Before opening GitHub issue, collect:

```bash
# System info
rustc --version
cargo --version
node --version

# IDE version
code --version                    # VS Code
idea --version                    # JetBrains

# Extension/plugin version
# VS Code: Extensions panel
# JetBrains: Settings → Plugins

# Server version
lsp-max-server --version

# Diagnostics
lsp-max-cli diagnostics export > diagnostics.json

# Server logs
# Export from Output panel or Debug console

# Config files
cat .vscode/settings.json
cat ~/.config/lsp-max/*.toml
```

### Report a Bug

1. Gather info above
2. Go to [GitHub Issues](https://github.com/seanchatmangpt/lsp-max/issues)
3. Create issue with:
   - Title: Clear, specific description
   - Environment: OS, IDE version, extension version
   - Steps to reproduce: Minimal example
   - Diagnostics: Attach `diagnostics.json`
   - Logs: Paste relevant server output
   - Expected vs actual behavior

### Request a Feature

Use [GitHub Discussions](https://github.com/seanchatmangpt/lsp-max/discussions) with:
- Clear use case
- Which IDE(s) affected
- Mock-ups or examples (if UI feature)
- Links to related issues

---

## Resources

- [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md) — Full feature documentation
- [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md) — Setup instructions
- [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md) — Feature support table
- [FEATURES.md](FEATURES.md) — LSP 3.18 feature list
- [GitHub Issues](https://github.com/seanchatmangpt/lsp-max/issues)
- [GitHub Discussions](https://github.com/seanchatmangpt/lsp-max/discussions)

---

**Last updated:** 2026-06-14  
**Version:** 26.6.9 (CalVer)  
**Need help?** Open an issue on GitHub or start a discussion.
