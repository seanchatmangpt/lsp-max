# Claude Code Agent Types & Subagents: Taxonomy, Specifications, and Decision Framework

**Author:** Claude Code Agent Research Team  
**Date:** 2026-06-14  
**Status:** OPEN  
**Scope:** Complete specification of Claude Code agent types, subagent architecture, decision framework, and integration patterns.

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Agent Type Taxonomy](#agent-type-taxonomy)
3. [Detailed Agent Specifications](#detailed-agent-specifications)
4. [Decision Framework](#decision-framework)
5. [Integration Patterns](#integration-patterns)
6. [Tool Availability Matrix](#tool-availability-matrix)
7. [Common Use Patterns](#common-use-patterns)
8. [Known Limitations and Gaps](#known-limitations-and-gaps)
9. [Best Practices](#best-practices)

---

## Executive Summary

Claude Code provides a multi-agent system for task automation in the Claude Code IDE and command-line interface. Six agent types are available, each optimized for specific problem domains:

| Agent Type | Primary Use | Specialty |
|---|---|---|
| **claude** | Catch-all default agent | Any task without a specialized agent match |
| **claude-code-guide** | Feature questions about Claude tools | Claude API, Claude Code CLI, Claude Agent SDK |
| **Explore** | Fast code search and file location | Pattern-based file discovery, grep for symbols |
| **general-purpose** | Complex multi-step research | Cross-file analysis, open-ended investigation |
| **Plan** | Architecture and design planning | Implementation strategy, critical files, trade-offs |
| **statusline-setup** | Configuration automation | Status line settings in Claude Code |

Agents operate with **isolated sessions** that inherit limited context from the parent. The `Λ_CD^runtime` gate (ANDON enforcement) applies to parent sessions but requires explicit gate-check preambles in subagent prompts.

---

## Agent Type Taxonomy

### Layer 1: Foundational Agents

These agents form the base layer and are triggered by default patterns or explicit invocation.

#### 1.1 `claude` — The Catch-All Agent

**Purpose:**  
Default agent for any task that does not match a specialized agent's trigger pattern.

**Tools Available:**  
All tools (`*`). Full capability parity with parent session except: no access to parent context window history.

**When to Use:**
- General-purpose coding tasks
- Debugging and troubleshooting
- Implementation of features without specialized concerns
- Fallback when task scope is ambiguous
- Tasks spanning multiple agents (fan-out work)

**Example Prompts:**
- "Implement a new LSP service endpoint"
- "Debug why this test is failing"
- "Refactor this module for clarity"
- "Write a utility function for X"

**Output Format:**
- No structured output required; results are free-form
- Status word adoption recommended but not enforced (ADMITTED, CANDIDATE, BLOCKED, OPEN)
- Final message summarizes work performed and any blockers

**Integration:**
- Parent session receives raw output in message block
- No automatic context injection into subsequent sibling agents
- Ideal for fan-out: launch multiple `claude` agents in parallel via single `Agent` tool block

**Key Characteristics:**
- Slowest decision path (widest scope match)
- Best for exploration and prototyping
- No special context or assumptions embedded in prompt
- Useful when problem taxonomy is unclear

---

#### 1.2 `general-purpose` — Multi-Step Research Agent

**Purpose:**  
Dedicated agent for complex, multi-step research and cross-file analysis tasks.

**Tools Available:**  
All tools (`*`). Optimized for sequential research rather than immediate action.

**When to Use:**
- Searching for code patterns across large codebases
- Open-ended questions requiring multiple lookup rounds
- Investigating root causes that span many files
- Questions where you are unsure of the first lookup target
- Tasks requiring adversarial verification or multiple information sources
- Synthesis of findings from many locations

**Example Prompts:**
- "Find all usages of the `ConformanceVector` type and summarize how it's used"
- "Research how the lsp-max-compositor merges diagnostics across servers"
- "Investigate the full control flow for ANDON gate checking"
- "Analyze the difference between ADMITTED and CANDIDATE status across the codebase"

**Output Format:**
- Comprehensive findings with file paths and line numbers
- Clear source attribution (where information came from)
- Status boundaries: ADMITTED (with receipts), CANDIDATE (incomplete), OPEN (unknown)
- Structured sections: Findings, Related Files, Implementation Patterns, Known Gaps

**Integration:**
- Use for pre-analysis before spawning specialized agents
- Results feed into Plan or claude agents for action
- Ideal when investigation path is non-linear

**Key Characteristics:**
- Expects to perform multiple file reads and grepping
- Comfortable with "not found" results and repivoting
- Automatically composes new search queries based on intermediate results
- Best for discovery-driven work

---

### Layer 2: Specialized Search & Analysis Agents

#### 2.1 `Explore` — Fast Code Search Agent

**Purpose:**  
Localized, high-velocity code search using pattern matching and grep without exhaustive cross-file traversal.

**Tools Available:**  
All tools **except**: Agent, ExitPlanMode, Edit, Write, NotebookEdit.  
Read-only; optimized for speed (no file mutations allowed).

**Search Breadth Parameter:**
When invoking, specify search scope as argument:

```
"quick" — single targeted lookup, one grep pass
"medium" — moderate exploration, multiple search patterns
"very thorough" — exhaustive search across multiple locations and naming conventions
```

**When to Use:**
- Find files by glob pattern (`src/components/**/*.tsx`)
- Locate symbol definitions or usages
- Quick "where is X defined?" queries
- Answer "which files reference Y?"
- Verify presence/absence of code patterns
- Find all implementations of a trait

**Example Prompts:**
- "Find all files implementing the LanguageServer trait"
- "Locate the definition of ConformanceVector"
- "Which files reference ANDON in their code?"
- "Find all test files related to gate checking"

**Output Format:**
- File paths with line numbers
- Context excerpts showing the match
- Grouped by location or pattern
- Concise; optimized for quick scanning

**Integration:**
- Use before Explore + Read combo for focused investigation
- Results feed into Plan agent for architecture work
- Best as first step in "find then read" workflows
- Can be chained: Explore to find files, then general-purpose to analyze them

**Key Characteristics:**
- Fastest agent for file location
- Does not read whole files (excerpts only)
- Cannot edit or write (read-only)
- Single search mindset (one task per invocation)

**Known Limitation:**  
The "very thorough" breadth setting does not scale to codebases with >100k files and complex patterns. For exhaustive codebase scans, use `general-purpose` instead.

---

### Layer 3: Planning & Design Agents

#### 3.1 `Plan` — Architecture & Design Agent

**Purpose:**  
Software architecture planning, implementation strategy design, critical-path analysis, and trade-off evaluation.

**Tools Available:**  
All tools **except**: Agent, ExitPlanMode, Edit, Write, NotebookEdit.  
Read-only; optimized for analysis, not execution.

**When to Use:**
- Design implementation strategy for a feature
- Identify critical files and dependencies
- Analyze architectural trade-offs
- Scope work for team handoff
- Plan multi-phase rollout
- Risk and effort assessment

**Example Prompts:**
- "Design the implementation strategy for adding a new gate type"
- "Identify all critical files that need changes for LSP 3.18 feature X"
- "What are the architectural trade-offs in using Oxigraph vs in-memory storage?"
- "Plan the refactoring strategy for this 1000-line module"

**Output Format:**
- Step-by-step implementation plan
- Identified critical files with paths
- Architectural decisions with rationale
- Risk factors and mitigation strategies
- Effort estimates (where applicable)
- Dependency graph or ordering constraints

**Integration:**
- Run before assigning work to multiple team agents
- Results drive `claude` agent work plans
- Ideal for pre-sprint planning
- Feeds into verification and code-review agents

**Key Characteristics:**
- Systematic exploration of solution space
- Identifies bottlenecks and dependencies
- Considers long-term architectural impact
- Conservative (flags unknowns rather than assuming)

---

### Layer 4: Feature & Tool-Specific Agents

#### 4.1 `claude-code-guide` — Claude Tools & Features Agent

**Purpose:**  
Answer feature questions about Claude Code, Claude API, and the Claude Agent SDK. Not for implementation; for understanding and guidance.

**Tools Available:**  
Grep, Read, Glob, WebFetch, WebSearch.  
Intentionally limited (no Bash, Edit, Write to prevent accidental configuration changes).

**When to Use:**
- Questions about Claude Code IDE features
- Claude API usage, models, pricing, parameters
- Claude Agent SDK architecture and examples
- Tool use, streaming, MCP integration
- Capability limits, token counting, caching
- How to debug Claude-specific issues

**Example Prompts:**
- "How do I configure MCP servers in Claude Code?"
- "What are the differences between claude-opus and claude-sonnet?"
- "How do I use tool_choice in the Claude API?"
- "Does Claude Code support keyboard shortcuts? How do I customize them?"

**Output Format:**
- Direct answer with source attribution (links/docs)
- When source is internal docs: page/section reference
- When source is web: URL with quote
- Caveats and version notes where relevant
- Links to further reading

**Integration:**
- Best used **before** implementation agents (understand scope first)
- Run when user asks "Can Claude..." or "How do I..."
- Prevents feature misunderstandings down the line

**Key Characteristics:**
- Heavy reliance on WebSearch and WebFetch
- Always cites sources
- Avoids implementation details (stays at feature level)
- Time-bound (API docs change; web content is transient)

**IMPORTANT:** Before spawning a new claude-code-guide agent, check if one is already running in the conversation. Resume it via `SendMessage` with the agent's ID. This preserves accumulated context about the feature being researched.

---

#### 4.2 `statusline-setup` — Status Line Configuration Agent

**Purpose:**  
Specialized configuration agent for Claude Code status line settings in `settings.json`.

**Tools Available:**  
Read, Edit only.  
Minimal scope (no Bash, no full file write, no other tools).

**When to Use:**
- Configure status line format/appearance
- Adjust status line update frequency
- Show/hide specific status fields
- Debug status line display issues

**Example Prompts:**
- "Show git branch in the status line"
- "Add an indicator for unsaved files"
- "Hide the word count widget"

**Output Format:**
- Change summary (what was modified)
- Before/after JSON snippet
- Verification of change acceptance

**Integration:**
- Single-purpose invocation (one setting change per agent)
- Use for configuration automation
- Best as part of onboarding/setup workflows

**Key Characteristics:**
- Tiny scope (one file, one concern)
- Very fast execution
- No risk of cascading changes
- Idempotent operations

---

## Detailed Agent Specifications

### Agent Specification Template

Each agent specification includes:

- **Identity**: Name, abbreviation, role
- **Scope**: Problem domains covered
- **Preconditions**: When the agent should be used
- **Tools**: Exact capabilities and restrictions
- **Input Format**: How to invoke; required prompt structure
- **Output Format**: Expected format; status word adoption
- **Context Inheritance**: What parent session state is available
- **Session Boundary**: Hooks, gates, and cross-agent communication
- **Performance Characteristics**: Typical latency, resource usage
- **Known Limitations**: Edge cases, failure modes
- **Integration Patterns**: How to combine with other agents

---

### Agent: `claude` (Catch-All)

**Identity:**  
General-purpose, catch-all agent for tasks without specialized handling.

**Scope:**
- Implementation tasks
- Debugging
- Refactoring
- Feature development
- General code writing

**Preconditions:**
- No specialized agent matches the task
- Task scope is within the agent's tool capabilities
- No special context or constraints required

**Tools:**
```
All: Bash, Edit, Write, Read, Glob, Grep, Agent, ToolSearch, Skill
```

**Input Format:**
```
Agent(
  description: "short 3-5 word task summary",
  prompt: "detailed instruction including what to do, 
           background context, expected output format,
           how to verify success"
)
```

**Output Format:**
```
Single message with:
- Task completion summary
- Bounded status (ADMITTED, CANDIDATE, OPEN, BLOCKED)
- Key findings and file paths
- Any blockers or unknowns
```

**Context Inheritance:**
```
Limited. Agent receives:
- Codebase read-only access
- Working directory (absolute paths)
- Tool call permissions in current scope
Does NOT receive:
- Parent conversation history
- Previous agent results
- Stored variables or state
```

**Session Boundary:**
- Runs in isolated session
- ANDON gate: if parent has `PreToolUse` hook, subagent does NOT inherit it
  - Mitigation: include `lsp-max-cli gate check || exit 1` as first Bash call if gate matters
- No automatic context propagation to sibling agents

**Performance:**
- Invocation time: ~500ms
- Tool call overhead: ~100ms per call
- Cleanup on exit: automatic

**Known Limitations:**
- Cannot access parent session conversation history
- ANDON gate not inherited; must be checked manually
- Large prompts (>2000 tokens) may hit context limits
- No memory of previous agent invocations

**Integration Pattern: Fan-Out Work**
```
Send multiple Agent tool calls in a single message block:

Agent(description: "Task A", prompt: "...")
Agent(description: "Task B", prompt: "...")
Agent(description: "Task C", prompt: "...")

All three execute in parallel; results returned sequentially
in the same response block.
```

**Integration Pattern: Sequential Dependency**
```
// First agent completes and returns findings
// Then in next message:

Agent(description: "Refactor based on findings",
      prompt: "The Explore agent found these files. 
               Now refactor them per these criteria...")
```

---

### Agent: `general-purpose` (Multi-Step Research)

**Identity:**  
Research agent for open-ended, multi-round investigation.

**Scope:**
- Complex codebase searches
- Root cause analysis
- Pattern discovery
- Synthesis across many files
- Adversarial verification

**Preconditions:**
- Initial search target is unknown or unclear
- Multiple file reads required
- Search path may pivot based on findings
- High degree of exploration

**Tools:**
```
All: Bash, Edit, Write, Read, Glob, Grep, Agent, ToolSearch, Skill
```

**Input Format:**
```
Agent(
  description: "short research objective",
  prompt: "clearly state the research question,
           indicate what constitutes a complete answer,
           describe the expected search path (if known),
           ask for status boundaries not victory claims"
)
```

**Output Format:**
```
Comprehensive research report with:
- Findings summary
- File paths and line numbers (absolute)
- Implementation patterns discovered
- Known gaps and unknowns
- Confidence level per finding
- Recommended next steps
```

**Context Inheritance:**
Same as `claude` agent. Limited; no parent history.

**Session Boundary:**
- Isolated session
- Gate check: include preamble if ANDON gate matters
- Results do not auto-propagate to siblings

**Performance:**
- Invocation time: ~1s (setup) + search time
- Typical search: 2-5s (depends on codebase size and query specificity)
- Large codebases: may timeout at 10s per search

**Known Limitations:**
- Very broad searches (e.g., "find all LSP methods") may return >1000 results and truncate
- Does not guarantee exhaustiveness on the first pass
- Search strategy pivots consume time (plan your search breadth upfront)
- Cannot guarantee finding all edge cases

**Integration Pattern: Research Then Act**
```
1. Spawn general-purpose agent to find all usages of X
2. Agent returns findings and file list
3. In next message, spawn Plan agent with file list
4. Plan agent designs refactoring strategy
5. Spawn claude agent to implement per Plan's strategy
```

---

### Agent: `Explore` (Fast Search)

**Identity:**  
Speed-optimized file and symbol search agent.

**Scope:**
- File location by pattern
- Symbol definition lookup
- Usage finding
- Presence/absence verification
- Code pattern matching

**Preconditions:**
- You know roughly where to look (directory, file type)
- You want fast results, not exhaustive analysis
- Task is read-only (no edits)

**Tools:**
```
All EXCEPT: Agent, ExitPlanMode, Edit, Write, NotebookEdit
Available: Bash, Read, Glob, Grep, ToolSearch, Skill
```

**Input Format:**
```
Agent(
  subagent_type: "Explore",
  description: "find X in Y",
  prompt: "specific pattern or file search,
           breadth parameter: 'quick' / 'medium' / 'very thorough',
           list expected file types or naming patterns,
           what to do if not found"
)
```

**Breadth Parameter Guidance:**
```
"quick"        — one Glob or one Grep, done in <100ms
"medium"       — 3-5 search patterns, 200-500ms
"very thorough" — 10+ patterns, all naming conventions, 1-2s
```

**Output Format:**
```
File list with:
- Absolute paths
- Line number(s) of matches
- Context snippet (1-3 lines per match)
- Count summary (N files, M matches)
```

**Context Inheritance:**
Same as other agents; limited.

**Session Boundary:**
- No edit/write capability
- ANDON gate not inherited
- Read-only safety guarantee

**Performance:**
- Quick breadth: <200ms
- Medium breadth: 300-800ms
- Very thorough breadth: 1-3s
- File system operations are the bottleneck (not agent startup)

**Known Limitations:**
- "very thorough" does not scale to >100k file codebase + complex patterns
- Cannot analyze file contents deeply (Explore only returns excerpts)
- If you need to understand what the code does, follow Explore with a Read/general-purpose agent

**Integration Pattern: Explore Then Read**
```
1. Explore(breadth: "quick", prompt: "find ConformanceVector definition")
   → Returns: "src/primitives/conformance_vector.rs:42"
2. Read("/home/user/lsp-max/src/primitives/conformance_vector.rs")
   → Full file content for analysis
```

**Integration Pattern: Multi-Agent Pipeline**
```
Explore (find files) → Plan (design strategy) → claude (implement)
```

---

### Agent: `Plan` (Architecture & Design)

**Identity:**  
Architecture and design planning agent.

**Scope:**
- Implementation strategy
- Critical path analysis
- Trade-off evaluation
- Dependency identification
- Effort estimation
- Risk assessment

**Preconditions:**
- Decision-making or planning needed (not immediate action)
- Complex cross-file changes
- Multiple valid approaches exist
- Team handoff or scheduling needed

**Tools:**
```
All EXCEPT: Agent, ExitPlanMode, Edit, Write, NotebookEdit
Available: Bash, Read, Glob, Grep, ToolSearch, Skill
```

**Input Format:**
```
Agent(
  subagent_type: "Plan",
  description: "design strategy for X",
  prompt: "describe the feature/refactor,
           list constraints (time, architectural),
           mention any stakeholders or dependencies,
           ask for specific outputs:
           - step-by-step plan
           - critical files
           - trade-offs
           - risk factors"
)
```

**Output Format:**
```
Architecture/Implementation Plan:

1. Overview of Approach
   - Core strategy and rationale
   
2. Critical Files (with paths)
   - Which files must change
   - Which are optional
   
3. Step-by-Step Implementation Plan
   - Phase 1: Setup/Prep
   - Phase 2: Core Implementation
   - Phase 3: Integration/Testing
   - Phase 4: Cleanup
   
4. Architectural Trade-Offs
   - Option A vs Option B with pros/cons
   - Recommended choice and why
   
5. Risk Factors & Mitigation
   - Identified risks
   - Mitigation strategies
   
6. Dependencies & Ordering
   - What must happen before what
   - Parallel work opportunities
   
7. Effort & Timeline Estimate
   - Days/hours per phase (realistic bounds)
```

**Context Inheritance:**
Limited; no parent conversation history.

**Session Boundary:**
- Read-only operation (no execution)
- Useful as a pre-check before spawning implementation agents
- Design can be reviewed before action

**Performance:**
- Invocation: ~1-2s (setup + file reading)
- Design time: 2-5s (depends on complexity)
- Complex architectural questions: 10-15s

**Known Limitations:**
- Cannot execute or verify the plan (read-only)
- Effort estimates are lower-bound heuristics, not hard commitments
- Assumes codebase is stable; does not account for ongoing changes
- Trade-off analysis is high-level; does not code-dive

**Integration Pattern: Plan Then Implement**
```
1. Plan agent designs implementation strategy
2. Review and approve plan (human or automated)
3. Spawn claude agent with Plan's strategy as requirements
4. claude implements per plan
```

---

### Agent: `claude-code-guide` (Claude Tools & Features)

**Identity:**  
Knowledge agent for Claude tools and features.

**Scope:**
- Claude Code (IDE) features and configuration
- Claude API usage and parameters
- Claude Agent SDK
- Tool use capabilities
- Model selection and pricing
- Integration with MCP and extensions

**Preconditions:**
- Question about Claude or its tools (not about general software engineering)
- Seeking feature guidance, not implementation
- Time-sensitive (may require web search for latest info)

**Tools:**
```
Limited: Glob, Grep, Read, WebFetch, WebSearch
NOT Available: Bash, Edit, Write, Agent, ToolSearch, Skill
```

**Input Format:**
```
Agent(
  subagent_type: "claude-code-guide",
  description: "answer question about X Claude feature",
  prompt: "specific feature question,
           note any version constraints,
           ask for source attribution and links,
           mention if this requires web search vs docs"
)
```

**Output Format:**
```
Feature Answer:

1. Direct Answer
   - Clear, concise response
   
2. Source Attribution
   - Link to official docs
   - Relevant version/date
   
3. Examples (if applicable)
   - Code snippet or config example
   - How to use the feature
   
4. Caveats & Limitations
   - What this does NOT do
   - Known issues
   
5. Further Reading
   - Links to related docs
```

**Context Inheritance:**
Limited; no parent history. Maintains its own research state during the session.

**Session Boundary:**
- **IMPORTANT**: Before spawning a new claude-code-guide agent, check if one is already running
  - Use `SendMessage(to: <agent_id>)` to resume it with follow-up questions
  - Preserves accumulated research context
  - Avoids duplicate web searches
- Isolated session otherwise

**Performance:**
- Invocation: ~500ms
- Web search (if required): +1-2s
- Docs lookup: ~500ms-1s
- Total typical: 1-3s

**Known Limitations:**
- Web search results are current-date dependent (may miss very recent changes)
- Claude API docs change; examples may become stale
- Does not answer general software engineering questions (out of scope)
- Cannot verify code changes work (read-only)

**Integration Pattern: Understand Before Implement**
```
1. claude-code-guide agent: "What MCP servers does Claude Code support?"
2. Agent returns list and links
3. Based on answer, spawn Plan agent to design integration
4. Based on plan, spawn claude agent to implement
```

---

### Agent: `statusline-setup` (Configuration)

**Identity:**  
Configuration automation agent for Claude Code status line settings.

**Scope:**
- `.claude/settings.json` status line fields
- Status display format and frequency
- Individual widget show/hide

**Preconditions:**
- Single, isolated configuration change needed
- No code implementation
- Focus is on settings.json modification

**Tools:**
```
Minimal: Read, Edit only
NOT Available: Bash, Write, Glob, Grep, Agent, etc.
```

**Input Format:**
```
Agent(
  subagent_type: "statusline-setup",
  description: "configure status line for X",
  prompt: "specific setting to change,
           desired value or format,
           verify change by showing before/after JSON"
)
```

**Output Format:**
```
Configuration Change Summary:

Setting Changed: [field name]
Before: [JSON snippet]
After: [JSON snippet]

Status: [ADMITTED | CANDIDATE | BLOCKED]
```

**Context Inheritance:**
None. Single-shot configuration change.

**Session Boundary:**
- Highly isolated
- Changes are reversible
- No cross-agent dependencies

**Performance:**
- Very fast: <500ms typical
- I/O bound: file read, edit, verify

**Known Limitations:**
- Single setting per invocation (avoid multi-step reconfigurations)
- Only edits `.claude/settings.json` (not settings.local.json or other configs)
- Cannot verify runtime behavior (no execution)

**Integration Pattern: Onboarding**
```
Launch multiple statusline-setup agents in parallel for bulk configuration:

Agent(subagent_type: "statusline-setup", prompt: "show git branch")
Agent(subagent_type: "statusline-setup", prompt: "hide word count")
Agent(subagent_type: "statusline-setup", prompt: "set update freq to 500ms")
```

---

## Decision Framework

### Flowchart: Choosing the Right Agent

```
Q1: Is this a feature/API question about Claude tools?
  YES → claude-code-guide
  NO  → Q2

Q2: Is this a single, isolated configuration change?
  YES → statusline-setup
  NO  → Q3

Q3: Is this a read-only search or file-location task?
  YES → Explore
  NO  → Q4

Q4: Does this require multi-step analysis and exploration?
  YES → general-purpose
  NO  → Q5

Q5: Is this an architecture/design/planning task?
  YES → Plan
  NO  → Q6

Q6: Everything else?
  DEFAULT → claude
```

### Decision Matrix by Task Type

| Task Type | Primary Agent | Fallback | Notes |
|---|---|---|---|
| Find a file or symbol | Explore | general-purpose | Use Explore if you know roughly where to look |
| Investigate root cause (unknown start) | general-purpose | Explore | Use when search path is unclear |
| Design implementation | Plan | general-purpose | Always use Plan before major changes |
| Implement a feature | claude | Plan + claude | Plan first to verify strategy |
| Debug a test failure | claude | general-purpose | Use claude first; if root cause unknown, escalate to general-purpose |
| Refactor a module | claude | Plan + claude | Large refactors: Plan first |
| Answer feature question | claude-code-guide | WebSearch manually | Always try claude-code-guide first |
| Configure settings | statusline-setup | — | Exclusive for this domain |
| Understand LSP architecture | general-purpose | Explore + Read | Use general-purpose for conceptual synthesis |
| Parallel independent work | claude × N | — | Fan out multiple claude agents in single Agent block |

---

## Integration Patterns

### Pattern 1: Fan-Out Parallelism

**Use Case:** Multiple independent tasks that can execute concurrently.

**Execution:**
```
Send multiple Agent tool calls in a single message block.
All execute in parallel; results returned in sequence.
```

**Example:**
```
Agent(description: "Find all LSP diagnostic types", 
      prompt: "search for DiagnosticSeverity usage")
Agent(description: "Find all ConformanceVector usages",
      prompt: "search for ConformanceVector trait implementations")
Agent(description: "Find ANDON gate integration points",
      prompt: "search for PreToolUse hook references")
```

**Best For:**
- Independent searches (no data dependencies)
- Parallel implementation work
- Distributed system tasks

**Caution:**
- Do not use for dependent tasks (Task B needs Task A result)
- Large fan-outs (>10 agents) risk token budgets

---

### Pattern 2: Research → Design → Implement

**Use Case:** Major feature or refactoring with unknown scope.

**Steps:**
1. **general-purpose agent**: Research and discover current implementation
2. Review findings
3. **Plan agent**: Design implementation strategy based on findings
4. Review and approve plan
5. **claude agent**: Implement per approved plan

**Example:**
```
Message 1: Launch general-purpose to research ConformanceVector usage
          (agent completes, returns findings)

Message 2: "Based on these findings, launch Plan agent to design refactoring"
          (Plan agent completes, returns strategy)

Message 3: "Based on the plan, implement the refactoring"
          (claude agent completes implementation)
```

**Best For:**
- Unknown scope (no design-before-code possible)
- Team collaboration (design phase allows review)
- Risk mitigation (validate approach before committing)

---

### Pattern 3: Explore → Read → General-Purpose

**Use Case:** Quick file location followed by deep analysis.

**Steps:**
1. **Explore agent**: Quickly find relevant files
2. **Read tool**: Open those files directly
3. **general-purpose agent**: Analyze patterns across the files

**Example:**
```
Explore(breadth: "quick", prompt: "find all gate-related files")
// Returns: ["src/gate.rs", "crates/lsp-max-cli/src/nouns/gate.rs"]

Read("/home/user/lsp-max/src/gate.rs")
Read("/home/user/lsp-max/crates/lsp-max-cli/src/nouns/gate.rs")

general-purpose(prompt: "analyze these gate files and explain 
                         how the gate file write mechanism works")
```

---

### Pattern 4: Sequential Dependency Chain

**Use Case:** Task B depends on Task A result.

**Execution:**
```
Message 1: Spawn Agent A
           (completes, returns result)

Message 2: Incorporate result into prompt for Agent B
           (B completes, returns result)

Message 3: Based on B, launch Agent C
           (C completes)
```

**Example:**
```
Message 1: Explore to find ANDON diagnostic codes
          (returns: WASM4PM-*, GGEN-*, ANTI-LLM-*)

Message 2: "Now research which components emit these codes"
          (general-purpose uses the codes to find emitters)

Message 3: "Design a test to verify all codes are emitted correctly"
          (Plan designs the test strategy)

Message 4: "Implement the test per the plan"
          (claude implements)
```

---

### Pattern 5: Feedback Loop (Human Review Between Agents)

**Use Case:** Human needs to review/approve between agent work.

**Execution:**
```
Agent(prompt: "design strategy")
  [HUMAN REVIEW: approve/request changes]
Agent(prompt: "implement approved strategy")
  [HUMAN REVIEW: test and validate]
Agent(prompt: "refactor per feedback")
```

**Best For:**
- High-stakes changes (architectural decisions)
- Multi-phase projects requiring checkpoints
- Learning loops (each phase informs the next)

---

## Tool Availability Matrix

| Tool | claude | general-purpose | Explore | Plan | claude-code-guide | statusline-setup |
|---|---|---|---|---|---|---|
| Bash | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| Edit | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ |
| Write | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| Read | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Glob | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| Grep | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| Agent | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| ToolSearch | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| Skill | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| WebFetch | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| WebSearch | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |

**Key Restrictions:**
- **Explore, Plan, claude-code-guide**: Read-only (no file mutations)
- **statusline-setup**: Minimal (Edit only; no Bash/Write)
- **Explore**: Cannot spawn subagents (no Agent tool)

---

## Common Use Patterns

### Finding Code

**Pattern: "Where is X defined?"**

```
Use Explore(breadth: "quick") with glob pattern or grep symbol name
Expected time: <200ms
Result: File path(s) and line number(s)
```

**Example:**
```
Explore(
  description: "find ConformanceVector definition",
  prompt: "search for 'struct ConformanceVector' or 'pub struct ConformanceVector'"
)
```

---

### Investigating Unknown Code Behavior

**Pattern: "How does X work?"**

```
Use general-purpose agent to:
1. Find definition of X
2. Find all usages of X
3. Analyze patterns and synthesize understanding
Expected time: 2-5s
Result: Comprehensive explanation with code references
```

**Example:**
```
general-purpose(
  prompt: "Research how the ANDON gate is checked. Find:
           1. Where gate file is written (lsp-max-compositor)
           2. Where gate file is read (lsp-max-cli gate check)
           3. How the PreToolUse hook integrates
           4. Show the complete flow from gate write to tool block"
)
```

---

### Designing a Large Refactor

**Pattern: "How should I refactor X?"**

```
1. general-purpose: Research current implementation
2. Plan: Design refactoring strategy
3. claude: Implement per plan
Expected time: 10-30s (research) + 30-60s (design) + implementation time
Result: Validated strategy + implementation
```

---

### Understanding Architecture

**Pattern: "Explain the relationship between X and Y"**

```
Use general-purpose agent with synthesis focus:
- Find both X and Y definitions
- Find all integration points
- Extract and synthesize interaction patterns
- Draw conclusions
Expected time: 5-10s
Result: Architectural relationship map with code evidence
```

---

### Bulk Configuration Changes

**Pattern: "Configure the IDE for X workflow"**

```
Fan-out multiple statusline-setup agents (parallel):

Agent(statusline-setup, "show git branch")
Agent(statusline-setup, "hide word count")
Agent(statusline-setup, "set update frequency to 1s")
```

---

## Known Limitations and Gaps

### Session Boundary Issues

**Issue: ANDON Gate Not Inherited by Subagents**

**Status:** OPEN (documented in AGENTS.md)

**Description:**  
The `PreToolUse` hook that enforces `Λ_CD^runtime` (ANDON gate) runs only in the parent Claude Code session. Subagents spawned via the `Agent` tool run in isolated sessions and do NOT inherit the parent's hook.

**Current Mitigation:**  
Include a gate-check preamble as the first Bash call in agent prompts:
```bash
lsp-max-cli gate check || exit 1
```

**Impact:**  
Subagents can proceed with Bash/Edit/Write operations even when ANDON is active in the parent session, violating `Λ_CD^runtime` enforcement.

**Proposed Fixes:**
1. Structural: Add hook propagation to Agent tool invocation (effort: weeks; affects architecture)
2. Convention: Document gate-check preamble as mandatory (current mitigation)
3. Context injection: Pass `D_t` (active diagnostics) as agent context before execution (RFC-1)

---

### Agent Context Inheritance

**Issue: Limited Parent Session Context**

**Status:** BY DESIGN

**Description:**  
Agents do not receive parent conversation history, stored variables, or prior agent results unless explicitly passed in the prompt.

**Workaround:**
```
Use SendMessage to continue an existing agent session
instead of launching a new agent.
```

**Impact:**  
Multi-round agent work requires explicit context passing (verbose prompts) or session continuity (SendMessage pattern).

---

### Search Scalability

**Issue: Very Thorough Explore Breadth Doesn't Scale**

**Status:** CANDIDATE (known limitation)

**Description:**  
Explore agent with `breadth: "very thorough"` does not efficiently handle codebases with >100k files and complex glob/grep patterns.

**Current Behavior:**  
- "very thorough" at 100k files: timeout risk
- Workaround: use `general-purpose` for exhaustive searches

**Recommendation:**  
For large codebases, use `general-purpose` agent instead.

---

### Tool Composition Gaps

**Issue: Some Agent Combinations Are Not Available**

**Status:** BY DESIGN

**Example:**  
Cannot spawn a subagent within Explore agent (Explore lacks Agent tool).

**Workaround:**  
Complete Explore search first, then use the results in a new agent invocation.

---

## Best Practices

### 1. Use Bounded Status Words

Always use bounded status in agent output:

```
✓ ADMITTED — fully verified (with receipt/test/code evidence)
✓ CANDIDATE — plausible but not yet verified
✓ OPEN — unknown or partially researched
✓ BLOCKED — blocked by external dependency or gate
✓ REFUSED — rejected by policy or architecture
✗ (AVOID) "done", "complete", "all clean", "fully working"
```

---

### 2. Be Explicit About Agent Scope

Write agent prompts as if the agent is a new hire who just walked in the room:

```
✓ "Research how ConformanceVector is used. Find: (1) definition, (2) usages,
   (3) construction sites, (4) test fixtures. Provide file paths and line numbers."

✗ "Look at ConformanceVector"
```

---

### 3. Verify Before Trusting

Agent outputs should be verified:

```
✓ Explore finds files → verify with Read before using results
✓ Plan suggests strategy → review and approve before implementation
✓ claude writes code → run tests and verify behavior before merge
```

---

### 4. Prefer Fan-Out Over Sequential When Possible

```
✓ Send 3 independent Explore agents in parallel (single Agent block)
✗ Send 3 Explore agents sequentially (separate messages)
```

Reduces total execution time from 3 × 200ms to ~200ms.

---

### 5. Use Plan Agent for Major Changes

Any change affecting >5 files or spanning multiple modules should have a Plan agent review first:

```
✓ Design architecture → Plan agent → Review → claude implements
✗ Direct implementation without design phase
```

---

### 6. Resume Existing Agents Instead of Spawning New Ones

For multi-round interactions with the same agent type:

```
✓ Agent spawns: "research LSP features"
  Returns findings
  Use SendMessage(to: <agent_id>, message: "follow-up question")
  
✗ Spawn a new agent for the follow-up
```

---

### 7. Gate-Check Preamble in Critical Operations

When Bash operations must respect the ANDON gate:

```bash
#!/bin/bash
set -e

# Gate check FIRST (before any actions)
lsp-max-cli gate check || exit 1

# Now proceed with protected operations
cargo test
cargo fmt
git push
```

---

### 8. Provide Negative Controls in Agent Prompts

When asking agents to verify something, include what should NOT happen:

```
✓ "Verify ANTI-LLM-TOWER-LSP-001 is emitted when plain tower-lsp is found.
   Verify it is NOT emitted for lsp-max references."

✗ "Check if the diagnostic works"
```

---

### 9. Avoid Extremely Large Fan-Outs

```
✓ Fan out 3-5 independent agents in parallel
✗ Fan out 20+ agents (token budget, context limits)
```

---

### 10. Name Agents Clearly When Spawning Multiple

```
✓ description: "research ANDON gate implementation"
   description: "research lsp-max-cli integration"
   description: "research PreToolUse hook"

✗ description: "research stuff"
   description: "research more stuff"
```

---

## Appendix: Quick Reference Card

### One-Liner Agent Selection

| Question | Agent | Tools |
|---|---|---|
| "Where is X defined?" | Explore | Glob, Grep, Read |
| "How does X work?" | general-purpose | All |
| "How should I design X?" | Plan | All read-only |
| "Does Claude Code support Y?" | claude-code-guide | WebSearch, Read |
| "Set status line to Z" | statusline-setup | Edit |
| "Implement X" | claude | All |

### Invocation Template by Agent Type

**Explore:**
```
Agent(
  subagent_type: "Explore",
  description: "find X by pattern Y",
  prompt: "Glob pattern or grep symbol. Breadth: quick/medium/very thorough."
)
```

**general-purpose:**
```
Agent(
  description: "research question about X",
  prompt: "Research question. What constitutes complete answer? 
           Bounded statuses only."
)
```

**Plan:**
```
Agent(
  subagent_type: "Plan",
  description: "design implementation for X",
  prompt: "Feature/refactor description. Constraints? 
           Output: step-by-step plan, critical files, trade-offs, risks."
)
```

**claude-code-guide:**
```
Agent(
  subagent_type: "claude-code-guide",
  description: "answer question about Claude API/Code feature",
  prompt: "Feature question. Cite sources. Include version constraints."
)
```

**statusline-setup:**
```
Agent(
  subagent_type: "statusline-setup",
  description: "configure status line for X",
  prompt: "Specific setting to change. Desired value. 
           Show before/after JSON."
)
```

**claude (catch-all):**
```
Agent(
  description: "implement X or debug Y",
  prompt: "Detailed instruction. Background. Expected output. Verification."
)
```

---

## Status & Maintenance

**Document Status:** OPEN  
**Last Updated:** 2026-06-14  
**Next Review:** 2026-07-14  

**Known Gaps:**
- Subagent gate propagation is OPEN; check AGENTS.md § Subagent Gate Propagation
- Agent context window limits not formally documented
- Performance benchmarks are heuristic-based, not measured

**Contributions:**
To update this document, please file an issue or pull request describing the gap or clarification needed.

---
