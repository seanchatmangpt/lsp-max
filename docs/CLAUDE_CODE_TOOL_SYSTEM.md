# Claude Code Tool System Documentation

## Overview

The Claude Code tool system provides a suite of specialized tools that Claude uses to interact with codebases, execute commands, and perform development tasks. This document provides a comprehensive taxonomy, decision matrix, and best practices for tool usage.

### Available Tools

Claude has access to the following tool categories:

1. **File Operations**: Read, Write, Edit
2. **Code Search**: Glob, Grep
3. **Execution**: Bash
4. **Advanced Tasks**: Agent, Skill
5. **Special Purpose**: ToolSearch

---

## Tool Catalog

### Read

**Purpose**: Read file contents from the filesystem.

**Parameters**:
- `file_path` (string, required): Absolute path to file
- `limit` (integer, optional): Number of lines to read (default: 2000)
- `offset` (integer, optional): Starting line number (for large files)
- `pages` (string, optional): For PDFs only, e.g., "1-5" (max 20 pages)

**Supported File Types**:
- Text files (code, config, markdown, etc.)
- Images (PNG, JPG, etc.) — returned visually
- PDFs (with page parameter for large files)
- Jupyter notebooks (.ipynb) — includes outputs and visualizations

**Return Format**:
```
line_number\tfile_content
line_number\tfile_content
```

**Key Properties**:
- Reads from line 1 onwards (1-indexed)
- Works with non-existent files (returns error)
- No permission prompt for read-only access
- Large files should use `limit` and `offset` parameters

**Best Practices**:
- Always use absolute paths (never relative)
- For large files, read only the relevant section if known
- Use `offset` to skip to specific line ranges
- Read before editing (Edit requires prior Read of the file)

**Gotchas**:
- Cannot read directories (use Bash `ls` only when necessary)
- Empty files return a system reminder warning
- PDFs >10 pages require explicit page range

---

### Write

**Purpose**: Write a file to the filesystem (creates new or overwrites existing).

**Parameters**:
- `file_path` (string, required): Absolute path to file
- `content` (string, required): Full file content

**Key Properties**:
- Overwrites existing files completely
- Requires prior Read of file if it already exists
- Better for bulk replacements or new file creation
- Triggers permission prompt for file creation

**Best Practices**:
- Prefer Edit for modifications (sends diff, not full content)
- Always read existing files before overwriting
- Use for complete rewrites or new files
- Avoid creating documentation files (.md) unless explicitly requested

**Gotchas**:
- Full content replacement (risk of data loss)
- Requires Read call first for existing files
- No incremental edits — inefficient for small changes

---

### Edit

**Purpose**: Make targeted string replacements in existing files.

**Parameters**:
- `file_path` (string, required): Absolute path to file
- `old_string` (string, required): Exact text to find and replace
- `new_string` (string, required): Replacement text
- `replace_all` (boolean, optional): Replace all occurrences (default: false)

**Key Properties**:
- Must have Read the file first
- Preserves exact indentation (tabs/spaces)
- Fails if `old_string` is not unique (unless `replace_all: true`)
- Line numbers in Read output must be stripped from old_string

**Indentation Rules**:
- Read output format: `line_number\tfile_content`
- Strip the line number + tab from old_string
- Preserve all whitespace AFTER the tab
- Mismatched indentation causes replacement failure

**Best Practices**:
- Always Read first to understand context
- Make old_string unique by including surrounding lines
- Use `replace_all: true` only when replacing all instances
- Keep diffs focused (one logical change per Edit)
- Include enough context to make pattern unambiguous

**Gotchas**:
- Non-unique old_string causes failure
- Indentation mismatch is a common failure mode
- Line numbers from Read must be stripped
- No partial matches — exact substring required

---

### Glob

**Purpose**: Fast file pattern matching using glob syntax.

**Parameters**:
- `pattern` (string, required): Glob pattern (e.g., "**/*.js", "src/**/*.tsx")
- `path` (string, optional): Directory to search in (defaults to cwd)

**Return Format**:
```
matching/file/1.js
matching/file/2.js
```

**Glob Syntax**:
- `**` — matches any directory depth
- `*` — matches any characters except `/`
- `?` — matches single character
- `{a,b}` — alternation
- `[abc]` — character class

**Key Properties**:
- Fast even on large codebases
- Sorted by modification time
- Returns paths only (not content)
- No regex support (use Grep for that)

**Best Practices**:
- Use for initial file discovery
- Chain with Grep for content search
- Prefer over `find` command in Bash
- Use explicit patterns to narrow results

**Gotchas**:
- Returns paths only — doesn't show content
- Case-sensitive on Linux
- No regex (literal glob patterns only)
- Large result sets require further filtering

---

### Grep

**Purpose**: Search file contents using regex patterns (ripgrep).

**Parameters**:
- `pattern` (string, required): Regex pattern (ripgrep syntax)
- `path` (string, optional): File or directory to search
- `glob` (string, optional): Glob filter (e.g., "*.js")
- `type` (string, optional): File type (e.g., "js", "py", "rust")
- `output_mode` (string, optional): "content" (default), "files_with_matches", "count"
- `-i` (boolean): Case-insensitive search
- `-A` (number): Lines after match (requires output_mode: "content")
- `-B` (number): Lines before match (requires output_mode: "content")
- `-C` / `context` (number): Lines before and after
- `-n` (boolean): Show line numbers (default: true)
- `-o` (boolean): Show only matched portions
- `multiline` (boolean): Match across newlines
- `head_limit` (number): Limit output (default: 250)
- `offset` (number): Skip first N results

**Return Format** (content mode):
```
path/to/file.js:42:    matched_line_with_pattern
path/to/file.js:45:    another_matched_line
```

**Key Properties**:
- Full regex support (ripgrep syntax)
- Respects .gitignore patterns
- Fast on large codebases
- Can filter by file type
- Supports multiline patterns

**Best Practices**:
- Use type filter for specific languages
- Combine with Glob results for targeted searches
- Use `-C` for context around matches
- Set `head_limit` to prevent token waste
- Use `output_mode: "files_with_matches"` to find relevant files first

**Gotchas**:
- Braces in Go code must be escaped (e.g., `interface\\{\\}`)
- Multiline patterns require `multiline: true`
- Default head_limit is 250 (set to 0 for unlimited)
- Context flags only work with `output_mode: "content"`
- Large result sets require limiting

---

### Bash

**Purpose**: Execute shell commands and get output.

**Parameters**:
- `command` (string, required): Shell command to execute
- `description` (string, required): Clear description of what it does
- `timeout` (number, optional): Milliseconds (max 600000/10 min, default 120000/2 min)
- `run_in_background` (boolean, optional): Run asynchronously

**Key Properties**:
- Working directory persists between calls
- Shell state does not persist (each call is independent)
- Quote file paths with spaces using double quotes
- Use absolute paths to maintain cwd across calls
- Can run in background with notification on completion

**Best Practices**:
- Prefer dedicated tools over Bash (e.g., use Read instead of `cat`)
- Use absolute paths, not relative paths
- Chain commands with `&&` if dependent, `;` if independent
- Quote paths with spaces: `"path with spaces/file.txt"`
- Run independent commands in parallel via multiple Bash calls
- Use `run_in_background: true` for long-running tasks

**Gotchas**:
- Working directory resets between calls (use absolute paths)
- Shell state doesn't persist (no environment variables, no `cd` state)
- `find`, `grep`, `cat`, `sed`, `awk`, `echo` should use dedicated tools instead
- Git commands don't need `cd` prefix (already in worktree)
- Force operations (`git reset --hard`, `git push --force`) require explicit user request
- No interactive input (can't use `-i` flags)
- Pre-commit hooks run by default (can't skip with `--no-verify` unless requested)

---

### Agent

**Purpose**: Launch a specialized agent to handle complex, multi-step tasks.

**Parameters**:
- `description` (string, required): Short 3-5 word task description
- `prompt` (string, required): Full task briefing (must be self-contained)
- `subagent_type` (string, optional): Agent type (see Available Agents below)
- `isolation` (string, optional): "worktree" for isolated git environment
- `model` (string, optional): Override model (sonnet, opus, haiku, fable)
- `run_in_background` (boolean, optional): Run asynchronously

**Available Agent Types**:
- `claude` — General-purpose agent for any task
- `claude-code-guide` — Claude Code CLI/API reference questions
- `Explore` — Fast code search and file location
- `general-purpose` — Complex research and multi-step tasks
- `Plan` — Software architecture and implementation planning
- `code-reviewer` — Code review with correctness and efficiency focus
- `statusline-setup` — Claude Code status line configuration

**Key Properties**:
- Each agent starts fresh (no memory of prior runs)
- Agent result is single message (not visible to user, relay findings)
- Trust but verify: agent's summary ≠ what actually happened
- Parallel execution: send multiple Agent calls in single message
- Background agents notify when complete (don't poll)
- Isolation modes create temporary git worktrees

**Best Practices**:
- Use self-contained prompts (agent has no context)
- Specify what you've learned or ruled out
- Request short responses for brevity
- Don't delegate understanding ("fix based on findings" anti-pattern)
- Use foreground for tasks whose results inform next steps
- Use background for genuinely independent work
- Verify code changes by reading actual files, not just agent reports

**Gotchas**:
- Agent doesn't see conversation history (brief it explicitly)
- Agent's summary is NOT proof of what it did (verify changes)
- Spawning new agent = fresh context (use SendMessage to resume)
- Background agents don't block — continue other work
- Complex tasks need context-heavy prompts to avoid mistakes

---

### Skill

**Purpose**: Invoke specialized skills with domain knowledge (slash commands).

**Parameters**:
- `skill` (string, required): Exact skill name (no leading slash)
- `args` (string, optional): Optional arguments

**Available Skills**:
- `session-start-hook` — Configure SessionStart hooks for web tests
- `deep-research` — Multi-source fact-checked research reports
- `update-config` — Configure settings.json and hooks
- `keybindings-help` — Customize keyboard shortcuts
- `verify` — Verify app behavior and test changes
- `code-review` — Review diff for bugs and cleanups
- `simplify` — Code cleanup and optimization
- `fewer-permission-prompts` — Reduce permission prompts
- `loop` — Run commands on recurring intervals
- `claude-api` — Claude API reference and pricing
- `run` — Launch and drive the project app
- `init` — Initialize CLAUDE.md documentation
- `review` — Review a pull request
- `security-review` — Security review of pending changes

**Key Properties**:
- Skill name must match exactly (from available-skills list)
- Skills provide domain-specific tools and knowledge
- Invoke via Skill tool (not as text command)
- Some skills have multiple parameters

**When to Use**:
- `code-review` — Verify correctness of changes
- `verify` — Test that changes work in the app
- `security-review` — Check for security issues
- `update-config` — Configure automated behaviors
- `deep-research` — Fact-checked research on specific topics
- `run` — Launch the app and see changes working

**Gotchas**:
- Skill name is case-sensitive
- Must invoke via Skill tool (not directly in prompt)
- Some skills require prior setup (e.g., git repo)
- Always check if skill is in available-skills list first

---

### ToolSearch

**Purpose**: Fetch schemas for deferred tools (tools not yet loaded).

**Parameters**:
- `query` (string, required): Tool lookup query
- `max_results` (number, optional): Max results to return (default: 5)

**Query Forms**:
- `select:ToolName,OtherTool` — Fetch specific tools by name
- `keyword search` — Search by keywords (ranks by relevance)
- `+required_term` — Require term in result

**Use Case**:
- MCP tools (Gmail, Google Calendar, GitHub, etc.) appear as deferred in system reminder
- Use ToolSearch to load their full schemas before invoking
- Once loaded, tools appear in normal function definitions

**Example**:
```
ToolSearch with query "select:mcp__Gmail__search_threads,mcp__Gmail__get_thread"
```

**Key Properties**:
- Deferred tools can't be called without loading schema
- ToolSearch returns JSONSchema definitions
- After loading, tools can be invoked like any other
- Respects max_results limit

---

## Tool Classification Taxonomy

### By Capability Domain

#### File & Code Inspection
- **Read** — View file contents
- **Glob** — Find files by pattern
- **Grep** — Search file contents

#### File Modification
- **Edit** — Targeted string replacements
- **Write** — Full file creation/overwrite

#### Execution
- **Bash** — Shell commands
- **Agent** — Complex multi-step tasks
- **Skill** — Domain-specific actions

#### Metadata & Discovery
- **ToolSearch** — Load deferred tool schemas

### By Scope

#### Single-File Operations
- Read, Edit, Write

#### Multi-File Operations
- Glob, Grep, Bash (with find/globbing), Agent

#### System-Level Operations
- Bash (git, build, test commands)
- Agent (complex workflows)
- Skill (specialized domains)

### By Permission Level

#### Read-Only
- Read, Glob, Grep (no permission prompt)

#### Read-Write
- Edit, Write (permission prompt on file creation)
- Bash (permission prompt on shell commands)

#### Special Permissions
- Bash with git `--force` requires explicit user request
- Bash with hook skip (`--no-verify`, `--no-gpg-sign`) requires request
- Agent with `isolation: "worktree"` creates temporary environments

### By Speed

#### Fast (< 100ms typical)
- Read (small-medium files)
- Glob
- Edit (small replacements)

#### Medium (100ms - 2s)
- Grep (large codebases)
- Read (large files)
- Bash (simple commands)

#### Slow (> 2s)
- Bash (build, test, compile)
- Agent (complex tasks)
- Write (large files)

---

## Decision Matrix: Tool Selection

### Task: Find where a symbol is defined

```
Is symbol a filename pattern?
├─ YES → Use Glob ("**/*symbol*")
└─ NO → Use Grep with regex ("function symbol|class Symbol|const symbol =")
```

### Task: Read a single file's content

```
Know the exact path?
├─ YES → Use Read (direct)
└─ NO → Use Glob to find path, then Read
```

### Task: Replace text in a file

```
Changing entire file?
├─ YES → Read, then Write (simpler)
└─ NO → Read, then Edit (more precise)
```

### Task: Search across many files

```
Searching for content patterns?
├─ YES → Grep (with type/glob filters)
└─ NO (names/paths only) → Glob
```

### Task: Run tests or build

```
Simple one-liner?
├─ YES → Bash
└─ NO (multi-step, complex logic) → Bash with chaining (&&) or Agent
```

### Task: Complex multi-step operation

```
Can break into independent steps?
├─ YES → Multiple Bash calls in parallel
└─ NO (dependent) → Chain with && in single Bash call
```

### Task: Specialized domain operation

```
Code review, app verification, security, etc.?
├─ YES → Check available Skills first
└─ NO → Use Agent or Bash
```

### Task: Need to load MCP tool schemas

```
Tool appears in deferred list?
├─ YES → ToolSearch("select:tool_name")
└─ NO → Tool already loaded, can invoke
```

---

## Tool Composition Patterns

### Pattern 1: Sequential File Inspection

**Goal**: Find and read a file

```
1. Glob(pattern: "**/*filename*") → returns paths
2. Read(file_path: path_from_glob) → returns content
3. Grep(pattern: "search_term", path: specific_file) → optional filtering
```

**When**: Locating a file, then inspecting it
**Risk**: Glob might return many results; use specific patterns

---

### Pattern 2: Search Then Modify

**Goal**: Find code and make changes

```
1. Grep(pattern: "symbol_to_find", output_mode: "files_with_matches")
2. Read(file_path: from_grep) → understand context
3. Edit(old_string: context, new_string: replacement)
```

**When**: Refactoring, bug fixes
**Risk**: Context must be unique for Edit to work

---

### Pattern 3: Parallel Multi-File Search

**Goal**: Search multiple files independently

```
Call in single message:
- Grep(pattern: "foo", glob: "*.rs")
- Grep(pattern: "bar", glob: "*.js")
- Bash(command: "git log --oneline | head")
```

**When**: Independent searches, metadata gathering
**Advantage**: All results available before next step
**Risk**: Too many parallel calls can exceed token budget

---

### Pattern 4: Conditional Modification

**Goal**: Modify file if conditions are met

```
1. Read(file_path) → check contents
2. IF condition met:
   3. Edit(old_string, new_string)
   ELSE:
   3. No-op (already correct or not applicable)
```

**When**: Defensive changes, feature flags
**Risk**: Must verify conditions before Edit

---

### Pattern 5: Bulk File Creation/Update

**Goal**: Create multiple files

```
1. FOR EACH file:
   2. IF file exists: Read(file_path)
   3. Construct new content
   4. Write(file_path, content) or Edit(...)
```

**When**: Scaffolding, code generation
**Risk**: Write overwrites; read first for existing files

---

### Pattern 6: Long-Running Task with Monitoring

**Goal**: Run build/test that takes time

```
1. Bash(command: "long_task", run_in_background: true)
   → Returns immediately; agent gets notified when done
2. Continue with other work (Read, Grep, Edit, etc.)
3. When agent notifies completion:
   4. Check results (Bash to read logs or Read log file)
```

**When**: Tests, builds, deployments
**Advantage**: Non-blocking; can parallelize
**Risk**: Don't poll; wait for notification

---

### Pattern 7: Isolated Multi-Step Git Operation

**Goal**: Make commits on a temporary branch

```
1. Agent(subagent_type: "...", isolation: "worktree")
   → Creates temp worktree
   → Performs multi-step git operations
   → Returns branch/path info
2. Use returned info for next steps
```

**When**: Complex git workflows, testing branch operations
**Risk**: Worktree auto-cleans if no changes

---

### Pattern 8: Expert Consultation via Agent

**Goal**: Get a second opinion on code

```
1. Agent(
     subagent_type: "code-reviewer",
     prompt: "Review file.rs for [specific concern]; report: [specific format]"
   )
2. Agent returns independent findings
3. Compare with own analysis
```

**When**: Security review, correctness verification, design feedback
**Risk**: Agent's findings aren't gospel; verify independently

---

## Parallel Tool Execution

### Guidelines

**When to Parallelize**:
- Tools are independent (no shared state)
- Multiple files being read/searched
- Metadata gathering (git, build info, etc.)
- Independent test suites

**How to Parallelize**:
- Send multiple tool calls in SINGLE message (batched)
- Tools execute concurrently
- Results returned in order of tool definition

**Example**:
```
Single message with three Bash calls:
- Bash(git status)
- Bash(git log)
- Bash(ls directory)
→ All three run in parallel
→ Results available before next step
```

**Limitations**:
- Bash working directory persists but shell state doesn't
- Can't share state between parallel Bash calls
- Dependent operations must be sequential

### Anti-Patterns

❌ **Sequential calls that could be parallel**:
```
Call 1: Bash(git status)
Call 2: Bash(git log)  ← Should batch with Call 1
```

❌ **Dependent operations run in parallel**:
```
Call 1: Read(file_path)  ← Creates file
Call 2: Edit(file_path)  ← Depends on Read
→ Should be sequential
```

✅ **Correct parallelization**:
```
Single message with 3 Bash calls:
- Bash(git status)
- Bash(ls -la)
- Bash(cargo --version)
→ All independent, all execute in parallel
```

---

## Tool Dependencies & Sequencing

### Hard Dependencies

| Tool | Requires | Reason |
|------|----------|--------|
| Edit | Read (prior) | Must know file contents/indentation |
| Write | Read (if existing) | Risk of overwrite; must read first |
| Bash | None | But cwd resets; use absolute paths |

### Soft Dependencies (Recommended)

| Tool | Recommended Prior | Reason |
|------|-------------------|--------|
| Grep | Glob | Narrow search scope; faster results |
| Edit | Glob + Read | Understand context before modifying |
| Agent | Grep/Read | Brief agent on relevant code |

### Sequencing Rules

**Rule 1: Read Before Edit**
```
sequence: [Read, then Edit]
reason: Edit needs indentation + context
```

**Rule 2: Read Before Write (if exists)**
```
if file_exists:
  sequence: [Read, then Write]
else:
  sequence: [Write directly (no Read needed)]
```

**Rule 3: Find Before Read**
```
if path_unknown:
  sequence: [Glob → Read]
if path_known:
  sequence: [Read directly]
```

**Rule 4: Parallelize Independent Operations**
```
sequence: [Bash (multiple), Grep (multiple), Read (multiple in batch)]
reason: No shared state; all execute concurrently
```

---

## Permission Model

### Read-Only Operations (No Prompt)
- Read file
- Glob patterns
- Grep patterns
- Bash (readonly git commands, ls, cat, etc.)

### Write Operations (Permission Prompt)
- Edit existing file
- Write new file
- Write to system locations

### Special Operations (Explicit User Request Required)
- `git push --force` (destructive)
- `git reset --hard` (destructive)
- `git checkout .` or `git restore .` (destructive)
- `--no-verify` (skip hooks)
- `--no-gpg-sign` or `-c commit.gpgsign=false` (skip signing)

### Permission Reduction

**Strategy**: Add allowlist to project `.claude/settings.json`

**Via Skill**:
```
Skill("fewer-permission-prompts")
→ Scans transcripts for common readonly Bash/MCP calls
→ Adds prioritized allowlist to settings.json
→ Reduces future permission prompts
```

---

## Error Handling & Recovery

### Read Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| File not found | Path is wrong | Use Glob to find correct path |
| EISDIR | Trying to read directory | Use Bash `ls` instead |
| Empty file warning | File exists but is empty | Expected; handle empty content |

### Edit Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| `old_string` not found | Typo, indentation, or context mismatch | Read file again; check indentation; provide more context |
| Not unique | Multiple matches for `old_string` | Use `replace_all: true` or make pattern more specific |
| Indentation mismatch | Tabs vs spaces | Read file to check exact whitespace; match exactly |

### Glob Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| No matches | Pattern too specific | Broaden pattern or use shorter substring |
| Too many results | Pattern too broad | Add more constraints (file extension, directory) |

### Grep Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| Regex syntax error | Invalid regex | Escape special chars (e.g., `interface\\{\\}` for Go) |
| No matches | Pattern doesn't exist in files | Simplify pattern; try keyword search |
| Multiline fail | Cross-line pattern without flag | Add `multiline: true` parameter |

### Bash Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| Command not found | Typo or tool not installed | Check PATH; use absolute paths |
| Permission denied | No execution permission | Check file permissions; use `chmod` if needed |
| Timeout (> 10 min) | Long-running process | Use `run_in_background: true` |
| Git hook failure | Pre-commit hook rejected | Fix code; re-stage; create NEW commit (not amend) |

### Agent Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| Agent gives bad results | Insufficient context in prompt | Re-run with more detailed prompt |
| Agent can't find code | Agent briefing was vague | Use Glob/Grep first; pass exact file paths |
| Agent changes wrong files | Isolation/context issue | Verify actual changes; re-run with isolation |

---

## Tool-Specific Gotchas

### Read

- **Gotcha**: Assumes absolute paths
  - **Fix**: Always use `/absolute/path`, never `./relative`

- **Gotcha**: Large files can exceed token budget
  - **Fix**: Use `limit` and `offset` to read sections

- **Gotcha**: Can't read directories
  - **Fix**: Use Bash `ls` (only when necessary; prefer Glob)

---

### Edit

- **Gotcha**: Line number prefixes from Read must be stripped
  - **Fix**: Read output is `NNN\tcontent`; use only content in old_string

- **Gotcha**: Indentation mismatch is invisible
  - **Fix**: Copy-paste from Read output, including ALL whitespace

- **Gotcha**: Non-unique old_string fails silently
  - **Fix**: Add surrounding context to make pattern unambiguous

- **Gotcha**: `replace_all: true` is dangerous
  - **Fix**: Use sparingly; verify intent before enabling

---

### Write

- **Gotcha**: Overwrites entire file
  - **Fix**: Read first if file exists; verify content before Write

- **Gotcha**: No incremental edits
  - **Fix**: Prefer Edit for small changes

---

### Glob

- **Gotcha**: Returns paths only, no content
  - **Fix**: Chain with Read or Grep

- **Gotcha**: Case-sensitive on Linux
  - **Fix**: Use correct case; combine with Grep for flexible search

- **Gotcha**: Result ordering is by mtime, not logical
  - **Fix**: Specify exact paths when order matters

---

### Grep

- **Gotcha**: Regex braces must be escaped (Go code)
  - **Fix**: Use `interface\\{\\}` instead of `interface{}`

- **Gotcha**: Multiline patterns need flag
  - **Fix**: Add `multiline: true` for cross-line patterns

- **Gotcha**: Default head_limit hides results
  - **Fix**: Set `head_limit: 0` for unlimited (sparingly)

- **Gotcha**: .gitignore patterns are respected
  - **Fix**: Use Bash for files in .gitignore

---

### Bash

- **Gotcha**: Working directory resets between calls
  - **Fix**: Use absolute paths; don't rely on `cd`

- **Gotcha**: Shell state doesn't persist
  - **Fix**: Can't set environment variables; chain with &&

- **Gotcha**: Interactive prompts hang
  - **Fix**: Can't use `-i` flags; avoid interactive tools

- **Gotcha**: Timeouts at 10 minutes
  - **Fix**: Use `run_in_background: true` for longer tasks

- **Gotcha**: Common tools have dedicated tool alternatives
  - **Fix**: Use Read instead of `cat`, Glob instead of `find`, etc.

- **Gotcha**: Force push / destructive git ops require explicit request
  - **Fix**: Don't suggest force operations; ask user first

---

### Agent

- **Gotcha**: Agent doesn't see conversation history
  - **Fix**: Write self-contained prompts; provide full context

- **Gotcha**: Agent's report ≠ what actually happened
  - **Fix**: Verify by reading files; don't trust summary alone

- **Gotcha**: Fresh agent spawns lose context
  - **Fix**: Use SendMessage to resume agent or re-brief

- **Gotcha**: Parallel agents need independent work
  - **Fix**: Batch in single message; don't send dependent agents serially

---

### Skill

- **Gotcha**: Skill name must match exactly (case-sensitive)
  - **Fix**: Copy from available-skills list; don't guess

- **Gotcha**: Some skills require prior state (git repo, config)
  - **Fix**: Check skill description; set up prerequisites

- **Gotcha**: Not all slash commands are skills
  - **Fix**: Check if in available-skills list; use Skill tool to invoke

---

## Best Practices Summary

### Code Search Workflow
1. **Find**: Use Glob for filenames, Grep for content
2. **Inspect**: Use Read to view context
3. **Locate** (if uncertain): Combine Glob + Grep

### File Modification Workflow
1. **Plan**: Read + understand existing structure
2. **Modify**: Use Edit for surgical changes, Write for full replacements
3. **Verify**: Read the result to confirm (or use Verify skill)

### Execution Workflow
1. **Simple**: Use Bash for single commands
2. **Complex**: Chain with && for dependencies, or use Agent
3. **Long-running**: Use `run_in_background: true`

### Multi-Step Workflow
1. **Independent steps**: Parallelize in single message
2. **Dependent steps**: Sequence with && or separate Bash calls
3. **Complex logic**: Consider Agent instead of chaining

### Permission Management
1. **Track prompts**: Note common permission requests
2. **Reduce**: Use `fewer-permission-prompts` skill
3. **Request**: Ask user for special permissions explicitly

### Agent Usage
1. **Self-contained prompt**: Include all context
2. **Verify results**: Read files; don't trust summary
3. **Parallelize**: Send independent agents in one message

### Error Recovery
1. **Read errors**: Use Glob to find path
2. **Edit errors**: Check indentation; add context
3. **Grep errors**: Simplify regex; escape special chars
4. **Bash errors**: Use absolute paths; check tools installed

---

## Checklist: Before Each Tool Call

### Before Read
- [ ] Have absolute path
- [ ] Understand file size (use limit for large files)
- [ ] For PDFs, specify page range if > 10 pages

### Before Edit
- [ ] Checked Read output first
- [ ] old_string is unique (or using replace_all)
- [ ] Indentation matches exactly
- [ ] Line numbers stripped from old_string

### Before Write
- [ ] For existing files: Read first
- [ ] Content is complete and correct
- [ ] Not creating unnecessary .md files

### Before Glob
- [ ] Pattern is specific enough (not too broad)
- [ ] Using correct syntax (**/ for depth, * for wildcard)

### Before Grep
- [ ] Regex pattern is valid
- [ ] Escaped special characters (e.g., `\\{\\}` for braces)
- [ ] Using multiline flag if needed
- [ ] Set head_limit to avoid token waste

### Before Bash
- [ ] Using absolute paths, not relative
- [ ] Understand timeout (120s default, 600s max)
- [ ] Check if dedicated tool exists (Read, Glob, Grep)
- [ ] For long tasks: set run_in_background

### Before Agent
- [ ] Prompt is self-contained
- [ ] Clear description (3-5 words)
- [ ] Included all relevant context
- [ ] Plan to verify results

### Before Skill
- [ ] Skill name matches available-skills list exactly
- [ ] Understand prerequisites
- [ ] Check if subagent_type needed

---

## Example Workflows

### Example 1: Find and Fix a Bug

```
1. Grep(pattern: "bug_symptom", output_mode: "files_with_matches")
   → Returns file list
2. Read(file_path: relevant_file)
   → Understand context
3. Grep(pattern: "bug_location_pattern", path: relevant_file)
   → Pinpoint exact location
4. Read(file_path: same, offset: nearby_lines)
   → Get full context for Edit
5. Edit(old_string: buggy_code, new_string: fixed_code)
   → Make fix
6. Verify(skill: "verify") or Bash(cargo test)
   → Confirm fix works
```

### Example 2: Refactor Across Multiple Files

```
1. Glob(pattern: "**/*.rs", path: "src/")
   → Find all Rust files
2. Grep(pattern: "old_function_name", glob: "*.rs")
   → Find all call sites
3. FOR EACH file:
   4. Read(file_path)
   5. Edit(old_pattern, new_pattern)
6. Bash(cargo test)
   → Verify all changes work
```

### Example 3: Deep Code Review

```
1. Bash(git diff main...HEAD)
   → See all changes in branch
2. Grep(pattern: "risky_pattern", glob: "**/*.rs")
   → Check for issues
3. Agent(subagent_type: "code-reviewer", prompt: "Review [specific concern]")
   → Get expert opinion
4. Read(file_path) for each finding
   → Verify agent's conclusions
5. Edit(...) if issues found
```

### Example 4: Parallel File Search

```
Single message with multiple Grep calls:
- Grep(pattern: "import_style_1", glob: "**/*.ts")
- Grep(pattern: "import_style_2", glob: "**/*.ts")
- Bash(cargo fmt --check)
- Bash(git log --oneline | head)
→ All run concurrently; results ready for analysis
```

---

## Relationship to Claude Code Settings

The Claude Code tool system integrates with `.claude/settings.json`:

### Permission Hooks
- **PreToolUse Hook**: Runs before Bash tool calls (ANDON gate)
  - Can block shell actions until conditions met
  - Configured via `update-config` skill

### Allowlists
- **Readonly Bash**: Common commands (git status, ls) can be allowlisted
- **Reduces**: Permission prompts for safe operations
- **Configured**: Via `fewer-permission-prompts` skill

### Environment
- **env vars**: Set via `update-config` skill
- **Paths**: Persist across Bash calls
- **Shell state**: Does NOT persist (each call is fresh)

---

## Related Documentation

- **Claude Code CLI**: `/help` in Claude Code (web or local)
- **Settings Configuration**: `update-config` skill
- **Keyboard Customization**: `keybindings-help` skill
- **API Reference**: `claude-api` skill
- **Project-Specific**: CLAUDE.md in repository

---

## Quick Reference

### Tools by Speed (fastest first)
1. Edit (small change)
2. Read (small file)
3. Glob
4. Grep
5. Write
6. Bash (simple)
7. Agent
8. Bash (complex)

### Tools by Permission Level
| No Prompt | Prompt | Requires Request |
|-----------|--------|------------------|
| Read      | Edit   | Force push       |
| Glob      | Write  | Skip hooks       |
| Grep      | Bash   | Force destructive |
| Bash (safe) |      |                  |

### Decision Tree Summary

```
Need to modify code?
├─ Small surgical change → Edit (read first)
└─ Full rewrite → Write (read if exists)

Need to find code?
├─ By filename → Glob
├─ By content → Grep
└─ Both → Glob then Grep

Need to execute commands?
├─ Simple one-liner → Bash
├─ Build/test/complex → Bash (chain with &&)
└─ Long-running → Bash (run_in_background: true)

Need expert analysis?
├─ Code review → Skill(code-review)
├─ Complex task → Agent
└─ Specific skill → Check Skills list
```

---

## Feedback & Updates

This documentation reflects Claude Code tool system as of February 2025. Tools and skills evolve; check `/help` in Claude Code for latest capabilities.

For questions about Claude Code itself (features, settings, MCP servers), use the `claude-code-guide` agent.
