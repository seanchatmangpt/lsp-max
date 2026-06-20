# IDE Feature Matrix

Detailed breakdown of lsp-max features across VS Code, JetBrains, Web, and Desktop environments with configuration options and notes.

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Fully supported with no limitations |
| ⚠️ | Partially supported; see notes for limitations |
| ❌ | Not supported in this environment |
| 🔧 | Requires additional configuration |
| ⏱️ | Performance-dependent; works better with optimization |
| 🔐 | Security-related feature (requires appropriate permissions) |

---

## Core LSP Features

### Basic Editing

| Feature | VS Code | JetBrains | Web | Desktop | Notes |
|---------|---------|-----------|-----|---------|-------|
| **Diagnostics** | ✅ | ✅ | ✅ | ✅ | Full LSP support; includes custom max/* diagnostics |
| **Hover Information** | ✅ | ✅ | ⚠️ | ✅ | Web: read-only; others support markup rendering |
| **Go to Definition** | ✅ | ✅ | ✅ | ⚠️ | Desktop: preview in sidebar; others: open in editor |
| **Find References** | ✅ | ✅ | ✅ | ⚠️ | Desktop: limited to current document |
| **Rename Symbol** | ✅ | ✅ | ⚠️ | ✅ | Web: read-only; requires API call in others |
| **Document Symbols** | ✅ | ✅ | ✅ | ✅ | Breadcrumb + outline view |
| **Workspace Symbols** | ✅ | ✅ | ✅ | ✅ | Search across all files |
| **Code Completion** | ✅ | ✅ | ⚠️ | ✅ | Web: IDE-native only (not from LSP) |
| **Signature Help** | ✅ | ✅ | ⚠️ | ✅ | Tooltip on function calls |

### Advanced Navigation

| Feature | VS Code | JetBrains | Web | Desktop | Notes |
|---------|---------|-----------|-----|---------|-------|
| **Type Hierarchy** | ✅ | ✅ | ✅ | ⚠️ | Supertypes/subtypes navigation |
| **Call Hierarchy** | ✅ | ✅ | ✅ | ⚠️ | Incoming/outgoing calls |
| **Implementation Peek** | ✅ | ✅ | ⚠️ | ⚠️ | Web/Desktop: list view only |
| **Definition Peek** | ✅ | ✅ | ✅ | ⚠️ | Inline preview; Desktop: sidebar |

### Code Actions

| Feature | VS Code | JetBrains | Web | Desktop | Notes |
|---------|---------|-----------|-----|---------|-------|
| **Quick Fixes** | ✅ | ✅ | ⚠️ | ✅ | Edit and resolve phases supported |
| **Code Lens** | ✅ | ⚠️ | ⚠️ | ✅ | Per-line metrics (test count, implementations) |
| **Refactoring** | ✅ | ✅ | ⚠️ | ✅ | Workspace-wide refactors |

### Semantic Analysis

| Feature | VS Code | JetBrains | Web | Desktop | Notes |
|---------|---------|-----------|-----|---------|-------|
| **Semantic Tokens** | ✅ | ✅ | ⚠️ | ✅ | Full (all/delta/range variants); Web: static HTML only |
| **Inlay Hints** | ✅ | ✅ | ⚠️ | ✅ | Type/parameter/chaining hints |
| **Inline Values** | ⚠️ | ⚠️ | ❌ | ⚠️ | Debug context only |
| **Document Color** | ✅ | ✅ | ✅ | ⚠️ | CSS/color literals; Desktop: basic only |
| **Code Folding** | ✅ | ✅ | ⚠️ | ✅ | Region markers supported |

---

## Conformance & Law-State Features

### Conformance Vector

| Aspect | VS Code | JetBrains | Web | Desktop | Config | Notes |
|--------|---------|-----------|-----|---------|--------|-------|
| **Vector Display** | ✅ | ✅ | ✅ | ✅ | Always on | Admitted/refused/unknown axes |
| **Live Updates** | ✅ | ✅ | ✅ | ✅ | `conformance.checkInterval` | Real-time as you type |
| **Axis Details** | ✅ | ✅ | ✅ | ✅ | Always on | Expand to see law rules |
| **Historical Tracking** | ⚠️ | ⚠️ | ✅ | ✅ | Always on | Timeline in Web/Desktop |
| **Export** | ✅ | ✅ | ✅ | ✅ | Command/UI | JSON format with provenance |

**Enabling Conformance Checks:**

VS Code:
```json
{
  "lsp-max.conformance.enableConformanceChecks": true,
  "lsp-max.conformance.checkInterval": 5000
}
```

JetBrains:
```toml
[conformance]
enabled = true
```

### Receipt Ledger

| Aspect | VS Code | JetBrains | Web | Desktop | Status |
|--------|---------|-----------|-----|---------|--------|
| **View Receipts** | ✅ | ✅ | ✅ | ✅ | In sidebar/panel/web/window |
| **Filter by Status** | ✅ | ✅ | ✅ | ✅ | ADMITTED/CANDIDATE/BLOCKED |
| **View Digest** | ✅ | ✅ | ✅ | ✅ | BLAKE3 hash with copy |
| **Receipt History** | ⚠️ | ⚠️ | ✅ | ✅ | Web/Desktop only |
| **Receipt Chain** | ⚠️ | ⚠️ | ✅ | ✅ | Requires `prev_receipt_hash` field |
| **Export Receipts** | ✅ | ✅ | ✅ | ✅ | JSON + CSV formats |

**Configuration:**

All IDEs:
- Set `receipt.showReceiptDigests: true` to see full BLAKE3 hash
- Receipts are read-only (computed by server)

### ANDON Gate

| Aspect | VS Code | JetBrains | Web | Desktop | Status |
|--------|---------|-----------|-----|---------|--------|
| **Gate Status** | ✅ | ✅ | ✅ | ✅ | OPEN or ANDON |
| **Real-Time Polling** | ✅ | ✅ | ✅ | ✅ | 1-second updates |
| **Active Diagnostics List** | ✅ | ✅ | ✅ | ✅ | WASM4PM-* / GGEN-* families |
| **Block Shell Actions** | 🔧 | 🔧 | ✅ | ✅ | Via `PreToolUse` hook (VS Code/JB only) |
| **Tray Indicator** | ❌ | ❌ | ❌ | ✅ | Desktop app has tray icon |

**Enabling ANDON Gate Blocking (VS Code):**

Create `.claude/settings.json`:
```json
{
  "hooks": {
    "PreToolUse": {
      "command": "lsp-max-cli gate check",
      "onFailure": "block"
    }
  }
}
```

---

## Advanced Features

### Multi-Server Composition

| Aspect | VS Code | JetBrains | Web | Desktop | Status |
|--------|---------|-----------|-----|---------|--------|
| **Fan-Out to N Servers** | ✅ | ✅ | ✅ | ✅ | Via `lsp-max-compositor` |
| **Diagnostic Merging** | ✅ | ✅ | ✅ | ✅ | Quorum-based debounce |
| **Server Attribution** | ✅ | ✅ | ✅ | ✅ | Per-diagnostic metadata |
| **Crash Detection** | ✅ | ✅ | ✅ | ✅ | Auto-restart on failure |
| **Compositor Receipts** | ✅ | ✅ | ✅ | ✅ | Merge provenance with digest |

**Setup:**

```bash
# Install compositor
cargo install lsp-max-compositor

# Configure to spawn multiple servers
lsp-max-compositor start \
  --server rust /path/to/rust-lsp \
  --server python /path/to/python-lsp \
  --merge-mode quorum
```

### Process Mining (OCEL)

| Aspect | VS Code | JetBrains | Web | Desktop | Status |
|--------|---------|-----------|-----|---------|-------|
| **View OCEL Graph** | ⚠️ | ⚠️ | ✅ | ✅ | Web/Desktop: interactive SVG |
| **Timeline Filter** | ⚠️ | ⚠️ | ✅ | ✅ | Full date range support |
| **Event Inspection** | ⚠️ | ⚠️ | ✅ | ✅ | Drill-down into event details |
| **Object Lifecycle** | ⚠️ | ⚠️ | ✅ | ✅ | Track object state over time |
| **Export Events** | ⚠️ | ⚠️ | ✅ | ✅ | CSV/JSON download |

**Viewing OCEL in VS Code:**

```bash
# Export OCEL to file
lsp-max-cli snapshot export --format=ocel > evidence.ocel.json

# View in text editor (basic)
code evidence.ocel.json

# Better: Use web app
# Navigate to http://localhost:3000/ocel
```

### Snapshot & Export

| Aspect | VS Code | JetBrains | Web | Desktop | Status |
|--------|---------|-----------|-----|---------|-------|
| **Full Snapshot Export** | ✅ | ✅ | ✅ | ✅ | All server state + diagnostics |
| **Format: JSON** | ✅ | ✅ | ✅ | ✅ | Complete state tree |
| **Format: OCEL 2.0** | ✅ | ✅ | ✅ | ✅ | Process mining format |
| **Format: Custom** | ⚠️ | ⚠️ | ⚠️ | ⚠️ | Requires custom handlers |
| **Incremental Snapshots** | ✅ | ✅ | ✅ | ✅ | Only changed state |

**Exporting from CLI:**

```bash
# Full snapshot
lsp-max-cli snapshot export --format=json > snapshot.json

# OCEL format (for process mining)
lsp-max-cli snapshot export --format=ocel > evidence.ocel.json

# Specific crate only
lsp-max-cli snapshot export --crate=lsp-max-protocol > protocol.json
```

---

## IDE-Specific Features

### VS Code Extensions

| Feature | Supported | Config Key | Notes |
|---------|-----------|-----------|-------|
| Command Palette Integration | ✅ | N/A | All commands prefixed with `lsp-max.` |
| Source Control Integration | ⚠️ | `scm.lsp-max.enabled` | Shows diagnostics in SCM panel |
| Status Bar | ✅ | Always on | Gate status, server status |
| Sidebar Explorer | ✅ | Always on | Diagnostics tree, receipts list |
| Problem Panel | ✅ | Always on | Integrated with VS Code problems |
| Debug Adapter | ⚠️ | `debug.lsp-max.*` | Limited to variable inspection |
| Tasks Integration | ✅ | `.vscode/tasks.json` | Run `lsp-max-cli` commands |

**VS Code Task Example:**

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Check Conformance",
      "type": "shell",
      "command": "lsp-max-cli conformance vector",
      "problemMatcher": ["$lsp-max-conformance"]
    },
    {
      "label": "Reset Gate",
      "type": "shell",
      "command": "lsp-max-cli gate reset"
    }
  ]
}
```

### JetBrains IDE Integrations

| Feature | Supported | Access | Notes |
|---------|-----------|--------|-------|
| Toolwindow | ✅ | Tools → LSP-Max | Dockable panel for diagnostics/state |
| Project Structure | ✅ | Project panel | Shows lsp-max project metadata |
| Inspection Framework | ✅ | Settings → Inspections | Custom LSP-Max diagnostic family |
| Run Configurations | ✅ | Run → Edit Configurations | Launch lsp-max server as config |
| Debug Console | ✅ | Tools → LSP-Max → Debug Console | REPL for server commands |
| Notifications | ✅ | Bottom-right popup | Gate changes, connection issues |
| Settings Search | ✅ | Settings → Tools → LSP-Max | Full settings with search |

### Web App Pages

| Page | Route | Features |
|------|-------|----------|
| Dashboard | `/` | Server status, quick links, recent receipts |
| Receipts | `/receipts` | Ledger view with filtering and export |
| Conformance | `/conformance` | Vector display, timeline, law explorer |
| Gate Status | `/gate` | ANDON indicator, active diagnostics, history |
| OCEL Graph | `/ocel` | Interactive event log visualization |
| CLI Commands | `/cli` | Searchable command reference |
| Coverage | `/coverage` | Documentation vs. examples coverage |

### Desktop App Windows

| Window | Purpose | Features |
|--------|---------|----------|
| **Main Dashboard** | Central hub | Server health, quick actions, recent data |
| **Conformance Panel** | Law state viewer | Vector with drill-down, timeline |
| **Receipt Inspector** | Artifact browser | Digest, status, provenance chain |
| **Gate Monitor** | ANDON display | Live status, diagnostic list |
| **OCEL Viewer** | Process mining | Graph with filters and export |
| **Server Console** | Debug view | Real-time logs, command input |
| **Preferences** | Configuration | Server, UI, features, performance |

---

## Performance Characteristics

### Feature Performance Impact

| Feature | CPU | Memory | Network | Recommendation |
|---------|-----|--------|---------|-----------------|
| Diagnostics (basic) | Low | Low | Minimal | Always on |
| Semantic Tokens | High | Medium | Minimal | On for active files only |
| Inlay Hints | High | Medium | Minimal | Disable on large files |
| Type Hierarchy | Medium | Medium | One request | On-demand only |
| Call Hierarchy | Medium | Medium | One request | On-demand only |
| Hover Information | Low | Low | One request | Always on |
| Conformance Checks | High | High | One request | Interval-based (5-10s) |
| OCEL Rendering | High | High | Full load | On-demand in web app |
| Receipt Fetching | Low | Low | One request per receipt | Cache results |

### Optimization Tips

**For Large Codebases (10k+ files):**

```json
// VS Code
{
  "lsp-max.performance.debounceMs": 2000,
  "lsp-max.semanticTokens.enabled": false,
  "lsp-max.inlayHints.enabled": false,
  "lsp-max.conformance.enableConformanceChecks": false,
  "[files.large]": {
    "editor.wordWrap": "off",
    "editor.semanticHighlighting.enabled": false
  }
}
```

```toml
// JetBrains jetbrains.toml
[performance]
debounce_ms = 2000
max_parallel = 1
compression = true
max_cached_documents = 20

[features]
semantic_tokens = false
inlay_hints = false
```

**For Low-Spec Machines (2GB RAM):**

```json
// VS Code
{
  "lsp-max.performance.gcIntervalMs": 30000,
  "lsp-max.performance.maxCachedDocuments": 10,
  "lsp-max.semanticTokens.enabled": false,
  "lsp-max.inlayHints.enabled": false
}
```

---

## Diagnostic Families

All LSP-Max diagnostics are categorized by family for filtering:

| Family | Count | Severity | Typical Cause |
|--------|-------|----------|---------------|
| `ANTI-LLM-*` | 12+ | Error/Warning | Victory language, fake receipts, tower-lsp usage |
| `WASM4PM-*` | 5+ | Error | Process mining violations (ANDON trigger) |
| `GGEN-*` | 8+ | Warning | Code generation issues (ANDON trigger) |
| `LSP-CONFORMANCE-*` | 15+ | Warning | LSP 3.18 compliance issues |
| `VERSION-*` | 3 | Error | CalVer violations |
| `GATE-*` | 2 | Error | Gate system issues |

**Filter in IDE:**

VS Code:
```json
{
  "lsp-max.diagnostics.families": ["ANTI-LLM", "WASM4PM"],
  "lsp-max.diagnostics.maxPerDocument": 50
}
```

JetBrains Settings → Inspections → LSP-Max → Select families

---

## Configuration Inheritance

Settings are applied in this order (highest precedence first):

1. **Environment variables** (e.g., `LSP_MAX_GATE_FILE`)
2. **User/workspace settings** (`.vscode/settings.json`, JetBrains Settings)
3. **Project config** (`~/.config/lsp-max/*.toml`)
4. **IDE defaults** (built-in presets)
5. **Server defaults** (hard-coded in lsp-max-server)

Example (VS Code):
```json
{
  // User setting (high precedence)
  "lsp-max.performance.debounceMs": 1000,
  
  "[rust]": {
    // Language-specific override
    "lsp-max.semanticTokens.enabled": false
  }
}
```

---

## Accessibility

| IDE | Screen Reader | High Contrast | Keyboard Navigation |
|-----|---------------|---------------|-------------------|
| VS Code | ✅ | ✅ | ✅ |
| JetBrains | ✅ | ✅ | ✅ |
| Web | ⚠️ | ✅ | ✅ |
| Desktop | ✅ | ✅ | ✅ |

**Web App Accessibility:**
- ARIA labels on all interactive elements
- Keyboard shortcuts for all major functions
- Color-blind friendly palette (configurable)

---

## Deprecated Features

Features marked for removal in future versions:

| Feature | Deprecated in | Removal in | Replacement |
|---------|---------------|-----------|------------|
| `textDocument/implementation` | 26.5 | 26.8 | Use `textDocument/definition` + filter |
| `$/setTrace` | 26.4 | 26.7 | Use IDE debug logging instead |
| `textDocument/moniker` | 26.3 | 26.6 | Use LSP semantic tokens |

---

## Version Compatibility

| Feature | Minimum LSP Version | Minimum IDE Version |
|---------|---------------------|-------------------|
| Semantic Tokens (full) | 3.16 | VS Code 1.59, IntelliJ 2022.1 |
| Inline Values | 3.17 | VS Code 1.62, IntelliJ 2022.2 |
| Type Hierarchy | 3.17 | VS Code 1.62, IntelliJ 2022.2 |
| Inlay Hints (resolve) | 3.17 | VS Code 1.67, IntelliJ 2023.1 |
| Diagnostic Pull | 3.17 | VS Code 1.68, IntelliJ 2023.2 |
| Notebook Documents | 3.17 | VS Code 1.63, N/A (JetBrains) |

---

## Support Matrix

| Issue | VS Code | JetBrains | Web | Desktop |
|-------|---------|-----------|-----|---------|
| Report bug | GitHub Issues | GitHub Issues | GitHub Issues | GitHub Issues |
| Request feature | GitHub Discussions | GitHub Discussions | GitHub Discussions | GitHub Discussions |
| Performance help | VS Code troubleshooting | JetBrains profiler | Browser DevTools | Activity Monitor |
| Security concern | GitHub Security Advisory | GitHub Security Advisory | GitHub Security Advisory | GitHub Security Advisory |

---

**Last updated:** 2026-06-14  
**Version:** 26.6.9 (CalVer)  
**Maintainers:** LSP-Max Core Team
