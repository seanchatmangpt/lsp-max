# Skill: /keybindings-help

**Status:** AVAILABLE | **Scope:** Keyboard Configuration | **Category:** Configuration & Setup

---

## Overview

Customize keyboard shortcuts and rebind keys in Claude Code. Manages `~/.claude/keybindings.json` for chord bindings, modifier keys, and command shortcuts.

## When to Use

Use `/keybindings-help` when you want to:
- Rebind individual keys (Ctrl+S → custom behavior)
- Create chord shortcuts (Ctrl+K Ctrl+R → review)
- Change submit/cancel keys
- Setup accessibility-friendly bindings
- Match editor muscle memory from other tools

## Parameters

```bash
/keybindings-help "intent"
```

| Intent | Examples |
|--------|----------|
| **Rebind** | "rebind ctrl+s to save-and-format" |
| **Chord** | "add a chord shortcut ctrl+k ctrl+r for review" |
| **Submit** | "change the submit key to alt+enter" |
| **General** | "customize keybindings" |

## Invocation

```bash
# Interactive customization
/keybindings-help

# Specific rebinding
/keybindings-help "rebind ctrl+s"

# Chord binding
/keybindings-help "add chord ctrl+k ctrl+r for code-review"

# Change submit key
/keybindings-help "change submit key to alt+enter"
```

## Expected Output

```
⌨️  Keybinding Configuration

Current keybindings:
  Ctrl+Enter → submit
  Escape → cancel
  Ctrl+/ → toggle comment
  Ctrl+S → (not bound)

Customization:
  New binding: Ctrl+S → save-and-format
  New binding: Ctrl+K Ctrl+R → /code-review --comment
  New binding: Alt+Enter → submit

Updated: ~/.claude/keybindings.json

Status: ADMITTED
Next: Test bindings in editor
```

## Example Keybindings File

```json
{
  "bindings": [
    {
      "key": "ctrl+s",
      "command": "save-and-format"
    },
    {
      "key": "ctrl+k ctrl+r",
      "command": "code-review",
      "args": ["--comment"]
    },
    {
      "key": "alt+enter",
      "command": "submit"
    },
    {
      "key": "ctrl+shift+p",
      "command": "command-palette"
    }
  ]
}
```

## See Also

- [`/update-config`](SKILL_UPDATE_CONFIG.md) — For non-keybinding settings
- [Keybindings Reference](../CLAUDE.md) — Full keybinding documentation

---

**Last Updated:** 2026-06-14 | **Status:** AVAILABLE
