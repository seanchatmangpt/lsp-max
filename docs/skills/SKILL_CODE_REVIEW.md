# Skill: /code-review

**Status:** AVAILABLE | **Scope:** Code Quality Assurance | **Category:** Validation & Verification

---

## Overview

Review the current diff for correctness bugs, inefficiency, and opportunities for simplification and reuse. The `/code-review` skill can post findings as inline PR comments or apply auto-fixes directly to your working tree.

**Key distinction:** `/verify` validates behavior; `/code-review` finds bugs and improves code quality.

## When to Use

Use `/code-review` when you want to:
- Find logic errors and bugs in changed code
- Identify inefficient patterns (O(n²), unnecessary allocations)
- Spot missing null checks, off-by-one errors, race conditions
- Post findings as PR comments
- Auto-apply safe refactoring recommendations

**Do NOT use `/code-review` for:**
- Finding security vulnerabilities (use `/security-review` instead)
- Pure refactoring without bug-hunting (use `/simplify` instead)
- Testing behavior (use `/verify` instead)

## Parameters

```bash
/code-review [effort] [--comment] [--fix]
```

| Parameter | Type | Optional? | Default | Examples |
|-----------|------|-----------|---------|----------|
| `effort` | Choice: low, medium, high, max | Yes | context-dependent | `low`, `high` |
| `--comment` | Flag | Yes | Not set | `--comment` (post as PR comments) |
| `--fix` | Flag | Yes | Not set | `--fix` (apply auto-fixes) |

## Invocation

```bash
# Basic review (report only)
/code-review

# Review at specific effort level
/code-review low
/code-review high
/code-review max

# Post findings as PR comments
/code-review --comment
/code-review high --comment

# Auto-apply safe fixes
/code-review --fix
/code-review medium --fix

# Both: detailed review with comments and auto-fixes
/code-review high --comment --fix
```

## Effort Levels

The skill scales investigation depth based on effort level:

### low — Essential bugs only

- **Coverage:** Core logic paths, obvious errors
- **Findings:** Fewer, highest-confidence bugs
- **Examples:** Null pointer dereference, array out-of-bounds, infinite loops
- **Time:** ~30 seconds per file
- **Confidence:** Very high; almost no false positives

```
/code-review low
  Checks: null checks, type errors, obvious logic bugs
  Findings: ~1-2 per file (if any)
  Confidence: 95%+
```

### medium — Common patterns (default)

- **Coverage:** Logic + common pitfalls (resource leaks, race conditions, inefficiency)
- **Findings:** Moderate scope with good confidence
- **Examples:** Unclosed file handles, missing bounds checks, O(n²) loops, unused variables
- **Time:** ~1-2 minutes per file
- **Confidence:** High; occasional false positives

```
/code-review medium
  Checks: logic, resource management, performance, unused code
  Findings: ~3-5 per file
  Confidence: 80-90%
```

### high — Broad scope

- **Coverage:** Logic, performance, style, architecture
- **Findings:** Broader, may include style issues and design suggestions
- **Examples:** Cache misses, deep nesting, missing documentation, inconsistent naming
- **Time:** ~3-5 minutes per file
- **Confidence:** Variable; may include some subjective findings

```
/code-review high
  Checks: logic, perf, style, architecture, consistency
  Findings: ~5-10 per file
  Confidence: 70-85%
```

### max — Exhaustive

- **Coverage:** All categories including edge cases and exploratory findings
- **Findings:** Maximum scope; deep analysis
- **Examples:** Complex interactions, theoretical race conditions, performance edge cases
- **Time:** ~10+ minutes per file
- **Confidence:** Lower; includes exploratory/uncertain findings

```
/code-review max
  Checks: everything (logic, perf, style, edge cases, design)
  Findings: ~10-20+ per file
  Confidence: 50-70%
```

## Finding Severity

Each finding is categorized by severity:

| Severity | Meaning | Action |
|----------|---------|--------|
| **CORRECTNESS** | Logic error, bug, or undefined behavior | **Must fix before merge** |
| **EFFICIENCY** | Performance issue or inefficiency | **Should fix** |
| **STYLE** | Naming, consistency, or formatting | **Nice to have** |

### Example Finding (CORRECTNESS)

```
Line 42: Off-by-one error in loop bounds
  Code: for (int i = 0; i <= items.len(); i++) {
                        ^^ Should be i < items.len()
  Problem: Accesses items[items.len()], which is out of bounds
  Suggestion: Change <= to <
```

### Example Finding (EFFICIENCY)

```
Line 15: Inefficient string concatenation
  Code: result = result + item.to_string();  // in a loop
  Problem: Creates new String on each iteration (O(n²) complexity)
  Suggestion: Use StringBuilder or Vec<String> then join()
  Impact: ~100x faster for 1000 items
```

### Example Finding (STYLE)

```
Line 8: Inconsistent naming convention
  Code: let user_data = ...;  // snake_case used elsewhere
        let userData = ...;    // camelCase here
  Problem: Inconsistent with file style (snake_case is standard)
  Suggestion: Rename to user_data
```

## Review Categories

The skill examines code across five categories:

### 1. Correctness

- **Logic errors** — If/else conditions, loop invariants
- **Null safety** — Missing null checks, unwrap without guard
- **Bounds checking** — Array/vector access, off-by-one errors
- **Type safety** — Type conversions, implicit coercions
- **Concurrency** — Data races, use-after-free, deadlocks
- **Resource management** — Unclosed handles, memory leaks

Example:
```rust
// ❌ CORRECTNESS: Missing bounds check
let value = array[index];  // What if index >= array.len()?

// ✅ Better: Guard the access
let value = array.get(index)?;
```

### 2. Efficiency

- **Time complexity** — O(n²) loops, unnecessary scans
- **Space complexity** — Redundant allocations, inefficient data structures
- **Cache misses** — Iteration order, stride patterns
- **Allocation hotspots** — Creating objects in tight loops
- **Redundant computation** — Repeated calculations

Example:
```rust
// ❌ EFFICIENCY: Repeated string parsing
for item in items {
  let config = Config::parse(&config_str)?;  // Reparsed 1000x
  process(item, &config);
}

// ✅ Better: Parse once
let config = Config::parse(&config_str)?;
for item in items {
  process(item, &config);
}
```

### 3. Simplification

- **Dead code** — Unused variables, unreachable branches
- **Redundancy** — Duplicated logic, code near misses
- **Over-engineering** — Unnecessary abstraction, premature generalization
- **Nesting depth** — Multiple levels of if/match that can be flattened
- **Intermediate variables** — Unnecessary binding steps

Example:
```rust
// ❌ SIMPLIFICATION: Unnecessary intermediate variable
let is_valid = validate(&input);
if is_valid {
  process();
}

// ✅ Better: Direct call
if validate(&input) {
  process();
}
```

### 4. Reuse

- **Duplicated logic** — Same pattern appears 2+ times
- **Missing abstractions** — Code that could be extracted into a helper
- **Library opportunities** — Using `for` when `filter().map()` would be clearer
- **Inconsistent patterns** — Same concept done differently in different places

Example:
```rust
// ❌ REUSE: Duplicated validation
if user.name.is_empty() || user.name.len() > 100 { ... }
if product.name.is_empty() || product.name.len() > 100 { ... }

// ✅ Better: Extract into helper
fn validate_name(name: &str) -> bool {
  !name.is_empty() && name.len() <= 100
}
```

### 5. Architecture/Design

- **Layering violations** — High-level code calling low-level details
- **Separation of concerns** — Mixed responsibilities
- **Data flow** — Mutation vs. immutability patterns
- **Error handling** — Inconsistent strategies across module

Example:
```rust
// ❌ ARCHITECTURE: Service layer directly touching storage
impl UserService {
  fn create_user(&self, name: &str) {
    let user = User { id: uuid(), name };
    fs::write(&format!("users/{}.json", user.id), serialize(&user))?;
  }
}

// ✅ Better: Delegate to repository
impl UserService {
  fn create_user(&self, name: &str) {
    let user = User { id: uuid(), name };
    self.repo.save(&user)?;
  }
}
```

## Flags

### --comment

**Post findings as inline PR comments** (requires active PR context)

- Creates a comment for each finding
- Includes line number, code snippet, and suggestion
- Helpful for code review workflows

```bash
/code-review --comment
# Output: PR comments created (visible on GitHub/GitLab)
```

**Only works if:**
- You have an active pull request (detected via git)
- You're on a feature branch (not main/master)

### --fix

**Auto-apply safe refactoring fixes** to working tree

- Automatically applies fixes for safe suggestions
- Creates a new commit (never amends)
- Commit message: `refactor: <summary of changes>`

```bash
/code-review --fix
# Output: New commit created with auto-fixes applied
```

**Safe auto-fixes include:**
- Removing dead code
- Simplifying obvious patterns (unnecessary variables, if/else flattening)
- Extracting small helpers (< 10 lines)
- Adding missing guards/checks

**NOT auto-fixed (manual review required):**
- Algorithm changes
- API modifications
- Complex refactoring

## Expected Output

### Basic Review (Report Only)

```
🔍 Code Review: lsp-max v26.6.9

Files analyzed: src/service.rs (128 lines changed)

Findings (medium effort):

1. ❌ CORRECTNESS - Line 42: Null pointer dereference
   Code: let value = data[index];
   Problem: No bounds check; index may be out of range
   Suggestion: Use data.get(index)? or bounds check first
   Confidence: 100%

2. ⚠️  EFFICIENCY - Line 56: O(n²) pattern
   Code: for item in items { find_in(&item, &big_list); }
   Problem: find_in does linear search; repeated 1000x
   Suggestion: Sort big_list, use binary search, or use HashSet
   Impact: ~50x speedup for 1000 items

3. 💡 STYLE - Line 78: Inconsistent naming
   Code: let userData = ...;
   Problem: File uses snake_case; this is camelCase
   Suggestion: Rename to user_data

Summary:
  Findings: 3 total
  - CORRECTNESS: 1 (must fix)
  - EFFICIENCY: 1 (should fix)
  - STYLE: 1 (nice to have)
  
Status: CANDIDATE (1 correctness issue must be addressed)
Next: Fix CORRECTNESS issue, then run /verify again
```

### With --comment Flag

```
🔍 Code Review: src/service.rs (128 lines)
Posting findings as PR comments...

✅ Posted 3 inline comments to PR #42:
  - Comment 1 (Line 42): Null pointer dereference
  - Comment 2 (Line 56): O(n²) pattern inefficiency
  - Comment 3 (Line 78): Inconsistent naming

PR comments visible at: https://github.com/user/repo/pull/42
Status: ADMITTED (comments posted; awaiting author feedback)
```

### With --fix Flag

```
🔍 Code Review: src/service.rs (128 lines)
Applying auto-fixes...

✅ Auto-fixes applied:
  - Line 78: Rename userData → user_data (consistency)
  - Line 42: Add bounds check with .get() (correctness)
  - Line 100: Remove unused variable debug_flag (dead code)

New commit created: refactor: simplify service logic

Commits:
  Your change: "Add user validation feature"
  Auto-fix:    "refactor: simplify service logic"

⚠️  NOT auto-fixed (manual review needed):
  - Line 56: O(n²) algorithm (needs algorithm change, not refactoring)

Status: CANDIDATE (1 efficiency issue remains)
Next: Manually optimize line 56, or /verify to test current state
```

## Integration with Other Skills

### Before `/code-review`

- **`/verify`** — Should run first to confirm behavior works
  - Chain: `/verify` → `/code-review`

### After `/code-review`

1. **If auto-fix applied (--fix):**
   - `/verify` — Re-test to ensure fixes don't break behavior
   - `/simplify` — Further refactor if desired
   - `/security-review` — Audit fixes

2. **If no auto-fix (report only):**
   - Manually fix CORRECTNESS issues
   - `/code-review --fix` for suggested refactoring
   - Commit and push

### Typical Chain

```
/verify
  ↓ (if works)
/code-review --comment --fix
  ↓
/verify (re-test after fixes)
  ↓
/simplify (further cleanup)
  ↓
/security-review
  ↓
commit
```

## Examples

### Example 1: Finding a Bug

```bash
$ git diff src/auth.rs | head -30
+ fn validate_email(email: &str) -> bool {
+   email.contains("@") && email.len() < 256
+ }

$ /code-review low

🔍 Code Review: src/auth.rs

Findings:

1. ❌ CORRECTNESS - Line 42: Incomplete email validation
   Code: email.contains("@") && email.len() < 256
   Problem: ".@.", "a@.b", and other invalid emails pass
   Suggestion: Use regex or proper email parsing library
   Impact: Security vulnerability; invalid emails accepted

Status: CANDIDATE (correctness issue found)
```

### Example 2: Identifying Inefficiency

```bash
$ git diff src/search.rs | head -30
+ for item in items {
+   if big_list.contains(&item) {  // Line 15: Linear search!
+     results.push(item);
+   }
+ }

$ /code-review medium

🔍 Code Review: src/search.rs

Findings:

1. ⚠️  EFFICIENCY - Line 15: Repeated linear search
   Code: big_list.contains(&item)  // Inside loop
   Problem: contains() is O(n); loop is O(m); total O(n*m)
   Suggestion: Build HashSet from big_list first (O(n) once, O(1) per search)
   Impact: ~1000x faster for 1000 items

Status: CANDIDATE (inefficiency found; logic is correct)
```

### Example 3: Simplifying Code

```bash
$ git diff src/utils.rs
+ let should_continue = check_condition(x);
+ if should_continue {
+   process();
+ }

$ /code-review high --fix

Auto-fixes applied:
  - Remove unnecessary intermediate variable
  - Inline should_continue into if condition

New commit: refactor: simplify control flow

Status: ADMITTED (fixes applied)
```

## Troubleshooting

### "Too many findings; how to prioritize?"

```
Focus on CORRECTNESS first (must fix), then EFFICIENCY (should fix).
STYLE findings are lowest priority.

To focus on one category:
  /code-review medium  # Shows all, but review CORRECTNESS first
```

### "Auto-fix changed my code incorrectly"

```
If --fix applied something wrong:

1. Inspect the change: git show HEAD
2. Revert if needed: git revert HEAD
3. Run /code-review without --fix to see suggestions
4. Apply manually with editor control
```

### "I disagree with a finding"

```
Code review is advisory; you are not obligated to follow suggestions.
Use your judgment. If safety-related (CORRECTNESS), strongly consider it.
If STYLE or EFFICIENCY, it's your call.
```

## Configuration

### Default Effort Level

To set default effort for your project:

```bash
/update-config "set CODE_REVIEW_EFFORT=high"
```

Then `/code-review` will default to high effort.

## Differences from Similar Skills

| Skill | Finds Bugs? | Posts Comments? | Auto-fixes? | For Security? |
|-------|------------|-----------------|------------|---------------|
| **`/code-review`** | ✓ Yes | ✓ Optional | ✓ Optional | ✗ No |
| **`/simplify`** | ✗ No | ✗ No | ✓ Always | ✗ No |
| **`/security-review`** | ✓ Yes | ✗ No | ✗ No | ✓ Yes |
| **`/verify`** | ✗ No | ✗ No | ✗ No | ✗ No |

## Best Practices

✓ **Do run `/verify` first** — Confirm the change works before reviewing
✓ **Do use effort levels** — Start with `low`, increase if needed
✓ **Do post comments** — Use `--comment` in PR workflows
✓ **Do auto-fix safely** — Use `--fix` for low-risk refactoring
✓ **Do focus on CORRECTNESS first** — Fix bugs before style issues

✗ **Don't skip verification** — Review works code
✗ **Don't auto-fix blindly** — Review suggestions before committing
✗ **Don't ignore CORRECTNESS findings** — These are bugs
✗ **Don't use for security** — Use `/security-review` instead

## See Also

- [`/verify`](SKILL_VERIFY.md) — Validate behavior before review
- [`/simplify`](SKILL_SIMPLIFY.md) — Refactor for reuse/simplification only
- [`/security-review`](SKILL_SECURITY_REVIEW.md) — Security-focused review
- [`/review`](SKILL_REVIEW.md) — PR-level review (higher level)

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
