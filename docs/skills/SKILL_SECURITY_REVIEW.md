# Skill: /security-review

**Status:** AVAILABLE | **Scope:** Security Assessment | **Category:** Validation & Verification

---

## Overview

Complete a security review of pending changes on the current branch. Identifies vulnerabilities, unsafe patterns, credential leaks, and OWASP-class issues.

## When to Use

Use `/security-review` when you want to:
- Identify vulnerabilities before merge
- Check for credential/secret leaks
- Audit cryptographic usage
- Validate input handling
- Check dependency vulnerabilities
- Ensure data is properly protected

**Do NOT use `/security-review` for:**
- General code quality (use `/code-review` instead)
- Functional testing (use `/verify` instead)
- Performance optimization (use `/simplify` instead)

## Parameters

**None** — Automatic comprehensive security assessment.

```bash
/security-review
```

## Review Categories

1. **Credential Leaks** — API keys, passwords, tokens in code
2. **Injection Vulnerabilities** — SQL injection, command injection, XSS
3. **Access Control** — Privilege escalation, authorization bypasses
4. **Cryptography** — Weak algorithms, improper key management
5. **Dependency Risks** — Known CVEs in dependencies
6. **Data Exposure** — Sensitive data in logs, unencrypted transit
7. **Input Validation** — Unsafe deserialization, path traversal

## Severity Levels

| Level | Definition | Action |
|-------|-----------|--------|
| **CRITICAL** | Exploitable immediately; blocks merge | Fix before merge |
| **HIGH** | Serious vulnerability; should fix | Plan fix |
| **MEDIUM** | Moderate risk; should review | Consider fixing |
| **LOW** | Minor issue; can address later | Track for future |

## Expected Output

```
🔒 Security Review: Branch feature/auth-improvements

Findings: 3 total (1 CRITICAL, 1 HIGH, 1 MEDIUM)

🔴 CRITICAL - Line 42: Hardcoded API key
   File: src/config.rs
   Code: const API_KEY = "sk-1234567890";
   Issue: Sensitive credential in source code
   Fix: Move to environment variable or secrets manager
   CWE: CWE-798 (Use of Hard-coded Password)

🟠 HIGH - Line 78: SQL injection risk
   File: src/db.rs
   Code: format!("SELECT * FROM users WHERE id = {}", user_input)
   Issue: Unsanitized user input in SQL query
   Fix: Use parameterized queries
   CWE: CWE-89 (Improper Neutralization of Special Elements in an SQL Command)

🟡 MEDIUM - Line 120: Missing input validation
   File: src/api/handlers.rs
   Code: let count = request.param("count").parse()?;
   Issue: No upper bound check on user input
   Fix: Validate count <= MAX_RESULTS before processing
   CWE: CWE-190 (Integer Overflow or Wraparound)

Status: REFUSED (1+ CRITICAL issues must be fixed)
Next: Fix CRITICAL issue, run /security-review again
```

## Integration

### Follows `/code-review` and `/verify`

```
/verify                   (test it works)
  ↓
/code-review --fix       (find bugs)
  ↓
/security-review         (audit security)
  ↓
/review --approve        (PR approval if clear)
```

## Examples

### Example 1: Credential Leak

```bash
$ /security-review

🔒 Security Review

🔴 CRITICAL - Hardcoded API key in code
   File: src/api.rs, Line 5
   Code: let api_key = "sk_test_123456789abc";
   
   Fix: Use environment variable
   let api_key = env::var("API_KEY")?;
```

### Example 2: SQL Injection

```bash
🟠 HIGH - SQL injection risk
   File: src/db.rs, Line 45
   Code: SELECT * FROM users WHERE email = '$email'
   
   Risk: User input directly in query
   Fix: Use parameterized query or prepared statement
```

## See Also

- [`/code-review`](SKILL_CODE_REVIEW.md) — General code quality
- [`/verify`](SKILL_VERIFY.md) — Functional validation
- [`/review`](SKILL_REVIEW.md) — PR-level review

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
