# Claude Code Agents: Complete Documentation Index

**Status:** OPEN  
**Last Updated:** 2026-06-14  
**Scope:** Comprehensive documentation suite for Claude Code agent types, subagents, taxonomies, and workflows.

---

## Quick Navigation

**In a Hurry?** Start here:
1. **[Agent Decision Tree](./agent-decision-tree.md)** — 2 min read, fast task-to-agent routing
2. **[Taxonomy § Agent Specifications](./claude-code-agents-taxonomy.md#detailed-agent-specifications)** — 5 min read per agent type
3. **[Integration Guide § Multi-Agent Workflow Patterns](./agent-integration-guide.md#multi-agent-workflow-patterns)** — Real-world examples

---

## Documentation Suite

### 1. **claude-code-agents-taxonomy.md** (41 KB, 1452 lines)

**Purpose:** Complete specification of all Claude Code agent types.

**Contents:**
- Executive summary
- Agent type taxonomy (Layer 1-4 breakdown)
- Detailed agent specifications (6 agents × 10 fields each)
- Decision framework flowchart and matrix
- Tool availability matrix
- Common use patterns
- Known limitations and gaps
- Best practices (10 key principles)

**Read This When:**
- You need complete reference documentation
- You're designing a new agent workflow
- You need to understand an agent's full capabilities
- You want to learn best practices

**Key Sections:**
| Section | Purpose | Read Time |
|---------|---------|-----------|
| [Executive Summary](./claude-code-agents-taxonomy.md#executive-summary) | One-page overview of all 6 agents | 2 min |
| [Agent Type Taxonomy](./claude-code-agents-taxonomy.md#agent-type-taxonomy) | Organized breakdown by layer | 10 min |
| [Detailed Agent Specifications](./claude-code-agents-taxonomy.md#detailed-agent-specifications) | Full spec for each agent (preconditions, tools, integration) | 30 min |
| [Decision Framework](./claude-code-agents-taxonomy.md#decision-framework) | Flowchart + matrix to choose right agent | 5 min |
| [Tool Availability Matrix](./claude-code-agents-taxonomy.md#tool-availability-matrix) | Which tools each agent can access | 2 min |
| [Best Practices](./claude-code-agents-taxonomy.md#best-practices) | 10 key principles for agent work | 5 min |

---

### 2. **agent-decision-tree.md** (14 KB, 450 lines)

**Purpose:** Fast decision-making guide; choose the right agent in seconds.

**Contents:**
- Interactive flowchart (START → agent selection)
- Quick task-to-agent map (30+ common tasks)
- Decision by agent type (when to use, when NOT to use)
- Decision by scenario (6 common scenarios)
- Selection checklist
- Quick reference card (one-liner matrix)
- Common mistakes & how to avoid them

**Read This When:**
- You need to decide which agent NOW
- You're learning the agent taxonomy
- You want to avoid common mistakes
- You need a one-page reference card

**Key Sections:**
| Section | Purpose | Read Time |
|---------|---------|-----------|
| [Interactive Decision Tree](./agent-decision-tree.md#interactive-decision-tree) | Flowchart-style routing | 1 min |
| [Quick Task-to-Agent Map](./agent-decision-tree.md#quick-task-to-agent-map) | 30+ tasks, agent assignments | 3 min |
| [Decision by Agent Type](./agent-decision-tree.md#decision-by-agent-type) | When to use each agent | 5 min |
| [Quick Reference Card](./agent-decision-tree.md#quick-reference-one-liner-agent-selection) | One-page cheat sheet | 1 min |

---

### 3. **agent-integration-guide.md** (22 KB, 846 lines)

**Purpose:** Advanced patterns for multi-agent workflows, ANDON gate integration, and CI/CD.

**Contents:**
- Multi-agent workflow patterns (Research→Design→Implement, fan-out, feedback loops)
- ANDON gate integration (enforcement, gate-check preambles, workflows)
- Session boundary handling (isolation, context passing, handoff patterns)
- Large feature implementation checklist
- CI/CD integration (GitHub Actions example, local dev workflow)
- Error recovery & rollback procedures
- Debugging agent behavior (timeouts, incomplete results)
- Performance tuning (optimization strategies)

**Read This When:**
- You're designing a complex multi-agent workflow
- You need to handle ANDON gate enforcement
- You're integrating agents into CI/CD
- You need to debug agent behavior
- You're implementing a large feature with parallel work

**Key Sections:**
| Section | Purpose | Read Time |
|---------|---------|-----------|
| [Multi-Agent Workflow Patterns](./agent-integration-guide.md#multi-agent-workflow-patterns) | Research→Design→Implement, fan-out, feedback loops | 10 min |
| [ANDON Gate Integration](./agent-integration-guide.md#andon-gate-integration) | How to work with the gate; gate-check preambles | 8 min |
| [Session Boundary Handling](./agent-integration-guide.md#session-boundary-handling) | Context passing, handoff strategies | 8 min |
| [Large Feature Implementation](./agent-integration-guide.md#large-feature-implementation) | 7-phase feature delivery checklist | 5 min |
| [Error Recovery & Rollback](./agent-integration-guide.md#error-recovery--rollback) | Handle failures gracefully | 5 min |

---

## Document Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                   Agent Documentation Suite                      │
└─────────────────────────────────────────────────────────────────┘

┌──────────────────────┐
│ claude-code-agents-  │  Detailed Taxonomy
│ taxonomy.md (41 KB)  │  - What each agent is
│                      │  - Full specifications
│                      │  - Tool matrix
└──────────────────────┘
         ↓↑
         │ References from
         ↓↑
┌──────────────────────┐
│ agent-decision-      │  Quick Router
│ tree.md (14 KB)      │  - Which agent to use
│                      │  - Common mistakes
│                      │  - Quick reference
└──────────────────────┘
         ↓↑
         │ Feeds into
         ↓↑
┌──────────────────────┐
│ agent-integration-   │  Advanced Patterns
│ guide.md (22 KB)     │  - Multi-agent workflows
│                      │  - Gate integration
│                      │  - CI/CD setup
└──────────────────────┘

Read Order (by role):
- New User:          decision-tree → taxonomy (skim)
- Practitioner:      decision-tree → integration-guide
- Designer:          taxonomy → integration-guide
- Team Lead:         all three (start with decision-tree)
```

---

## Agent Types at a Glance

### 1. claude — Catch-All Implementation Agent
**Use for:** Implementing, debugging, refactoring, running tests  
**Tools:** All  
**Time:** ~5s startup + task time  
**Best for:** Direct code work

### 2. general-purpose — Multi-Step Research Agent
**Use for:** Complex research, root cause analysis, pattern discovery  
**Tools:** All  
**Time:** 2-10s depending on search scope  
**Best for:** Unknown starting points, synthesis across files

### 3. Explore — Fast Code Search Agent
**Use for:** File location, symbol lookup, quick searches  
**Tools:** Read-only (no Edit/Write)  
**Time:** 100-500ms depending on breadth  
**Best for:** "Where is X defined?"

### 4. Plan — Architecture & Design Agent
**Use for:** Implementation strategy, design, critical path analysis  
**Tools:** Read-only (no Edit/Write)  
**Time:** 5-15s depending on complexity  
**Best for:** Design before implementation

### 5. claude-code-guide — Claude Tools & Features Agent
**Use for:** Questions about Claude API, Claude Code, Agent SDK  
**Tools:** Read, WebSearch, WebFetch  
**Time:** 1-3s including potential web search  
**Best for:** "Can Claude...?" / "How do I configure...?"

### 6. statusline-setup — Configuration Agent
**Use for:** Configuring Claude Code status line settings  
**Tools:** Edit only  
**Time:** <500ms  
**Best for:** Single, isolated settings changes

---

## Workflow Examples

### Example 1: Understanding a Component
```
1. Explore:         Find definition file
   → Time: <200ms
   → Output: file path

2. Read:            Open and read file
   → Time: ~100ms
   → Output: file content

3. general-purpose: Analyze how it's used
   → Time: 5-10s
   → Output: comprehensive understanding

Result: Full understanding of component behavior
```

### Example 2: Implementing a Feature
```
1. general-purpose: Research current implementation
   → Time: 5-10s
   → Output: findings, affected files

2. Plan:            Design implementation strategy
   → Time: 5-10s
   → Output: step-by-step plan, trade-offs

3. claude:          Implement per plan
   → Time: task time
   → Output: implemented code

4. claude:          Run tests and verify
   → Time: 30s-5m
   → Output: test results, verification

Result: Feature implemented and verified
```

### Example 3: Parallel Module Development
```
1. Plan (×3):       Design module A, B, C in parallel
   → Time: ~10s (all three execute concurrently)
   → Output: 3 designs

2. claude (×3):     Implement modules A, B, C in parallel
   → Time: max(task_A, task_B, task_C)
   → Output: 3 implemented modules

3. claude:          Run integration tests
   → Time: 30s-5m
   → Output: integration test results

Result: Features implemented faster via parallelism
```

---

## Special Topics

### ANDON Gate Integration

The `Λ_CD^runtime` gate blocks shell-side actions when active. See:
- **Taxonomy:** [ANDON Gate — PreToolUse Hook](./claude-code-agents-taxonomy.md#andon-gate--pretooluse-hook-lambda_cdruntime)
- **Integration Guide:** [ANDON Gate Integration](./agent-integration-guide.md#andon-gate-integration)

**Key Point:** Include `lsp-max-cli gate check || exit 1` as first Bash call in agent prompts to respect parent session gate.

---

### Session Boundaries & Context Passing

Subagents run in isolated sessions with limited context. See:
- **Taxonomy:** [Session Boundary](./claude-code-agents-taxonomy.md#session-boundary)
- **Integration Guide:** [Session Boundary Handling](./agent-integration-guide.md#session-boundary-handling)

**Key Point:** Pass context explicitly in prompt or use SendMessage to resume existing agent.

---

### Known Limitations & Gaps

Document status: OPEN (not all agent capabilities fully specified).

**Key Gaps:**
- Subagent gate propagation: PreToolUse hooks do not cross session boundaries (OPEN)
- Agent context window limits: not formally documented (CANDIDATE)
- Performance benchmarks: heuristic-based, not measured (CANDIDATE)

See [Known Limitations and Gaps](./claude-code-agents-taxonomy.md#known-limitations-and-gaps) for details.

---

## How to Use These Documents

### For Individual Contributors
1. **First time using agents?** Read decision-tree.md (10 min)
2. **Need help choosing an agent?** Use decision-tree flowchart (1 min)
3. **Curious about a specific agent?** Read that agent's spec in taxonomy.md (5 min)
4. **Implementing a complex feature?** Read integration-guide.md patterns (15 min)

### For Team Leads / Architects
1. **Understanding the system?** Read taxonomy.md (30 min)
2. **Designing large workflows?** Read integration-guide.md (20 min)
3. **Setting up CI/CD?** Read integration-guide.md § CI/CD Integration (10 min)
4. **Reviewing agent work?** Use best practices from taxonomy.md (5 min)

### For New Users
1. **Quick orientation:** decision-tree.md (5 min)
2. **Deep dive:** taxonomy.md (30 min)
3. **Real examples:** integration-guide.md (15 min)
4. **Practice:** Choose a task, use decision tree, execute workflow

---

## References to AGENTS.md

These documents complement the project constitution in `/home/user/lsp-max/AGENTS.md`. Cross-references:

- **Bounded Status Words:** [Taxonomy § Best Practices](./claude-code-agents-taxonomy.md#1-use-bounded-status-words) references AGENTS.md § No Victory Language
- **ANDON Gate:** [Integration Guide § ANDON Gate Integration](./agent-integration-guide.md#andon-gate-integration) implements AGENTS.md § Λ_CD Predicate
- **Receipt-Based Admission:** [Taxonomy § Best Practices](./claude-code-agents-taxonomy.md#2-be-explicit-about-agent-scope) aligns with AGENTS.md § Test output is not a receipt
- **Gate-Check Preamble:** [Integration Guide § Gate Enforcement in Subagents](./agent-integration-guide.md#gate-enforcement-in-subagents) implements AGENTS.md § Subagent Gate Propagation — Proposed mitigation

---

## Status & Maintenance

**Document Status:** OPEN  
**Completeness:** CANDIDATE (core agents documented; advanced patterns partially documented)  
**Last Updated:** 2026-06-14  
**Next Review:** 2026-07-14

**Maintenance Notes:**
- Update decision trees when new agent types are added
- Add workflow examples as patterns mature
- Document limitations as they're discovered
- Seek contributions for gaps or clarifications

---

## Contributing

To improve these documents:

1. **Report gaps:** File an issue describing what's missing
2. **Suggest improvements:** PR with edits or new sections
3. **Share patterns:** Contribute new workflow examples
4. **Ask questions:** Use these docs to ask for clarifications

---

## Quick Commands

**Check gate status (parent session):**
```bash
lsp-max-cli gate check && echo "OPEN" || echo "BLOCKED"
```

**View active diagnostics:**
```bash
lsp-max-cli diagnostic snapshot
```

**Resume an agent session:**
```
Use SendMessage(to: <agent_id>, message: "follow-up question")
```

**Launch multiple agents in parallel:**
```
Agent(...) 
Agent(...)
Agent(...)
(all execute concurrently)
```

---

## Document Stats

| Document | Size | Lines | Sections | Purpose |
|----------|------|-------|----------|---------|
| claude-code-agents-taxonomy.md | 41 KB | 1452 | 15+ | Complete reference |
| agent-decision-tree.md | 14 KB | 450 | 10+ | Fast routing |
| agent-integration-guide.md | 22 KB | 846 | 8 | Advanced patterns |
| **TOTAL** | **77 KB** | **2748** | **33+** | Full suite |

---

## Status Summary

```
Document Coverage:    PARTIAL (core agents ADMITTED; advanced patterns CANDIDATE)
Decision Framework:   ADMITTED (verified against AGENTS.md)
Best Practices:       CANDIDATE (subject to refinement as patterns mature)
ANDON Integration:    ADMITTED (aligned with AGENTS.md § Λ_CD)
Session Boundaries:   OPEN (subagent gate propagation documented but not structurally enforced)
Performance Data:     CANDIDATE (heuristic-based; not measured at scale)

Overall Status:       OPEN — ready for use; continuous improvement expected
```

---

**Last Updated:** 2026-06-14  
**Next Review:** 2026-07-14  
**Maintainer:** Claude Code Agent Documentation Team
