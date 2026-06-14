# Claude Code Tool System Documentation Index

Complete reference documentation for using Claude Code tools effectively.

---

## Overview

Claude Code provides a powerful set of tools for interacting with codebases:

| Tool | Purpose | Speed | Use Case |
|------|---------|-------|----------|
| **Read** | View file contents | Fast | Understanding code, reviewing changes |
| **Write** | Create/overwrite files | Medium | New files, complete rewrites |
| **Edit** | Targeted replacements | Fast | Bug fixes, refactoring, modifications |
| **Glob** | Find files by pattern | Fast | File discovery, inventory |
| **Grep** | Search file contents | Medium | Code search, pattern matching |
| **Bash** | Execute shell commands | Variable | Build, test, git, utilities |
| **Agent** | Complex multi-step tasks | Slow | Architecture, deep analysis, reviews |
| **Skill** | Domain-specific operations | Variable | Code review, app verification, config |

---

## Documentation Structure

### 1. **CLAUDE_CODE_TOOL_SYSTEM.md** — Main Reference
**Comprehensive tool catalog with:**
- Complete parameter reference for each tool
- Return formats and file type support
- Permissions model and error handling
- Classification taxonomy (by domain, scope, speed, permissions)
- Tool dependencies and sequencing rules
- Best practices and quick reference

**Read this when**: You need definitive information about a tool, understanding parameters, or how permissions work.

**Key sections**:
- Tool Catalog (Read, Write, Edit, Glob, Grep, Bash, Agent, Skill, ToolSearch)
- Classification Taxonomy
- Decision Matrix by Use Case
- Tool Composition Patterns (8 core patterns)
- Parallel Execution Guidelines
- Tool Dependencies & Sequencing
- Permission Model
- Error Handling & Recovery
- Checklist: Before Each Tool Call

---

### 2. **TOOL_COMPOSITION_PATTERNS.md** — Practical Workflows
**Real-world patterns for combining tools:**
- 7 core composition patterns (Discover→Inspect→Modify, Parallel Search, Sequential Chains, etc.)
- Decision matrices for specific tasks
- 10 common anti-patterns and how to avoid them
- Parallel execution strategies
- Sequential workflows for refactoring, bug fixes, features
- Error recovery patterns
- Performance optimization techniques

**Read this when**: Planning your approach to a task, learning how to combine tools effectively, understanding anti-patterns.

**Key sections**:
- Core Composition Patterns (7 complete workflows)
- Decision Matrices by Task (find symbols, search files, modify, build, review, etc.)
- Anti-Patterns (sequential when should be parallel, no context before edit, etc.)
- Parallel Execution Strategies (discovery, metadata, multi-pattern, mixed)
- Sequential Workflows (refactor with safety, bug fix, feature implementation)
- Error Recovery Patterns (edit fails, bash fails, grep empty)
- Performance Optimization (token efficiency, parallelization, pattern specificity, caching)

---

### 3. **TOOL_ADVANCED_TECHNIQUES.md** — Deep Dives
**Advanced techniques, edge cases, and optimization:**
- Read tool: Precise section reading, progressive strategies, multi-format handling
- Edit tool: Context-rich matching, multi-line edits, atomic vs batch, special characters
- Grep power searching: Multi-pattern, context windows, metrics, anchored regex, multiline, filter chains
- Bash advanced: Complex chains, absolute paths, process substitution, timeout management, git workflows
- Agent optimization: Surgical briefing, progressive tasks, verification, context management
- Tool interaction gotchas: 5 common edge cases with solutions
- Performance profiling: Token costs, wall-clock time, tool selection by speed
- Debugging: 4 common debug scenarios with diagnosis and recovery

**Read this when**: Optimizing performance, troubleshooting tool issues, learning advanced techniques, handling edge cases.

**Key sections**:
- Read Tool Advanced Usage (offset, progressive reading, multi-format, empty files)
- Edit Tool Precision (indentation, multi-line, special chars, atomic vs batch)
- Grep Power Searching (multi-pattern, context, metrics, anchored regex, multiline)
- Bash Advanced (chains, absolute paths, timeout, git, output capture)
- Agent Optimization (briefing, progressive tasks, verification, context)
- Tool Interaction Gotchas (5 edge cases)
- Performance Profiling (token costs, wall-clock time, speed rankings)
- Debugging Tool Issues (edit fails, grep empty, bash errors, agent problems)

---

## Quick Decision Tree

**What's your goal?**

```
Find code?
├─ By filename → TOOL_SYSTEM: Glob section
├─ By content → TOOL_SYSTEM: Grep section
└─ Both → PATTERNS: Decision Matrix for Task 1

Understand code?
├─ Know path → TOOL_SYSTEM: Read section
└─ Don't know path → PATTERNS: Discover→Inspect→Modify pattern

Modify code?
├─ Small change → TOOL_SYSTEM: Edit section → Read first
├─ Full rewrite → TOOL_SYSTEM: Write section → Read if exists
└─ Multiple files → PATTERNS: Bulk Operations pattern

Run commands?
├─ Simple → TOOL_SYSTEM: Bash section
├─ Build/test/complex → PATTERNS: Sequential Workflows
└─ Long-running → ADVANCED: Timeout Management

Get expert opinion?
├─ Code review → TOOL_SYSTEM: Skill section
├─ Security review → TOOL_SYSTEM: Skill section
└─ Deep analysis → PATTERNS: Expert Consultation pattern

Optimize for speed?
└─ PATTERNS: Parallel Execution or ADVANCED: Performance Profiling

Hitting an error?
├─ Edit fails → ADVANCED: Debug 1
├─ Grep empty → ADVANCED: Debug 2
├─ Bash fails → ADVANCED: Debug 3
└─ Agent bad output → ADVANCED: Debug 4
```

---

## By Use Case

### Code Search & Discovery
| Task | Document | Section |
|------|----------|---------|
| Find a symbol | TOOL_SYSTEM | Decision Matrix |
| Search across files | TOOL_SYSTEM | Grep tool |
| Find files by pattern | TOOL_SYSTEM | Glob tool |
| Complex multi-pattern search | ADVANCED | Grep Power Searching |

### Code Modification
| Task | Document | Section |
|------|----------|---------|
| Make small edit | TOOL_SYSTEM | Edit tool |
| Full file rewrite | TOOL_SYSTEM | Write tool |
| Understand before editing | PATTERNS | Discover→Inspect→Modify |
| Refactor multiple files | PATTERNS | Bulk Operations |
| Safe refactoring | PATTERNS | Refactor Workflow |

### Building & Testing
| Task | Document | Section |
|------|----------|---------|
| Run single command | TOOL_SYSTEM | Bash tool |
| Run test suite | PATTERNS | Sequential Workflows |
| Build pipeline | ADVANCED | Bash Advanced Patterns |
| Long-running operation | TOOL_SYSTEM | run_in_background |

### Code Review & Analysis
| Task | Document | Section |
|------|----------|---------|
| Manual code review | PATTERNS | Code Review Task |
| Expert review | PATTERNS | Expert Consultation |
| Find tech debt | ADVANCED | Grep Power Searching |
| Gather metrics | ADVANCED | Grep Power Searching |

### Performance & Optimization
| Task | Document | Section |
|------|----------|---------|
| Reduce token usage | ADVANCED | Performance Profiling |
| Speed up operations | PATTERNS | Parallel Execution |
| Avoid timeouts | ADVANCED | Timeout Management |
| Token budgeting | ADVANCED | Token Efficiency |

---

## Common Workflows

### Workflow: Find and Fix a Bug
1. Read: PATTERNS, "Bug Fix Workflow"
2. Execute: Grep to locate → Read context → Edit → Bash test

### Workflow: Refactor Code
1. Read: PATTERNS, "Refactor with Safety Checks"
2. Execute: Baseline test → Find locations → Edit each → Full test

### Workflow: Implement Feature
1. Read: PATTERNS, "Feature Implementation Workflow"
2. Execute: Design review → Implement → Unit test → Integration test → Code review → Lint → Commit

### Workflow: Audit Codebase
1. Read: PATTERNS, "Parallel Search & Gather"
2. Execute: Parallel Glob + Grep + Bash to gather metrics

### Workflow: Review PR
1. Read: PATTERNS, "Code Review Task"
2. Execute: Bash (git diff) → Read changes → Grep risks → Agent review → Verify findings

---

## Tool Selection Quick Reference

### By Task Type

**Finding code**: Glob (filenames), Grep (content)
**Understanding code**: Read (direct view), Grep + Read (context)
**Modifying code**: Edit (small), Write (large), Bash (with sed)
**Running commands**: Bash (simple), Bash chain (complex)
**Complex tasks**: Agent (multi-step), Skill (specialized)
**Status/metadata**: Bash, Glob
**Analysis/metrics**: Grep (with count), Bash (piped)

### By Performance Requirement

**Fastest**: Edit, Glob, Read (small)
**Medium**: Grep (targeted), Bash (quick), Read (medium)
**Slow**: Read (large), Bash (complex), Agent, Write (large)

### By Scope

**Single file**: Read, Edit, Write
**Multiple files**: Glob, Grep, Bash (with find)
**System-level**: Bash, Agent
**Parallel**: Multiple tools in one message

---

## Permission Model Quick Summary

| Operation | Permission | Notes |
|-----------|-----------|-------|
| Read files | None | No permission prompt |
| Glob patterns | None | No permission prompt |
| Grep patterns | None | No permission prompt |
| Edit files | Prompt | Writing to file |
| Write files | Prompt | Creating/overwriting |
| Bash (readonly) | None | git status, ls, cat |
| Bash (mutations) | Prompt | Build, test, npm install |
| Force operations | Explicit request | git push --force |
| Skip hooks | Explicit request | --no-verify, --no-gpg-sign |

---

## Common Gotchas & Solutions

### Gotcha: Edit Fails with "old_string not found"
**Solution**: ADVANCED, Debug 1 or TOOL_SYSTEM, Edit section → Read first, include context

### Gotcha: Grep Returns Nothing
**Solution**: ADVANCED, Debug 2 or PATTERNS, Task 2 → Verify files exist, simplify pattern

### Gotcha: Bash Working Directory Resets
**Solution**: TOOL_SYSTEM, Bash section or ADVANCED, Absolute Paths → Use absolute paths always

### Gotcha: Indentation Mismatch in Edit
**Solution**: ADVANCED, Edit Precision or TOOL_SYSTEM, Edit section → Copy exact whitespace from Read

### Gotcha: Agent Output is Unreliable
**Solution**: PATTERNS, Expert Consultation or ADVANCED, Agent Optimization → Verify independently

### Gotcha: Slow Performance
**Solution**: ADVANCED, Performance Profiling or PATTERNS, Parallel Execution → Batch independent calls

---

## Learning Path

### Beginner
1. TOOL_SYSTEM: Tool Catalog (Read, Edit, Glob, Grep)
2. TOOL_SYSTEM: Tool Dependencies
3. PATTERNS: Pattern A (Discover→Inspect→Modify)

### Intermediate
1. PATTERNS: All patterns (A-G)
2. PATTERNS: Decision Matrices
3. TOOL_SYSTEM: Parallel Execution

### Advanced
1. ADVANCED: All advanced techniques sections
2. ADVANCED: Performance Profiling
3. ADVANCED: Debugging Tool Issues

### Expert
1. ADVANCED: All edge cases and gotchas
2. PATTERNS: Anti-patterns
3. Combine learnings for custom workflows

---

## Quick Reference Cards

### Read & Edit Sequence
```
1. Read(file_path) → Understand structure
2. Edit(old_string: context, new_string: replacement)
3. (Optional) Read(file_path) → Verify change
```

### Find & Modify Sequence
```
1. Glob(pattern) OR Grep(pattern) → Locate
2. Read(file_path) → Understand
3. Edit(...) → Modify
4. Bash(cargo test) → Verify
```

### Multi-File Refactor
```
1. Glob(pattern) → Find all files
2. FOR each file:
   - Read(file)
   - Edit(...)
3. Bash(cargo test) → Verify all
```

### Parallel Info Gathering
```
1. Single message with multiple calls:
   - Bash(git status)
   - Bash(git log)
   - Glob(pattern)
   - Grep(pattern)
2. Analyze results together
```

---

## Document Sizes & Scope

| Document | Lines | Sections | Use |
|----------|-------|----------|-----|
| TOOL_SYSTEM | ~900 | 20+ | Complete reference |
| PATTERNS | ~700 | 20+ | Practical workflows |
| ADVANCED | ~800 | 8+ | Deep techniques |
| INDEX | This file | Quick navigation |

---

## Related Resources

- **Claude Code CLI Help**: Type `/help` in Claude Code
- **API Reference**: Use `claude-api` skill
- **Settings**: Use `update-config` skill
- **Keyboard Shortcuts**: Use `keybindings-help` skill
- **Code Guide**: Use `claude-code-guide` agent for questions

---

## How to Use This Documentation

1. **Find your task** in the "By Use Case" section
2. **Go to recommended document**
3. **Read the relevant section**
4. **Check examples** for your specific scenario
5. **Reference gotchas** if things go wrong
6. **Use decision trees** to choose tools
7. **Follow workflows** for complex operations

---

## Feedback & Updates

This documentation represents Claude Code tool system as of February 2025. The system evolves; check `/help` for latest features.

For questions about Claude Code itself (features, settings, hooks), use the `claude-code-guide` agent.

For questions about these docs, review the specific tool sections in the main reference documents.

---

**Quick Start**: New to Claude Code tools? Start with TOOL_SYSTEM "Tool Catalog" section.

**Optimization**: Already familiar? See PATTERNS "Parallel Execution Strategies" or ADVANCED "Performance Profiling".

**Troubleshooting**: Something broken? See ADVANCED "Debugging Tool Issues" or PATTERNS "Error Recovery Patterns".
