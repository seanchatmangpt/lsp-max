# Skill: /simplify

**Status:** AVAILABLE | **Scope:** Code Refactoring | **Category:** Validation & Verification

---

## Overview

Review changed code for reuse, simplification, efficiency, and altitude cleanups, then apply the fixes automatically. **Quality-only:** Does not hunt for bugs (use `/code-review` for that).

## When to Use

Use `/simplify` when you want to:
- Extract duplicated logic into helpers
- Flatten deeply nested code
- Reduce intermediate variables
- Optimize loop patterns
- Remove dead code
- Improve code clarity without changing behavior

**Do NOT use `/simplify` for:**
- Finding bugs (use `/code-review` instead)
- Security audits (use `/security-review` instead)
- Testing behavior (use `/verify` instead)

## Parameters

**None** — Automatic analysis and application of safe refactoring.

```bash
/simplify
```

## How It Works

### Phase 1: Analysis

Examine changed code for:
1. **Duplicated patterns** — Same code in multiple places
2. **Nested complexity** — Deep if/match/loops that can flatten
3. **Dead code** — Unused variables, unreachable branches
4. **Intermediate steps** — Unnecessary variable bindings
5. **Inefficient patterns** — Redundant operations

### Phase 2: Refactoring

Apply safe transformations:
- Extract helpers (< 10 lines)
- Flatten conditionals
- Remove dead code
- Inline trivial operations
- Cache expensive computations

### Phase 3: Validation

- Verify behavior unchanged (no logic modification)
- Check tests still pass
- Create new commit with changes

## Expected Output

```
✨ Simplification Complete

Refactorings applied:
  1. Extracted validate_email() helper (3 duplicates consolidated)
  2. Flattened nested if-else (5 levels → 2 levels)
  3. Removed unused variable: debug_flag
  4. Inlined trivial get_timestamp() function

New commit: refactor: simplify service logic

Statistics:
  Lines removed: 12
  Duplications eliminated: 3
  Dead code: 1 unused variable
  
Status: ADMITTED
Next: /verify to confirm behavior unchanged
```

## Integration

### Follows `/code-review --fix`

```
/code-review --fix    (fix bugs)
  ↓
/simplify             (clean up)
  ↓
/verify               (test again)
```

## Examples

### Example 1: Consolidate Duplicates

```rust
// ❌ Before: Validation repeated
impl User {
  fn validate_email(&self) -> bool {
    !self.email.is_empty() && self.email.contains("@")
  }
}

impl Product {
  fn validate_email(&self) -> bool {
    !self.email.is_empty() && self.email.contains("@")
  }
}

// ✅ After: Extracted helper
fn validate_email(email: &str) -> bool {
  !email.is_empty() && email.contains("@")
}

impl User {
  fn validate_email(&self) -> bool {
    validate_email(&self.email)
  }
}

impl Product {
  fn validate_email(&self) -> bool {
    validate_email(&self.email)
  }
}
```

### Example 2: Flatten Nesting

```rust
// ❌ Before: Deep nesting
if user.is_active {
  if user.email_verified {
    if user.subscription.is_valid() {
      process_user(&user);
    }
  }
}

// ✅ After: Early return
if !user.is_active { return; }
if !user.email_verified { return; }
if !user.subscription.is_valid() { return; }
process_user(&user);
```

## See Also

- [`/code-review`](SKILL_CODE_REVIEW.md) — Find bugs and inefficiencies
- [`/verify`](SKILL_VERIFY.md) — Validate behavior after simplification

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
