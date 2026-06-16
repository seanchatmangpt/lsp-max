# Skill: /claude-api

**Status:** AVAILABLE | **Scope:** Claude API Reference | **Category:** Research & Learning

---

## Overview

Reference for the Claude API / Anthropic SDK. Provides authoritative information on model IDs, pricing, parameters, streaming, tool use, MCP, agents, caching, token counting, and model migration.

**Authority:** Definitive source; **never answer from memory** about Claude API—always use this skill.

## When to Use

Use `/claude-api` when you want to:
- Look up current model IDs and capabilities
- Find pricing information (cost per token)
- Learn about API parameters (temperature, top_k, etc.)
- Understand streaming, tool use, or MCP
- Get token counting guidance
- Plan model migration or upgrades

**Auto-triggered by:** System reminder when Claude/Anthropic is mentioned in prompts.

## Trigger Pattern

The skill is automatically invoked when prompts include:
- Claude/Anthropic product mentions (Claude, Fable, Opus, Sonnet, Haiku)
- LLM-specific questions (pricing, model choice, limits, caching)
- LLM-shaped tasks (agent design, MCP, tool-definition, generation, classification)

**Skip if:** Another provider (OpenAI, Gemini, Llama, etc.) is being worked on

## Parameters

```bash
/claude-api [question]
```

| Parameter | Type | Optional? | Examples |
|-----------|------|-----------|----------|
| `args` | string | Yes | "What are current model IDs?", "Token counting with tools" |

## Invocation

```bash
# Without question (browse reference)
/claude-api

# Specific question
/claude-api "What are current model IDs?"
/claude-api "How does prompt caching work?"
/claude-api "Token counting with tools"
```

## Reference Categories

### 1. Models

**Current model IDs, capabilities, deprecation schedule**

```
claude-opus-4-1          (Latest flagship; most capable)
  - Context: 200k tokens
  - Input: $15/M tokens
  - Output: $75/M tokens
  - Strengths: Complex reasoning, code, analysis

claude-sonnet-4          (Mid-tier; best for most use cases)
  - Context: 200k tokens
  - Input: $3/M tokens
  - Output: $15/M tokens
  - Strengths: Speed, cost, general purpose

claude-haiku-4           (Fast, compact; cost-efficient)
  - Context: 100k tokens
  - Input: $0.80/M tokens
  - Output: $4/M tokens
  - Strengths: Speed, cost, simple tasks
```

### 2. Pricing

**Token costs, per-model pricing, batch API discounts**

```
Pricing (as of Feb 2025):

Per-token pricing:
  Opus:   $15 input / $75 output
  Sonnet: $3 input / $15 output
  Haiku:  $0.80 input / $4 output

Batch API (20% discount if non-urgent):
  Run jobs offline, pay less

Token counting:
  Always count before spending
  Tools add 1-5 tokens overhead
```

### 3. Parameters

**Temperature, top_k, top_p, max_tokens, system prompts**

```
temperature (0.0 - 1.0)
  0.0 = Deterministic (same output)
  1.0 = Maximum randomness
  Typical: 0.7-0.9

max_tokens
  Limits output length
  Must be < context_window - input_tokens

top_p (0.0 - 1.0)
  Nucleus sampling; use 0.9-0.95 typically

top_k (integer)
  Sample from top-k tokens
  Less common; use top_p instead
```

### 4. Tool Use

**Tool definitions, tool_choice, parallel tools, result handling**

```
Tool use pattern:
  1. Define tools in system prompt
  2. API returns tool_calls
  3. Execute tools locally
  4. Send results back to Claude
  5. Claude uses results to answer

Tool constraints:
  - Up to 100 tools per request
  - Parallel tool calls supported
  - Tool results can be streamed
```

### 5. MCP (Model Context Protocol)

**Server/client patterns, integration, best practices**

```
MCP enables:
  - External tools and resources
  - Dynamic capability extension
  - Secure, standardized protocol

Setup:
  1. MCP server (external process)
  2. MCP client (Claude application)
  3. Bidirectional messaging
```

### 6. Streaming

**Server-sent events, chunking, event types, cancellation**

```
Streaming benefits:
  - Faster perceived latency
  - Lower memory (processes incrementally)
  - Can cancel mid-generation

Event types:
  content_block_start
  content_block_delta
  content_block_stop
  message_stop
```

### 7. Agents

**Agent framework, tool composition, loop control, LLM as judge**

```
Agent loop:
  1. Claude decides: think, call tool, or respond
  2. If tool: execute and provide results
  3. Repeat until "respond" is chosen

Agent termination:
  - Max iterations (e.g., 10)
  - Tool call returns final_answer
  - Timeout (e.g., 5 minutes)
```

### 8. Caching

**Prompt caching, cache headers, cost reduction, best practices**

```
Cache mechanisms:
  - Prompt prefix caching (static context)
  - Recent messages caching

Cost savings:
  - Cached input: ~90% cheaper
  - Fresh output: full price

Example:
  Long document (cached) + new query (not cached)
  = 90% savings on document cost
```

### 9. Token Counting

**Token counting API, edge cases, tools overhead**

```
Token counting:
  - Use official API: count_tokens()
  - Special tokens for tools: ~5 per tool
  - Chat messages: ~4 tokens overhead

Edge cases:
  - Unicode: ~1-4 tokens per character
  - Code: Often 2-3 tokens per word
  - Numbers: 1 token per digit (usually)
```

### 10. Model Migration

**Upgrading to newer models, deprecation timeline, API stability**

```
Deprecation policy:
  - 6 months notice before sunset
  - Older models still work (may be deprecated)
  - Upgrade when ready

Migration checklist:
  1. Test with new model
  2. Compare output quality
  3. Check pricing impact
  4. Update API calls if needed
```

## Expected Output

```
🔍 Claude API Reference

Topic: Current model IDs (as of Feb 2025)

Available models:

1. claude-opus-4-1
   Context: 200k tokens
   Pricing: $15 input / $75 output (per 1M tokens)
   Best for: Complex reasoning, long context
   Status: Latest flagship model

2. claude-sonnet-4
   Context: 200k tokens
   Pricing: $3 input / $15 output
   Best for: General purpose (speed + quality)
   Status: Recommended for most use cases

3. claude-haiku-4
   Context: 100k tokens
   Pricing: $0.80 input / $4 output
   Best for: Simple tasks, low latency
   Status: Cost-efficient, fast

Deprecated (still available; not recommended):
  - claude-opus-3 (use opus-4-1)
  - claude-sonnet-3 (use sonnet-4)

Authority: Official Anthropic API documentation
Knowledge cutoff: Feb 2025
Status: ADMITTED
```

## Integration

### Auto-triggered Examples

When you mention Claude/Anthropic in a prompt:

```
User: "Can I use Claude Opus for real-time analysis?"
System reminder: Triggers /claude-api
Result: Opus capabilities + latency info + pricing
```

```
User: "How do tokens work with tool calls?"
System reminder: Triggers /claude-api
Result: Token counting + tool overhead details
```

### Usage with Development

```bash
# Research API question
/claude-api "What are the rate limits?"

# Read the answer

# Implement based on findings
# (code using API)

# Test
/verify

# Review
/code-review
```

## Common Questions Answered

**Q: What's the latest Claude model?**
A: Check `/claude-api "current models"` (never assume from memory)

**Q: How much does it cost?**
A: Depends on model; use `/claude-api "pricing"` for current rates

**Q: Can I use tool use with streaming?**
A: Yes; see `/claude-api "streaming with tools"` for details

**Q: How do I count tokens?**
A: `/claude-api "token counting"` shows API and edge cases

## See Also

- [`/deep-research`](SKILL_DEEP_RESEARCH.md) — For non-Claude-API research
- Anthropic SDK — Python/JavaScript libraries for API usage
- Official docs — https://docs.anthropic.com (authoritative source)

---

**Last Updated:** 2026-06-14 | **Status:** ADMITTED  
**Knowledge Cutoff:** Feb 2025 | **Authority:** Official Anthropic documentation
