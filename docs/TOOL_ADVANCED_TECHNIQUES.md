# Claude Code Tools: Advanced Techniques & Deep Dives

Comprehensive reference for power users: tool internals, edge cases, optimization techniques, and troubleshooting.

---

## Table of Contents

1. [Read Tool Advanced Usage](#read-tool-advanced-usage)
2. [Edit Tool Precision Techniques](#edit-tool-precision-techniques)
3. [Grep Power Searching](#grep-power-searching)
4. [Bash Advanced Patterns](#bash-advanced-patterns)
5. [Agent Optimization](#agent-optimization)
6. [Tool Interaction Gotchas](#tool-interaction-gotchas)
7. [Performance Profiling](#performance-profiling)
8. [Debugging Tool Issues](#debugging-tool-issues)

---

## Read Tool Advanced Usage

### Technique 1: Precise Section Reading with Offset

**Problem**: File is 5000 lines; you need lines 2500-2600.

```javascript
// Inefficient: reads all 2000 lines from start
Read(file_path: "/path/to/large.rs")

// Efficient: read only target section
Read(file_path: "/path/to/large.rs", 
     offset: 2499,           // start at line 2500 (0-indexed)
     limit: 101)             // read 101 lines (2500-2600)
```

**How offset works**:
- offset: 0 = start at line 1
- offset: 10 = start at line 11
- limit: n = read n lines total

**Use Case**: Large generated files, log files with millions of lines

---

### Technique 2: Progressive Reading Strategy

**Problem**: Don't know where to look; file is huge.

**Strategy**:
```
Step 1: Get overview
  Read(file_path, limit: 100)
  → See structure, imports, early content

Step 2: Use Grep to find target
  Grep(pattern: "target_symbol", path: file_path)
  → Get line numbers

Step 3: Read exact section
  Read(file_path, offset: line_num - 5, limit: 20)
  → Read target section + context
```

**Advantage**: Token-efficient; targeted reading

---

### Technique 3: Multi-Format File Handling

**Images**:
```
Read(file_path: "/path/to/screenshot.png")
→ Returns visual representation
→ Claude can analyze screenshots, diagrams, flowcharts
```

**Use Case**: Review UI changes, verify visual design, analyze diagrams

**PDFs** (with page limits):
```
Read(file_path: "/path/to/report.pdf", pages: "1-5")
→ Read pages 1-5 only
→ Max 20 pages per request
```

**Use Case**: Extract from long PDFs, research documents

**Jupyter Notebooks**:
```
Read(file_path: "/path/to/analysis.ipynb")
→ Returns all cells (code + output + markdown)
→ Preserves execution results, visualizations
```

**Use Case**: Review data analysis, verify computational results

---

### Technique 4: Reading Empty or Special Files

**Empty files**:
```
Read returns system warning
But operation succeeds
Treated as valid (not an error)
```

**Binary files**:
```
Read will fail or return garbage
Use Bash strings command instead: Bash(command: "strings file.bin")
```

**Very large files (> 10MB)**:
```
Read may timeout or hit limits
Use Bash tail/head: Bash(command: "head -100 file.log")
Or use offset + limit for sections
```

---

## Edit Tool Precision Techniques

### Technique 1: Context-Rich Indentation Matching

**Problem**: Indentation mismatch causes Edit failure.

```rust
// Source file (actual):
    pub fn foo() {
        return true;
    }

// Read output format:
42	    pub fn foo() {
43	        return true;
44	    }

// Correct Edit (strip line numbers, preserve indentation):
Edit(
  old_string: "    pub fn foo() {\n        return true;",
  new_string: "    pub fn pub async fn foo_async() {\n        return true;"
)

// Wrong Edit (indentation mismatch):
Edit(
  old_string: "pub fn foo() {",  // Missing leading spaces!
  new_string: "pub async fn foo_async() {"
)
// FAILS: Can't find match (indentation doesn't match)
```

**Key Rule**: Match indentation exactly, including leading spaces/tabs

---

### Technique 2: Multi-Line Edit with Boundary Context

**Problem**: Replace a function but multiple similar functions exist.

```rust
// Multiple functions exist:
pub fn update_a() { ... }
pub fn update() { ... }
pub fn update_user() { ... }

// Ambiguous edit (could match multiple):
Edit(old_string: "pub fn update() {",
     new_string: "pub fn update() -> Result {")
// Error: multiple matches

// Precise edit (boundary context):
Edit(
  old_string: "pub fn update() -> bool {\n    self.validate()",
  new_string: "pub fn update() -> Result {\n    self.validate()"
)
// Success: unique match
```

**Strategy**: Include function body, return type, parameters—anything unique

---

### Technique 3: Handling Multi-Line Strings and Comments

**Problem**: Edit fails because target spans multiple lines.

```rust
// Multi-line string literal
Edit(
  old_string: r#"Some long\nmulti-line\nstring"#,
  new_string: r#"Updated long\nmulti-line\nstring"#
)

// Multi-line comment
Edit(
  old_string: "/* TODO: This is a long\n   multi-line comment\n   describing work */",
  new_string: "/* DONE: Work completed */"
)

// Multi-line macro invocation
Edit(
  old_string: "vec![\n    1,\n    2,\n    3,\n]",
  new_string: "vec![1, 2, 3]"
)
```

**Gotcha**: Newlines in old_string must match exactly (be careful with CRLF vs LF)

---

### Technique 4: Atomic Edits vs Batch Edits

**Atomic**: One change per Edit call
```
Edit(pattern1, replacement1)
Edit(pattern2, replacement2)
Edit(pattern3, replacement3)
→ Three calls; each independent
→ Safe; if one fails, others unaffected
```

**Batch**: Replace all with replace_all
```
Edit(pattern, replacement, replace_all: true)
→ One call; replaces all occurrences
→ Fast but risky; one typo affects all
```

**Best Practice**: Use atomic edits for safety, batch for efficiency

```
Atomic pattern:
FOR pattern in patterns:
  Read(file) → verify context
  Edit(old_string: context, new_string: replacement)
  → Incremental; verifiable

Batch pattern:
Edit(pattern, replacement, replace_all: true)
→ Only if pattern is very specific and you're confident
```

---

### Technique 5: Handling Special Characters and Escaping

**Problem**: Edit fails because pattern contains special regex chars.

```
Edit uses **literal string matching**, NOT regex
So special characters don't need escaping in old_string

OLD (wrong - not needed):
Edit(old_string: "fn\\(\\)")

CORRECT (literal string):
Edit(old_string: "fn()")
```

**But**: In Grep, special chars DO need escaping
```
Grep(pattern: "fn\\(\\)")  // Regex needs escaping
Edit(old_string: "fn()")    // Literal string doesn't
```

---

## Grep Power Searching

### Technique 1: Multi-Pattern Simultaneous Search

**Goal**: Find multiple patterns across codebase efficiently.

```
Parallel strategy (one message):
  Grep(pattern: "fn foo", glob: "**/*.rs")
  Grep(pattern: "impl foo", glob: "**/*.rs")
  Grep(pattern: "use.*foo", glob: "**/*.rs")

All run in parallel
Results returned together
Total time: 1 × latency (not 3 ×)
```

**Use Case**: Symbol analysis, dependency tracking, pattern recognition

---

### Technique 2: Context Window Optimization

**Goal**: Get just enough context without overwhelming.

```
Minimal context (line numbers + match):
  Grep(pattern: "TODO", glob: "**/*.rs")
  → Just matching lines

More context (surrounding lines):
  Grep(pattern: "TODO", glob: "**/*.rs", context: 2)
  → 2 lines before + after

Maximum context (function scope):
  Grep(pattern: "TODO", glob: "**/*.rs", context: 10)
  → 10 lines before + after
```

**Trade-off**: More context = more tokens, but easier to understand

---

### Technique 3: Counting & Metrics via Grep

**Goal**: Gather code quality metrics.

```
Tech debt count:
  Grep(pattern: "TODO|FIXME", glob: "**/*.rs", 
       output_mode: "count")
  → Returns: 42 matches in src/

Risk areas:
  Grep(pattern: "unsafe|unwrap", glob: "**/*.rs", 
       output_mode: "count")
  → Returns: 12 unsafe blocks

Code duplication:
  Grep(pattern: "copy_paste_func", glob: "**/*.rs", 
       output_mode: "files_with_matches")
  → Returns: [file1, file2, file3]
```

**Use Case**: Baseline metrics, audit trails, risk assessment

---

### Technique 4: Anchored Regex for Precision

**Goal**: Match specific syntax (function definitions, imports, etc).

```
Function definition:
  Grep(pattern: "^fn \\w+\\(", glob: "**/*.rs")
  → ^ = line start; fn = keyword; \\w+ = name; \\( = paren

Import statement:
  Grep(pattern: "^use ", glob: "**/*.rs")
  → ^ = line start; use = keyword

Class definition:
  Grep(pattern: "^class \\w+", glob: "**/*.py")

Trailing comments:
  Grep(pattern: "\\S+\\s*//\\s*", glob: "**/*.rs")
  → Code followed by comment
```

**Advantage**: Anchors prevent false matches (e.g., "fn" in comment)

---

### Technique 5: Multiline Grep for Complex Patterns

**Goal**: Find patterns spanning multiple lines.

```
Without multiline:
  Grep(pattern: "fn foo\\(\\).*->.*Result")
  → Only matches if all on one line
  → Misses multi-line function signatures

With multiline:
  Grep(pattern: "fn foo\\([\\s\\S]*?->.*Result", 
       multiline: true)
  → [\\s\\S]*? = any chars including newlines
  → Matches across lines

Example use: Find all functions returning Result
  Grep(pattern: "fn \\w+\\([\\s\\S]*?-> Result", 
       glob: "**/*.rs", multiline: true)
```

**Gotcha**: Multiline patterns can match huge blocks; use non-greedy quantifiers (*?)

---

### Technique 6: Filter Chain for Precision

**Goal**: Narrow search through multiple filters.

```
Step 1: Find by type
  Grep(pattern: "foo", type: "rust")
  → Only .rs files

Step 2: Find in directory
  Grep(pattern: "foo", glob: "src/**/*.rs")
  → Only src directory

Step 3: Exclude unwanted
  Grep(pattern: "foo", glob: "src/**/*.rs", 
       path: "src/service")  // implicit path filter
  → Only in src/service

Result: Highly targeted, minimal false positives
```

---

## Bash Advanced Patterns

### Technique 1: Complex Command Chains

**Goal**: Multi-step operations with error handling.

```bash
# Basic chain (fails on first error):
cargo build && cargo test && cargo fmt

# With recovery (run all, check each):
cargo build; \
cargo test; \
cargo fmt; \
git status

# Conditional execution:
cargo test --lib || echo "Tests failed"
cargo clippy && echo "No clippy warnings"

# Pipe with error check:
cargo test 2>&1 | grep -E "test result|error"

# Background + notification:
./long_task.sh &
wait
echo "Task done"
```

---

### Technique 2: Absolute Paths in Scripts

**Problem**: Working directory resets between Bash calls; relative paths fail.

```
Wrong (breaks between calls):
  Bash(cd src && cargo build)  # cwd = root after
  Bash(cargo test)             # ERROR: not in src anymore

Right (explicit absolute paths):
  Bash(cargo build --manifest-path /home/user/project/Cargo.toml)
  Bash(cargo test --manifest-path /home/user/project/Cargo.toml)
```

**Key Rule**: Use absolute paths or -manifest-path / --cwd flags

---

### Technique 3: Process Substitution (advanced)

**Goal**: Compare outputs or run commands in subshells.

```bash
# Compare two builds:
diff <(cargo tree --release) <(cargo tree --dev)

# Aggregate logs:
<(find . -name "*.log" -exec cat {} \;) | sort

# Conditional based on output:
if grep -q "error" <(cargo build 2>&1); then
  echo "Build has errors"
fi
```

---

### Technique 4: Timeout Management

**Problem**: Long-running Bash calls block; can timeout.

```
Timeout defaults:
  - Default: 120 seconds (2 minutes)
  - Maximum: 600 seconds (10 minutes)

For short operations:
  Bash(command: "cargo test --lib", timeout: 60000)
  → 60 seconds (fast test suite)

For medium operations:
  Bash(command: "cargo build --release", timeout: 300000)
  → 300 seconds (5 minutes, slow build)

For long operations:
  Bash(command: "cargo test --all", run_in_background: true)
  → Background execution; wait for notification
  → No timeout limit (process runs independently)
```

**Strategy**: Use run_in_background for operations > 5 minutes

---

### Technique 5: Git Workflows with Bash

**Safe commit workflow**:
```bash
# 1. Check what changed
git status

# 2. Review diff
git diff

# 3. Stage specific files (not -A!)
git add src/specific_file.rs

# 4. Commit with message (from stdin):
git commit -m "fix: [issue description]"

# 5. Verify commit
git log -1 --stat
```

**Atomic operations**:
```bash
# All-or-nothing: build, test, commit
cargo build && cargo test && git add . && git commit -m "feat: new feature"
# If any step fails, whole chain stops (prevented partial commit)
```

**Preventing force operations**:
```
Force push requires explicit user request
Cannot use: git push --force
Instead ask user: "Push --force to deploy? Dangerous operation."
```

---

### Technique 6: Capturing Output for Analysis

**Goal**: Get command output for parsing/analysis.

```bash
# Capture with timestamp
cargo test 2>&1 | tee /tmp/test_output.txt
# Output goes to file AND stdout
# Later: Read(/tmp/test_output.txt) to analyze

# Capture stderr separately
cargo build 2>/tmp/build_errors.txt
# Later: Check /tmp/build_errors.txt for issues

# Count lines/errors
cargo test 2>&1 | grep -c "test result"
# Output: number of test result lines

# Extract specific info
git log --oneline --all | grep "pattern" | wc -l
# Output: commit count matching pattern
```

---

## Agent Optimization

### Technique 1: Surgical Briefing

**Problem**: Agent briefing is vague; output is generic.

```
Vague briefing:
  Agent(prompt: "Review the code")
  → Agent doesn't know which code, what concerns, format needed

Surgical briefing:
  Agent(
    prompt: """
    Review src/service.rs lines 42-80.
    Concerns: concurrency issues, memory safety, API compatibility.
    Report format: 
      [Concern]: [finding] at line [N]
      [Recommendation]: [action]
    """
  )
  → Agent knows exactly what to look for, where, and how to report
```

**Key**: Specific location, explicit concerns, required format

---

### Technique 2: Progressive Agent Tasks

**Problem**: Complex task; agent gets lost.

```
One massive task:
  Agent(prompt: "Implement feature X, refactor, test, document")
  → Too much; agent loses focus
  → Output is unfocused; needs multiple rounds

Progressive tasks:
  Task 1: Agent(prompt: "Design feature X; report: [specific format]")
  Task 2: Agent(prompt: "Implement X per design from Task 1; report: [format]")
  Task 3: Agent(prompt: "Add tests for X; report: [format]")
  Task 4: Agent(prompt: "Document X; report: [format]")
  → Each task focused
  → Output is structured
  → Progress is measurable
```

---

### Technique 3: Verification Pattern

**Problem**: Agent claims to have made changes; no guarantee.

```
Unverified:
  Agent(prompt: "Fix the bug in src/service.rs")
  → Agent returns "Fixed ✓"
  → But did it actually fix anything?

Verified:
  Agent(prompt: "Fix the bug in src/service.rs")
  
  Then:
  Read(file_path: "src/service.rs")  // Read actual file
  Grep(pattern: "bug_location")      // Verify fix is there
  Bash(cargo test)                   // Confirm tests pass
  
  → Hard evidence that fix was applied
```

**Rule**: Always verify agent work independently; don't trust summaries

---

### Technique 4: Context Window Management

**Goal**: Agent has limited context; brief efficiently.

```
Too much context:
  Include entire codebase in prompt
  → Agent becomes confused
  → Wastes tokens

Efficient context:
  Include only relevant:
    - Specific file paths (not entire tree)
    - Key function signatures (not all code)
    - Relevant examples (not all patterns)
    - Clear constraints (what NOT to do)
```

**Example**:
```
Agent(prompt: """
Review src/service.rs for security issues.
Focus on: authentication, authorization, input validation.
Ignore: styling, performance (separate review).
Context: This service handles user requests; must be production-safe.
Report: [Issue]: [description] at line [N]
""")
```

---

### Technique 5: Isolation & Cleanup

**Goal**: Complex operations in isolated environment.

```
Without isolation:
  Agent(prompt: "Make commits and push")
  → Changes happen directly on main branch
  → Risk if something breaks

With isolation:
  Agent(
    prompt: "Make commits and push",
    isolation: "worktree"
  )
  → Happens in temporary worktree
  → Main branch untouched
  → Can verify before merging
  
  Then:
  Bash(git merge temp_branch)  // Only merge after verification
```

---

## Tool Interaction Gotchas

### Gotcha 1: Read-Edit-Read Validation

**Problem**: Edit succeeds, but did it do what you wanted?

```
Risky:
  Edit(old_string, new_string)
  → Trust it worked
  → But maybe pattern was ambiguous

Safe:
  Edit(old_string, new_string)
  Read(file_path)  // Read back
  Grep(pattern: new_string, path: file_path)  // Verify new text is there
  → Hard proof of success
```

---

### Gotcha 2: Glob Results Ordering

**Problem**: Glob returns files in mtime order, not logical order.

```
Expected:
  src/module1.rs
  src/module2.rs
  src/module3.rs

Actual (mtime order):
  src/module2.rs  (edited 2 minutes ago)
  src/module1.rs  (edited 1 hour ago)
  src/module3.rs  (edited yesterday)

If order matters:
  Bash(find src -name "*.rs" -type f | sort)
  → Returns sorted results
```

---

### Gotcha 3: Grep Case Sensitivity

**Problem**: Grep respects case by default.

```
Looking for "TODO":
  Grep(pattern: "TODO")  ← Finds only uppercase
  Grep(pattern: "TODO|todo|Todo")  ← Finds all cases
  Grep(pattern: "todo", -i: true)  ← Case-insensitive search
```

---

### Gotcha 4: Mixed Tool State

**Problem**: Tools have independent views of file state.

```
Edit modifies file
Bash(cargo build) → sees new version (cwd is shared)

But:
Read(file_path) at time T1
Edit(...) changes file
Read(file_path) at time T2  ← Returns new content
→ Each Read is independent; no caching
```

---

### Gotcha 5: Indentation Tab vs Space

**Problem**: Mix of tabs/spaces causes Edit to fail.

```
File uses tabs:
  \tpub fn foo() {

But Edit uses spaces:
  Edit(old_string: "    pub fn foo() {",  // 4 spaces
       ...)
  → FAILS: Can't find match (indentation is tabs, not spaces)

Solution:
  Read(file_path) to see actual indentation
  Copy-paste from Read output (preserves exact whitespace)
```

---

## Performance Profiling

### Metric 1: Token Efficiency Calculation

**Goal**: Understand which tools consume most tokens.

```
Token costs (approximate):
  Read(small file):        100-500 tokens
  Read(large file):        2000+ tokens
  Edit(small change):      50-200 tokens
  Write(full file):        1000+ tokens
  Glob(1000 files):        50-100 tokens
  Grep(10 matches):        100-300 tokens
  Grep(100 matches):       500-1000 tokens
  Bash(simple):            50-200 tokens
  Bash(complex):           200-500 tokens
  Agent(simple):           500-2000 tokens
  Agent(complex):          5000+ tokens
```

**Optimization**: Use targeted searches + Read small sections

---

### Metric 2: Wall-Clock Time Estimation

**Goal**: Predict how long operations take.

```
Parallel execution (best case):
  Glob + Grep + Bash in one message
  → Total latency: 1 request roundtrip
  → Real time: 500ms - 2s

Sequential execution (worst case):
  Call 1, wait for result
  Call 2, wait for result
  Call 3, wait for result
  → Total latency: 3 request roundtrips
  → Real time: 2-6s

Long-running Bash:
  Bash(run_in_background: true)
  → Executes independently
  → Return time: <100ms
  → Execution time: until completion (may be minutes)
```

---

### Metric 3: Tool Selection by Speed

```
Fastest:
  Edit (small context)      < 100ms
  Glob                      < 100ms
  Grep (targeted)           < 200ms

Medium:
  Read (medium files)       100-500ms
  Bash (quick commands)     100-500ms
  Grep (broad search)       500-2000ms

Slow:
  Read (huge files)         1-5s
  Bash (build/test)         5-120s
  Agent (complex)           5-60s
  Write (large file)        500-2000ms
```

---

## Debugging Tool Issues

### Debug 1: Edit Fails with No Clear Reason

**Diagnosis**:
```
Try this checklist:
  ☐ Read file immediately before Edit
  ☐ Copy old_string EXACTLY from Read (no modifications)
  ☐ Check indentation: tabs or spaces?
  ☐ Add surrounding context to ensure uniqueness
  ☐ Look for hidden characters (CRLF vs LF)
  ☐ Escape special chars (but literal Edit doesn't need escaping)
  ☐ Is pattern actually in file? Try Grep
```

**Recovery**:
```
Read(file_path)
Grep(pattern: "search_for_exact_content", path: file_path)
→ Verify pattern exists and where

Then Edit with more context:
Edit(old_string: "3 lines of context",
     new_string: "3 lines updated")
```

---

### Debug 2: Grep Returns Nothing

**Diagnosis**:
```
Checklist:
  ☐ Is file in codebase? Use Glob to verify
  ☐ Is pattern too specific? Try simpler regex
  ☐ Regex syntax correct? Escape special chars
  ☐ Check case (TODO vs todo vs Todo)
  ☐ Multiline pattern needs flag? Use multiline: true
  ☐ File filtered out by .gitignore? Use Bash
  ☐ Wrong type filter? Try type: "rust" or remove type
```

**Recovery**:
```
Step 1: Verify files exist
  Glob(pattern: "**/*.rs")
  → Confirm files are searchable

Step 2: Simplify pattern
  Grep(pattern: "keyword")  ← Simple substring
  → If this works, pattern was too complex

Step 3: Use simpler pattern
  Grep(pattern: "key", glob: "src/**/*.rs")
  → More likely to match
```

---

### Debug 3: Bash Command Fails

**Diagnosis**:
```
Checklist:
  ☐ Is tool installed? Check with which/--version
  ☐ Correct working directory? Use absolute paths
  ☐ Correct permissions? Check chmod
  ☐ Correct command syntax? Run locally first
  ☐ Env vars set? Shell state doesn't persist
  ☐ Output redirected correctly? Use 2>&1 for stderr
  ☐ Timeout exceeded? Use run_in_background for long ops
```

**Recovery**:
```
Step 1: Diagnostic
  Bash(command: "which tool && tool --version")
  → Check tool is installed

Step 2: Simplify
  Bash(command: "ls /absolute/path")
  → Verify path and permissions

Step 3: Run with error capture
  Bash(command: "your_command 2>&1 | tail -50")
  → See actual error message

Step 4: Fix and retry
  Bash(command: "fixed_command")
```

---

### Debug 4: Agent Produces Bad Output

**Diagnosis**:
```
Checklist:
  ☐ Prompt is self-contained (agent sees no context)
  ☐ Prompt has specific location/file info
  ☐ Prompt has example format for output
  ☐ Prompt states constraints (what NOT to do)
  ☐ Too much context confuses agent? Pare down
  ☐ Task too big? Break into steps
```

**Recovery**:
```
Step 1: Improve briefing
  Add specific:
    - File paths (not "the code")
    - Line ranges (not "the function")
    - Constraints (not "be careful")
    - Format (not "tell me what you find")

Step 2: Verify independently
  Read actual files
  Grep for changes agent claims to have made
  Test changes with Bash
  → Don't trust agent summary

Step 3: Re-run if needed
  Bash(SendMessage) to agent with better prompt
  OR spawn new Agent with clearer briefing
```

---

## Summary: Performance Tuning Checklist

### Token Optimization
- [ ] Use Grep to find exact locations before Read
- [ ] Read only necessary sections (use limit + offset)
- [ ] Filter Glob/Grep results (type, glob, head_limit)
- [ ] Use output_mode: "count" or "files_with_matches" for metrics
- [ ] Avoid reading entire large files

### Speed Optimization
- [ ] Batch independent operations in one message
- [ ] Use run_in_background for long Bash commands
- [ ] Use specific patterns (anchored regex) for Grep
- [ ] Parallelize file searches (multiple Grep calls)
- [ ] Chain dependent Bash commands with &&

### Reliability Optimization
- [ ] Always Read before Edit
- [ ] Verify Edit worked (Read + Grep result)
- [ ] Verify Agent work independently
- [ ] Use atomic Edits (not replace_all) unless confident
- [ ] Test changes with Bash (run tests, build, lint)

### Safety Optimization
- [ ] Use absolute paths in Bash
- [ ] Batch write operations carefully
- [ ] Never force-push without user consent
- [ ] Commit frequently (checkpoint work)
- [ ] Verify diffs before committing

---

End of advanced techniques guide.
