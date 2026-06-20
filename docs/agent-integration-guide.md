# Claude Code Agent Integration Guide

**Purpose:** Advanced patterns for combining agents, handling ANDON gates, multi-agent workflows, and integration with lsp-max tooling.

**Audience:** Teams automating complex tasks; agents working on large features; CI/CD integration.

---

## Table of Contents

1. [Multi-Agent Workflow Patterns](#multi-agent-workflow-patterns)
2. [ANDON Gate Integration](#andon-gate-integration)
3. [Session Boundary Handling](#session-boundary-handling)
4. [Large Feature Implementation](#large-feature-implementation)
5. [Continuous Integration Integration](#continuous-integration-integration)
6. [Error Recovery & Rollback](#error-recovery--rollback)
7. [Debugging Agent Behavior](#debugging-agent-behavior)
8. [Performance Tuning](#performance-tuning)

---

## Multi-Agent Workflow Patterns

### Pattern 1: Research → Design → Implement → Verify

**Use Case:** Complex feature requiring full lifecycle.

**Workflow:**

```
Step 1: Research Current State
├─ general-purpose agent
│  ├ Question: "How is ConformanceVector currently used?"
│  ├ Deliverable: Findings report
│  └ Time: 5-10s
│
Step 2: Design Implementation
├─ Plan agent (uses research findings)
│  ├ Prompt: "Based on findings, design X implementation"
│  ├ Deliverable: Step-by-step plan, critical files, trade-offs
│  ├ Human Review: Approve/request changes
│  └ Time: 5-10s
│
Step 3: Implement Changes
├─ claude agent (uses approved plan)
│  ├ Prompt: "Implement per approved plan: [paste plan]"
│  ├ Deliverable: Modified files, tests passing
│  └ Time: Implementation time + test time
│
Step 4: Verify Changes
├─ claude agent (verify behavior)
│  ├ Task: Run tests, check behavior, verify no regressions
│  ├ Deliverable: Test output, verification report
│  └ Time: ~30s-5m depending on test suite
│
Step 5: Gate Check
├─ lsp-max-cli gate check
│  ├ Check: Are ANDON signals clear?
│  ├ If BLOCKED: resolve diagnostics, go to step 3
│  └ If OPEN: proceed to commit
```

**Total Time:** 20-30s research+design + N seconds implementation.

**Checkpoints:**
- After step 2: Human approves plan
- After step 3: Code compiles, tests pass
- Before step 5: Gate must be clear

---

### Pattern 2: Parallel Implementation (Fan-Out)

**Use Case:** Multiple independent modules/features can be implemented concurrently.

**Setup:**

```
Identify independent work items:
1. Feature A (affects module X)
2. Feature B (affects module Y)
3. Feature C (affects module Z)

Verify no shared file dependencies.
```

**Execution:**

```
Message 1: Plan all three in parallel

Agent(subagent_type: "Plan", description: "design feature A", prompt: "...")
Agent(subagent_type: "Plan", description: "design feature B", prompt: "...")
Agent(subagent_type: "Plan", description: "design feature C", prompt: "...")

(All three complete in ~10s; get approval on all three)

Message 2: Implement all three in parallel

Agent(description: "implement feature A", prompt: "...")
Agent(description: "implement feature B", prompt: "...")
Agent(description: "implement feature C", prompt: "...")

(All three execute concurrently; results returned in sequence)

Message 3: Verify all together

Agent(description: "run full test suite and verify no conflicts",
      prompt: "Run cargo test --workspace; check for conflicts
               between features A, B, C")
```

**Key Conditions:**
- No shared file modifications (confirm with Plan agents)
- Independent test success (each agent can verify its own tests)
- Final integration test (at least one verifies all together)

---

### Pattern 3: Iterative Refinement (Feedback Loop)

**Use Case:** Feature requires multiple rounds of feedback.

**Workflow:**

```
Round 1: Implement Initial Version
├─ claude agent: implement feature skeleton
│  └ Deliverable: basic functionality working
│
Round 2: Review & Request Changes
├─ Human review: identify gaps, request improvements
│  └ Feedback: list of changes needed
│
Round 3: Refine Implementation
├─ claude agent: incorporate feedback
│  ├ Prompt: "Based on review feedback [list], refine implementation"
│  └ Deliverable: updated code with feedback applied
│
Round 4: Verify Fixes
├─ claude agent: run tests, verify behavior
│  └ Deliverable: verification that all feedback addressed
│
Round 5: Final Review
├─ Human approval (if satisfied) → Ready to merge
│  └ Status: ADMITTED
```

**When to Use:**
- Feature complexity makes single-shot implementation risky
- User/team feedback needed mid-development
- Iterative refinement of architecture

---

### Pattern 4: Troubleshooting & Root Cause Analysis

**Use Case:** Test failures or unexpected behavior.

**Workflow:**

```
Step 1: Understand the Symptom
├─ claude agent: run failing test, capture output
│  ├ Command: cargo test <test_name> -- --nocapture
│  └ Output: test output, stack trace (if applicable)
│
Step 2: Diagnose Root Cause
├─ general-purpose agent: analyze failure
│  ├ Question: "Research how this component works; 
│  │            find where behavior diverges from expected"
│  └ Deliverable: root cause identified, related code sections
│
Step 3: Fix the Issue
├─ claude agent: implement fix
│  ├ Prompt: "Based on root cause analysis, fix the issue here: [path:line]"
│  └ Deliverable: modified code
│
Step 4: Verify Fix
├─ claude agent: re-run failing test
│  ├ Command: cargo test <test_name>
│  └ Verify: test passes, no new failures
```

---

## ANDON Gate Integration

### Understanding the Gate

The ANDON gate (`Λ_CD^runtime`) blocks shell-side actions when active:

```
State: OPEN    → no ANDON signals → Bash/Edit/Write allowed
State: BLOCKED → ANDON signals present → Bash/Edit/Write blocked
```

ANDON signals are active when Error-severity diagnostics matching these prefixes exist:
- `WASM4PM-*`
- `GGEN-*`
- `ANTI-LLM-*`

---

### Gate Enforcement in Parent Session

**Parent Session (your Claude Code IDE):**

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write|TaskCreate|NotebookEdit",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      }
    ]
  }
}
```

Every Bash/Edit/Write call is gated. Exit code 0 = proceed, Exit code 1 = blocked.

---

### Gate Enforcement in Subagents

**Subagent Session:**

The PreToolUse hook does NOT cross session boundaries. Subagents are NOT automatically gated.

**Mitigation: Gate-Check Preamble**

Include as the first Bash call in agent prompts:

```bash
#!/bin/bash
set -e

# Gate check FIRST
lsp-max-cli gate check || exit 1

# Now safe to proceed
cargo test
cargo fmt
git push
```

**Implementation:**

```
Agent(
  description: "run tests and verify",
  prompt: "Run cargo test --workspace. 
           IMPORTANT: First Bash command must be 'lsp-max-cli gate check || exit 1'
           to ensure ANDON gate is respected.
           If gate returns exit 1, stop and report."
)
```

---

### Workflow: Building with Gate Enforcement

**Safe Build Pattern:**

```
Step 1: Check Gate Status (in parent session)
├─ $ lsp-max-cli diagnostic snapshot
│  └ Output: active diagnostics (if any)
│
Step 2: If Gate is BLOCKED
├─ Resolve all WASM4PM-*, GGEN-*, ANTI-LLM-* errors
│  └ Typically: lsp-max-cli repair plan (shows fixes)
│
Step 3: Verify Gate is OPEN
├─ $ lsp-max-cli gate check && echo "Gate is clear"
│  └ Output: "Gate is clear" (or blocked if diagnostics remain)
│
Step 4: Spawn Build Agent (with gate-check preamble)
├─ Agent(prompt: "First do: lsp-max-cli gate check || exit 1
│                   Then: cargo test --workspace")
│  └ Output: test results
│
Step 5: Push Changes
├─ Bash: git push
│  └ Gate check runs automatically via PreToolUse hook
```

---

### Gate File Format (For Advanced Use)

**Location:**
```
$XDG_RUNTIME_DIR/lsp-max-gate-{fnv1a(cwd):016x}
or
/tmp/lsp-max-gate-{fnv1a(cwd):016x}
```

**Content:**
- `b"0"` (single byte ASCII '0') = gate is OPEN, safe to proceed
- `b"1"` (single byte ASCII '1') = gate is BLOCKED, ANDON signals present
- Absent = compositor not running (gate not enforced)

**Reading the Gate (Advanced):**

```rust
use std::fs::read;

fn check_gate(cwd: &str) -> Result<bool> {
    let hash = fnv1a_hash(cwd);
    let gate_path = format!("/tmp/lsp-max-gate-{:016x}", hash);
    
    match read(&gate_path) {
        Ok(bytes) if bytes == b"0" => Ok(true),   // open
        Ok(bytes) if bytes == b"1" => Ok(false),  // blocked
        _ => Ok(true), // absent = not enforced
    }
}
```

See `crates/lsp-max-cli/src/nouns/gate.rs` and `crates/lsp-max-compositor/src/gate_file.rs` for reference.

---

## Session Boundary Handling

### Understanding Session Isolation

**Parent Session:**
- Access to: all tools
- Hooks: PreToolUse gate enforcement
- Context: full conversation history
- Lifetime: user's entire Claude Code IDE session

**Subagent Session:**
- Access to: specified tools (depends on agent type)
- Hooks: **NONE** inherited from parent
- Context: only what's in the Agent prompt
- Lifetime: single agent invocation (~1-5s)

**Implication:** Subagents are completely isolated; they cannot access parent state or prior agent results unless explicitly passed in the prompt.

---

### Context Passing Strategies

#### Strategy 1: Inline Context in Prompt

**Use for:** Small amounts of context (< 500 tokens).

```
Agent(
  description: "refactor based on findings",
  prompt: "Based on the research from the previous agent:
           - ConformanceVector is defined in src/primitives/conformance_vector.rs
           - It's used in 12 locations (list: [path:line, ...])
           - Current pattern: always constructed in gate processing
           
           Now refactor to: ..."
)
```

**Pros:** No infrastructure needed; simple.  
**Cons:** Verbose; doesn't scale to large datasets.

---

#### Strategy 2: File-Based Handoff

**Use for:** Large context (> 500 tokens) or structured data.

```
Step 1: claude agent generates structured output
└─ Write findings to /tmp/agent-findings.json

Step 2: General-purpose agent reads findings
├─ Read("/tmp/agent-findings.json")
└─ Process findings and generate next-level analysis

Step 3: Plan agent reads findings
├─ Read("/tmp/agent-findings.json")
└─ Design implementation strategy
```

**Pros:** Scales; preserves structure; queryable.  
**Cons:** File I/O overhead; cleanup needed.

---

#### Strategy 3: SendMessage (Resume Existing Agent)

**Use for:** Multi-round conversation with same agent.

```
Message 1: Launch general-purpose agent
├─ Agent(description: "research X")
└─ Returns: findings + agent_id

Message 2: Continue with same agent
├─ SendMessage(to: agent_id, message: "Based on findings, now research Y")
└─ Returns: new findings (with context from message 1)

Message 3: Continue again
├─ SendMessage(to: agent_id, message: "Follow-up: clarify Z")
└─ Returns: clarification
```

**Pros:** 
- Preserves agent context across rounds
- No re-initialization overhead
- Natural follow-up conversation

**Cons:** 
- Only works within same agent invocation lifetime (not across new Agent calls)
- Limited to agent types that support continuation

**Agent Types Supporting SendMessage:**
- `claude` ✓
- `general-purpose` ✓
- `Explore` (read-only; limited continuation)
- `Plan` (read-only; limited continuation)
- `claude-code-guide` ✓ (recommended for multi-round questions)
- `statusline-setup` (single-shot; limited use)

---

### Multi-Agent Handoff Pattern

**Safe Handoff Checklist:**

```
Before spawning Agent B (which depends on Agent A result):

✓ Agent A completed and returned results
✓ Results are in a format Agent B can consume
✓ Agent B prompt explicitly references Agent A findings
✓ No shared file modifications (check via Plan or manual review)
✓ Gate is OPEN (if Agent B will execute Bash/Edit/Write)
```

**Example:**

```
Message 1: Research phase
├─ Agent(description: "research ConformanceVector usage",
         prompt: "find definition, usages, patterns")
└─ [Agent completes, returns file paths and analysis]

Message 2: Design phase (based on research)
├─ Agent(subagent_type: "Plan",
         description: "design refactoring",
         prompt: "Based on research findings [copy key points],
                  design a refactoring strategy")
└─ [Agent completes, returns design document]

Message 3: Verify gate is open
├─ Bash: lsp-max-cli gate check && echo "OK" || echo "BLOCKED"
│  [If output is "OK", proceed; if "BLOCKED", resolve diagnostics]
└─

Message 4: Implement phase (based on design)
├─ Agent(description: "implement refactoring",
         prompt: "Implement the approved plan: [paste plan]
                  Include gate-check preamble in all Bash operations")
└─ [Agent completes, returns implemented changes]
```

---

## Large Feature Implementation

### Checklist: Multi-Agent Feature Delivery

**Phase 1: Specification (0-1 hour)**

```
✓ Define feature scope (requirements document)
✓ Identify affected modules/files
✓ List test cases that verify the feature
✓ Identify any architectural constraints
✓ Estimate effort (rough)
```

**Phase 2: Research (1-2 hours)**

```
✓ Spawn general-purpose agent to research current implementation
✓ Identify integration points
✓ Find existing code that can be reused
✓ Document findings with file paths
```

**Phase 3: Design (1-2 hours)**

```
✓ Spawn Plan agent to design implementation strategy
✓ Review design for architectural soundness
✓ Identify critical path and dependencies
✓ Verify dependencies are not blocked by other work
✓ Approve design (human sign-off if team project)
```

**Phase 4: Implementation (varies)**

```
✓ If large (>5 files): fan-out multiple claude agents per module
✓ If small (<5 files): single claude agent
✓ Each agent includes gate-check preamble in Bash operations
✓ Each agent runs tests after implementation
✓ Consolidate changes (merge if parallel work)
```

**Phase 5: Integration Testing (varies)**

```
✓ Spawn claude agent to run full test suite
✓ Check for conflicts between parallel implementations
✓ Verify no regressions in other modules
✓ Run lsp-max-specific tests (cargo test -p anti-llm-cheat-lsp)
```

**Phase 6: Gate Check & Cleanup (5-10 minutes)**

```
✓ $ lsp-max-cli diagnostic snapshot
│  └ Verify no active diagnostics
✓ $ lsp-max-cli gate check
│  └ Verify gate is OPEN
✓ If diagnostics remain:
│  $ lsp-max-cli repair plan
│  └ Get suggested fixes; spawn claude agent to fix
✓ Re-run gate check
```

**Phase 7: Submission**

```
✓ Commit changes: git commit -m "..."
✓ Push changes: git push
│  └ PreToolUse hook runs gate check before push
✓ Create PR (if using GitHub)
✓ CI/CD pipeline runs (if configured)
```

---

## Continuous Integration Integration

### Setting Up Gated Builds in CI

**Example: GitHub Actions Workflow with Gate Check**

```yaml
name: Build & Test (Gate-Protected)

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Start lsp-max Compositor
        run: |
          # Start compositor (writes gate file)
          nohup lsp-max-compositor &
          sleep 2  # Wait for compositor startup
      
      - name: Gate Check
        run: lsp-max-cli gate check
        # If gate is BLOCKED (exit 1), build fails immediately
        # This prevents merging while ANDON signals are active
      
      - name: Cargo Test
        run: cargo test --workspace
      
      - name: Cargo Fmt Check
        run: cargo fmt --all -- --check
      
      - name: Cargo Clippy
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      
      - name: Post-Build Diagnostics
        if: always()  # Run even if tests fail
        run: lsp-max-cli diagnostic snapshot
        # Captures final diagnostic state for analysis
```

---

### Local Development Workflow (Compatible with CI)

**Developer machine setup:**

```bash
# 1. Install lsp-max CLI
cargo install --path crates/lsp-max-cli

# 2. Start compositor (writes gate file)
lsp-max-compositor &
COMPOSITOR_PID=$!

# 3. Develop normally
# - Open editor (Claude Code)
# - Make changes
# - PreToolUse hook enforces gate on Bash/Edit/Write

# 4. Before committing, check gate
lsp-max-cli gate check
# If exit 1: resolve diagnostics
# If exit 0: safe to commit

# 5. Commit and push
git commit -m "..."
git push  # PreToolUse hook checks gate again

# 6. Cleanup (optional)
kill $COMPOSITOR_PID
```

---

## Error Recovery & Rollback

### Handling Build Failures

**Scenario: Agent-generated code doesn't compile**

```
Step 1: Identify the error
├─ Bash: cargo build 2>&1 | head -50
│  └ Capture first compiler error
│
Step 2: Send error details to claude agent
├─ Agent(description: "fix compilation error",
│        prompt: "Fix this compiler error: [paste error output]
│                 File: [path], Line: [line]")
│  └ Agent fixes the error
│
Step 3: Verify fix
├─ Bash: cargo build
│  └ Confirm it compiles
│
Step 4: Run tests
├─ Bash: cargo test
│  └ Confirm tests pass
```

---

### Handling Test Failures

**Scenario: Agent-generated code fails tests**

```
Step 1: Run failing test with output
├─ Bash: cargo test <test_name> -- --nocapture
│  └ Capture full test output
│
Step 2: Diagnose root cause
├─ Agent(type: "general-purpose",
│        description: "diagnose test failure",
│        prompt: "Analyze this test failure: [paste output]
│                 The test is in: [path/to/test]
│                 What's the root cause?")
│  └ Agent identifies root cause
│
Step 3: Fix the issue
├─ Agent(description: "fix failing test",
│        prompt: "Based on root cause analysis: [findings],
│                 fix the code in [path] to make test pass")
│  └ Agent implements fix
│
Step 4: Re-run test
├─ Bash: cargo test <test_name>
│  └ Confirm test passes
```

---

### Rolling Back Changes (Emergency)

**If something goes wrong and you need to revert:**

```bash
# View recent commits
git log --oneline -10

# Show what changed in last commit
git show HEAD

# Revert the last commit (creates a new commit that undoes it)
git revert HEAD

# OR hard reset (destructive; only if commits not pushed)
git reset --hard HEAD~1

# Check gate after rollback
lsp-max-cli gate check
```

**Important:** 
- Use `git revert` for published commits (creates undo commit)
- Use `git reset --hard` only for unpublished local commits
- Gate may still be BLOCKED after rollback; resolve diagnostics if needed

---

## Debugging Agent Behavior

### Agent Won't Run or Times Out

**Checklist:**

```
✓ Gate is OPEN?
  $ lsp-max-cli gate check
  
✓ Prompt is valid (not too large)?
  Prompts >5000 tokens may hit limits
  
✓ Working directory exists and is accessible?
  All file paths must be absolute and valid
  
✓ For Bash operations: set -e?
  Failing silently can hide problems
  
✓ Check agent output for errors?
  If agent returns "<error>" block, read it carefully
```

---

### Agent Returns Incomplete Results

**Common causes:**

```
1. Timeout (search took too long)
   → Narrow search scope
   → Use "quick" breadth instead of "very thorough"
   
2. Token limit (large codebase)
   → Split task into smaller units
   → Use file-based handoff (write to /tmp, read next agent)
   
3. Search hit max results
   → Narrow grep pattern
   → Use more specific file glob
   
4. Agent chose wrong tool
   → Check agent's tool availability
   → Explicitly tell agent which tool to use
```

---

### Agent Uses Wrong Tool

**Example:** Explore agent (read-only) tries to edit files.

**Root cause:** Prompt asked for a write operation.

**Fix:** 
```
✓ Confirm agent type supports the operation
✓ Reword prompt to remove write request
✓ Use appropriate agent (claude for writes, Explore for reads)
```

---

## Performance Tuning

### Optimizing Multi-Agent Workflows

**Baseline:**
```
Explore (find files):      100-200ms
general-purpose (research): 2-5s
Plan (design):             2-5s
claude (implement):        5s + implementation time
```

**Optimizations:**

```
1. Use Explore for file location, not general-purpose
   Before: general-purpose searches, 5-10s
   After:  Explore finds files, <200ms → general-purpose analyzes, 2-5s
   
2. Combine searches into one agent
   Before: Explore(task A), Explore(task B), Explore(task C) = 600ms
   After:  Explore(tasks A, B, C combined) = 200ms
   
3. Fan-out independent agents
   Before: claude(feature A), claude(feature B) = N+M seconds (sequential)
   After:  claude(A) + claude(B) in parallel = max(N, M) seconds
   
4. Use SendMessage for follow-ups
   Before: Agent(question 1), Agent(question 2) = 2 × startup overhead
   After:  Agent(question 1), SendMessage(follow-up) = 1 × startup overhead
```

---

### Caching Agent Results

**Pattern: Save findings for reuse**

```
Step 1: Run research once
├─ general-purpose(prompt: "find all LSP methods")
├─ Write findings to /tmp/lsp-methods.json
└─ Time: 5-10s

Step 2: Reuse findings multiple times
├─ Agent(prompt: "Based on findings from /tmp/lsp-methods.json, ...")
├─ No search needed; direct analysis
└─ Time: 2-3s per agent

Benefit: First research pays cost; subsequent reuse is fast.
```

---

## Status: OPEN

This integration guide is CANDIDATE. As agent patterns mature, this document will be refined.

Contributions welcome: describe new patterns, integration scenarios, or improvements.

---
