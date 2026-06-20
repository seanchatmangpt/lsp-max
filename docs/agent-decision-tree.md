# Claude Code Agent Decision Tree & Quick Reference

**Purpose:** Fast decision-making guide for choosing the right agent for your task.

**Format:** Flowchart + quick tables. Use this document to quickly route to the right agent without reading the full taxonomy.

---

## Interactive Decision Tree

```
START: What's your task?
│
├─→ [Is it a feature/API question about Claude?]
│   ├─ YES → claude-code-guide
│   │        (Questions about Claude Code, Claude API, Agent SDK)
│   │
│   └─ NO → [Continue below]
│
├─→ [Do you need to find a file or symbol fast?]
│   ├─ YES (you know roughly where to look) → Explore
│   │                                          (Quick file location)
│   │
│   ├─ MAYBE (you don't know where to start) → general-purpose
│   │                                            (Multi-step research)
│   │
│   └─ NO → [Continue below]
│
├─→ [Is this ONLY a configuration change?]
│   ├─ YES (status line settings) → statusline-setup
│   │                                (Single config edit)
│   │
│   └─ NO → [Continue below]
│
├─→ [Do you need to design/plan before implementing?]
│   ├─ YES (architecture, strategy, scope) → Plan
│   │                                         (Design + trade-offs)
│   │
│   └─ NO → [Continue below]
│
└─→ [Everything else: implement, debug, refactor] → claude
    (Catch-all for code work)
```

---

## Quick Task-to-Agent Map

Scan left column for your task; use right column to find the agent.

| Task | Agent | Why |
|------|-------|-----|
| **Find a function definition** | Explore | Fast pattern search |
| **Find all usages of a symbol** | Explore + general-purpose | Explore finds files, general-purpose analyzes |
| **Understand how a component works** | general-purpose | Synthesizes info from many sources |
| **Root cause analysis (unknown start)** | general-purpose | Explores iteratively |
| **Design a refactor** | Plan | Strategic thinking; read-only |
| **Implement a feature** | claude | All tools, write capability |
| **Debug a test failure** | claude | Direct execution and diagnosis |
| **Refactor a large module** | Plan + claude | Plan first, then implement |
| **Answer "does Claude support X?"** | claude-code-guide | Feature knowledge |
| **Answer "how do I configure Y?"** | claude-code-guide | Tool knowledge |
| **Configure IDE settings** | statusline-setup | Settings specialist |
| **Do multiple independent tasks** | claude × N | Fan-out parallel execution |
| **Investigate codebase architecture** | general-purpose | Cross-file synthesis |
| **Verify that a change works** | claude | Can run tests and verify |

---

## Decision by Agent Type

### Explore: When to Use

**Use Explore if:**
- You know roughly where to look (e.g., "in src/" or "*.rs files")
- You want fast results (target time: <200ms)
- The task is read-only (no edits/writes needed)
- You want specific file paths and line numbers

**Do NOT use Explore if:**
- You don't know where to start (use general-purpose instead)
- You need to understand code semantics (use general-purpose instead)
- You need to edit files (use claude instead)
- You need architectural synthesis (use Plan instead)

**Breadth Parameter:**
- `"quick"` — one targeted search, <100ms, exact pattern
- `"medium"` — 3-5 search patterns, 300-500ms, moderate exploration
- `"very thorough"` — 10+ patterns, 1-2s, exhaustive search (warning: doesn't scale >100k files)

---

### general-purpose: When to Use

**Use general-purpose if:**
- You're researching something with unclear starting point
- You need to understand patterns across multiple files
- You want comprehensive findings with synthesis
- You're comfortable waiting 5-10s for results

**Do NOT use general-purpose if:**
- You just need a file path (use Explore instead)
- You need to implement something immediately (use claude instead)
- You need a design/strategy (use Plan instead)
- You need features knowledge (use claude-code-guide instead)

**Example Triggers:**
- "How does the ANDON gate work end-to-end?"
- "Find all places where diagnostics are emitted"
- "Explain the relationship between X and Y"

---

### Plan: When to Use

**Use Plan if:**
- You're designing a feature or refactoring
- You need to identify critical files
- You need to weigh trade-offs
- You want to document a strategy before implementing

**Do NOT use Plan if:**
- You just need to locate a file (use Explore instead)
- You need to implement immediately (use claude instead)
- The task is small/isolated (use claude directly instead)
- You need to edit files (Plan is read-only)

**Red Flags for Plan:**
- Changes affecting >5 files → use Plan
- Architectural implications → use Plan
- Multiple valid approaches → use Plan to compare
- Team handoff needed → use Plan to document

---

### claude-code-guide: When to Use

**Use claude-code-guide if:**
- Your question starts with "Can Claude..." or "Does Claude..."
- You're asking about Claude API, Claude Code IDE, or Agent SDK
- You need feature documentation or examples
- You want to understand capabilities/limitations

**Do NOT use claude-code-guide if:**
- You're asking about lsp-max architecture (use general-purpose)
- You're implementing a feature (use claude or Plan)
- You're debugging code (use claude)
- You're doing software engineering unrelated to Claude tools

---

### statusline-setup: When to Use

**Use statusline-setup if:**
- You're ONLY changing `.claude/settings.json` status line settings
- You don't need to edit other files
- You don't need to run code or tests

**Do NOT use statusline-setup if:**
- You're configuring multiple files (use claude instead)
- You need general settings changes (use /config command instead)
- You need to test the change (use claude instead)

---

### claude: When to Use

**Use claude if:**
- None of the above agents match your task
- You need to implement code
- You need to debug or run tests
- You need full tool access

**This is the catch-all for:**
- Feature implementation
- Bug fixes
- Refactoring (for large refactors, use Plan first)
- Testing and verification

---

## Decision by Common Scenario

### Scenario: "I want to understand how the system works"

1. **Is scope well-defined?**
   - YES → general-purpose (research the specific area)
   - NO → Explore (find key files first), then general-purpose (understand)

2. **Do you need architectural insight?**
   - YES → Plan (get strategic overview), then general-purpose (deep dive)
   - NO → general-purpose (sufficient)

---

### Scenario: "I need to implement a feature"

1. **Is scope clear?**
   - YES → claude (implement directly)
   - NO → general-purpose (research scope), then Plan (design), then claude (implement)

2. **Is it a big feature (>5 files)?**
   - YES → Plan (design first), then claude (implement)
   - NO → claude (direct implementation)

3. **Need to verify it works?**
   - YES → claude (has test capability)
   - NO → claude (still recommended)

---

### Scenario: "I need to fix a bug"

1. **Do you know the bug location?**
   - YES → Explore (find the code), then claude (fix it)
   - NO → general-purpose (root cause analysis), then claude (fix)

2. **Is the bug behavior obvious?**
   - YES → claude (direct fix)
   - NO → general-purpose (investigate), then claude (fix)

---

### Scenario: "I need to refactor code"

1. **Size of refactoring:**
   - Small (<5 files, low risk) → claude (direct refactoring)
   - Large (>5 files, architectural) → Plan (design), then claude (implement)

2. **Uncertain about approach?**
   - YES → general-purpose (research current state), then Plan (design), then claude (implement)
   - NO → Plan (confirm design), then claude (implement)

---

### Scenario: "I need to configure the IDE"

1. **What are you configuring?**
   - Status line only → statusline-setup
   - Other settings → use /config command
   - Complex configuration → claude

---

### Scenario: "I'm answering a question for someone"

1. **Is the question about Claude tools?**
   - YES → claude-code-guide (get official answer)
   - NO → proceed with code task

2. **Is the question about lsp-max?**
   - YES → general-purpose (research), then explain
   - NO → claude-code-guide (Claude questions)

---

## Agent Selection Checklist

Before spawning an agent, check:

- [ ] **Right agent chosen?** (Use decision tree above)
- [ ] **Prompt is specific?** (Not vague; includes constraints/examples)
- [ ] **Context is provided?** (Agent knows background and success criteria)
- [ ] **Output format is clear?** (Agent knows what answer looks like)
- [ ] **Status words will be bounded?** (No victory language)
- [ ] **Gate check included if needed?** (For Bash operations that must respect ANDON)

---

## Quick Reference: Tool Access by Agent

```
✓ = has access  |  ✗ = no access

                    Explore  general-purpose  Plan  claude  claude-code-guide  statusline-setup
─────────────────────────────────────────────────────────────────────────────────────────────
Read                  ✓          ✓            ✓      ✓          ✓                ✓
Glob                  ✓          ✓            ✓      ✓          ✓                ✗
Grep                  ✓          ✓            ✓      ✓          ✓                ✗
Bash                  ✓          ✓            ✓      ✓          ✗                ✗
Edit                  ✗          ✓            ✗      ✓          ✗                ✓
Write                 ✗          ✓            ✗      ✓          ✗                ✗
WebSearch/WebFetch    ✓          ✓            ✓      ✓          ✓                ✗
Agent (spawn)         ✗          ✓            ✗      ✓          ✗                ✗
```

**Key Constraints:**
- Explore, Plan, claude-code-guide: **read-only**
- statusline-setup: **minimal** (Edit only)
- Explore: **cannot spawn subagents**

---

## Common Mistakes & How to Avoid Them

### Mistake 1: Using claude for every task

**Wrong:**
```
Agent(description: "find files related to diagnostics",
      prompt: "search for diagnostic files")
```

**Better:**
```
Explore(description: "find diagnostic files",
        prompt: "search for src/**/*diagnostic*")
```

**Why:** Explore is 10× faster for file location.

---

### Mistake 2: Using Explore for understanding

**Wrong:**
```
Explore(breadth: "very thorough",
        prompt: "find all usages of ConformanceVector")
```

**Better:**
```
general-purpose(prompt: "research ConformanceVector: 
                 find definition, all usages, patterns of use,
                 and how it relates to ANDON gate")
```

**Why:** Explore returns file paths; general-purpose synthesizes understanding.

---

### Mistake 3: Not using Plan for large changes

**Wrong:**
```
claude(prompt: "refactor the ANDON gate implementation")
```

**Better:**
```
Plan(prompt: "design a refactoring strategy for the ANDON gate")
[REVIEW & APPROVE]
claude(prompt: "implement per the approved plan: ...")
```

**Why:** Big changes need design review before implementation.

---

### Mistake 4: Using multiple agents when one suffices

**Wrong:**
```
Explore(find "gate.rs")
Explore(find "gate_file.rs")
Explore(find "andon.rs")
(three separate agents, 600ms total)
```

**Better:**
```
Explore(breadth: "medium",
        prompt: "find all gate-related files: gate.rs, gate_file.rs, andon.rs")
(one agent, 200ms total)
```

**Why:** Combine searches into one agent call when possible.

---

### Mistake 5: Spawning subagents when the parent has all tools

**Wrong:**
```
Agent(prompt: "spawn another agent to refactor this")
```

**Better:**
```
claude(prompt: "refactor this module")
```

**Why:** Avoid unnecessary nesting. Use subagents only for specialization.

---

### Mistake 6: No status words in agent results

**Wrong:**
```
"The ANDON gate works by checking a file written by the compositor."
```

**Better:**
```
"ADMITTED: The ANDON gate works by checking a file written by the compositor.
 Verified in crates/lsp-max-compositor/src/gate_file.rs:42."
```

**Why:** Bounded status clarifies confidence and evidence.

---

## When Agents Are Overkill

Sometimes just using tools directly is faster:

- **Single file operation?** → Use Read/Edit/Write directly (no agent needed)
- **One grep search?** → Use Grep tool directly (no agent needed)
- **One command?** → Use Bash tool directly (no agent needed)
- **Agent setup overhead > task time?** → Skip agent, use tool directly

**When to use agents:**
- Task requires multiple steps or decisions
- Task benefits from specialized behavior (Plan vs claude)
- Task needs synthesis or analysis
- Result will be used by multiple downstream tasks

---

## Parallel Execution: Fan-Out Pattern

Use when you have 2+ **independent** tasks:

```
Agent(description: "task A", prompt: "...")
Agent(description: "task B", prompt: "...")
Agent(description: "task C", prompt: "...")
```

All three execute in parallel. Results returned in order.

**Ideal parallelism scenarios:**
- 3-5 independent Explore searches
- Multiple claude agents implementing separate modules
- Parallel data collection (research tasks)

**Avoid parallelism if:**
- Task B needs Task A result (use sequential instead)
- >10 agents (token budget, context limits)
- Interdependent work

---

## Status: OPEN

This decision tree is CANDIDATE and will be refined as agent patterns emerge in real usage.

Contributions welcome: file an issue describing a decision gap or scenario this tree doesn't cover.
