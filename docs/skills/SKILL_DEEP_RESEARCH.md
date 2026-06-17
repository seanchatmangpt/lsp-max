# Skill: /deep-research

**Status:** AVAILABLE | **Scope:** Fact-Checked Research | **Category:** Research & Learning

---

## Overview

Conduct multi-source, adversarially verified research. Fans out web searches, fetches sources, verifies claims, and synthesizes a cited report. Ideal for deep technical questions requiring factual accuracy and source traceability.

## When to Use

Use `/deep-research` when you want to:
- Fact-check a claim across multiple sources
- Research a technical topic with full citations
- Verify consensus vs. contested claims
- Understand a security vulnerability
- Evaluate library/tool comparedness with evidence

**Do NOT use `/deep-research` for:**
- Quick facts (use Claude directly)
- API reference (use `/claude-api` instead)
- Internal project knowledge (not available online)

## Parameters

```bash
/deep-research "question"
```

| Parameter | Type | Required? | Examples |
|-----------|------|-----------|----------|
| `args` | string | Yes | "Is OAuth 2.0 still secure?", "Rust 1.80 breaking changes" |

## Pre-invocation Clarification

If your question is underspecified, the skill will ask 2-3 clarifying questions:

```
Question: "What database should I choose?"
Clarification needed:
  - Scale: How many rows? (thousands, millions, billions?)
  - Use case: OLTP (transactional) or OLAP (analytics)?
  - Region: Where is your data stored?

Refine question and retry.
```

## Invocation

```bash
# Question (auto-refined if underspecified)
/deep-research "Is the Log4j vulnerability patched in 2.17.0?"

# With context
/deep-research "Rust 2024 migration guide: is it production-ready?"
```

## How It Works

### Phase 1: Question Clarification

- If underspecified: ask 2-3 clarifying questions
- Refine scope and context
- Proceed with refined question

### Phase 2: Parallel Web Search

Fan out multiple search queries:
- Official documentation
- Blog posts and tutorials
- Academic papers
- Community discussions
- Security advisories (if relevant)

### Phase 3: Source Fetching

Retrieve full content from top sources:
- Extract key claims and evidence
- Note publication dates and authors
- Identify authoritative vs. opinion sources

### Phase 4: Adversarial Verification

Challenge claims against contradictory sources:
- Identify consensus vs. outlier claims
- Note confidence levels for each claim
- Highlight caveats and limitations

### Phase 5: Synthesis

Generate comprehensive report:
- Organize findings by topic
- Include citations with URLs
- Structure: problem → evidence → consensus → caveats

## Expected Output

```
📚 Research Report: Is OAuth 2.0 still secure in 2026?

Sources analyzed: 12 (docs, papers, blogs, advisories)
Confidence: HIGH (strong consensus across sources)
Last updated: 2026-06-14

Executive summary:
OAuth 2.0 remains secure for most use cases when properly 
implemented. However, specific patterns (implicit flow, 
client secrets in SPAs) have known vulnerabilities.

Key findings:

1. ✅ STRONG_CONSENSUS: OAuth 2.0 is cryptographically sound
   Sources: RFC 6749 (official), OWASP guidelines, 3 papers
   Note: Issues are in implementation, not core design

2. ⚠️  CONTESTED: Authorization Code Flow with PKCE is 
   the recommended pattern for SPAs
   Sources: Auth0 blog, Google identity docs
   Contrary: Some argue implicit flow still acceptable
   Consensus: PKCE is better (leaning HIGH)

3. ❌ REFUSED: Client secrets should not be embedded in 
   client-side JavaScript
   Sources: OWASP Top 10, multiple advisories
   Confidence: UNIVERSAL (no disagreement)

4. ✅ SUPPORTED: OAuth 2.0 + OpenID Connect is modern standard
   Sources: Identity provider implementations (Google, GitHub, etc.)
   Confidence: HIGH

Caveats:
- Implementation quality varies widely
- Token storage in browsers is still a risk
- Refresh token rotation best practices evolving

Recommendations:
- Use authorization code + PKCE for SPAs
- Use OAuth 2.0 + OpenID Connect for identity
- Validate implementations against OWASP guidelines
- Keep libraries updated (see advisories)

Sources:
[1] RFC 6749 - OAuth 2.0 Authorization Framework
    https://tools.ietf.org/html/rfc6749
[2] Auth0 Blog: OAuth 2.0 Security Patterns
    https://auth0.com/blog/oauth-2-best-practices/
[3] OWASP: Authentication Cheat Sheet
    https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
... (9 more sources)

Status: ADMITTED
Research depth: Comprehensive; confidence: HIGH
```

## Confidence Levels

| Level | Meaning | Example |
|-------|---------|---------|
| **STRONG_CONSENSUS** | All sources agree | "OAuth 2.0 is cryptographically sound" |
| **SUPPORTED** | Most sources agree | "Use PKCE for SPAs" |
| **CONTESTED** | Disagreement among sources | "Is implicit flow acceptable?" |
| **UNKNOWN** | No sources found | Niche topic with no published info |
| **REFUSED** | Evidence contradicts claim | "Client secrets are safe in SPAs" |

## Integration with Development

### Research → Coding Workflow

```bash
# 1. Research decision
/deep-research "Should we use library X or Y?"

# 2. Review findings
# (read the report)

# 3. Code based on findings
# (implement)

# 4. Test and verify
/verify

# 5. Code review
/code-review
```

## Examples

### Example 1: Security Vulnerability

```bash
$ /deep-research "Is CVE-2024-1234 patched in package@2.0.0?"

📚 Research Report: CVE-2024-1234 in package@2.0.0

Finding: ❌ REFUSED (not patched in 2.0.0)
  - Vulnerability: Remote code execution in parser
  - Affected: package < 2.1.0
  - Fixed in: 2.1.0 (released 2026-03-15)
  - Workaround: Update to 2.1.0+

Sources:
  [1] Official security advisory: https://...
  [2] NVD entry: https://...
  [3] GitHub issue: https://...

Recommendation: Upgrade to 2.1.0+ immediately
```

### Example 2: Technology Evaluation

```bash
$ /deep-research "Rust vs Go for CLI tools in 2026?"

📚 Research Report: Rust vs Go for CLI tools

Key findings:

Rust:
  ✅ Performance: Slightly faster
  ✅ Memory: Lower footprint
  ⚠️  Learning curve: Steeper
  ✅ Ecosystem: Growing

Go:
  ✅ Development speed: Faster to write
  ✅ Learning curve: Gentler
  ✅ Concurrency: Better patterns
  ⚠️  Memory: Slightly higher (GC overhead)

Consensus: Both are excellent for CLI tools
  - Choose Rust for performance-critical tools
  - Choose Go for rapid development
  - Team expertise matters more than language

Sources: [Multiple blogs, benchmarks, and GitHub projects]
```

## See Also

- [`/claude-api`](SKILL_CLAUDE_API.md) — For Claude API questions (static docs)
- Built-in Claude knowledge — For general questions not requiring multi-source research

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED
