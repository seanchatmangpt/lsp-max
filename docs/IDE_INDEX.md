# IDE Integration Documentation Index

Complete documentation for integrating **lsp-max** with Claude Code across VS Code, JetBrains IDEs, web applications, and desktop environments.

## 📋 Quick Start

**Choose your workflow:**

- **VS Code user?** → [VS Code Setup (5 min)](IDE_SETUP_GUIDES.md#for-vs-code-users)
- **JetBrains IDE user?** → [JetBrains Setup (5 min)](IDE_SETUP_GUIDES.md#for-jetbrains-users)
- **Desktop app preferred?** → [Desktop Setup (5 min)](IDE_SETUP_GUIDES.md#for-desktop-users-no-ide-required)
- **Building a web app?** → [Full-Stack Setup (30 min)](IDE_SETUP_GUIDES.md#3-full-stack-development-web-app--server)

---

## 📚 Documentation Files

### 1. **[IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md)** — Complete Integration Guide

Comprehensive documentation for all IDE environments:

- **VS Code Extension**
  - Installation (Marketplace, GitHub, source)
  - Configuration (`settings.json`, environment variables)
  - Keyboard shortcuts
  - Features matrix with status
  - Debugging integration
  - Performance tuning
  - Troubleshooting

- **JetBrains Plugins**
  - Installation (Marketplace, GitHub, source)
  - Configuration (IDE settings, `jetbrains.toml`)
  - Keyboard shortcuts per IDE
  - Features matrix (IDEA, CLion, RustRover, WebStorm, etc.)
  - IDE-specific debugging
  - Performance optimization
  - Troubleshooting

- **Web App Integration**
  - Docker and manual deployment
  - Environment configuration
  - Built-in features (receipts, conformance, OCEL)
  - Keyboard shortcuts
  - Performance optimization
  - Troubleshooting

- **Desktop App (Mac/Windows)**
  - Installation via Homebrew, Chocolatey, or direct download
  - Configuration files and startup
  - Keyboard shortcuts
  - Built-in features (server, dashboard, tray)
  - Performance tuning
  - Troubleshooting

- **Cross-IDE Feature Matrix** — See what works where
- **Development Workflows** — Common usage patterns

**Start here for:** Complete reference of all IDE features and options

---

### 2. **[IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md)** — Step-by-Step Setup

Detailed, copy-paste-ready setup instructions for different scenarios:

- **Quick Setup (5 minutes each)**
  - VS Code
  - JetBrains IDEs
  - Desktop app

- **Full Setup Guides (15-30 minutes)**
  1. Rust project with VS Code + CLI
  2. Multi-language project with JetBrains
  3. Full-stack development (Web + Server)
  4. Team collaboration (shared server)
  5. CI/CD pipeline integration
  6. Desktop + IDE unified workflow
  7. Advanced custom server configuration

- **Configuration Reference**
  - VS Code `settings.json`
  - JetBrains `jetbrains.toml`
  - Desktop `config.toml`
  - Environment variables

- **Troubleshooting Setup Issues**

**Start here for:** Step-by-step instructions for your specific setup

---

### 3. **[IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md)** — Feature Support Table

Detailed feature support matrix across all IDEs with status and notes:

- **Core LSP Features**
  - Diagnostics, hover, go to definition, references, rename
  - Code actions, code lens, document symbols, workspace symbols

- **Advanced Features**
  - Semantic tokens, inlay hints, type hierarchy, call hierarchy
  - Inline values, document color, code folding

- **lsp-max Custom Features**
  - Conformance vector with live updates
  - Receipt ledger with filtering and export
  - ANDON gate with real-time status
  - Snapshot & export in multiple formats
  - Process mining (OCEL) visualization

- **IDE-Specific Features**
  - VS Code: command palette, tasks, status bar
  - JetBrains: toolwindow, run configurations, inspections
  - Web: page structure and built-in views
  - Desktop: windows and tray integration

- **Performance Characteristics**
  - CPU/memory/network impact per feature
  - Optimization recommendations

- **Configuration Inheritance** — How settings override each other

- **Accessibility** — Screen readers, high contrast, keyboard navigation

**Start here for:** "Does feature X work in IDE Y?"

---

### 4. **[IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md)** — Debugging & Optimization

Comprehensive troubleshooting and performance guide:

- **Quick Diagnostics**
  - Check server status across all IDEs
  - Verify connection and gate

- **Common Issues & Solutions** (with copy-paste fixes)
  - Server not found / not running
  - Port already in use
  - Connection refused
  - Slow startup / high CPU / high memory
  - IDE-specific issues (extension crashes, diagnostics missing, etc.)

- **Performance Optimization Guide**
  - Tier 1: Quick wins (5 min, 30-50% improvement)
  - Tier 2: Medium optimizations (15 min, 40-60% improvement)
  - Tier 3: Heavy optimizations (30 min, 60-80% improvement)
  - Tier 4: Architectural changes (1+ hour)

- **Profiling & Metrics**
  - CPU profiling (VS Code, JetBrains, CLI)
  - Memory analysis
  - Flamegraph generation

- **Diagnostic Export & Analysis**
  - Export diagnostics in various formats
  - Analyze by family, severity, or context
  - Share for bug reports

- **Gate Troubleshooting**
  - Gate stuck on ANDON
  - Gate not blocking shell actions
  - Force reset procedures

- **Network Troubleshooting**
  - Remote server connection issues
  - Firewall debugging
  - CORS problems

**Start here for:** Solving a specific problem or optimizing performance

---

## 🗺️ Feature Coverage Map

### By IDE

| IDE | Core LSP | Advanced | max/* | Custom |
|-----|----------|----------|-------|--------|
| **VS Code** | ✅ Full | ✅ Full | ✅ Full | ✅ Tasks, palette |
| **JetBrains** | ✅ Full | ⚠️ Partial | ✅ Full | ✅ Toolwindow, runs |
| **Web** | ✅ Full | ⚠️ Partial | ✅ Full | ✅ Pages, export |
| **Desktop** | ✅ Full | ✅ Full | ✅ Full | ✅ Dashboard, tray |

### By Feature Category

| Feature | Doc Section | Notes |
|---------|-------------|-------|
| Installation | [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md#full-setup-guides) | Copy-paste commands |
| Configuration | [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#configuration) | Settings & files |
| Keyboard shortcuts | [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#keyboard-shortcuts) | Per-IDE reference |
| Debugging | [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#debugging-integration) + [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md) | Enable logs, attach debugger |
| Performance | [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#performance-tuning) + [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md#performance-optimization-guide) | Optimize settings, profile |
| Troubleshooting | [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md) | Common issues with solutions |
| Feature support | [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md) | What works where |

---

## 🎯 Common Tasks

### "I just want to start"

1. Choose your IDE:
   - **VS Code:** [IDE_SETUP_GUIDES.md: Quick Setup](IDE_SETUP_GUIDES.md#for-vs-code-users)
   - **JetBrains:** [IDE_SETUP_GUIDES.md: Quick Setup](IDE_SETUP_GUIDES.md#for-jetbrains-users)
   - **Desktop:** [IDE_SETUP_GUIDES.md: Quick Setup](IDE_SETUP_GUIDES.md#for-desktop-users-no-ide-required)

2. Copy the commands
3. Run them
4. Done!

### "I want to enable feature X"

1. Check support: [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md)
2. Find configuration: [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#configuration) or [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md#configuration-reference)
3. Add to your IDE settings
4. Restart IDE

### "Feature X isn't working"

1. Check if supported: [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md)
2. Check IDE-specific section: [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md)
3. Try troubleshooting: [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md#common-issues--solutions)

### "My IDE is slow"

1. Diagnosis: [IDE_TROUBLESHOOTING.md: Quick Diagnostics](IDE_TROUBLESHOOTING.md#quick-diagnostics)
2. Optimize: [IDE_TROUBLESHOOTING.md: Performance Optimization](IDE_TROUBLESHOOTING.md#performance-optimization-guide)
   - Tier 1 (5 min) → Tier 2 (15 min) → Tier 3 (30 min)

### "I'm setting up for my team"

1. Read: [IDE_SETUP_GUIDES.md: Team Collaboration](IDE_SETUP_GUIDES.md#4-team-collaboration-shared-server)
2. Deploy server on shared machine
3. Each developer: [IDE_SETUP_GUIDES.md: Developer Setup (JetBrains)](IDE_SETUP_GUIDES.md#step-2-developer-setup-jetbrains)

### "I'm setting up CI/CD"

1. Read: [IDE_SETUP_GUIDES.md: CI/CD Integration](IDE_SETUP_GUIDES.md#5-cicd-pipeline-integration)
2. Choose your CI system (GitHub Actions, GitLab CI, etc.)
3. Copy the workflow file
4. Commit and trigger build

### "I want keyboard shortcuts"

- **VS Code:** [IDE_INTEGRATIONS.md: Keyboard Shortcuts](IDE_INTEGRATIONS.md#keyboard-shortcuts)
- **JetBrains:** [IDE_INTEGRATIONS.md: Keyboard Shortcuts](IDE_INTEGRATIONS.md#keyboard-shortcuts-1)
- **Web:** [IDE_INTEGRATIONS.md: Keyboard Shortcuts](IDE_INTEGRATIONS.md#keyboard-shortcuts-2)
- **Desktop:** [IDE_INTEGRATIONS.md: Keyboard Shortcuts](IDE_INTEGRATIONS.md#keyboard-shortcuts-3)

### "I found a bug"

1. Collect diagnostics: [IDE_TROUBLESHOOTING.md: Diagnostic Export](IDE_TROUBLESHOOTING.md#diagnostic-export--analysis)
2. Report: [IDE_TROUBLESHOOTING.md: Report a Bug](IDE_TROUBLESHOOTING.md#report-a-bug)

---

## 📖 Related Documentation

- **[CLAUDE.md](../CLAUDE.md)** — Project constitution and core laws
- **[AGENTS.md](../AGENTS.md)** — Law-state runtime overview
- **[FEATURES.md](FEATURES.md)** — Complete LSP 3.18 feature matrix
- **[TEST_INFRA.md](TEST_INFRA.md)** — Testing and conformance system
- **[EXAMPLES.md](EXAMPLES.md)** — Example servers and use cases

---

## 🔍 Search Index

### Keywords

- **Setup:** [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md)
- **Keyboard:** [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#keyboard-shortcuts) (all 4 environments)
- **Performance:** [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md#performance-optimization-guide)
- **Configuration:** [IDE_SETUP_GUIDES.md](IDE_SETUP_GUIDES.md#configuration-reference)
- **Debugging:** [IDE_INTEGRATIONS.md](IDE_INTEGRATIONS.md#debugging-integration) + [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md)
- **Diagnostics:** [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md#diagnostic-export--analysis)
- **Gate:** [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md#gate-troubleshooting)
- **Network:** [IDE_TROUBLESHOOTING.md](IDE_TROUBLESHOOTING.md#network-troubleshooting)
- **Features:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md)

### IDEs

- **VS Code:** [IDE_INTEGRATIONS.md#vs-code-extension](IDE_INTEGRATIONS.md#vs-code-extension)
- **IntelliJ IDEA:** [IDE_INTEGRATIONS.md#jetbrains-plugins](IDE_INTEGRATIONS.md#jetbrains-plugins)
- **CLion, RustRover, WebStorm:** [IDE_INTEGRATIONS.md#jetbrains-plugins](IDE_INTEGRATIONS.md#jetbrains-plugins)
- **Web App:** [IDE_INTEGRATIONS.md#web-app-integration](IDE_INTEGRATIONS.md#web-app-integration)
- **Desktop (Mac/Windows):** [IDE_INTEGRATIONS.md#desktop-app-macwindows](IDE_INTEGRATIONS.md#desktop-app-macwindows)

### Features

- **Conformance Vector:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#conformance-vector)
- **Receipt Ledger:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#receipt-ledger)
- **ANDON Gate:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#andon-gate)
- **Semantic Tokens:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#semantic-analysis)
- **Type Hierarchy:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#advanced-navigation)
- **Call Hierarchy:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#advanced-navigation)
- **OCEL/Process Mining:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#process-mining-ocel)
- **Code Actions:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#code-actions)
- **Multi-Server:** [IDE_FEATURE_MATRIX.md](IDE_FEATURE_MATRIX.md#multi-server-composition)

---

## 📋 Document Structure

```
docs/IDE_INDEX.md (you are here)
├── IDE_INTEGRATIONS.md       — Complete reference (all features, all IDEs)
├── IDE_SETUP_GUIDES.md       — Copy-paste setup instructions
├── IDE_FEATURE_MATRIX.md     — Feature support table with notes
└── IDE_TROUBLESHOOTING.md    — Problems and solutions
```

---

## 📞 Getting Help

1. **Check the docs** — Search this index
2. **Browse issues** — [GitHub Issues](https://github.com/seanchatmangpt/lsp-max/issues)
3. **Ask a question** — [GitHub Discussions](https://github.com/seanchatmangpt/lsp-max/discussions)
4. **Report a bug** — [IDE_TROUBLESHOOTING.md: Report a Bug](IDE_TROUBLESHOOTING.md#report-a-bug)

---

## 📊 Version Information

- **lsp-max version:** 26.6.9 (CalVer)
- **Last updated:** 2026-06-14
- **Minimum IDE versions:**
  - VS Code: 1.85+
  - IntelliJ IDEA: 2024.1+
  - JetBrains plugins: 2024.1+
  - Node.js (web): 18+

---

## 🔄 Contributing

Found an issue in the docs? Have a better explanation?

1. Open an issue: [GitHub Issues](https://github.com/seanchatmangpt/lsp-max/issues)
2. Submit a PR with the fix
3. Use bounded language (per CLAUDE.md)

---

**Last updated:** 2026-06-14  
**Maintainers:** LSP-Max Core Team  
**License:** MIT OR Apache-2.0
