# Claude Code Skills Registry

**Version:** 26.6.9 | **Last Updated:** 2026-06-14 | **Status:** ADMITTED

A comprehensive registry of all available Claude Code skills, including their parameters, trigger patterns, use cases, and integration points. This registry is the authoritative source for skill documentation and serves as the reference for skill adoption patterns.

---

## Table of Contents

1. [Registry Overview](#registry-overview)
2. [Skills by Category](#skills-by-category)
3. [Quick Reference Index](#quick-reference-index)
4. [Integration Patterns](#integration-patterns)
5. [Best Practices](#best-practices)
6. [Skill Lifecycle](#skill-lifecycle)

---

## Registry Overview

### What is a Skill?

A Claude Code **skill** is a specialized capability invoked via the `/skillname` syntax. Skills encapsulate domain knowledge, automation workflows, and procedural patterns that extend Claude's native capabilities within the Claude Code environment.

### Skill Invocation

```bash
/skillname [args]
```

**Example:**
```bash
/code-review --comment
/loop 5m /verify
/update-config
```

### Skill States

- **AVAILABLE** — Fully functional, documented, ready for use.
- **CANDIDATE** — Experimental or proposed; may require explicit opt-in.
- **BLOCKED** — Unavailable due to missing dependencies, configuration, or prerequisites.
- **PARTIAL** — Partial functionality; some features operational, others blocked.

### Registration Authority

Skills are declared in the system reminder and loaded on-demand. A skill may be:
- **Built-in** — Shipped with Claude Code, always available.
- **Contextual** — Loaded based on active project context (e.g., `session-start-hook` for Claude Code web).
- **Deferred** — Named in the skill list but schema must be loaded via `ToolSearch` before invocation.

---

## Skills by Category

### Category 1: Project Configuration & Setup

Skills for initializing projects, configuring tools, and managing development environment.

#### 1.1 session-start-hook

**Status:** AVAILABLE | **Scope:** Web Session Setup  
**Availability:** Contextual (Claude Code on Web)

**Purpose:**  
Create and develop startup hooks for Claude Code on the web. Ensures a repository can run tests and linters during web sessions without manual configuration.

**Parameters:**
- None (accepts interactive guidance)

**Typical Usage:**
```bash
/session-start-hook
```

**Trigger Pattern:**
- User wants to set up a repository for Claude Code on web.
- User needs to ensure tests and linters run automatically during sessions.

**Output:**
- SessionStart hook configuration in `.claude/settings.json`
- Validated test/lint commands
- Status report: `ADMITTED`

**Related Files:**
- `.claude/settings.json` — Hook configuration (modified)
- `.claude/hooks/` — Hook script directory (created if needed)

**Integration Points:**
- Works with `update-config` to manage hook lifecycle
- Precedes `verify` for test validation
- Pairs with `run` for project initialization

**See Also:**
- `/update-config` — Manage hooks after creation
- `/verify` — Test hook functionality

---

#### 1.2 update-config

**Status:** AVAILABLE | **Scope:** Configuration Management  
**Availability:** Built-in

**Purpose:**  
Configure the Claude Code harness via `settings.json` and `settings.local.json`. Manages permissions, environment variables, hooks, and behavioral automation rules. This is the **authoritative tool for all configuration changes**.

**Parameters:**
- `args` (string, optional) — Configuration intent (e.g., "allow npm commands", "set DEBUG=true", "add bq permission")

**Typical Usage:**
```bash
/update-config "allow npm commands"
/update-config "set DEBUG=true"
/update-config "add bq permission to global settings"
/update-config "move permission to user settings"
/update-config "when claude stops show X"
```

**Trigger Patterns:**
- **Permissions** — "allow X", "add permission", "move permission to"
- **Environment variables** — "set X=Y", "export X=Y"
- **Hooks** — Automated behaviors ("from now on when X", "each time X", "whenever X", "before/after X")
- **Troubleshooting** — Hook failures, permission issues
- **Simple Settings** — Theme, model (suggestion: use `/config` for simple settings instead)

**Configuration Categories:**

| Category | Examples | Notes |
|----------|----------|-------|
| Permissions | npm, git, bash, bq, file-system | Uses allow/deny lists; can be global or user-scoped |
| Environment | DEBUG=true, API_KEY=xxx, NODE_ENV=dev | Scope: workspace or session |
| Hooks | PreToolUse, PostToolUse, PreCommit | ANDON gate enforcement via hooks (see CLAUDE.md) |
| Auto-behaviors | "whenever X, do Y" | Harness executes, not Claude memory |
| Settings | theme, model, keybindings | Simple settings → use `/config` instead |

**Output:**
- Modified `settings.json` or `settings.local.json`
- Validation status: `ADMITTED`, `CANDIDATE`, or `REFUSED`
- Description of applied configuration

**Example Workflow:**
```bash
# Step 1: Add a permission
/update-config "allow npm commands"

# Step 2: Set environment variable
/update-config "set NODE_ENV=development"

# Step 3: Configure a hook
/update-config "whenever tests fail, show summary"
```

**Related Files:**
- `.claude/settings.json` — Global Claude Code configuration (modified)
- `.claude/settings.local.json` — User-local overrides (created/modified)
- `.claude/hooks/` — Hook scripts directory

**Integration Points:**
- **ANDON Gate** — `PreToolUse` hook in settings.json runs `lsp-max-cli gate check` before every Bash call
- **Prerequisite for** — `session-start-hook`, `keybindings-help`
- **Complements** — `loop` (configure recurring task behavior), `fewer-permission-prompts` (reduce permission alerts)

**See Also:**
- `/config` — Simple theme/model settings (recommended for non-programmatic changes)
- `/keybindings-help` — Configure keyboard shortcuts

---

#### 1.3 keybindings-help

**Status:** AVAILABLE | **Scope:** Keyboard Configuration  
**Availability:** Built-in

**Purpose:**  
Customize keyboard shortcuts and rebind keys in Claude Code. Manages `~/.claude/keybindings.json` for chord bindings, modifier keys, and command shortcuts.

**Parameters:**
- `args` (string, optional) — Keybinding intent (e.g., "rebind ctrl+s", "add a chord shortcut", "change submit key")

**Typical Usage:**
```bash
/keybindings-help "rebind ctrl+s to save-and-format"
/keybindings-help "add a chord shortcut ctrl+k ctrl+r for review"
/keybindings-help "change the submit key to alt+enter"
/keybindings-help "customize keybindings"
```

**Trigger Patterns:**
- User wants to customize keyboard shortcuts
- User needs to rebind keys or create chord bindings
- User wants to modify submit/cancel/command-palette keys
- Accessibility customization required

**Output:**
- Modified `~/.claude/keybindings.json`
- Validation of new bindings
- Conflict detection (if binding shadows existing command)
- Status: `ADMITTED` or `REFUSED` (if conflict)

**Related Files:**
- `~/.claude/keybindings.json` — Keybinding configuration (modified)
- `~/.claude/settings.json` — Referenced for context

**Integration Points:**
- Works with `update-config` for broader settings
- Helps reduce friction when using other skills (e.g., custom shortcut for `/verify`)

**See Also:**
- `/update-config` — Configure non-keybinding settings
- `/config` — View current keybindings

---

### Category 2: Validation & Verification

Skills for testing, verifying, and reviewing code changes before committing or shipping.

#### 2.1 verify

**Status:** AVAILABLE | **Scope:** Change Validation  
**Availability:** Built-in

**Purpose:**  
Verify that a code change actually does what it's supposed to by running the app and observing behavior. This is the hands-on validation skill: **launch and observe in a real environment**, not just test output.

**Parameters:**
- None (interactive guidance based on project type)

**Typical Usage:**
```bash
/verify
```

**Trigger Patterns:**
- Asked to verify a PR works
- Confirm a bug fix actually resolves the issue
- Test a change manually before pushing
- Check that a feature works end-to-end
- Validate local changes in the live app

**Workflow:**

1. **Project Type Detection**
   - Looks for a project skill that covers launching the app
   - Falls back to built-in patterns: CLI, server, TUI, Electron, browser-driven, library

2. **App Launch**
   - Executes appropriate start command (npm start, cargo run, etc.)
   - Captures app state and readiness indicators

3. **Behavior Observation**
   - Performs manual steps to trigger the changed functionality
   - Observes output: console logs, UI changes, network requests
   - Compares against expected behavior

4. **Receipt Generation**
   - Documents observed behavior with artifacts (logs, screenshots)
   - Generates receipt: `ADMITTED` (works as expected) or `REFUSED` (unexpected behavior)
   - If unexpected: proposes diagnostic steps

**Output:**
- Verification receipt with status `ADMITTED` or `REFUSED`
- Evidence artifacts: logs, screenshots, command output
- Pass/fail summary for each observed behavior
- Recommendations for next steps

**Related Files:**
- Project-specific launch scripts (detected automatically)
- `.claude/settings.json` — May contain launch configuration
- Test results and logs

**Integration Points:**
- **Precedes** `code-review` (verify functionality before peer review)
- **Complements** `run` (run launches; verify validates behavior)
- **Works with** `loop` (recurring verification)

**Difference from `run`:**
- `run` — Launch the app, see it work
- `verify` — Launch app AND validate behavior against specification, generate receipt

**See Also:**
- `/run` — Launch and drive the app
- `/code-review` — Review code after verification
- `/test` (built-in) — Run test suite

---

#### 2.2 code-review

**Status:** AVAILABLE | **Scope:** Code Quality Assurance  
**Availability:** Built-in

**Purpose:**  
Review the current diff for correctness bugs and reuse/simplification/efficiency cleanups at the specified effort level. Can post findings as inline PR comments or apply auto-fixes to the working tree.

**Parameters:**
- `--comment` (flag, optional) — Post findings as inline PR comments instead of just reporting
- `--fix` (flag, optional) — Apply findings to the working tree after review
- `effort` (choice: "low", "medium", "high", "max") — Optional effort level (default: depends on context)

**Typical Usage:**
```bash
/code-review
/code-review --comment
/code-review --fix
/code-review high --comment
```

**Effort Levels:**

| Level | Coverage | Findings | Confidence |
|-------|----------|----------|-----------|
| **low** | Core logic only | Fewer, highest-confidence bugs | High |
| **medium** | Logic + common patterns | Moderate scope, good confidence | Medium |
| **high** | Broad coverage (logic, style, efficiency) | Broader, may include uncertain findings | Variable |
| **max** | Exhaustive (all categories, edge cases) | Maximum coverage; may include exploratory findings | Lower |

**Review Categories:**
1. **Correctness Bugs** — Logic errors, off-by-one, null checks, race conditions
2. **Reuse** — Duplicate code, missing abstractions, library opportunities
3. **Simplification** — Complex conditionals, unused variables, dead code
4. **Efficiency** — Performance issues, O(n²) patterns, unnecessary allocations
5. **Style** — Naming, formatting, consistency violations (post-cleanup only)

**Flags:**
- `--comment` — Post inline comments on PR (requires active PR context)
- `--fix` — Apply auto-fix recommendations to working tree (creates new commit, never amends)

**Output:**
- Categorized findings with line numbers and code snippets
- Severity: `CORRECTNESS` (must fix), `EFFICIENCY` (should fix), `STYLE` (nice-to-have)
- If `--comment`: inline PR comments created
- If `--fix`: new commit with prefix `refactor:` or `fix:`
- Status: `ADMITTED` (review complete), `REFUSED` (unsafe to auto-fix)

**Related Files:**
- Current git diff (auto-detected)
- Active PR metadata (if `--comment` used)
- Working tree (if `--fix` applied)

**Integration Points:**
- **Follows** `verify` (verify works before reviewing code)
- **Precedes** commit/push
- **Complements** `simplify` (code-review finds bugs; simplify handles reuse/cleanup)
- **Works with** pull request workflow

**Example Workflow:**
```bash
# 1. Verify the change works
/verify

# 2. Request code review
/code-review --comment

# 3. Address feedback manually or auto-apply fixes
/code-review --fix
```

**See Also:**
- `/simplify` — Refactor for reuse/simplification/efficiency (no bugs)
- `/security-review` — Security-focused code review
- `/verify` — Validate behavior before review

---

#### 2.3 simplify

**Status:** AVAILABLE | **Scope:** Code Refactoring  
**Availability:** Built-in

**Purpose:**  
Review changed code for reuse, simplification, efficiency, and altitude cleanups, then apply the fixes. **Quality-only**: it does not hunt for bugs (use `/code-review` for that). Focuses on making good code better.

**Parameters:**
- None (automatic application of fixes)

**Typical Usage:**
```bash
/simplify
```

**Refactoring Categories:**
1. **Reuse** — Consolidate duplicates, extract common patterns
2. **Simplification** — Reduce nesting, clarify intent, remove intermediate variables
3. **Efficiency** — Optimize loop bounds, cache expensive operations
4. **Altitude** — Raise abstraction level, inline trivial helpers, promote common logic

**Constraints:**
- Does NOT find correctness bugs (use `/code-review`)
- Does NOT change behavior
- Does NOT break tests
- Only refactors code that is already working

**Output:**
- Applied refactoring with new commit (never amends)
- Commit message summarizes changes: `refactor: consolidate X patterns` or `refactor: simplify Y logic`
- Before/after comparison
- Status: `ADMITTED` (refactoring applied), `CANDIDATE` (unsafe; requires manual review), or `REFUSED`

**Related Files:**
- Current git diff (auto-detected)
- Working tree (modified)
- New commit with prefix `refactor:`

**Integration Points:**
- **Follows** `code-review` (review finds bugs; simplify makes it cleaner)
- **Precedes** commit/push
- **Complements** `verify` (keeps behavior intact)

**Example Workflow:**
```bash
# 1. Code review finds bugs
/code-review --fix

# 2. Simplify the resulting code
/simplify

# 3. Verify it still works
/verify
```

**See Also:**
- `/code-review` — Find bugs AND suggest refactoring
- `/verify` — Validate behavior unchanged after simplification

---

#### 2.4 security-review

**Status:** AVAILABLE | **Scope:** Security Assessment  
**Availability:** Built-in

**Purpose:**  
Complete a security review of pending changes on the current branch. Identifies vulnerabilities, unsafe patterns, credential leaks, and OWASP-class issues.

**Parameters:**
- None (automatic security assessment)

**Typical Usage:**
```bash
/security-review
```

**Review Categories:**
1. **Credential Leaks** — API keys, passwords, tokens in plaintext
2. **Injection Vulnerabilities** — SQL injection, command injection, XSS
3. **Access Control** — Privilege escalation, authorization bypasses
4. **Cryptography** — Weak algorithms, improper key management
5. **Dependency Risks** — Known CVEs, unmaintained libraries
6. **Data Exposure** — Sensitive data in logs, unencrypted transit
7. **Input Validation** — Unsafe deserialization, path traversal

**Output:**
- Categorized security findings
- Severity: `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`
- CWE/OWASP references
- Remediation guidance for each issue
- Status: `ADMITTED` (no critical issues), `REFUSED` (critical issues found)

**Related Files:**
- Current git diff (auto-detected)
- Dependency manifests (Cargo.toml, package.json, etc.)
- Configuration files (.env, secrets, etc.)

**Integration Points:**
- **Precedes** PR approval and merge
- **Works with** `code-review` (broader code review includes style; security-review is focused)
- **May block** release gates (ANDON gate enforcement)

**Example Workflow:**
```bash
# Security review before merge
/security-review

# If CRITICAL found: fix and re-review
/security-review
```

**See Also:**
- `/code-review` — Broader code quality review
- `/verify` — Functional testing (not security)

---

### Category 3: Code Execution & Automation

Skills for running code, driving apps, and automating repetitive workflows.

#### 3.1 run

**Status:** AVAILABLE | **Scope:** App Execution  
**Availability:** Built-in

**Purpose:**  
Launch and drive this project's app to see a change working. First looks for a project skill that already covers launching the app; otherwise falls back to built-in patterns per project type (CLI, server, TUI, Electron, browser-driven, library).

**Parameters:**
- None (auto-detection of app type and launch command)

**Typical Usage:**
```bash
/run
```

**Supported Project Types:**
1. **CLI** — Node, Python, Rust CLI tools; launches with entry script
2. **Server** — Node/Python/Rust backends; listens on port, health check
3. **TUI** — Terminal user interfaces; launches and captures output
4. **Electron** — Desktop app; launches with window/DevTools
5. **Browser-driven** — Web app; opens in browser or dev server
6. **Library** — Test harness or documentation site

**Workflow:**

1. **Project Type Detection**
   - Examines `package.json`, `Cargo.toml`, `.claude/settings.json`
   - Looks for project-specific skill (e.g., `run-webpack-app`)

2. **Launch Command**
   - Executes detected start command (npm start, cargo run, python app.py, etc.)
   - Waits for app readiness (port binding, console output, file watch ready)

3. **Observation**
   - Captures console output, opened windows, or browser navigation
   - Reports app state: `RUNNING`, `READY`, `FAILED`

4. **Next Steps**
   - On success: reports launch completion; ready for manual testing
   - On failure: suggests diagnostic steps (check logs, port conflicts, dependencies)

**Output:**
- App running with observable state
- Console/network output visible
- Status: `RUNNING`, `READY`, `BLOCKED`, or `FAILED`
- Next action suggestions

**Related Files:**
- Package/build manifest (package.json, Cargo.toml, setup.py)
- `.claude/settings.json` — May contain launch config
- App entry point (main.js, src/main.rs, etc.)

**Integration Points:**
- **Precedes** `verify` (run starts app; verify validates behavior)
- **Complements** `/loop` (run with `--loop` to restart on changes)
- **Works with** project-specific skills

**Difference from `verify`:**
- `run` — Launch the app, observe it starts
- `verify` — Launch app AND validate behavior, generate receipt

**See Also:**
- `/verify` — Validate app behavior after run
- `/loop` — Restart app on interval or event

---

#### 3.2 loop

**Status:** AVAILABLE | **Scope:** Recurring Task Automation  
**Availability:** Built-in

**Purpose:**  
Run a prompt, skill, or command on a recurring interval. Set up polling for status, recurring tests, or continuous monitoring. Defaults to 10-minute interval; specify custom intervals with syntax `<number><unit>`.

**Parameters:**
- `interval` (string, optional) — Interval in format `<number><unit>` (e.g., "5m", "30s", "1h"); default is "10m"
- `command` (string) — The skill or command to run repeatedly

**Interval Syntax:**
```
5m     → every 5 minutes
30s    → every 30 seconds
1h     → every 1 hour
2d     → every 2 days
```

**Typical Usage:**
```bash
/loop /verify
/loop 5m /verify
/loop 30s "cargo test"
/loop 10m /code-review
/loop 1h "/check the deploy"
```

**Trigger Patterns:**
- Polling for deployment status
- Recurring test runs
- Continuous monitoring of a service
- Periodic linter checks
- Status check loops before release

**Workflow:**

1. **Initialization**
   - Parses interval and command
   - Validates command syntax
   - Sets up recurring timer

2. **Execution**
   - Runs command at specified interval
   - Captures output and status
   - Logs results with timestamp

3. **Monitoring**
   - Continues until user cancels (Ctrl+C) or condition is met
   - Reports each iteration result
   - Suggests exit conditions (e.g., "stop when all tests pass")

**Output:**
- Recurring task running with status after each iteration
- Timestamps and result summary
- Option to stop or adjust interval

**Related Files:**
- None (metadata stored in session state)

**Integration Points:**
- **Works with** any skill or bash command
- **Complements** `verify`, `code-review`, `run`
- **No blocking** (runs in background; other work continues)

**Example Workflows:**

**Workflow A: Poll for deploy completion**
```bash
/loop 5m "/verify deploy-status"
# Runs every 5 minutes until you Ctrl+C
```

**Workflow B: Test-driven development**
```bash
/loop 10s "cargo test --lib"
# Quick feedback loop: run tests every 10 seconds
```

**Workflow C: Babysit pull requests**
```bash
/loop 1m /review
# Check PR status every minute
```

**See Also:**
- `/verify` — One-time verification
- `/run` — One-time app launch
- `TaskStop` — Cancel a loop (built-in command)

---

### Category 4: Research & Learning

Skills for deep research, API reference, and exploratory analysis.

#### 4.1 deep-research

**Status:** AVAILABLE | **Scope:** Fact-Checked Research  
**Availability:** Built-in (requires `deep-research` skill to be available)

**Purpose:**  
Conduct multi-source, adversarially verified research. Fan out web searches, fetch sources, verify claims, and synthesize a cited report. Ideal for deep technical questions requiring factual accuracy and source traceability.

**Parameters:**
- `args` (string) — Research question (auto-refined if underspecified)

**Typical Usage:**
```bash
/deep-research "Is OAuth 2.0 still secure for 2026?"
/deep-research "What are breaking changes in Rust 1.80?"
```

**Trigger Patterns:**
- When the question is specific enough to research directly
- Need multi-source fact-checking
- Source traceability is critical
- Claim verification required
- Technical accuracy is load-bearing

**Pre-invocation Check:**
If the question is underspecified (e.g., "what database to choose" without context), the skill will ask 2-3 clarifying questions:
- Budget/scale constraints?
- Geographic region?
- Use case (OLTP, analytics, etc.)?

**Workflow:**

1. **Question Clarification**
   - If underspecified: ask 2-3 clarifying questions
   - Refine scope and context

2. **Parallel Web Search**
   - Fan out multiple search queries
   - Gather sources from blogs, docs, official resources, discussions

3. **Source Fetching**
   - Retrieve full content from top sources
   - Extract key claims and evidence

4. **Adversarial Verification**
   - Challenge claims against contradictory sources
   - Identify consensus vs. outlier claims
   - Note confidence levels

5. **Synthesis**
   - Generate comprehensive report
   - Include citations with source URLs
   - Structure: problem → evidence → consensus → caveats

**Output:**
- Cited research report with source URLs
- Claim confidence levels: `STRONG_CONSENSUS`, `SUPPORTED`, `CONTESTED`, `UNKNOWN`
- Evidence summary for each major claim
- Caveats and open questions
- Recommendations if applicable

**Related Files:**
- None (results typically output to console)

**Integration Points:**
- Complements other research skills
- Feeds into `/code-review` decisions (e.g., "research best practice X before coding")
- Often precedes architecture decisions

**Example Workflow:**
```bash
# Research a dependency concern
/deep-research "Is the Log4j vulnerability patched in 2.17.0?"
# Output: Cited report, evidence, consensus status
```

**See Also:**
- `/claude-api` — API reference (static; use for known questions)
- Built-in web search — Quick, unsourced search

---

#### 4.2 claude-api

**Status:** AVAILABLE | **Scope:** Claude API Reference  
**Availability:** Built-in

**Purpose:**  
Reference for the Claude API / Anthropic SDK. Provides authoritative information on model IDs, pricing, parameters, streaming, tool use, MCP, agents, caching, token counting, and model migration. **Do not answer from memory; always check with this skill.**

**Parameters:**
- `args` (string, optional) — Specific API topic or question

**Typical Usage:**
```bash
/claude-api
/claude-api "What are current model IDs?"
/claude-api "How does prompt caching work?"
/claude-api "Token counting with tools"
```

**Trigger Patterns:**
- **When prompted mentions** Claude, Anthropic, Fable, Opus, Sonnet, Haiku, `anthropic`, `@anthropic-ai`, `claude-*`, `us.anthropic.*`
- **When asking about** LLMs, pricing, model choice, limits, caching, token counts
- **When task is LLM-shaped:**
  - Agent/MCP/tool-definition design
  - Multi-agent systems
  - RAG or knowledge retrieval
  - LLM judging or classification
  - Text generation, summarization, extraction, rewriting, conversation

**Skip Rules:**
- **SKIP** if another provider is named (OpenAI/GPT, Gemini, Llama, Mistral, Cohere, Ollama)
- **SKIP** if `grep -rE 'openai|langchain_openai|google.generativeai|genai|mistralai|cohere|ollama'` matches in project

**Reference Categories:**

| Category | Topics |
|----------|--------|
| **Models** | Current model IDs, deprecation schedule, capability matrix |
| **Pricing** | Token costs, per-model pricing, batch API discounts |
| **Parameters** | Temperature, top_k, top_p, max_tokens, system prompts |
| **Streaming** | Server-sent events (SSE), chunking, event types |
| **Tool Use** | Tool definitions, tool_choice, parallel tools, result handling |
| **MCP** | Model Context Protocol integration, server/client patterns |
| **Agents** | Agent framework, tool composition, loop control |
| **Caching** | Prompt caching, cache headers, cost reduction, best practices |
| **Token Counting** | Token counting API, counting with tools, edge cases |
| **Model Migration** | Upgrading to newer models, deprecation timeline |

**Output:**
- Authoritative reference information
- Current as of knowledge cutoff (Feb 2025)
- Links to official documentation
- Code examples if applicable

**Related Files:**
- None (reference skill; does not modify code)

**Integration Points:**
- **Triggered by** system reminder when Claude/Anthropic mentioned in prompt
- **Feeds into** LLM-related design decisions
- **No blocking** (informational only)

**Example Workflow:**
```bash
# User asks: "Can I use Claude Opus for this task?"
# System reminder triggers: `/claude-api`
# Result: Model capabilities, pricing, and recommendation
```

**See Also:**
- `/deep-research` — Multi-source research (for questions not in API docs)
- Built-in knowledge — Claude's training (use only if `/claude-api` is unavailable)

---

### Category 5: Project-Specific & Specialized

Skills for initialization, reviews, and project-specific workflows.

#### 5.1 init

**Status:** AVAILABLE | **Scope:** Project Documentation  
**Availability:** Built-in

**Purpose:**  
Initialize a new `CLAUDE.md` file with codebase documentation. Establishes a project constitution, coding standards, and architectural guidelines for Claude Code sessions.

**Parameters:**
- None (interactive generation based on project introspection)

**Typical Usage:**
```bash
/init
```

**Generated Sections:**

| Section | Content |
|---------|---------|
| **What this is** | Project name, purpose, tech stack summary |
| **Code layout conventions** | File structure, naming patterns, module organization |
| **Commands** | Build, test, and development commands |
| **Architecture** | Layer/crate diagram, component relationships |
| **External dependencies** | Required sibling repos, patch dependencies |
| **Validation gates** | Linting, formatting, testing requirements |
| **Coding standards** | Language-specific conventions, anti-patterns |
| **Configuration** | Hook setup, CI/CD integration points |

**Output:**
- `CLAUDE.md` file created in project root (replaces existing if present)
- Status: `ADMITTED` (documentation generated) or `CANDIDATE` (manual review needed)
- Next steps: review and customize CLAUDE.md

**Related Files:**
- `CLAUDE.md` (created/overwritten)
- `Cargo.toml`, `package.json`, `.cargo/config.toml` (read for context)
- `Justfile`, `Makefile`, `scripts/` (read for commands)

**Integration Points:**
- **Precedes** other skills (establishes baseline context)
- **Works with** `update-config` (CLAUDE.md references hook setup)
- **Referenced by** all other skills during context loading

**Example Workflow:**
```bash
# Initialize documentation for new project
/init

# Review and customize
# (editor opens CLAUDE.md)

# Commit the result
git add CLAUDE.md
git commit -m "docs: initialize CLAUDE.md"
```

**See Also:**
- `/update-config` — Configure hooks referenced in CLAUDE.md
- `CLAUDE.md` format reference — In this registry (Section: "CLAUDE.md Structure")

---

#### 5.2 review

**Status:** AVAILABLE | **Scope:** Pull Request Review  
**Availability:** Built-in

**Purpose:**  
Review a pull request comprehensively. Analyzes changes, checks correctness, assesses completeness, and generates a review summary. Typically used to audit PR readiness before merge.

**Parameters:**
- `--approve` (flag, optional) — Approve PR if review is clear
- `--request-changes` (flag, optional) — Request changes from PR author
- `--comment` (flag, optional) — Post review as PR comment

**Typical Usage:**
```bash
/review
/review --comment
/review --approve
/review --request-changes
```

**Review Scope:**

1. **Completeness** — All required files changed? Missing documentation? Migration steps?
2. **Correctness** — Logic errors, edge cases, null checks?
3. **Consistency** — Matches project style? Follows CLAUDE.md standards?
4. **Testing** — Tests added? Existing tests still pass? Coverage adequate?
5. **Documentation** — Commit messages clear? CHANGELOG updated? API docs?
6. **Performance** — Regressions? Inefficient patterns? Memory leaks?

**Output:**
- PR review summary
- Findings categorized by category (completeness, correctness, etc.)
- Severity for each finding: `BLOCKING`, `SHOULD_FIX`, `NICE_TO_HAVE`
- If `--approve`: approval posted
- If `--request-changes`: change request with feedback
- If `--comment`: inline comments for major findings

**Related Files:**
- Active PR metadata (auto-detected from git)
- PR description and title
- Changed files diff
- Base branch state

**Integration Points:**
- **Follows** `verify` (verify PR works before reviewing)
- **Precedes** PR merge
- **Works with** `code-review` (detailed code review) and `security-review` (security audit)

**Example Workflow:**
```bash
# 1. Verify the PR works
/verify

# 2. Review comprehensively
/review --comment

# 3. Approve if clear
/review --approve
```

**See Also:**
- `/code-review` — Detailed code quality review (not PR-specific)
- `/security-review` — Security-focused review
- `/verify` — Functional validation

---

#### 5.3 fewer-permission-prompts

**Status:** AVAILABLE | **Scope:** Permission Management  
**Availability:** Built-in

**Purpose:**  
Scan transcripts for common read-only Bash and MCP tool calls, then add a prioritized allowlist to `.claude/settings.json` to reduce permission prompts during sessions.

**Parameters:**
- None (automatic analysis of transcript)

**Typical Usage:**
```bash
/fewer-permission-prompts
```

**Workflow:**

1. **Transcript Analysis**
   - Scans current session transcript
   - Identifies repeated read-only tool calls: Bash (git status, ls, cat), MCP calls (GitHub, etc.)

2. **Tool Categorization**
   - Separates read-only from write/dangerous calls
   - Prioritizes by frequency

3. **Allowlist Generation**
   - Creates prioritized allow-list
   - Adds to `.claude/settings.json` under `permissions` section

4. **Validation**
   - Checks for security conflicts (e.g., allow git but not git push)
   - Reports changes to user

**Output:**
- Updated `.claude/settings.json` with new allowlist
- Summary: X new tools whitelisted, Y prompts reduced
- Status: `ADMITTED` (allowlist safe), `CANDIDATE` (manual review recommended)

**Example Allowlist Output:**
```json
{
  "permissions": {
    "bash": ["git status", "git log", "ls", "find"],
    "read": ["*"],
    "mcp": ["github:search_code", "github:list_issues"]
  }
}
```

**Related Files:**
- `.claude/settings.json` (modified)
- Session transcript (analyzed, not modified)

**Integration Points:**
- **Complements** `update-config` (both modify settings)
- **Reduces friction** for other skills (fewer permission prompts = faster iteration)
- **Security review** suggested after generation

**See Also:**
- `/update-config` — Manual permission configuration
- `/security-review` — Validate permission allowlist is safe

---

## Quick Reference Index

### Skills by Trigger Pattern

| Trigger | Skill | Status |
|---------|-------|--------|
| "run app", "start", "see it working" | `/run` | AVAILABLE |
| "verify works", "test manually", "confirm fix" | `/verify` | AVAILABLE |
| "review code", "check for bugs", "PR review" | `/code-review` | AVAILABLE |
| "clean up code", "simplify", "refactor" | `/simplify` | AVAILABLE |
| "security", "vulnerability", "audit" | `/security-review` | AVAILABLE |
| "research", "fact check", "sources" | `/deep-research` | AVAILABLE |
| "API docs", "pricing", "model IDs" | `/claude-api` | AVAILABLE |
| "setup hooks", "web session", "initialize" | `/session-start-hook` | AVAILABLE |
| "settings", "permissions", "env vars", "hooks" | `/update-config` | AVAILABLE |
| "keybindings", "shortcuts", "rebind" | `/keybindings-help` | AVAILABLE |
| "poll status", "recurring", "interval" | `/loop` | AVAILABLE |
| "initialize docs", "CLAUDE.md", "setup" | `/init` | AVAILABLE |
| "review PR", "pull request", "audit" | `/review` | AVAILABLE |
| "reduce prompts", "allowlist", "permissions" | `/fewer-permission-prompts` | AVAILABLE |

### Skills by Execution Context

| Context | Skills |
|---------|--------|
| **Before coding** | `/init`, `/session-start-hook`, `/update-config` |
| **During development** | `/run`, `/verify`, `/loop` |
| **Before committing** | `/code-review`, `/simplify`, `/security-review` |
| **Before merging** | `/review`, `/verify`, `/security-review` |
| **For research** | `/deep-research`, `/claude-api` |
| **For configuration** | `/update-config`, `/keybindings-help`, `/fewer-permission-prompts` |

### Skills by Dependency Chain

```
init
  ↓
session-start-hook
  ↓
update-config (permissions, env vars, hooks)
  ↓
run
  ↓
verify
  ↓
code-review ──→ simplify
  ↓                  ↓
security-review ←───┘
  ↓
review (PR-level)
  ↓
(merge)
```

---

## Integration Patterns

### Pattern 1: Development Cycle

**Workflow for feature development:**

```bash
# 1. Initialize project docs
/init

# 2. Setup environment
/session-start-hook
/update-config "set NODE_ENV=development"

# 3. Run the app
/run

# 4. Develop feature (editor work)

# 5. Verify it works
/verify

# 6. Review code quality
/code-review --fix

# 7. Simplify if needed
/simplify

# 8. Security check
/security-review

# 9. Commit (manual git work)

# 10. Create PR and wait for CI
```

### Pattern 2: Code Review Process

**Workflow for reviewing and merging a PR:**

```bash
# 1. Verify PR functionality
/verify

# 2. Deep code review
/code-review --comment

# 3. Security audit
/security-review

# 4. Comprehensive PR review
/review --comment

# 5. If all clear: approve
/review --approve

# 6. (Merge via GitHub)
```

### Pattern 3: Continuous Monitoring

**Workflow for ongoing deployment or testing:**

```bash
# Poll for deploy status every 5 minutes
/loop 5m /verify

# Ctrl+C when done, or:
# (automatically stops on success condition)
```

### Pattern 4: Configuration & Setup

**Workflow for new project or onboarding:**

```bash
# 1. Initialize docs
/init

# 2. Add permissions as needed
/update-config "allow npm commands"
/update-config "allow git push"

# 3. Reduce permission prompts
/fewer-permission-prompts

# 4. Customize keybindings (optional)
/keybindings-help "add a chord shortcut"

# 5. Setup web session hooks
/session-start-hook

# 6. Verify everything works
/verify
```

### Pattern 5: Research-Driven Development

**Workflow for feature requiring research:**

```bash
# 1. Research a technology
/deep-research "Is library X still maintained?"

# 2. Check API details
/claude-api "What are the limits for tool Y?"

# 3. Develop based on findings

# 4. Test thoroughly
/verify

# 5. Review and ship
/code-review /security-review /review --approve
```

---

## Best Practices

### Do's

✓ **Do invoke skills by name** — Use `/skillname`, not "run the skill" or "execute skill"  
✓ **Do provide context** — When invoking, include relevant args (e.g., `/code-review --comment`)  
✓ **Do verify before reviewing** — Always run `/verify` before `/code-review`  
✓ **Do follow the chain** — Use skills in dependency order (see Integration Patterns)  
✓ **Do read skill output** — Each skill generates a receipt or report; review the findings  
✓ **Do use `--fix` carefully** — Auto-fixes apply commits; review before pushing  
✓ **Do research before deciding** — Use `/deep-research` for uncertain technical questions  

### Don'ts

✗ **Don't skip verification** — Always run `/verify` before `code-review`, `security-review`, or `review`  
✗ **Don't amend commits from skill output** — Skills create new commits; never use `git commit --amend` afterward  
✗ **Don't ignore security findings** — If `/security-review` returns `REFUSED`, fix before proceeding  
✗ **Don't use `run` for validation** — Use `/verify` for behavior validation; `/run` just launches the app  
✗ **Don't stack --fix flags** — Run one skill at a time; don't do `/code-review --fix /simplify --fix` simultaneously  
✗ **Don't assume victory** — Bounded language only: `ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`  

### Performance Tips

- **Fast feedback:** Use `/verify` with `--timeout 30s` for quick iteration
- **Batch reviews:** Combine `code-review`, `simplify`, and `security-review` into a workflow
- **Reduce prompts:** Run `/fewer-permission-prompts` once per session to minimize permission dialogs
- **Avoid `loop` for one-time checks:** Use individual skill invocations; reserve `/loop` for polling

---

## Skill Lifecycle

### Skill Maturity Levels

**AVAILABLE** — Fully functional, documented, ready for production use  
**CANDIDATE** — Experimental or undergoing pilot; available but may change  
**BLOCKED** — Unavailable; missing dependencies or prerequisites  
**PARTIAL** — Some features work; others blocked or deprecated  
**DEPRECATED** — No longer recommended; use alternative skill instead  

### Skill Versioning

Skills version independently. Consult the registry for current version and status:

```
/skill-name
Status: AVAILABLE | Version: 1.2.3 | Last Updated: 2026-06-14
```

### Skill Dependencies

Some skills depend on others:

| Skill | Depends On |
|-------|-----------|
| `code-review` | None (but typically follows `verify`) |
| `simplify` | None (but typically follows `code-review`) |
| `security-review` | None (but typically follows `code-review`) |
| `review` | None (but typically follows `verify`) |
| `session-start-hook` | `update-config` (for hook registration) |
| `fewer-permission-prompts` | `update-config` (to apply results) |
| `loop` | Target skill/command (must be valid) |

### Extending the Registry

To add a new skill:

1. Create a section in the appropriate category (use existing categories; create new if needed)
2. Follow the template: Name, Status, Scope, Purpose, Parameters, Usage, Trigger Patterns, Output, Related Files, Integration Points, See Also
3. Update the Quick Reference Index
4. Update the dependency chain diagram if applicable
5. Submit PR with updated SKILLS_REGISTRY.md

---

## Appendix: File Reference

### CLAUDE.md Structure

The `CLAUDE.md` file (generated by `/init`) contains project constitution:

```markdown
# CLAUDE.md

## What this is
[Project name, purpose, tech stack]

## Versioning
[CalVer or SemVer; version-specific laws]

## Sibling repo dependencies
[Path dependencies, patch dependencies]

## Commands
[Build, test, development commands]

## Workspace architecture
[Layer/crate model, mapping to code]

## Code layout conventions
[File structure, module naming, size limits]

## External consumers
[Guidelines for downstream projects]

## [Custom sections as needed]
```

### Settings Files

**`.claude/settings.json`** — Global Claude Code configuration
- Permissions (bash, git, npm, mcp, etc.)
- Environment variables
- Hooks (PreToolUse, PostToolUse, etc.)
- Behavioral automation

**`.claude/settings.local.json`** — User-local overrides (git-ignored)
- User-specific preferences
- Session-scoped settings
- Temporary overrides

**`~/.claude/keybindings.json`** — User's keybinding configuration
- Key remapping (Ctrl+X → Cmd+Z)
- Chord bindings (Ctrl+K Ctrl+R → review)
- Command palette shortcuts

---

## Registry Metadata

| Field | Value |
|-------|-------|
| **Registry Version** | 26.6.9 (CalVer: YY.M.D) |
| **Generated** | 2026-06-14 |
| **Authority** | Claude Code Skills Registry |
| **Format** | Markdown |
| **Update Frequency** | Monthly or as skills change |
| **Status** | ADMITTED |

---

## See Also

- [CLAUDE.md](CLAUDE.md) — Project constitution and laws
- [AGENTS.md](AGENTS.md) — Agent architecture and enforcement
- [FEATURES.md](FEATURES.md) — LSP 3.18 feature coverage
- [TEST_INFRA.md](TEST_INFRA.md) — Testing infrastructure and patterns

---

**End of Skills Registry**
