# Claude Code Tool Composition Patterns & Decision Matrices

A practical guide to combining tools effectively, with decision flows and anti-patterns.

---

## Table of Contents

1. [Core Composition Patterns](#core-composition-patterns)
2. [Decision Matrices by Task](#decision-matrices-by-task)
3. [Anti-Patterns & How to Avoid Them](#anti-patterns--how-to-avoid-them)
4. [Parallel Execution Strategies](#parallel-execution-strategies)
5. [Sequential Workflows](#sequential-workflows)
6. [Error Recovery Patterns](#error-recovery-patterns)
7. [Performance Optimization](#performance-optimization)

---

## Core Composition Patterns

### Pattern A: Discover → Inspect → Modify

**Scenario**: You need to find, understand, and change code.

**Tools Sequence**:
1. **Discover**: Glob or Grep to locate files/symbols
2. **Inspect**: Read to understand context
3. **Plan**: Decide approach (Edit vs Write vs Agent)
4. **Modify**: Execute change (Edit or Write)
5. **Verify**: Read result or run tests

**Example: Rename a function parameter**

```
Step 1 (Glob):
  Glob(pattern: "**/*.rs", path: "src/")
  → Returns all Rust files

Step 2 (Grep):
  Grep(pattern: "fn old_param_name", glob: "*.rs")
  → Finds function definitions

Step 3 (Read):
  Read(file_path: "src/service.rs", limit: 100)
  → Understand function signature and usage

Step 4 (Edit):
  Edit(old_string: "fn foo(old_param_name: Type)", 
       new_string: "fn foo(new_param_name: Type)")

Step 5 (Verify):
  Bash(command: "cargo test")
  → Confirm changes work
```

**Gotchas**:
- Glob might return many files; add type/pattern constraints
- Edit requires exact old_string; add context to ensure uniqueness
- Multi-use parameters may need multiple edits
- Consider refactoring scope before starting

**When to Use**: Refactoring, bug fixes, adding parameters

---

### Pattern B: Parallel Search & Gather

**Scenario**: You need information from multiple sources at once.

**Tools Sequence**:
1. Launch multiple independent Grep/Bash calls in ONE message
2. Wait for all results
3. Analyze results together
4. Proceed with next phase

**Example: Architecture audit**

```
Single message with 4 parallel calls:

Bash(git status) → See uncommitted changes
Bash(cargo build --all 2>&1 | grep error) → Build errors
Grep(pattern: "TODO|FIXME", glob: "**/*.rs") → Tech debt
Grep(pattern: "unsafe|unwrap", glob: "**/*.rs") → Risk areas
```

**Return Format**:
```
[Bash result 1]
[Bash result 2]
[Grep result 1]
[Grep result 2]
```

**Gotchas**:
- All calls execute in parallel; results available together
- Working directory is persistent BUT shell state is not
- Can't pass state between parallel Bash calls
- Set head_limit on Grep to avoid token waste

**When to Use**: Status checks, multi-file analysis, gathering metadata

---

### Pattern C: Sequential Chain with Dependencies

**Scenario**: Each step depends on previous step's output.

**Tools Sequence**:
1. Use Bash with `&&` to chain dependent commands
2. OR use sequential tool calls with output-based logic

**Example: Build, test, then commit**

```
Single Bash call with chained commands:
  cargo build && cargo test && git status
```

**Alternative (sequential tool calls)**:
```
Call 1 (Bash):
  cargo build
  → If succeeds, proceed to Call 2

Call 2 (Bash):
  cargo test
  → If succeeds, proceed to Call 3

Call 3 (Bash):
  git diff
  → Review before Call 4

Call 4 (Bash):
  git add . && git commit -m "message"
```

**Gotchas**:
- && fails the whole chain if first command fails (desired for safety)
- Working directory persists; shell state does not
- Can't use cd effectively; use absolute paths
- For complex logic, consider Agent instead

**When to Use**: Build pipelines, deployment scripts, test suites

---

### Pattern D: Conditional Modification

**Scenario**: Only modify if certain conditions are met.

**Tools Sequence**:
1. Read file
2. Evaluate condition (in prompt logic)
3. IF condition true: Edit or Write
4. ELSE: Skip modification
5. Verify: Read result

**Example: Add import only if not already present**

```
Step 1 (Grep):
  Grep(pattern: "use std::collections::HashMap", glob: "src/lib.rs")
  
If NOT found:
  Step 2 (Read):
    Read(file_path: "src/lib.rs", limit: 50)
    
  Step 3 (Edit):
    Edit(old_string: "use std::io;",
         new_string: "use std::io;\nuse std::collections::HashMap;")
Else:
  Step 2: Skip (import already present)
```

**Gotchas**:
- Grep with head_limit might miss matches
- Conditions must be clear before execution
- Remember to handle both branches

**When to Use**: Defensive edits, feature flags, configuration updates

---

### Pattern E: Bulk Operations (Create/Update Many Files)

**Scenario**: Modify or create multiple files with similar changes.

**Tools Sequence**:
1. Find all files (Glob)
2. For each file:
   - Read (if exists)
   - Edit or Write
3. Verify all changes (Bash with tests)

**Example: Update license headers in all files**

```
Step 1 (Glob):
  Glob(pattern: "src/**/*.rs")
  → Returns list of files

Step 2-4 (For each file):
  FOR file in [list]:
    Read(file) → Get current content
    IF missing license header:
      Edit(old_string: "first line",
           new_string: "license header + first line")

Step 5 (Verify):
  Bash(cargo fmt && cargo clippy)
```

**Gotchas**:
- Large number of files can explode token usage
- Batch edits can have unintended scope
- Test each pattern before bulk apply
- Use dry-run (like `cargo fmt --check`) before real changes

**When to Use**: License updates, migration across files, bulk refactoring

---

### Pattern F: Isolated Multi-Step Operation (Worktree)

**Scenario**: Multi-step operation that needs isolation from main state.

**Tools Sequence**:
1. Agent(isolation: "worktree") with complex task
2. Agent returns branch/path information
3. Use returned path for next steps
4. Verify results before merging

**Example: Complex branch operation**

```
Step 1 (Agent with isolation):
  Agent(
    description: "Create and test feature branch",
    isolation: "worktree",
    prompt: "Create branch, make commits, run tests, report results"
  )
  → Returns branch_name + test_results

Step 2 (Verify):
  Read or Bash to check results from returned path

Step 3 (Integrate):
  Bash(git merge feature_branch)
  → Merge into main after verification
```

**Gotchas**:
- Worktree auto-cleans if no changes
- Returned path/branch is temporary
- Must extract useful artifacts before agent completes
- Verification is crucial

**When to Use**: Complex git workflows, multi-feature development, testing changes in isolation

---

### Pattern G: Expert Consultation (Agent Review)

**Scenario**: Need a second opinion or deeper analysis.

**Tools Sequence**:
1. Prepare code/context
2. Agent with specialized subagent_type
3. Compare agent findings with own analysis
4. Verify independently

**Example: Security code review**

```
Step 1 (Context preparation):
  Bash(git diff HEAD~1..HEAD) → Get diff
  
  OR
  
  Read(file_path) → Get code to review

Step 2 (Agent consultation):
  Agent(
    subagent_type: "code-reviewer",
    prompt: "Review code for [specific concern]; report: [format]"
  )
  → Returns findings

Step 3 (Independent verification):
  FOR each finding:
    Read(file_path)
    → Manually verify agent's claim

Step 4 (Act on findings):
  Edit or Write changes
  → Based on verified concerns
```

**Gotchas**:
- Agent's summary doesn't guarantee accuracy
- Must independently verify findings
- Agent doesn't see conversation history
- Brief agent comprehensively

**When to Use**: Code review, security analysis, design feedback, complex refactoring decisions

---

## Decision Matrices by Task

### Task 1: Find a Symbol or Function

```
flowchart
  A[Know exact filename?]
  A -->|YES| B[Use Read directly]
  A -->|NO| C{Search by name or content?}
  C -->|Name| D[Use Glob<br/>e.g., '**/*function*']
  C -->|Definition| E[Use Grep<br/>e.g., 'fn function\|function =']
  D --> F[Read matching files]
  E --> F
```

**Tools**:
- Glob("**/*symbol_name*") — fast filename search
- Grep("fn symbol_name|const symbol_name") — more precise
- Read(file_path) — get contents

**Time Complexity**:
- Glob: O(file count)
- Grep: O(file count × file size)
- Total: O(n) linear

---

### Task 2: Search Across Many Files for a Pattern

```
flowchart
  A[Type of search]
  A -->|Filename pattern| B[Glob + Read]
  A -->|Content/regex| C[Grep<br/>with type/glob filter]
  A -->|Complex expression| D[Grep<br/>with context flags]
```

**Tools**:
- Grep(pattern: "regex", glob: "*.rs", type: "rust") — Fast with filters
- Grep(pattern: "regex", context: 5) — Show context
- Glob(pattern: "**/*.rs") — Find files, then Grep

**Time Complexity**:
- Grep with filter: O(n × log(m)) where m = filtered files
- Unfiltered: O(n × size) where n = all files

**Optimization**:
```
FAST: Grep(pattern, type: "rust", glob: "src/**/*.rs")
SLOW: Grep(pattern)  ← Searches entire codebase
```

---

### Task 3: Understand File Structure

```
flowchart
  A[File size?]
  A -->|Small| B[Read entire file]
  A -->|Large| C[Read with limit]
  C --> D[Read subsequent<br/>sections with offset]
  B --> E[Analyze structure]
  D --> E
```

**Tools**:
- Read(file_path) — Full file (small files)
- Read(file_path, limit: 100, offset: 0) — Sections
- Grep(pattern, path: file_path) — Find specific content

**Best Practice**:
```
Step 1: Read(file_path, limit: 50)
Step 2: Grep(pattern, path: file_path)
Step 3: Read(file_path, offset: where_match_found)
```

---

### Task 4: Modify a Single File

```
flowchart
  A[Type of change?]
  A -->|Surgical/targeted| B[Edit<br/>+ Read first]
  A -->|Complete rewrite| C[Write<br/>+ Read if exists]
  A -->|Multi-location| D[Multiple Edits<br/>or Agent]
```

**Tools**:
- Edit(...) — Targeted string replacement (preferred for small changes)
- Write(...) — Full file replacement (use for complete rewrites)
- Agent — Complex multi-location refactoring

**Workflow**:
```
Surgical change:
  1. Read(file_path)
  2. Edit(old_string, new_string)
  3. (optional) Read to verify

Full rewrite:
  1. Read(file_path)
  2. Construct new content
  3. Write(file_path, new_content)
  4. Read to verify
```

---

### Task 5: Update Multiple Files with Same Change

```
flowchart
  A[Files affected?]
  A -->|Few| B[Manual Edit each]
  A -->|Many| C{Pattern consistent?}
  C -->|YES| D[Edit with replace_all<br/>or Agent]
  C -->|NO| E[Loop: Grep, Read, Edit]
```

**Tools**:
- Edit(..., replace_all: true) — Same pattern across files
- Loop Grep → Read → Edit — Variable patterns
- Agent — Complex refactoring with logic

**Workflow**:
```
Consistent pattern:
  FOR each file matching:
    Edit(old_pattern, new_pattern, replace_all: true)

Variable patterns:
  files = Glob(pattern)
  FOR file in files:
    context = Read(file)
    IF pattern matches:
      Edit(specific_old, specific_new)
```

---

### Task 6: Run Tests or Build

```
flowchart
  A[Complexity?]
  A -->|Single command| B[Bash<br/>e.g., 'cargo test']
  A -->|Pipeline| C[Bash chain<br/>with &&]
  A -->|Complex logic| D[Agent or Script]
  B --> E{Long-running?}
  E -->|Yes| F[run_in_background: true]
  E -->|No| G[Wait for result]
```

**Tools**:
- Bash(command: "simple_cmd") — Direct execution
- Bash(command: "cmd1 && cmd2 && cmd3") — Pipeline
- Bash(command: "...", run_in_background: true) — Long tasks
- Agent — Complex workflows

**Example Pipelines**:
```
Build → Test → Lint:
  cargo build && cargo test && cargo fmt --check

Setup → Test → Cleanup:
  ./setup.sh && cargo test && ./cleanup.sh
```

---

### Task 7: Gather Project Metadata

```
flowchart
  A[What to gather?]
  A -->|File structure| B[Glob<br/>+ optional Read]
  A -->|Git info| C[Bash<br/>git commands]
  A -->|Code metrics| D[Grep patterns<br/>+ count mode]
  A -->|Multiple sources| E[Parallel Bash calls]
```

**Tools**:
- Bash(git status, git log, git diff)
- Glob(pattern) — File inventory
- Grep(pattern, output_mode: "count") — Metric gathering
- Multiple Bash in one message — Parallel collection

**Example**:
```
Single message with parallel calls:

Bash(git status)
Bash(git log --oneline | head -10)
Bash(cargo test --lib 2>&1 | grep "test result")
Grep(pattern: "TODO|FIXME", output_mode: "count")
```

---

### Task 8: Code Review or Audit

```
flowchart
  A[Review scope?]
  A -->|Single file| B[Read + Grep<br/>+ manual analysis]
  A -->|Diff| C[Bash git diff<br/>+ Read changes]
  A -->|Expert opinion| D[Agent<br/>code-reviewer]
  A -->|Security| E[Agent<br/>+ Grep for risks]
```

**Tools**:
- Read(file_path) — Understand code
- Grep(pattern: "risky_pattern") — Find issues
- Bash(git diff) — See changes
- Agent(subagent_type: "code-reviewer") — Expert analysis
- Skill(code-review) — Structured review

**Workflow**:
```
Thorough review:
  1. Bash(git diff) → See all changes
  2. Read(changed_files) → Understand context
  3. Grep(risky_patterns) → Find potential issues
  4. Agent(code-reviewer) → Expert opinion
  5. FOR each finding: Read + evaluate
  6. Edit if issues found
```

---

## Anti-Patterns & How to Avoid Them

### Anti-Pattern 1: Sequential Calls for Independent Operations

❌ **Bad**:
```
Call 1: Bash(git status)
Call 2: Bash(ls -la)
Call 3: Bash(cargo --version)
Each call waits for previous result
Total time: 3 × latency
```

✅ **Good**:
```
Single message with 3 Bash calls:
Bash(git status)
Bash(ls -la)
Bash(cargo --version)
All run in parallel
Total time: 1 × latency
```

**Why It Matters**: Token efficiency, wall-clock time

---

### Anti-Pattern 2: No Context Before Edit

❌ **Bad**:
```
Edit(old_string: "function old_name",
     new_string: "function new_name")
Fails because "function old_name" appears 5 times
```

✅ **Good**:
```
Read(file_path) → understand structure
Edit(old_string: "pub fn old_name(param: Type) ->",
     new_string: "pub fn new_name(param: Type) ->")
Unique pattern succeeds
```

**Why It Matters**: Edit success, prevents wrong replacements

---

### Anti-Pattern 3: Writing Documentation Files Unprompted

❌ **Bad**:
```
User: "Find unused imports"
Agent: Finds them, creates CLEANUP.md with results
File never read; context pollution
```

✅ **Good**:
```
User: "Find unused imports"
Agent: Returns findings as text message (no file)
User can read summary directly
```

**Why It Matters**: Token efficiency, user focuses on critical work

---

### Anti-Pattern 4: Over-broad Glob Patterns

❌ **Bad**:
```
Glob(pattern: "**/*")
Returns tens of thousands of results
Overwhelming; hard to filter
```

✅ **Good**:
```
Glob(pattern: "src/**/*.rs")
OR
Glob(pattern: "**/*.{js,ts}")
Targeted results
```

**Why It Matters**: Token usage, clarity

---

### Anti-Pattern 5: Forgetting Line Number Prefixes in Edit

❌ **Bad** (after Read):
```
Read returns:
  42	pub fn old_function() {

Edit fails:
  Edit(old_string: "42\tpub fn old_function",
       new_string: "pub fn new_function")
Line number included in old_string!
```

✅ **Good**:
```
Strip line number + tab:
  Edit(old_string: "pub fn old_function",
       new_string: "pub fn new_function")
```

**Why It Matters**: Edit success

---

### Anti-Pattern 6: Relying on Agent Reports Without Verification

❌ **Bad**:
```
Agent(code-reviewer) → Returns findings
Trust findings without checking
→ Risk of false positives / missed issues
```

✅ **Good**:
```
Agent(code-reviewer) → Returns findings
FOR each finding:
  Read(file_path) → Manually verify
  Confirm accuracy
  Act on verified findings
```

**Why It Matters**: Correctness, catching agent mistakes

---

### Anti-Pattern 7: Chaining Too Many Operations in Single Bash

❌ **Bad**:
```
Bash(cargo build && cargo test && cargo clippy && \
     cargo fmt && git diff && git status)
If any command fails, all stop
Hard to debug which part failed
```

✅ **Good**:
```
Bash(cargo build && cargo test)
→ Check result
Then:
Bash(cargo clippy && cargo fmt)
→ Separate concerns
```

**Why It Matters**: Debugging, fault isolation

---

### Anti-Pattern 8: Using Write Instead of Edit for Small Changes

❌ **Bad**:
```
Read(file_path) → get 500 LOC
Change 1 line
Write(file_path, all_content) → 500 LOC through tool
Inefficient token usage
```

✅ **Good**:
```
Read(file_path) → understand context (optional full read)
Edit(old_string: context, new_string: new_value)
One-liner replacement
```

**Why It Matters**: Token efficiency, simplicity

---

### Anti-Pattern 9: Unspecified Agent Prompts

❌ **Bad**:
```
Agent(prompt: "Review the code")
Agent has no context:
- Which code?
- What concerns?
- What format for findings?
Agent's response is vague/generic
```

✅ **Good**:
```
Agent(
  prompt: "Review src/service.rs lines 42-80 for race conditions. \
           Report: [list of potential issues with line numbers]"
)
Agent has context; findings are actionable
```

**Why It Matters**: Agent usefulness, actionable results

---

### Anti-Pattern 10: Using Bash When Dedicated Tool Exists

❌ **Bad** (unnecessary Bash):
```
Bash(command: "cat src/main.rs")
When Read tool exists
Bash(command: "grep pattern *.rs")
When Grep tool exists
Bash(command: "find . -name '*.rs'")
When Glob tool exists
```

✅ **Good**:
```
Read(file_path: "src/main.rs")
Grep(pattern: "pattern", glob: "*.rs")
Glob(pattern: "**/*.rs")
More efficient, better error handling
```

**Why It Matters**: Token usage, reliability, permissions

---

## Parallel Execution Strategies

### Strategy 1: File Discovery Parallelization

**Goal**: Search multiple file patterns concurrently

```
Single message with parallel Glob calls:

Glob(pattern: "**/*.rs")
Glob(pattern: "**/*.toml")
Glob(pattern: "**/Makefile")
Glob(pattern: "**/README.md")

→ All patterns searched in parallel
→ Results aggregated
→ Total time: 1 × latency (not 4 ×)
```

**Use Case**: Audit, inventory, architecture mapping

---

### Strategy 2: Metadata Gathering Parallelization

**Goal**: Collect info from multiple sources

```
Single message with parallel Bash calls:

Bash(git status)
Bash(cargo tree | head -20)
Bash(ls -la target/)
Bash(git log --oneline | head -5)

→ All execute concurrently
→ Status snapshot captured
→ Total time: 1 × latency (not 4 ×)
```

**Use Case**: Status checks, diagnostics, pre-flight validation

---

### Strategy 3: Multi-Pattern Grep Parallelization

**Goal**: Search for multiple patterns across codebase

```
Single message with parallel Grep calls:

Grep(pattern: "TODO", glob: "**/*.rs", output_mode: "count")
Grep(pattern: "FIXME", glob: "**/*.rs", output_mode: "count")
Grep(pattern: "unsafe", glob: "**/*.rs", output_mode: "count")
Grep(pattern: "unwrap", glob: "**/*.rs", output_mode: "count")

→ All patterns searched in parallel
→ Tech debt + risk metrics gathered
→ Total time: 1 × latency (not 4 ×)
```

**Use Case**: Metrics gathering, risk assessment, code quality baseline

---

### Strategy 4: Mixed-Tool Parallelization

**Goal**: Different tools gathering different information

```
Single message with mixed calls:

Glob(pattern: "**/*.rs")
Bash(git log --stat)
Grep(pattern: "impl.*Trait", glob: "**/*.rs")
Read(file_path: "Cargo.toml")

→ All execute concurrently
→ File structure + change history + architecture + deps → holistic view
→ Total time: 1 × latency
```

**Use Case**: Comprehensive codebase analysis, baseline before refactoring

---

### Parallelization Rules

**Rule 1**: Tools are independent
```
OK to parallelize: Read, Glob, Grep, Bash (safe commands)
NOT OK: Edits to same file, dependent Bash chains
```

**Rule 2**: No shared state assumptions
```
Can't use output from Call 1 in Call 2 (called in parallel)
Each call is independent
```

**Rule 3**: Batch in single message
```
To parallelize: Send all tools in ONE message
Each tool call in separate tool block
They execute concurrently
```

**Rule 4**: Large result sets
```
Each parallel call can return large output
Total output = sum of all calls
Manage with head_limit, output_mode filters
```

---

## Sequential Workflows

### Workflow 1: Refactor with Safety Checks

```
Step 1 (Analyze):
  Bash(cargo test --lib)
  Bash(cargo build)
  → Ensure baseline passing

Step 2 (Find locations):
  Grep(pattern: "old_name", glob: "**/*.rs", output_mode: "files_with_matches")
  → Identify all files affected

Step 3 (For each file):
  Read(file_path)
  Grep(pattern: "old_name", path: file_path, context: 3)
  → Understand context
  
  Edit(old_string: context, new_string: new_name)
  → Make change

Step 4 (Verify):
  Bash(cargo test --lib)
  Bash(cargo clippy)
  → Ensure refactoring correct

Step 5 (Commit):
  Bash(git add -A && git commit -m "refactor: rename old_name to new_name")
  → Checkpoint work
```

**Safety Features**:
- Baseline tests pass before changes
- Each change made deliberately (Read before Edit)
- Tests re-run after all changes
- Changes committed with message

---

### Workflow 2: Bug Fix with Root Cause Analysis

```
Step 1 (Reproduce):
  Bash(cargo test bug_test -- --nocapture)
  → See failure

Step 2 (Locate):
  Grep(pattern: "bug_symptom", glob: "**/*.rs")
  → Find related code

Step 3 (Analyze):
  Read(file_path)
  → Understand root cause
  
  Grep(pattern: "function_call", path: file_path)
  → Find callers

Step 4 (Fix):
  Read(file_path)
  Edit(old_string: buggy_code, new_string: fixed_code)

Step 5 (Verify):
  Bash(cargo test bug_test)
  → Test now passes

Step 6 (Regression):
  Bash(cargo test)
  → All tests pass

Step 7 (Commit):
  Bash(git add -A && git commit -m "fix: [issue description]")
```

**Safety Features**:
- Reproduce bug before fixing
- Understand cause before changing
- Verify fix works
- Regression test all tests
- Clear commit message

---

### Workflow 3: Feature Implementation

```
Step 1 (Design):
  Read(related_files) → understand existing patterns
  Agent(prompt: "Design review for [feature]") → Expert input

Step 2 (Implement):
  FOR each component:
    Read(file_path) → understand structure
    Edit or Write changes
    Bash(cargo test) → unit tests pass

Step 3 (Integration Test):
  Bash(cargo test --all)
  → All tests pass

Step 4 (Code Review):
  Bash(git diff) → see all changes
  Agent(subagent_type: "code-reviewer") → expert review
  
  FOR each finding:
    Read(file_path)
    Evaluate finding
    Edit if needed

Step 5 (Lint):
  Bash(cargo fmt && cargo clippy)
  → Code clean

Step 6 (Commit):
  Bash(git add -A && git commit -m "feat: [feature description]")
```

**Safety Features**:
- Design review before coding
- Unit tests per component
- Integration test all together
- Code review before commit
- Lint enforced
- Clear commit message

---

## Error Recovery Patterns

### Recovery Pattern 1: Edit Fails (old_string not found)

```
Step 1: Edit fails
  Error: "old_string not found"

Step 2: Diagnose
  Read(file_path)
  → Check what's actually there

Step 3: Analyze mismatch
  Possible causes:
    - Indentation (tabs vs spaces)
    - Context changed (line wrapped differently)
    - Pattern is too specific
    - File already modified

Step 4: Fix
  Edit(old_string: broader_context, new_string: ...)
  → Include more context to ensure uniqueness
  
  OR
  
  Use replace_all: true
  → Replace all matching patterns

Step 5: Verify
  Read(file_path) → confirm change
```

**Prevention**:
- Always Read before Edit
- Include surrounding context in old_string
- Verify indentation matches exactly

---

### Recovery Pattern 2: Bash Command Fails

```
Step 1: Bash fails
  Error: "command not found" OR "exit code 1"

Step 2: Diagnose
  Bash(command: "which command_name")
  → Check if installed
  
  Bash(command: "cargo --version")
  → Check tool versions

Step 3: Understand failure
  Bash(command: "previous_command 2>&1 | tail -20")
  → See error output

Step 4: Fix
  Root cause determines action:
    - Missing tool: Install or use absolute path
    - Wrong directory: Use absolute path (not cd)
    - Syntax error: Fix command, re-run
    - Permission: Check chmod, use sudo if needed

Step 5: Retry
  Bash(fixed_command)
  → Confirm success
```

**Prevention**:
- Use absolute paths, never cd-dependent commands
- Check tools are installed before using
- Pipe stderr to stdout with 2>&1
- Use set -e for scripts (exit on first error)

---

### Recovery Pattern 3: Grep Returns No Results

```
Step 1: Grep returns empty
  No matches for pattern

Step 2: Diagnose
  Possibilities:
    - Pattern is too specific
    - Regex syntax error
    - Pattern doesn't exist in files
    - Files are filtered (glob/type)

Step 3: Verify pattern
  Try simpler pattern:
    Grep(pattern: "keyword", glob: "**/*.rs")
    → Search for substring instead of regex

Step 4: Check files
  Glob(pattern: "**/*.rs")
  → Verify files exist in expected locations

Step 5: Widen search
  Grep(pattern: "partial_pattern", output_mode: "files_with_matches")
  → Find relevant files first
  
  Then:
  Grep(pattern: "specific_pattern", path: specific_file)
  → Search specific file

Step 6: Succeed
  Now have accurate pattern + file list
```

**Prevention**:
- Start with broad patterns; narrow down
- Escape special regex characters
- Use output_mode: "files_with_matches" first to find relevant files

---

## Performance Optimization

### Optimization 1: Token Efficiency

**Goal**: Use tokens wisely for large codebases

```
Inefficient:
  Read(file_path)  ← Read entire 2000+ LOC file
  Edit small change
  Write(...)  ← Write entire file back

Efficient:
  Read(file_path, limit: 100)  ← Read beginning
  Grep(pattern, path: file_path)  ← Find exact location
  Read(file_path, offset: line_num, limit: 50)  ← Read section
  Edit(old_string: context, new_string: ...)
```

**Strategy**:
- Use Grep to find exact locations first
- Read only sections needed
- Limit result sets (head_limit on Grep)
- Use output_mode to filter (count, files_with_matches)

---

### Optimization 2: Parallelization for Speed

**Goal**: Reduce wall-clock time

```
Slow (sequential):
  Call 1: Bash(git status)
  Call 2: Bash(git log)
  Call 3: Bash(cargo build)
  Total latency: 3 × latency

Fast (parallel):
  Single message:
    Bash(git status)
    Bash(git log)
    Bash(cargo build)
  Total latency: 1 × latency
```

**Strategy**:
- Batch independent operations
- Run in single message
- Reduces overall time significantly

---

### Optimization 3: Pattern Specificity

**Goal**: Return fewer, more relevant results

```
Broad pattern (slow):
  Grep(pattern: "foo", glob: "**/*.rs")
  → Searches all files; may return hundreds of matches

Specific pattern (fast):
  Grep(pattern: "^fn foo\\(", glob: "src/**/*.rs", head_limit: 10)
  → Anchored regex, filtered glob, limited results

Trade-off:
  More specific = fewer results, less to parse
  Less specific = more results, more to filter
```

**Strategy**:
- Start with specific patterns
- Use type filters (--type rust)
- Use glob filters (--glob src/**)
- Set head_limit to prevent explosion

---

### Optimization 4: Caching/Reuse

**Goal**: Avoid re-reading same files

```
Inefficient:
  Read(file_path) → get structure
  Grep(pattern1, path: file_path)
  Read(file_path) → re-read same file
  Grep(pattern2, path: file_path)
  Read(file_path) → re-read again

Efficient:
  Read(file_path) → cache structure
  Use cached understanding for patterns
  Grep(pattern1, path: file_path)
  Grep(pattern2, path: file_path)
  → Both use same file read; one Read call
```

**Strategy**:
- Plan all changes before reading
- Read once; use result for multiple purposes
- Batch searches on same file

---

## Quick Reference: Tool Combinations

### Find & Read
```
Glob(pattern) → Read(result)
```

### Find & Modify
```
Glob(pattern) → Read(result) → Edit(...)
```

### Search & Modify
```
Grep(pattern) → Read(context) → Edit(...)
```

### Audit & Review
```
Grep(risky_pattern) → Read(matches) → Review findings
```

### Verify & Fix
```
Bash(test) → (if fails) Grep(issue) → Read(code) → Edit(fix)
```

### Parallel Status Check
```
Bash(git status) + Bash(git log) + Bash(cargo build) [parallel]
```

### Safe Refactor
```
Bash(test baseline) → Glob(files) → FOR Read + Edit → Bash(test final)
```

### Expert Review
```
Read(code) → Agent(code-reviewer) → FOR findings: Read(verify)
```

---

## Summary: Decision Tree

```
START
  ↓
What's the goal?
  ├─ Find code? 
  │  ├─ By filename? → Glob
  │  └─ By content? → Grep
  │     └─ Then Read
  │
  ├─ Understand code?
  │  ├─ Know path? → Read
  │  └─ Don't know? → Glob/Grep first
  │
  ├─ Modify code?
  │  ├─ Small change? → Read first, then Edit
  │  └─ Full rewrite? → Read if exists, then Write
  │
  ├─ Run commands?
  │  ├─ Simple? → Bash
  │  └─ Complex/dependent? → Bash chain (&&) or Agent
  │
  ├─ Get expert opinion?
  │  └─ Agent(subagent_type: "code-reviewer")
  │
  └─ Multiple independent tasks?
     └─ Batch in single message (parallelize)
```

---

End of tool composition guide.
